## Context

The marathon-web build has two input mapping bugs that make browser gameplay unplayable. WASD keys are wired to the wrong movement directions (W/S swapped, A/D swapped), and the mouse Y-axis is inverted relative to the desktop build. The desktop build in marathon-game has the correct mappings. These are straightforward wiring errors in `marathon-web/src/render.rs`, not architectural issues.

## Goals / Non-Goals

**Goals:**
- Fix WASD key mappings in web keydown/keyup handlers so W=forward, S=backward, A=strafe_left, D=strafe_right
- Fix mouse pitch sign convention so moving the mouse up pitches the camera up, matching the desktop build's negation of `mouse_dy`

**Non-Goals:**
- Refactoring the input system architecture or adding configurable bindings to the web build
- Changing any behavior in marathon-game, marathon-sim, or any other crate
- Adjusting mouse sensitivity values or adding sensitivity configuration to the web build

## Decisions

1. **Swap field names in match arms, not key codes.** The keydown and keyup handlers each have four WASD match arms where the field being set is simply wrong. The fix is to swap the field names (`forward`/`backward` and `strafe_left`/`strafe_right`) so they match the key being pressed. The key code matching (`"KeyW"`, `"KeyS"`, etc.) is already correct.

2. **Negate `mouse_dy` in camera pitch calculation.** The desktop build at `marathon-game/src/render.rs:1318-1319` applies `-self.input.mouse_dy` when computing pitch, following the standard convention that negative screen-Y delta (mouse moved up) should produce positive pitch (look up). The web build at `marathon-web/src/render.rs:206` omits this negation. The fix is to negate `mouse_dy` in the web build's pitch line to match.

## Risks / Trade-offs

- **Risk: Regression in keyup handler.** The keyup handler has the same swapped fields as keydown. Both must be fixed in lockstep or keys will get stuck. This is mitigated by the fix being a simple mechanical swap in both handlers.
- **Trade-off: No sensitivity multiplier added to web build.** The desktop build applies `MOUSE_SENSITIVITY` to the pitch delta; the web build currently does not. This change only fixes the sign inversion and does not add the sensitivity multiplier, keeping scope minimal. Sensitivity tuning is a separate concern.
