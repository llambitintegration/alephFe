import { test, expect } from '@playwright/test';

test.describe('Runtime stability', () => {
  test('no console errors after game loads and runs for 5 seconds', async ({ page }) => {
    const consoleErrors: string[] = [];
    page.on('console', (msg) => {
      if (msg.type() === 'error') {
        consoleErrors.push(msg.text());
      }
    });

    await page.goto('/');

    // Wait for game to fully load
    await expect(page.locator('#loading')).toBeHidden({ timeout: 45_000 });

    // Let the game run for 5 seconds
    await page.waitForTimeout(5_000);

    // Filter out known non-critical warnings (e.g. WebGPU feature detection)
    const criticalErrors = consoleErrors.filter(
      (e) => !e.includes('Unable to retrieve adapter') && !e.includes('GPU'),
    );
    expect(criticalErrors).toHaveLength(0);
  });
});
