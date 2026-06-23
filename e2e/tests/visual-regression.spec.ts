import { test, expect } from '@playwright/test';

/**
 * Sample canvas pixels from the actual composited frame.
 *
 * The marathon canvas is driven by wgpu (WebGL2 backend), whose drawing buffer
 * is NOT preserved after compositing. Reading it in-page via
 * `ctx2d.drawImage(canvas)` + `getImageData` therefore returns an all-zero
 * (false-black) buffer regardless of what is on screen. Instead we capture the
 * element with Playwright's screenshot (the real composited pixels) and decode
 * the PNG back in the browser through an `<img>` — no extra dependencies.
 *
 * Returns RGBA pixel data sampled every 4th pixel (to bound serialization),
 * preserving the {data, width, height} contract the assertions rely on.
 */
async function sampleCanvasPixels(
  canvas: import('@playwright/test').Locator,
): Promise<{ data: number[]; width: number; height: number }> {
  const shot = await canvas.screenshot({ type: 'png' });
  const dataUrl = `data:image/png;base64,${shot.toString('base64')}`;
  return canvas.page().evaluate(async (url) => {
    const img = new Image();
    await new Promise<void>((resolve, reject) => {
      img.onload = () => resolve();
      img.onerror = () => reject(new Error('failed to decode canvas screenshot'));
      img.src = url;
    });
    const tmp = document.createElement('canvas');
    tmp.width = img.naturalWidth;
    tmp.height = img.naturalHeight;
    const ctx = tmp.getContext('2d')!;
    ctx.drawImage(img, 0, 0);
    const imageData = ctx.getImageData(0, 0, tmp.width, tmp.height);
    // Sample every 4th pixel for performance.
    const sampled: number[] = [];
    for (let i = 0; i < imageData.data.length; i += 16) {
      sampled.push(
        imageData.data[i],
        imageData.data[i + 1],
        imageData.data[i + 2],
        imageData.data[i + 3],
      );
    }
    return { data: sampled, width: tmp.width, height: tmp.height };
  }, dataUrl);
}

test.describe('Visual regression baseline', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('#loading')).toBeHidden({ timeout: 45_000 });
    // Let the renderer settle for a couple seconds
    await page.waitForTimeout(2000);
  });

  test('rendered scene has sufficient non-black pixel coverage', async ({ page }) => {
    const canvas = page.locator('#marathon-canvas');
    const { data } = await sampleCanvasPixels(canvas);

    let nonBlackCount = 0;
    const totalPixels = data.length / 4;

    for (let i = 0; i < data.length; i += 4) {
      const r = data[i];
      const g = data[i + 1];
      const b = data[i + 2];
      if (r !== 0 || g !== 0 || b !== 0) {
        nonBlackCount++;
      }
    }

    const coverage = nonBlackCount / totalPixels;
    expect(coverage).toBeGreaterThan(0.2);
  });

  test('rendered scene has color variety', async ({ page }) => {
    const canvas = page.locator('#marathon-canvas');
    const { data } = await sampleCanvasPixels(canvas);

    // Quantize to 6-bit per channel (64 levels) to group similar colors
    const uniqueColors = new Set<number>();
    for (let i = 0; i < data.length; i += 4) {
      const r = data[i] >> 2;
      const g = data[i + 1] >> 2;
      const b = data[i + 2] >> 2;
      uniqueColors.add((r << 12) | (g << 6) | b);
    }

    expect(uniqueColors.size).toBeGreaterThan(50);
  });

  test('rendered scene covers multiple screen regions', async ({ page }) => {
    const canvas = page.locator('#marathon-canvas');
    const { data, width, height } = await sampleCanvasPixels(canvas);

    // Since we sampled every 4th pixel, the effective stride is different.
    // Each sampled pixel corresponds to 4 original pixels.
    // Reconstruct approximate (x, y) for each sampled pixel.
    const sampleStride = 4; // we skip 4 pixels (16 bytes / 4 bytes per pixel)
    const pixelsPerRow = width;

    const quadrantHasContent = [false, false, false, false]; // TL, TR, BL, BR
    const halfW = width / 2;
    const halfH = height / 2;

    const sampledPixelCount = data.length / 4;
    for (let si = 0; si < sampledPixelCount; si++) {
      const originalPixelIndex = si * sampleStride;
      const x = originalPixelIndex % pixelsPerRow;
      const y = Math.floor(originalPixelIndex / pixelsPerRow);

      const r = data[si * 4];
      const g = data[si * 4 + 1];
      const b = data[si * 4 + 2];

      if (r === 0 && g === 0 && b === 0) continue;

      const col = x < halfW ? 0 : 1;
      const row = y < halfH ? 0 : 1;
      quadrantHasContent[row * 2 + col] = true;
    }

    const quadrantsWithContent = quadrantHasContent.filter(Boolean).length;
    expect(quadrantsWithContent).toBeGreaterThanOrEqual(3);
  });
});

