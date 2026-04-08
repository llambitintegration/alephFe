import { test, expect } from '@playwright/test';

test.describe('WebGL2 compatibility', () => {
  test('no GPU adapter errors', async ({ page }) => {
    const consoleErrors: string[] = [];
    page.on('console', (msg) => {
      if (msg.type() === 'error') {
        consoleErrors.push(msg.text());
      }
    });

    await page.goto('/');
    await expect(page.locator('#loading')).toBeHidden({ timeout: 45_000 });

    const adapterErrors = consoleErrors.filter(
      (e) => e.includes('No available adapters') || e.includes('No GPU adapter'),
    );
    expect(adapterErrors).toHaveLength(0);
  });

  test('no INVALID_ENUM texture errors', async ({ page }) => {
    const webglErrors: string[] = [];
    page.on('console', (msg) => {
      const text = msg.text();
      if (
        text.includes('INVALID_ENUM: bindTexture') ||
        text.includes('INVALID_ENUM: texSubImage3D') ||
        text.includes('INVALID_ENUM: glTexStorage3D')
      ) {
        webglErrors.push(text);
      }
    });

    await page.goto('/');
    await expect(page.locator('#loading')).toBeHidden({ timeout: 45_000 });

    // Allow a brief settle for any deferred texture uploads
    await page.waitForTimeout(2000);

    expect(webglErrors).toHaveLength(0);
  });

  test('no texture array dimension heuristic warnings', async ({ page }) => {
    const heuristicWarnings: string[] = [];
    page.on('console', (msg) => {
      const text = msg.text();
      if (text.includes('wgpu-hal heuristics assumed')) {
        heuristicWarnings.push(text);
      }
    });

    await page.goto('/');
    await expect(page.locator('#loading')).toBeHidden({ timeout: 45_000 });
    await page.waitForTimeout(2000);

    expect(heuristicWarnings).toHaveLength(0);
  });

  test('no storage buffer errors', async ({ page }) => {
    const storageErrors: string[] = [];
    page.on('console', (msg) => {
      const text = msg.text();
      if (
        text.includes('VERTEX_STORAGE') ||
        text.includes('max_storage_buffers_per_shader_stage')
      ) {
        storageErrors.push(text);
      }
    });

    await page.goto('/');
    await expect(page.locator('#loading')).toBeHidden({ timeout: 45_000 });

    expect(storageErrors).toHaveLength(0);
  });

  test('canvas renders non-blank content', async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('#loading')).toBeHidden({ timeout: 45_000 });

    // Wait for a couple frames to render
    await page.waitForTimeout(1000);

    const canvas = page.locator('#marathon-canvas');
    const isNonBlank = await canvas.evaluate((el: HTMLCanvasElement) => {
      const ctx = el.getContext('2d') || el.getContext('webgl2');
      if (!ctx) {
        // For WebGL canvases, read pixels via a temporary 2D canvas
        const tmp = document.createElement('canvas');
        tmp.width = el.width;
        tmp.height = el.height;
        const ctx2d = tmp.getContext('2d')!;
        ctx2d.drawImage(el, 0, 0);
        const data = ctx2d.getImageData(0, 0, tmp.width, tmp.height).data;
        // Check if any non-zero non-alpha pixels exist
        for (let i = 0; i < data.length; i += 4) {
          if (data[i] !== 0 || data[i + 1] !== 0 || data[i + 2] !== 0) {
            return true;
          }
        }
        return false;
      }
      return true;
    });

    expect(isNonBlank).toBe(true);
  });
});
