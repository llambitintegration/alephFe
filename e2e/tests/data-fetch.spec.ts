import { test, expect } from '@playwright/test';

test.describe('Game data file serving', () => {
  const dataFiles = [
    { path: '/data/Map.sceA', label: 'Map' },
    { path: '/data/Shapes.shpA', label: 'Shapes' },
    { path: '/data/Physics.phyA', label: 'Physics' },
  ];

  for (const { path, label } of dataFiles) {
    test(`${label} data file serves successfully`, async ({ page }) => {
      const response = await page.request.get(path);
      expect(response.status()).toBe(200);
      const body = await response.body();
      expect(body.length).toBeGreaterThan(0);
    });
  }
});
