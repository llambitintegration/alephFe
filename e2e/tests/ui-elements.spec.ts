import { test, expect } from '@playwright/test';

test.describe('UI elements', () => {
  test('loading screen shows MARATHON heading on initial load', async ({ page }) => {
    await page.goto('/');

    // Loading screen should be visible immediately
    const loading = page.locator('#loading');
    await expect(loading).toBeVisible();
    await expect(loading.locator('h1')).toContainText('MARATHON');
  });

  test('loading screen disappears after init', async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('#loading')).toBeVisible();
    await expect(page.locator('#loading')).toBeHidden({ timeout: 45_000 });
  });

  test('controls overlay visible with instructions', async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('#loading')).toBeHidden({ timeout: 45_000 });

    const controls = page.locator('#controls');
    await expect(controls).toBeVisible();
    await expect(controls).toContainText('WASD: Move');
  });
});
