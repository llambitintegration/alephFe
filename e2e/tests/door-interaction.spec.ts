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
 * UNBLOCKED: the capture/diff machinery and interaction sequence were always
 * verified working against the proxy-net stack (movement provably changes the
 * composited frame, screenshots capture real pixels). What was missing was a
 * DETERMINISTIC way to reach an action-activatable door: in marathon-sim, the
 * action key only activates a platform when the player is within a control
 * panel's `max_distance` and facing its line within ~60°
 * (marathon-sim/src/world_mechanics/panels.rs::can_activate_panel), and from
 * the real Marathon 2 spawn point blind navigation never landed there.
 *
 * The marathon-web build now exposes a debug hook,
 * `window.__marathonDebug.faceNearestDoor()` (wasm-bindgen export
 * `debug_face_nearest_door` → `SimWorld::debug_face_nearest_door`), which
 * repositions and re-faces the player directly in front of the nearest
 * activatable door so the action-key raycast hits it. This test calls that
 * hook, then presses Space and asserts the door-region pixels change.
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
  test('pressing the action key at a door changes the door-region pixels', async ({
    page,
  }) => {
    // This test loads the full WASM game, waits for first render, repositions
    // via the debug hook, then animates a door — heavy enough that CI's
    // software-GL environment runs it in ~95s (vs ~47s locally), exceeding the
    // 60s suite default. Give it explicit headroom; the waits below are fixed,
    // so the cost is CI render speed, not flakiness.
    test.setTimeout(150_000);

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

    // Deterministically face the nearest activatable door via the debug hook
    // shipped in the marathon-web build (wasm-bindgen export
    // debug_face_nearest_door → SimWorld::debug_face_nearest_door). This
    // repositions + re-faces the player so the action-key raycast lands on a
    // door, which blind keyboard nav from the Marathon 2 spawn never achieves.
    await expect
      .poll(
        () =>
          page.evaluate(
            () =>
              typeof (window as any).__marathonDebug?.faceNearestDoor ===
              'function',
          ),
        { timeout: 10_000 },
      )
      .toBe(true);

    const faced = await page.evaluate(() =>
      (window as any).__marathonDebug.faceNearestDoor(),
    );
    expect(faced, 'faceNearestDoor() should find and face a door').toBe(true);

    // Let the new camera pose settle and render a couple of frames.
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
    // Surface the magnitude for the run log / diagnostics.
    console.log(`door-region changed fraction: ${changedFraction.toFixed(4)}`);

    // The animating door must visibly alter the central region. A static
    // scene (the failure mode this change fixes) leaves this at ~0.
    expect(changedFraction).toBeGreaterThan(0.02);

    // The interaction must not have crashed the game loop.
    await expect(canvas).toBeVisible();
    const gameErrors = consoleErrors.filter((e) => e.includes('Game error'));
    expect(gameErrors).toHaveLength(0);
  });
});
