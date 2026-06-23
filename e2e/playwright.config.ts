import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './tests',
  timeout: 60_000,
  retries: 1,
  // Run serially: the door-interaction and action-key-verify specs each load
  // the full WASM game and drive GL rendering. Running two such heavy tests
  // concurrently on a CI runner thrashes CPU/GPU and ~doubles their wall time
  // (96s → 216s), blowing their per-test timeouts. One worker keeps each heavy
  // test at its solo cost. The rest of the suite is fast, so the serial cost is
  // modest.
  workers: 1,
  use: {
    baseURL: process.env.BASE_URL || 'http://web:80',
    headless: true,
    browserName: 'chromium',
    launchOptions: {
      args: ['--use-gl=angle', '--enable-unsafe-webgpu'],
    },
  },
  projects: [
    {
      name: 'chromium',
      use: { browserName: 'chromium' },
    },
  ],
  reporter: [['list'], ['html', { open: 'never' }]],
});
