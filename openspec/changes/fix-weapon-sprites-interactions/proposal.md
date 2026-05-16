## Why

The game renders visually but three critical systems are broken or incomplete: (1) first-person weapon sprites show the entire sprite instead of clipping to the viewport like the original engine, causing a "disembodied fist" effect; (2) the ACTION key (Space) is mapped but never consumed by the simulation, so doors requiring manual activation and control panels are non-functional; (3) world sprites for items and scenery use symmetric billboard centering instead of the C++ engine's asymmetric world-bound offsets, causing visual distortion.

## What Changes

- **Weapon overlay positioning**: Reimplement `position_sprite_axis` logic from the C++ engine's `render.cpp` so the weapon overlay uses `idle_height`, `vertical_position`, `vertical_positioning_mode`, and shape `world_top`/`world_bottom` to clip the weapon sprite correctly — most of the sprite should be below the viewport, with only the business end visible.
- **Action key dispatch**: Wire the `ActionFlags::ACTION` flag into the tick loop with a `find_action_key_target` ray-cast that identifies platforms (doors) and control panels in front of the player, then dispatches to the appropriate handler.
- **Platform action-key activation**: Check `PLATFORM_ACTIVATE_ON_ACTION_KEY` flag in `update_platforms` when the ACTION flag is set, in addition to the existing player-entry trigger.
- **Control panel integration**: Spawn `ControlPanel` entities from map data during level load, store them as an ECS resource, and check them in the tick loop when the ACTION flag is set.
- **Sprite billboard anchoring**: Pass `world_left`, `world_right`, `world_top`, `world_bottom` individually through `SpriteDrawCall` instead of computing symmetric width/height, so billboards respect the original asymmetric bounds and vertical anchor point.

## Capabilities

### New Capabilities
- `action-key-dispatch`: Ray-cast from player position + facing to find interaction targets (platforms, control panels), dispatch to appropriate handler on ACTION flag.

### Modified Capabilities
- `world-mechanics`: Control panel entities must be spawned from map side data at level load and checked each tick when ACTION is set. Platform activation must also check `PLATFORM_ACTIVATE_ON_ACTION_KEY`.
- `hud-rendering`: Weapon overlay must use `position_sprite_axis` positioning logic (vertical/horizontal positioning modes, idle_height offsets, shape world bounds) instead of fixed NDC placement.
- `level-rendering`: World sprite billboards must use asymmetric world bounds (`world_left`, `world_right`, `world_top`, `world_bottom`) for correct anchoring instead of symmetric centering.

## Impact

- **marathon-web/src/sprites.rs**: `WeaponOverlayRenderer::render()` rewritten to use position_sprite_axis math; `SpriteDrawCall` extended with world bounds; billboard construction updated.
- **marathon-sim/src/tick.rs**: ACTION flag consumption added; `find_action_key_target` ray-cast integrated; control panel check added to tick loop.
- **marathon-sim/src/world_mechanics/platforms.rs**: `ACTION_KEY` trigger check added alongside existing `PLAYER_ENTRY`.
- **marathon-sim/src/world_mechanics/panels.rs**: Control panel spawning from map side data; integration with tick loop.
- **marathon-web/src/render.rs**: Weapon overlay call updated to pass positioning parameters from sim weapon state.
- **marathon-formats/src/shapes.rs**: Ensure `world_left`/`world_right`/`world_top`/`world_bottom` fields are exposed on `LowLevelShape`.
