## MODIFIED Requirements

### Requirement: Composite HUD as 2D overlay on 3D scene with Lua HUD support
The system SHALL render the HUD as a wgpu render pass that writes to the same framebuffer as the 3D scene, composited on top. The HUD render pass SHALL execute after the 3D scene render pass completes. HUD elements SHALL support alpha transparency. When a Lua HUD script is loaded, the system SHALL execute the Lua HUD `draw()` callback during the HUD render pass and render the resulting `HudDrawCommand` buffer after (or instead of, depending on script behavior) the built-in HUD elements. The Lua HUD draw commands SHALL be rendered using the same wgpu render pass as the built-in HUD.

#### Scenario: HUD over gameplay without Lua
- **WHEN** a frame is rendered during the Playing state with no HUD script loaded
- **THEN** the 3D scene SHALL render first, followed by the built-in HUD overlay pass (health bars, motion sensor, etc.)

#### Scenario: HUD over gameplay with Lua
- **WHEN** a frame is rendered during the Playing state with a HUD script loaded
- **THEN** the 3D scene SHALL render first, then the built-in HUD elements, then the Lua HUD draw commands SHALL be rendered on top

#### Scenario: Lua HUD draw commands rendered
- **WHEN** the HUD script's `draw()` produces a buffer of 10 `HudDrawCommand` entries
- **THEN** the renderer SHALL translate each command (FillRect, DrawText, DrawShape, etc.) to wgpu draw calls within the HUD overlay render pass

#### Scenario: Transparent HUD regions
- **WHEN** a HUD element (built-in or Lua-drawn) has transparent pixels
- **THEN** the 3D scene SHALL be visible through those transparent regions

### Requirement: Render Lua HUD draw commands
The system SHALL translate `HudDrawCommand` variants to wgpu draw calls within the HUD render pass. `FillRect` SHALL render a filled quad with the specified color and alpha. `FrameRect` SHALL render four line quads forming a rectangle outline. `DrawText` SHALL render text glyphs using the specified font (looked up from Shapes interface font data or a fallback font atlas). `DrawShape` SHALL render a Shapes bitmap sprite at the specified screen coordinates. `SetClipRect` SHALL configure the scissor rect for subsequent draws. `ClearClipRect` SHALL reset the scissor rect to the full viewport.

#### Scenario: Render filled rectangle
- **WHEN** the draw command buffer contains `HudDrawCommand::FillRect { x: 10, y: 20, w: 100, h: 30, color: (1.0, 0.0, 0.0, 0.8) }`
- **THEN** the renderer SHALL draw a red semi-transparent filled rectangle at screen position (10, 20) with size 100x30

#### Scenario: Render text
- **WHEN** the draw command buffer contains `HudDrawCommand::DrawText { text: "HP: 100", x: 50, y: 400, font: Interface, color: (0, 1, 0, 1) }`
- **THEN** the renderer SHALL render the text string using the interface font at the specified position in green

#### Scenario: Render shape sprite
- **WHEN** the draw command buffer contains `HudDrawCommand::DrawShape { descriptor: 0x1234, x: 200, y: 350 }`
- **THEN** the renderer SHALL look up the shape bitmap from Shapes data and render it at the specified screen coordinates

#### Scenario: Scissor clipping
- **WHEN** the draw command buffer contains `SetClipRect(0, 0, 320, 240)` followed by draw commands
- **THEN** the renderer SHALL set the scissor rect to (0, 0, 320, 240) so draws outside this region are clipped

### Requirement: HUD rendering reads Lua draw buffer from LuaScriptEngine
The HUD render path SHALL obtain the Lua draw command buffer by calling `LuaScriptEngine::hud_draw_commands()` after the HUD `draw()` callback has executed. The returned `Vec<HudDrawCommand>` SHALL be consumed by the renderer. If no HUD script is loaded, the method SHALL return an empty vector.

#### Scenario: Obtain draw commands
- **WHEN** the HUD render path calls `engine.hud_draw_commands()` after calling `engine.dispatch_hud_draw()`
- **THEN** the system SHALL return the commands produced by the last `draw()` call

#### Scenario: No HUD script returns empty
- **WHEN** `engine.hud_draw_commands()` is called with no HUD script loaded
- **THEN** the system SHALL return an empty `Vec<HudDrawCommand>`
