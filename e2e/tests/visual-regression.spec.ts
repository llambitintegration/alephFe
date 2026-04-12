import { test, expect } from '@playwright/test';

/**
 * Sample canvas pixels by drawing the WebGL canvas onto a temporary 2D canvas
 * and reading the ImageData. Returns RGBA pixel data.
 */
async function sampleCanvasPixels(
  canvas: import('@playwright/test').Locator,
): Promise<{ data: number[]; width: number; height: number }> {
  return canvas.evaluate((el: HTMLCanvasElement) => {
    const tmp = document.createElement('canvas');
    tmp.width = el.width;
    tmp.height = el.height;
    const ctx = tmp.getContext('2d')!;
    ctx.drawImage(el, 0, 0);
    const imageData = ctx.getImageData(0, 0, tmp.width, tmp.height);
    // Transfer a sampled subset to avoid serializing millions of bytes.
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
  });
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
