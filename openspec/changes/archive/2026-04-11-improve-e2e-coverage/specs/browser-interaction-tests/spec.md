## ADDED Requirements

### Requirement: Keyboard input dispatches to the game
The Playwright e2e suite SHALL verify that keyboard input is received and processed by the running game.

#### Scenario: WASD keys are accepted without error
- **WHEN** the game has started (loading overlay hidden, canvas visible) and keyboard keys W, A, S, D are pressed sequentially via `page.keyboard.press()`
- **THEN** no console errors SHALL appear related to input handling, and the game SHALL continue running (canvas remains visible, no error overlay)

#### Scenario: Space key triggers action
- **WHEN** the Space key is pressed after game start
- **THEN** no console errors SHALL appear, and the game SHALL continue running without crashing

### Requirement: Pointer lock activates on canvas click
The Playwright e2e suite SHALL verify that clicking the game canvas activates pointer lock.

#### Scenario: Click canvas requests pointer lock
- **WHEN** the game has started and the user clicks on `#marathon-canvas`
- **THEN** `document.pointerLockElement` SHALL equal the canvas element (or the request SHALL have been made, verifiable via the `pointerlockchange` event being fired)

### Requirement: HUD elements are visible and contain numeric values
The Playwright e2e suite SHALL verify that HUD elements appear with valid content after the game starts.

#### Scenario: Health and shield bars display values
- **WHEN** the game has started and the HUD is visible (`#hud` element has `display` other than `none`)
- **THEN** `#health-val` SHALL contain a numeric text value and `#shield-val` SHALL contain a numeric text value

#### Scenario: HUD becomes visible during gameplay
- **WHEN** the game has started and at least 2 seconds have elapsed
- **THEN** the `#hud` element SHALL be visible (not `display: none`)

### Requirement: Missing Physics file shows error
The Playwright error-handling suite SHALL verify that a missing Physics data file produces a user-visible error.

#### Scenario: Physics 404 shows error overlay
- **WHEN** the Physics data file request (`/data/Physics.phyA`) is intercepted and returns HTTP 404
- **THEN** the `#error` element SHALL become visible within 30 seconds and SHALL contain the text "Physics"
