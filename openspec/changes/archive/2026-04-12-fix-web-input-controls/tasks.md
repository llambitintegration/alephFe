## 1. Fix WASD key mappings in web input handlers
- [x] 1.1 In `marathon-web/src/render.rs` `setup_input_handlers` keydown closure, swap the match arm fields: change `"KeyW" | "ArrowUp" => st.input.backward = true` to `st.input.forward = true`, change `"KeyS" | "ArrowDown" => st.input.forward = true` to `st.input.backward = true`, change `"KeyA" => st.input.strafe_right = true` to `st.input.strafe_left = true`, change `"KeyD" => st.input.strafe_left = true` to `st.input.strafe_right = true`
- [x] 1.2 In the same file's keyup closure, apply the identical field name swaps: `"KeyW" | "ArrowUp" => st.input.forward = false`, `"KeyS" | "ArrowDown" => st.input.backward = false`, `"KeyA" => st.input.strafe_left = false`, `"KeyD" => st.input.strafe_right = false`

## 2. Fix mouse Y-axis inversion in web camera pitch
- [x] 2.1 In `marathon-web/src/render.rs` line ~206, change `self.input.mouse_dy as f32` to `-self.input.mouse_dy as f32` in the pitch calculation, matching the desktop build's negation at `marathon-game/src/render.rs:1319`

## 3. Verify the fix
- [x] 3.1 Build the marathon-web crate via Docker to confirm compilation succeeds with no errors
- [x] 3.2 Run the existing Playwright e2e test suite to confirm no regressions (the "WASD keys are accepted without error" scenario should continue to pass)
