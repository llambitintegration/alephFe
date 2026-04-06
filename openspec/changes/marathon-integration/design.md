## Context

The `marathon-formats`, `marathon-audio`, `marathon-viewer` (planned), and `marathon-sim` (planned) crates provide data parsing, audio, rendering, and simulation respectively. Currently there is no crate that wires these together into a runnable game. The original C++ Aleph One engine scatters integration across `shell.cpp`, `Misc/` (~87K LOC), and `RenderOther/` (~22K LOC). This design defines how a new `marathon-integration` crate consolidates these concerns in Rust with clean module boundaries.

The workspace currently contains `marathon-formats` (parsing) and `marathon-audio` (spatial sound, ambient, music via kira). The crate uses `bevy_ecs` for scheduling, `winit` for windowing/input, and `wgpu` for rendering (shared with the future `marathon-viewer`).

## Goals / Non-Goals

**Goals:**
- Define the architecture of `marathon-integration` as the top-level orchestration crate
- Establish how input flows from winit events to marathon-sim action flags
- Design the game state machine and level lifecycle
- Define HUD, menu, and terminal rendering as wgpu 2D overlay passes
- Support save/load, film recording/playback, and all Marathon game modes
- Keep all rendering GPU-side via wgpu (no immediate-mode UI framework)

**Non-Goals:**
- Implementing marathon-viewer or marathon-sim (those are separate crates/changes)
- Networking for multiplayer (this change defines game modes and rules, not netcode)
- Lua scripting support (Aleph One extension, not original Marathon)
- Map editor or content creation tools
- Plugin/mod loading beyond what marathon-formats already supports

## Decisions

### D1: bevy_ecs for scheduling, not full Bevy engine

**Choice**: Use `bevy_ecs` as a standalone crate for system scheduling and component storage, not the full `bevy` engine with its renderer/windowing.

**Rationale**: Marathon's rendering is highly specialized (2.5D BSP, transparent surfaces, teleporter effects) and doesn't map well to Bevy's PBR pipeline. Using `bevy_ecs` gives us a powerful scheduling framework (system ordering, run conditions, states) without fighting a renderer designed for different assumptions. We keep `winit` for windowing and `wgpu` directly for rendering.

**Alternatives considered**:
- Full Bevy engine: Too much impedance mismatch with Marathon's rendering model
- No ECS, manual game loop: Workable but loses system scheduling, parallel dispatch, and clean state management
- specs/legion: Less maintained, smaller ecosystem than bevy_ecs

### D2: State machine via bevy_ecs States

**Choice**: Model the game state machine using `bevy_ecs::prelude::States` with run conditions that gate system execution per state.

**States**: `Loading`, `MainMenu`, `Playing`, `Paused`, `Terminal`, `Intermission`, `GameOver`

**Rationale**: bevy_ecs States provide built-in enter/exit hooks and system run conditions, so each subsystem naturally activates only in its relevant states. For example, input-to-action-flag translation runs only in `Playing`, terminal scrolling runs only in `Terminal`, and menu navigation runs only in `MainMenu`/`Paused`.

**Alternatives considered**:
- Manual enum + match: More boilerplate, no automatic system gating
- Statechart library: Overkill; Marathon's states are a simple flat FSM with known transitions

### D3: Input layering via context-dependent action maps

**Choice**: Define separate input contexts (Gameplay, Menu, Terminal) each with their own key binding map. The active context is derived from the current game state. Raw winit events are translated to context-specific actions, not directly to marathon-sim action flags.

**Rationale**: Marathon uses the same physical keys for different purposes depending on context (e.g., arrow keys navigate menus vs. move the player). A context-based approach cleanly separates these without complex conditional logic in a single handler.

### D4: HUD rendering as a separate wgpu render pass

**Choice**: HUD elements (health bar, oxygen, motion sensor, weapon display, inventory) are rendered as a 2D wgpu render pass that composites on top of the 3D scene framebuffer. HUD assets come from marathon-formats `ShapesFile` collections (interface collection 0).

**Rationale**: The HUD is purely 2D overlay content. A separate render pass keeps it decoupled from the 3D scene pipeline, simplifies resolution independence, and matches how the original engine treats HUD rendering as a post-step.

