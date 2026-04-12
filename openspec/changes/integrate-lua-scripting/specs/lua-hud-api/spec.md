## ADDED Requirements

### Requirement: HUD script draw callback
The system SHALL call the `draw()` function in the HUD script VM once per render frame when a HUD script is loaded. The draw call SHALL occur after the 3D scene render pass and before frame presentation. The HUD script SHALL have access to current game state (player health, shield, oxygen, weapon, position, etc.) through read-only UserData types.

#### Scenario: Draw called every frame
- **WHEN** the game is in `Playing` state with a HUD script loaded and the display runs at 60 Hz
- **THEN** the system SHALL call `draw()` in the HUD VM 60 times per second

#### Scenario: No HUD script loaded
- **WHEN** no HUD script is loaded
- **THEN** the system SHALL skip the HUD Lua draw call with negligible overhead, and the built-in HUD SHALL render normally

#### Scenario: HUD script error does not crash
- **WHEN** the HUD `draw()` function raises a Lua runtime error
- **THEN** the system SHALL log the error and continue rendering subsequent frames without the HUD script (or retry next frame)

### Requirement: Screen drawing API - fill_rect
The `Screen` global object SHALL provide a `fill_rect(x, y, w, h, color)` method that records a filled rectangle draw command. The `x`, `y` parameters specify the top-left corner in screen coordinates. The `w`, `h` parameters specify width and height in pixels. The `color` parameter SHALL accept a color table `{r, g, b, a}` with values 0.0-1.0, or a named color constant.

#### Scenario: Draw filled rectangle
- **WHEN** a HUD script calls `Screen.fill_rect(10, 20, 100, 30, {r=1, g=0, b=0, a=0.8})`
- **THEN** the system SHALL record a `HudDrawCommand::FillRect` with position (10, 20), size (100, 30), and color RGBA (1.0, 0.0, 0.0, 0.8)

### Requirement: Screen drawing API - frame_rect
The `Screen` global object SHALL provide a `frame_rect(x, y, w, h, color, width)` method that records a rectangle outline draw command. The `width` parameter specifies the line width in pixels (default 1).

#### Scenario: Draw rectangle outline
- **WHEN** a HUD script calls `Screen.frame_rect(10, 20, 100, 30, {r=1, g=1, b=1, a=1}, 2)`
- **THEN** the system SHALL record a `HudDrawCommand::FrameRect` with a 2-pixel white border

### Requirement: Screen drawing API - draw_text
The `Screen` global object SHALL provide a `draw_text(text, x, y, font, color, style)` method that records a text draw command. The `font` parameter SHALL accept font constants: `Screen.fonts.interface`, `Screen.fonts.computer`, `Screen.fonts.computer_large`, `Screen.fonts.title`. The `style` parameter SHALL accept a bitmask of bold, italic, underline flags.

#### Scenario: Draw HUD text
- **WHEN** a HUD script calls `Screen.draw_text("HP: 100", 50, 400, Screen.fonts.interface, {r=0, g=1, b=0, a=1})`
- **THEN** the system SHALL record a `HudDrawCommand::DrawText` with the specified text, position, font, and color

#### Scenario: Default font and color
- **WHEN** a HUD script calls `Screen.draw_text("Hello", 10, 10)` with no font or color
- **THEN** the system SHALL use the default interface font and white color

### Requirement: Screen drawing API - draw_shape
The `Screen` global object SHALL provide a `draw_shape(shape_descriptor, x, y)` method that records a sprite draw command. The `shape_descriptor` SHALL reference a Shapes collection/texture/clut combination. The sprite SHALL be drawn at the specified screen coordinates.

#### Scenario: Draw HUD icon
- **WHEN** a HUD script calls `Screen.draw_shape(shape_desc, 200, 350)`
- **THEN** the system SHALL record a `HudDrawCommand::DrawShape` with the shape descriptor and position

### Requirement: Screen drawing API - world_to_screen
The `Screen` global object SHALL provide a `world_to_screen(x, y, z)` method that projects a 3D world coordinate to 2D screen coordinates. The method SHALL return `screen_x, screen_y, visible` where `visible` is a boolean indicating whether the point is in front of the camera and within the viewport. World coordinates SHALL be in Marathon world units (matching the Lua coordinate convention).

#### Scenario: Project visible point
- **WHEN** a HUD script calls `local sx, sy, vis = Screen.world_to_screen(1024, 2048, 512)` and the point is visible
- **THEN** `sx` and `sy` SHALL be the screen pixel coordinates, and `vis` SHALL be `true`

#### Scenario: Project behind-camera point
- **WHEN** a HUD script calls `Screen.world_to_screen(x, y, z)` for a point behind the camera
- **THEN** `vis` SHALL be `false`

### Requirement: Screen drawing API - clip_rect and unclip_rect
The `Screen` global object SHALL provide `clip_rect(x, y, w, h)` to set a clipping rectangle and `unclip_rect()` to remove the clipping rectangle. All subsequent draw commands after `clip_rect` SHALL be clipped to the specified region until `unclip_rect` is called.

#### Scenario: Clipped drawing
- **WHEN** a HUD script calls `Screen.clip_rect(0, 0, 320, 240)` then draws a rectangle at (300, 230, 100, 100)
- **THEN** only the portion of the rectangle within the (0, 0, 320, 240) region SHALL be visible

### Requirement: Screen dimension queries
The `Screen` global object SHALL provide `width()` and `height()` methods that return the current display resolution in pixels.

#### Scenario: Read screen dimensions
- **WHEN** a HUD script calls `local w, h = Screen.width(), Screen.height()` on a 1920x1080 display
- **THEN** `w` SHALL be 1920 and `h` SHALL be 1080

### Requirement: Color and font constants
The `Screen` global object SHALL provide named color constants: `Screen.colors.white`, `Screen.colors.black`, `Screen.colors.red`, `Screen.colors.green`, `Screen.colors.blue`, `Screen.colors.yellow`, `Screen.colors.light_gray`, `Screen.colors.dark_gray`. Each constant SHALL be a table `{r, g, b, a}` with appropriate values. The `Screen` global SHALL provide font constants: `Screen.fonts.interface`, `Screen.fonts.computer`, `Screen.fonts.computer_large`, `Screen.fonts.title`.

#### Scenario: Use color constant
- **WHEN** a HUD script calls `Screen.fill_rect(0, 0, 50, 10, Screen.colors.red)`
- **THEN** the system SHALL record a fill_rect with color `{r=1, g=0, b=0, a=1}`

### Requirement: HUD draw command buffer
The system SHALL buffer all draw commands produced by the HUD script's `draw()` function into a `Vec<HudDrawCommand>`. The buffer SHALL be cleared at the start of each `draw()` call. The renderer SHALL consume the buffer after `draw()` returns and translate each command to wgpu draw calls in the HUD overlay render pass. The `HudDrawCommand` enum SHALL include variants: `FillRect`, `FrameRect`, `DrawText`, `DrawShape`, `SetClipRect`, `ClearClipRect`.

#### Scenario: Buffer round-trip
- **WHEN** a HUD script issues 5 draw commands in `draw()`
- **THEN** the buffer SHALL contain exactly 5 `HudDrawCommand` entries after `draw()` returns

#### Scenario: Buffer cleared each frame
- **WHEN** `draw()` is called on two consecutive frames
- **THEN** the second frame's buffer SHALL contain only the commands from the second `draw()` call, not accumulated commands from both frames
