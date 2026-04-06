import { test, expect } from '@playwright/test';

test.describe('Game startup', () => {
  test('game starts without error, loading hides, canvas visible', async ({ page }) => {
    const consoleErrors: string[] = [];
    page.on('console', (msg) => {
      if (msg.type() === 'error') {
        consoleErrors.push(msg.text());
      }
    });

    await page.goto('/');

    // Wait for loading overlay to disappear (game started)
    await expect(page.locator('#loading')).toBeHidden({ timeout: 45_000 });

    // Canvas should be visible
    const canvas = page.locator('#marathon-canvas');
    await expect(canvas).toBeVisible();

    // No "Game error" in console
    const gameErrors = consoleErrors.filter((e) => e.includes('Game error'));
    expect(gameErrors).toHaveLength(0);
  });

  test('canvas has non-zero dimensions', async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('#loading')).toBeHidden({ timeout: 45_000 });

    const canvas = page.locator('#marathon-canvas');
    const box = await canvas.boundingBox();
    expect(box).not.toBeNull();
    expect(box!.width).toBeGreaterThan(0);
    expect(box!.height).toBeGreaterThan(0);
  });
});