### D5: Terminal as a modal state with its own renderer

**Choice**: When a player activates a terminal, the game transitions to the `Terminal` state. The terminal renderer takes over the display area (partially or fully), rendering styled text pages from `marathon-formats` terminal data. Gameplay simulation pauses.

**Rationale**: Marathon terminals are modal -- they fully capture input and display a separate interface. Modeling this as a state transition rather than an overlay simplifies input routing and ensures the simulation doesn't advance while reading.

### D6: Film recording captures action flags per tick

**Choice**: Record the sequence of `ActionFlags` values fed to `marathon-sim` each tick, along with the starting random seed and level index. Playback replays these flags through the same simulation code.

**Rationale**: This matches the original Marathon film system exactly. Since marathon-sim is deterministic given the same inputs and seed, recording just the inputs produces compact files and guaranteed-accurate replays.

### D7: Save/load via serde serialization of simulation state

**Choice**: Save game state by serializing the full `marathon-sim` game state (player, monsters, map modifications, item pickups, terminal read status) via serde. Save slots are stored as files in a user data directory.

**Rationale**: Serde serialization is the idiomatic Rust approach and can target multiple formats (bincode for compactness, JSON for debugging). Serializing full state avoids the fragility of checkpoint-based save systems.

### D8: Module structure within the crate

```
marathon-integration/
  src/
    lib.rs
    input/
      mod.rs          # Input context management
      bindings.rs     # Key/button binding configuration
      action_flags.rs # Conversion to marathon-sim action flags
    hud/
      mod.rs          # HUD orchestration
      health.rs       # Health/shield/oxygen bars
      motion_sensor.rs # Radar display
      weapon.rs       # Weapon/ammo display
      inventory.rs    # Inventory panel
    menu/
      mod.rs          # Menu state management
      main_menu.rs    # Main menu screen
      pause.rs        # Pause menu
      preferences.rs  # Settings screens
    terminal/
      mod.rs          # Terminal state and renderer
      pages.rs        # Page layout and text rendering
    shell/
      mod.rs          # Game shell / main loop
      states.rs       # State machine definitions
      level.rs        # Level loading and transitions
      save.rs         # Save/load system
      film.rs         # Film recording/playback
    modes/
      mod.rs          # Game mode trait and selection
      campaign.rs     # Single-player campaign
      cooperative.rs  # Co-op mode
      deathmatch.rs   # Deathmatch variants
    sprites/
      mod.rs          # Entity state -> sprite rendering bridge
```

## Risks / Trade-offs

**[bevy_ecs version coupling]** bevy_ecs follows Bevy's rapid release cycle; API churn between versions could require migration effort.
  - Mitigation: Pin to a specific bevy_ecs version. The crate uses only core ECS features (World, Systems, States, Resources) which are stable across releases.

**[marathon-sim and marathon-viewer don't exist yet]** This crate depends on two crates that haven't been built. Design decisions may need revision once those APIs solidify.
  - Mitigation: Define clear interface boundaries (traits/structs) that marathon-integration expects. Build against those interfaces, implementing stubs initially. The proposal explicitly states no changes to existing crates are needed.

**[wgpu HUD rendering complexity]** Custom 2D rendering with wgpu is more work than using an immediate-mode GUI library (egui, iced).
  - Mitigation: Marathon's HUD is fixed-layout bitmap rendering, not general UI. The HUD draws pre-authored sprite frames from ShapesFile at fixed screen positions. This is simpler than general 2D rendering and avoids pulling in a UI framework dependency.

**[Terminal text rendering]** Rendering styled, scrollable text with wgpu requires either a text rendering library or custom glyph atlas implementation.
  - Mitigation: Use `glyphon` (wgpu-native text rendering library) for terminal text. Menus can use the same approach. This is a single focused dependency rather than a full UI framework.

**[Game mode correctness]** Marathon has many subtle game mode rules (KOTH timing, ball possession scoring, tag logic). Getting these exactly right requires careful reference to the original source.
  - Mitigation: Each game mode is its own module with independent tests. Rules are specified in the specs and validated against Aleph One's behavior.
