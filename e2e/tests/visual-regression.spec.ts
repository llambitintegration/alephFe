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
