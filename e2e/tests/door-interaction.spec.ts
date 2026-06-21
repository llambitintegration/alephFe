import { test, expect } from '@playwright/test';

/**
 * Door interaction e2e (OpenSpec fix-dynamic-geometry-rendering box 7.2).
 *
 * Goal: walk to a door, press the action key (Space), screenshot the door
 * region before/after, and assert the door-region pixels change — exercising
 * the full dynamic-geometry pipeline end-to-end (edge-triggered action key in
 * the sim, per-polygon data-texture upload each frame, shader height/light
 * offset) against the proxy-net container.
 *
 * STATUS: marked `test.fixme` — see the block comment on the test below.
 * The capture/diff machinery and interaction sequence are verified working
 * against the proxy-net stack (movement provably changes the composited
 * frame, screenshots capture real pixels). What is NOT yet satisfiable in the
 * headless e2e environment is DETERMINISTICALLY reaching an
 * action-activatable door: in marathon-sim, the action key only activates a
 * platform when the player is within a control panel's `max_distance` and
 * facing its line within ~60° (marathon-sim/src/world_mechanics/panels.rs::
 * can_activate_panel). From the real Marathon 2 spawn point, blind navigation
 * (forward walks + yaw sweeps across multiple rings) never lands on such a
 * panel — a 12-probe sweep produced 0.0000 door-region change at every probe —
 * and the WASM exposes no debug hook to position the player at a known door.
 * Flipping box 7.2 would therefore require either a player-teleport/debug API
 * (`window.__marathonDebug.faceNearestDoor()` or similar) or a level fixture
 * with a door panel adjacent to spawn. Until one exists, this test stays
 * `fixme` rather than falsely green.
 */

interface SampledFrame {
  data: number[];
  width: number;
  height: number;
}

/**
 * Capture the composited canvas as full-resolution RGBA pixel data.
 *
 * The marathon canvas is driven by wgpu (WebGL2 backend), whose drawing buffer
 * is NOT preserved after compositing, so an in-page `getImageData` on the live
 * canvas returns false-black. We capture composited pixels via Playwright's
 * element screenshot and decode the PNG back in the browser through an `<img>`
 * — the same approach as visual-regression.spec.ts, no extra dependencies.
 */
async function captureFrame(
  canvas: import('@playwright/test').Locator,
): Promise<SampledFrame> {
  const shot = await canvas.screenshot({ type: 'png' });
  const dataUrl = `data:image/png;base64,${shot.toString('base64')}`;
  return canvas.page().evaluate(async (url) => {
    const img = new Image();
    await new Promise<void>((resolve, reject) => {
      img.onload = () => resolve();
      img.onerror = () => reject(new Error('failed to decode canvas screenshot'));
      img.src = url;
    });
    const tmp = document.createElement('canvas');
    tmp.width = img.naturalWidth;
    tmp.height = img.naturalHeight;
    const ctx = tmp.getContext('2d')!;
    ctx.drawImage(img, 0, 0);
    const imageData = ctx.getImageData(0, 0, tmp.width, tmp.height);
    return {
      data: Array.from(imageData.data),
      width: tmp.width,
      height: tmp.height,
    };
  }, dataUrl);
}

/**
 * Fraction of pixels in a centered rectangular region (the forward viewport
 * where a faced door renders) that differ between two frames beyond a
 * per-channel noise threshold.
 */
function changedFractionInDoorRegion(
  before: SampledFrame,
  after: SampledFrame,
  channelNoise = 16,
): number {
  expect(before.width).toBe(after.width);
  expect(before.height).toBe(after.height);

  const { width, height } = before;
  const x0 = Math.floor(width * 0.25);
  const x1 = Math.floor(width * 0.75);
  const y0 = Math.floor(height * 0.3);
  const y1 = Math.floor(height * 0.7);

  let changed = 0;
  let total = 0;
  for (let y = y0; y < y1; y++) {
    for (let x = x0; x < x1; x++) {
      const i = (y * width + x) * 4;
      total++;
      const dr = Math.abs(before.data[i] - after.data[i]);
      const dg = Math.abs(before.data[i + 1] - after.data[i + 1]);
      const db = Math.abs(before.data[i + 2] - after.data[i + 2]);
      if (dr > channelNoise || dg > channelNoise || db > channelNoise) {
        changed++;
      }
    }
  }
  return total === 0 ? 0 : changed / total;
}

test.describe('Door interaction (dynamic geometry)', () => {
  // See the file-level STATUS note. Verified against the proxy-net stack as far
  // as movement/screenshot/diff; gated only on a deterministic way to face an
  // action-activatable door from spawn.
  test.fixme(
    'pressing the action key at a door changes the door-region pixels',
    async ({ page }) => {
      const consoleErrors: string[] = [];
      page.on('console', (msg) => {
        if (msg.type() === 'error') consoleErrors.push(msg.text());
      });

      await page.goto('/');
      await expect(page.locator('#loading')).toBeHidden({ timeout: 45_000 });
      const canvas = page.locator('#marathon-canvas');
      await expect(canvas).toBeVisible();
      await page.waitForTimeout(2000);

      // Focus the canvas (the game's key listeners live on it) the same way a
      // player does — click to capture the mouse.
      await canvas.click();
      await page.waitForTimeout(200);

      // Walk forward to the door ahead of the spawn point.
      // NOTE: once a deterministic door-facing hook exists, replace this blind
      // walk with a call that positions the player at a known door panel.
      await page.keyboard.down('w');
      await page.waitForTimeout(1500);
      await page.keyboard.up('w');
      await page.waitForTimeout(500);

      // Capture the door region BEFORE triggering the action.
      const before = await captureFrame(canvas);

      // Edge-triggered action key: a clean press-release activates the
      // platform exactly once (sim box 6.x).
      await page.keyboard.down('Space');
      await page.waitForTimeout(150);
      await page.keyboard.up('Space');

      // Let the door animate; the per-polygon data texture re-uploads each
      // frame so the door rises/lowers without rebuilding vertex buffers.
      await page.waitForTimeout(1500);

      // Capture the door region AFTER the door has had time to move.
      const after = await captureFrame(canvas);

      const changedFraction = changedFractionInDoorRegion(before, after);

      // The animating door must visibly alter the central region. A static
      // scene (the failure mode this change fixes) leaves this at ~0.
      expect(changedFraction).toBeGreaterThan(0.02);

      // The interaction must not have crashed the game loop.
      await expect(canvas).toBeVisible();
      const gameErrors = consoleErrors.filter((e) => e.includes('Game error'));
      expect(gameErrors).toHaveLength(0);
    },
  );
});