/**
 * Box 7.1 — static-scene visual-regression baseline (OpenSpec
 * fix-dynamic-geometry-rendering).
 *
 * The dynamic-geometry refactor (sections 1–3) stopped baking floor/ceiling
 * height into vertex `position.y` and light into a vertex attribute, moving
 * them into a per-polygon data texture the shader samples each frame. The
 * regression risk this box guards: that a FULLY STATIC scene (no door, no
 * action, nothing animating under our control) still renders the SAME coherent
 * Marathon image it did before the refactor — i.e. the un-baking did not blank
 * the scene, flatten its colors, or drop geometry.
 *
 * Why heuristic, not an exact-pixel reference PNG (authorized approach (b)):
 * software-GL rendering here is NOT frame-stable for a no-action scene. A probe
 * (e2e/tests/_probe-static-variance.spec.ts during development) measured, across
 * five frames captured ~0.7s apart with zero interaction, up to ~35% of pixels
 * differing beyond a generous per-channel noise threshold and a max per-channel
 * delta of 168/255 — Marathon's own idle animation (texture animation, view
 * bob) plus software-GL non-determinism. An exact-pixel baseline, even with a
 * generous tolerance or downscaling, would therefore be irreducibly flaky in
 * CI. The same probe showed the AGGREGATE per-frame statistics are tightly
 * stable: coverage 0.63–0.88, ~5300 ± 150 distinct quantized colors, and all
 * 4 screen quadrants populated, on EVERY frame.
 *
 * So this test asserts the stable, non-degenerate signature across MULTIPLE
 * independent frames (requiring every frame to pass — strictly stronger than
 * the single-frame heuristics above): substantial non-black coverage, rich
 * color variety, and full-screen geometry. A static-rendering regression from
 * the refactor (black/near-black frames → coverage collapse; monochrome/garbage
 * → color collapse; missing geometry → empty quadrants) fails this; healthy
 * static rendering passes it on every frame.
 */

interface FrameStats {
  coverage: number;
  colorCount: number;
  quadrantsWithContent: number;
}

/**
 * Capture the composited canvas (full resolution — no subsampling, so quadrant
 * coverage is exact) and reduce it to the aggregate stats the static-scene
 * assertion depends on. Same screenshot→<img>→getImageData decode path as
 * sampleCanvasPixels (the live wgpu/WebGL2 drawing buffer is not preserved, so
 * an in-page getImageData on the canvas reads false-black).
 */
