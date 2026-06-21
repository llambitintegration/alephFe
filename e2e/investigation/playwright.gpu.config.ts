import { defineConfig } from '@playwright/test';

// GPU investigation Playwright config — separate from the CI config
// (../playwright.config.ts, testDir ./tests) so the probe is opt-in and never
// runs in the SwiftShader CI job. Driven by env so one config serves both the
// host-headed path (when a graphical session exists) and the GPU-passthrough
// container path:
//
//   HEADED=1            -> visible Chromium (needs an X/Wayland session)
//   WEB_PORT=8090       -> dev server port (default 8090)
//   BASE_URL=...        -> overrides WEB_PORT (the container uses host networking)
//   PW_EXTRA_ARGS=a,b   -> extra Chromium flags (e.g. Vulkan flags for container)
//   EXPECT_HARDWARE=1   -> probe fails if the renderer is software (SwiftShader)

const truthy = (v?: string) => ['1', 'true', 'yes', 'on'].includes((v || '').toLowerCase());

const headed = truthy(process.env.HEADED);
const port = process.env.WEB_PORT || '8090';
const extraArgs = (process.env.PW_EXTRA_ARGS || '')
  .split(',')
  .map((s) => s.trim())
  .filter(Boolean);

export default defineConfig({
  testDir: __dirname,
  testMatch: /gpu-probe\.spec\.ts/,
  timeout: 120_000,
  retries: 0,
  use: {
    baseURL: process.env.BASE_URL || `http://localhost:${port}`,
    headless: !headed,
    browserName: 'chromium',
    launchOptions: {
      args: ['--ignore-gpu-blocklist', '--enable-unsafe-webgpu', ...extraArgs],
    },
  },
  reporter: [['list']],
});
