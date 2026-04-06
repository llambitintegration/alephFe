import { test, expect } from '@playwright/test';

test.describe('WASM initialization', () => {
  test('module loads and logs initialization message', async ({ page }) => {
    const consoleLogs: string[] = [];
    const consoleErrors: string[] = [];

    page.on('console', (msg) => {
      if (msg.type() === 'log' || msg.type() === 'info') {
        consoleLogs.push(msg.text());
      }
      if (msg.type() === 'error') {
        consoleErrors.push(msg.text());
      }
    });

    await page.goto('/');
    // Wait for WASM to initialize (the log message comes from lib.rs main())
    await expect(async () => {
      expect(consoleLogs.some((log) => log.includes('Marathon Web initialized'))).toBe(true);
    }).toPass({ timeout: 30_000 });

    // No WASM compile/link errors
    const wasmErrors = consoleErrors.filter(
      (e) => e.includes('CompileError') || e.includes('LinkError'),
    );
    expect(wasmErrors).toHaveLength(0);
  });
});