async function captureFrameStats(
  canvas: import('@playwright/test').Locator,
): Promise<FrameStats> {
  const shot = await canvas.screenshot({ type: 'png' });
  const dataUrl = `data:image/png;base64,${shot.toString('base64')}`;
  return canvas.page().evaluate(async (url) => {
    const img = new Image();
    await new Promise<void>((resolve, reject) => {
      img.onload = () => resolve();
      img.onerror = () => reject(new Error('failed to decode canvas screenshot'));
      img.src = url;
    });
    const tmp = document.createElement('canvas');
    tmp.width = img.naturalWidth;
    tmp.height = img.naturalHeight;
    const ctx = tmp.getContext('2d')!;
    ctx.drawImage(img, 0, 0);
    const { data } = ctx.getImageData(0, 0, tmp.width, tmp.height);

    const width = tmp.width;
    const height = tmp.height;
    const halfW = width / 2;
    const halfH = height / 2;
    const totalPixels = data.length / 4;

    let nonBlack = 0;
    const colors = new Set<number>();
    const quad = [false, false, false, false]; // TL, TR, BL, BR

    for (let p = 0; p < totalPixels; p++) {
      const r = data[p * 4];
      const g = data[p * 4 + 1];
      const b = data[p * 4 + 2];
      if (r === 0 && g === 0 && b === 0) continue;
      nonBlack++;
      // Quantize to 6-bit/channel to group near-identical colors.
      colors.add(((r >> 2) << 12) | ((g >> 2) << 6) | (b >> 2));
      const x = p % width;
      const y = Math.floor(p / width);
      quad[(y < halfH ? 0 : 1) * 2 + (x < halfW ? 0 : 1)] = true;
    }

    return {
      coverage: nonBlack / totalPixels,
      colorCount: colors.size,
      quadrantsWithContent: quad.filter(Boolean).length,
    };
  }, dataUrl);
}

test.describe('Visual regression — static scene (box 7.1)', () => {
  test('a fully static scene renders a consistent, non-degenerate image', async ({
    page,
  }) => {
    // Loads the full WASM game and drives software-GL rendering; on CI this is
    // heavy. Give explicit headroom per the box-7.1 contract.
    test.setTimeout(150_000);

    const consoleErrors: string[] = [];
    page.on('console', (msg) => {
      if (msg.type() === 'error') consoleErrors.push(msg.text());
    });

    await page.goto('/');
    await expect(page.locator('#loading')).toBeHidden({ timeout: 45_000 });
    const canvas = page.locator('#marathon-canvas');
    await expect(canvas).toBeVisible();
    // Settle the renderer. We deliberately do NOT click/focus, press any key,
    // or call any debug hook: the scene must stay static (player at spawn, no
    // door/platform triggered) so this measures *static-scene* rendering only.
    await page.waitForTimeout(3000);

    // Thresholds chosen well inside the measured stable bands (coverage
    // 0.63–0.88, colors ~5300, quads 4/4) so normal idle animation + software-GL
    // jitter never trips them, while a real static-rendering regression does.
    const MIN_COVERAGE = 0.4; // collapse to a black/near-black frame fails
    const MIN_COLORS = 1000; // flatten to monochrome/garbage fails
    const MIN_QUADRANTS = 4; // dropped geometry / partial frame fails

    // Require the signature on EVERY one of several independent frames, so a
    // single lucky/unlucky frame can neither pass nor fail the test on its own.
    const NUM_FRAMES = 4;
    for (let i = 0; i < NUM_FRAMES; i++) {
      const stats = await captureFrameStats(canvas);
      console.log(
        `static frame ${i}: coverage=${stats.coverage.toFixed(3)} ` +
          `colors=${stats.colorCount} quads=${stats.quadrantsWithContent}`,
      );

      expect(
        stats.coverage,
        `frame ${i}: static scene coverage collapsed (renderer blanked?)`,
      ).toBeGreaterThan(MIN_COVERAGE);
      expect(
        stats.colorCount,
        `frame ${i}: static scene color variety collapsed (flat/garbage frame?)`,
      ).toBeGreaterThan(MIN_COLORS);
      expect(
        stats.quadrantsWithContent,
        `frame ${i}: static scene geometry not covering all quadrants`,
      ).toBe(MIN_QUADRANTS);

      if (i < NUM_FRAMES - 1) await page.waitForTimeout(600);
    }

    // The static scene must not be throwing game-loop errors.
    const gameErrors = consoleErrors.filter((e) => e.includes('Game error'));
    expect(gameErrors).toHaveLength(0);
  });
});
