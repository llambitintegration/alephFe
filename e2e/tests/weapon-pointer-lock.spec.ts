import { test, expect } from '@playwright/test';

/**
 * Bug 1 regression: the click that captures the mouse (acquires pointer lock)
 * must NOT fire the weapon.
 *
 * Root cause: in marathon-web the canvas `click` handler calls
 * `requestPointerLock()` while a separate `mousedown` handler set
 * `input.fire_primary = true` unconditionally. The lock-acquiring click is a
 * `mousedown`+`mouseup`+`click`, so the pistol discharged the instant the
 * player clicked to take mouse control. The fix gates fire on the canvas
 * already owning the pointer lock (`fire_allowed_while`) and clears fire on
 * pointer-lock loss.
 *
 * This test drives the REAL input path (DOM mouse events on the canvas → the
 * wasm input handlers → the sim weapon tick) and asserts ammo is unchanged
 * across a capture-style click. In an automated browser the canvas never
 * actually acquires pointer lock (the headless root document rejects it), so
 * `pointer_lock_element()` stays null — which is exactly the "not yet locked"
 * condition the lock-acquiring click hits, and under the fix fire is suppressed.
 *
 * Primary-ammo is observed via the game's `window.updateWeaponDisplay(def,
 * pri, sec)` HUD callback (no debug hook — the real render path reports it).
 */
test.describe('Weapon does not fire on pointer-lock-acquiring click', () => {
  test('a click-to-capture leaves primary ammo unchanged', async ({ page }) => {
    test.setTimeout(120_000);

    await page.goto('/');
    await expect(page.locator('#loading')).toBeHidden({ timeout: 45_000 });
    const canvas = page.locator('#marathon-canvas');
    await expect(canvas).toBeVisible();

    // Intercept the HUD weapon-display callback to record primary ammo.
    await page.evaluate(() => {
      (window as any).__ammoLog = [];
      const orig = (window as any).updateWeaponDisplay;
      (window as any).updateWeaponDisplay = function (
        def: number,
        pri: number,
        sec: number,
      ) {
        (window as any).__ammoLog.push(pri);
        if (orig) return orig.apply(this, arguments as any);
      };
    });

    // Wait for the HUD to report a baseline ammo count.
    await expect
      .poll(() => page.evaluate(() => (window as any).__ammoLog.length), {
        timeout: 10_000,
      })
      .toBeGreaterThan(0);
    const baseline = await page.evaluate(
      () => (window as any).__ammoLog.slice(-1)[0],
    );

    // Simulate a real click-to-capture: a physical click holds the button for
    // a few sim ticks before release. This is precisely the gesture that fired
    // the weapon before the fix.
    await page.evaluate(async () => {
      const c = document.getElementById('marathon-canvas')!;
      const r = c.getBoundingClientRect();
      const o = {
        bubbles: true,
        cancelable: true,
        button: 0,
        clientX: r.left + r.width / 2,
        clientY: r.top + r.height / 2,
      };
      c.dispatchEvent(new MouseEvent('mousedown', o));
      await new Promise((res) => setTimeout(res, 150)); // hold across sim ticks
      c.dispatchEvent(new MouseEvent('mouseup', o));
      c.dispatchEvent(new MouseEvent('click', o)); // triggers requestPointerLock
    });

    // Let several sim ticks + HUD updates pass.
    await page.waitForTimeout(800);
    const after = await page.evaluate(
      () => (window as any).__ammoLog.slice(-1)[0],
    );

    expect(
      after,
      `primary ammo must not drop on the capture click (was ${baseline}, now ${after})`,
    ).toBe(baseline);
  });
});
