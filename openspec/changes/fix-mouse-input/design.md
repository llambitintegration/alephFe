## Context

The web build converts mouse movement into binary ActionFlags (TURN_LEFT/TURN_RIGHT, LOOK_UP/LOOK_DOWN) using a threshold check (`|mouse_dx| > 2.0`). The sim then applies a fixed angular acceleration per tick when flags are set. This produces jerky movement because:

1. Mouse speed is quantized — slow and fast movements produce identical turn rates
2. Vertical look has no angular momentum — immediate ±0.05 per tick on/off
3. The 30Hz tick rate compounds the quantization (only 30 decision points per second)

Original Marathon used keyboard/joystick where binary flags are appropriate. Mouse look requires proportional input.

Current pipeline:
```
mousemove → accumulate dx/dy → threshold → binary flag → sim angular velocity
```

Target pipeline:
```
mousemove → accumulate dx/dy → pass as float → sim applies directly to facing
```

## Goals / Non-Goals

**Goals:**
- Smooth, proportional mouse look that maps mouse movement speed to turn speed
- Consistent feel across frame rates (delta-time independent at the sim tick level)
- Keyboard turning still works via existing ActionFlags angular velocity system

**Non-Goals:**
- Mouse acceleration curves or advanced input processing
- Configurable sensitivity UI (hardcoded constant is fine for now)
- Gamepad stick input (not applicable to web build)
- Changing the 30Hz tick rate

## Decisions

### Decision 1: Extend TickInput with mouse delta fields

**Choice:** Add `mouse_yaw: f32` and `mouse_pitch: f32` fields to the existing `TickInput` resource.

**Alternative considered:** New `MouseDelta` component or resource — rejected because `TickInput` already carries per-tick input and adding fields is simpler than a new resource. The sim already reads `TickInput` in `run_player_physics()`.

**Rationale:** Minimal change surface. ActionFlags remain for keyboard. Mouse deltas are additive — when both are present, mouse delta is applied first, then ActionFlags angular velocity is layered on top.

### Decision 2: Apply mouse delta directly to facing angle (bypass angular velocity)

**Choice:** When `mouse_yaw != 0.0`, add it directly to `facing` rather than going through the angular acceleration/deceleration system. Similarly for `mouse_pitch` → `vertical_look`.

**Alternative considered:** Feed mouse delta into the angular velocity system as a velocity impulse — rejected because the acceleration/deceleration curve adds lag that feels wrong for mouse input. Mouse users expect 1:1 mapping.

**Rationale:** Original Marathon's angular velocity system was designed for keyboard repeat-rate input. Mouse look should be direct. The two systems compose: if the user holds a keyboard turn key while moving the mouse, both contribute to facing changes.

### Decision 3: Sensitivity scaling in the web layer, not the sim

**Choice:** The web layer scales raw browser pixel deltas by a sensitivity constant and converts to radians before passing to the sim. The sim receives radians and applies them directly.

**Rationale:** Sensitivity is a platform concern — different input devices need different scaling. The sim should receive normalized radians regardless of input source. This also means marathon-game (native) can use its own sensitivity scaling for winit mouse events.

## Risks / Trade-offs

- [Sensitivity tuning] The chosen constant may feel wrong on different DPI mice → Mitigation: pick a reasonable default (0.003 rad/pixel is common for FPS games), adjust based on testing. Future work can add a settings slider.
- [Tick-rate coupling] Mouse deltas are accumulated between ticks, so at 30Hz with fast mouse movement, each tick applies a large delta → Mitigation: This is acceptable; 30Hz is the sim rate and matches original Marathon. The delta is still proportional.
- [Keyboard+mouse interaction] Both systems modify facing in the same tick → Mitigation: Apply mouse delta first, then angular velocity. They compose naturally.
