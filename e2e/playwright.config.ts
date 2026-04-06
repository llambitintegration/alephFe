import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './tests',
  timeout: 60_000,
  retries: 1,
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
