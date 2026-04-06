import { test, expect } from '@playwright/test';

test.describe('Error handling for missing data files', () => {
  test('missing Map file shows error', async ({ page }) => {
    // Intercept the Map request and return 404
    await page.route('**/data/Map.sceA', (route) =>
      route.fulfill({ status: 404, body: 'Not Found' }),
    );

    await page.goto('/');

    const errorEl = page.locator('#error');
    await expect(errorEl).toBeVisible({ timeout: 30_000 });
    await expect(errorEl).toContainText('Map');
  });

  test('missing Shapes file shows error', async ({ page }) => {
    // Intercept the Shapes request and return 404
    await page.route('**/data/Shapes.shpA', (route) =>
      route.fulfill({ status: 404, body: 'Not Found' }),
    );

    await page.goto('/');

    const errorEl = page.locator('#error');
    await expect(errorEl).toBeVisible({ timeout: 30_000 });
    await expect(errorEl).toContainText('Shapes');
  });
});
