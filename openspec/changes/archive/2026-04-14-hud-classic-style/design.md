## Context

The current HUD is a minimal DOM overlay: a 48px semi-transparent bar with three progress bars (health, shield, oxygen). It floats over the 3D viewport. The `update_hud()` function in render.rs sets bar widths and values via DOM manipulation from WASM. The sim layer already exposes `player_health()`, `player_shield()`, `player_oxygen()`, and `player_weapon_state()` (which returns shape/animation data but not ammo counts directly). `WeaponInventory` and `WeaponSlot` in inventory.rs hold ammo fields (`primary_magazine`, `secondary_magazine`) and `definition_index`.

No entity query API currently exists for getting nearby entity positions from the sim.

## Goals / Non-Goals

**Goals:**
- Redesign HUD to a three-column opaque panel (~128px) matching Marathon 2's classic aesthetic
- Add motion sensor (Canvas 2D radar) showing nearby entities as colored dots
- Add weapon display showing name and ammo counts
- Shrink the 3D viewport so it does not overlap the HUD
- Expose weapon ammo and nearby-entity data from marathon-sim to the web layer

**Non-Goals:**
- Pixel-perfect reproduction of original Marathon 2 HUD art assets
- Inventory panel (keycards, powerups) -- deferred to a separate change
- HUD resolution scaling / responsive design beyond basic functionality
- Animated transitions or visual effects on the HUD elements
- Sprite-based weapon silhouettes (text name only for now)

## Decisions

### 1. DOM-based HUD with Canvas 2D for radar

**Decision**: Keep the HUD as HTML/CSS DOM elements. Use a dedicated `<canvas>` element only for the motion sensor radar circle.

**Rationale**: DOM elements are simple to style and update from WASM. The radar requires drawing rotated dots on a circle, which is natural for Canvas 2D but awkward with pure CSS. This avoids adding a second wgpu render pass for HUD.

**Alternative considered**: Full wgpu HUD rendering. Rejected because it adds significant complexity (text rendering, sprite atlases) for a feature that works well as DOM overlay.

### 2. Viewport resize via canvas height adjustment

**Decision**: Reduce the WebGL canvas height by the HUD height so the 3D scene ends where the HUD begins. The HUD panel sits below the canvas, not overlapping it.

**Rationale**: Avoids rendering 3D content that will be occluded. Matches the original Marathon behavior where the game viewport was a smaller rectangle above the HUD. Implementation: set canvas CSS `height: calc(100vh - 128px)` and position HUD at `bottom: 0`.

**Alternative considered**: Keep canvas full-height and just layer HUD on top. Rejected because it wastes GPU rendering behind the opaque panel and doesn't match original behavior.

### 3. New sim API methods for HUD data

**Decision**: Add three new public methods to `SimWorld` (in tick.rs):
- `player_weapon_info() -> Option<(usize, u16, u16)>` -- returns (definition_index, primary_ammo, secondary_ammo)
- `nearby_entities(range: f32) -> Vec<(f32, f32, u8)>` -- returns (relative_x, relative_z, entity_type) for entities within range

**Rationale**: These are thin accessors over existing ECS data. The weapon info reads from `WeaponInventory`. The nearby entities query iterates monsters/items with `Position` components and computes positions relative to the player. Entity type is encoded as a u8 (0=enemy, 1=ally, 2=item).

### 4. Weapon names as a static lookup table

**Decision**: Map `definition_index` to weapon name strings in the web layer (JavaScript), not in Rust/WASM.

**Rationale**: Weapon names are display-only strings. Keeping them in JS avoids passing strings across the WASM boundary. The mapping is a small static array: `["Fists", "Magnum", "Fusion Pistol", "Assault Rifle", "Missile Launcher", "Flamethrower", "Alien Weapon", "Shotgun", "SMG", "TOZT"]`.

### 5. HUD styling approach

**Decision**: Use CSS Grid for three-column layout. Retro aesthetic via monospace font, segmented bar appearance using CSS `repeating-linear-gradient`, dark opaque background (#1a1a1a).

**Rationale**: Pure CSS solution, no external assets needed. Grid provides clean column alignment. Segmented bars approximate the original look without requiring sprite assets.

## Risks / Trade-offs

- **[Performance]** Nearby-entity query runs every frame iterating all entities. Mitigation: cap results at 16 entities, only query entities with Position component, skip if no entities exist. For typical Marathon levels (< 50 active monsters), this is negligible.
- **[Radar accuracy]** Without full entity-type differentiation in the ECS, the radar may not correctly distinguish allies from enemies in all cases. Mitigation: initially treat all monsters as enemies (red dots); refine later when monster allegiance tracking is added.
- **[Canvas sizing]** Reducing canvas height changes the aspect ratio passed to the projection matrix. Mitigation: the existing resize handler in render.rs already reads canvas dimensions dynamically, so the projection will adapt automatically.
