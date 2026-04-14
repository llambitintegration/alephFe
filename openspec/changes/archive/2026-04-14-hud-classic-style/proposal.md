## Why

The current HUD is a minimal 48px semi-transparent bar showing only health/shield/oxygen progress bars. This does not match the Marathon 2 aesthetic and is missing key gameplay information (weapon/ammo display, motion sensor radar). Players need weapon and radar information to play effectively, and the visual style should evoke the original game's dark metallic panel.

## What Changes

- Redesign HUD layout from a thin 48px bar to a ~128px opaque panel occupying ~15-20% of screen height
- Add three-column layout: motion sensor (left) | vitals (center) | weapon info (right)
- Add a Canvas 2D motion sensor circle rendering nearby entities as colored dots relative to player position/facing
- Restyle health/shield/oxygen bars with segmented appearance and numeric values in a retro font
- Add weapon display showing current weapon name and primary/secondary ammo counts
- Shrink the 3D viewport so it ends where the HUD begins (no overlap)
- Expose new sim data to the web layer: nearby entity positions, current weapon name, ammo counts

## Capabilities

### New Capabilities
- `hud-motion-sensor`: Canvas 2D radar display showing nearby entities as colored dots relative to player position and facing direction
- `hud-weapon-display`: Current weapon name and primary/secondary ammo count display in the HUD

### Modified Capabilities
- `hud-rendering`: Restructure HUD layout to three-column opaque panel, restyle vitals bars, shrink 3D viewport to avoid overlap

## Impact

- **marathon-web/static/index.html**: Complete HUD HTML/CSS rewrite (structure, styling, canvas element for radar)
- **marathon-web/src/render.rs**: Expand `update_hud()` to pass weapon name, ammo, and entity data to DOM; resize 3D viewport to end above HUD
- **marathon-sim/src/tick.rs**: Add public methods to expose nearby entity positions and weapon/ammo info for HUD consumption
- **marathon-sim/src/player/inventory.rs**: No structural changes, but WeaponSlot fields (primary_magazine, secondary_magazine, definition_index) will be read by new accessors
