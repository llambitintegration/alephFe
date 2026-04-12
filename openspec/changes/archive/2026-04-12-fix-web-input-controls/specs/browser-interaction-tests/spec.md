## MODIFIED Requirements

### Requirement: Keyboard input dispatches to the game
The Playwright e2e suite SHALL verify that keyboard input is received and processed by the running game. After the WASD fix, pressing W SHALL result in forward movement (not backward), so any test asserting forward-movement behavior from the W key SHALL pass.

#### Scenario: WASD keys are accepted without error
- **WHEN** the game has started (loading overlay hidden, canvas visible) and keyboard keys W, A, S, D are pressed sequentially via `page.keyboard.press()`
- **THEN** no console errors SHALL appear related to input handling, and the game SHALL continue running (canvas remains visible, no error overlay)

#### Scenario: W key produces forward movement
- **WHEN** the canvas has focus and the user presses W
- **THEN** `input.forward` SHALL be true (not `input.backward`), confirming the WASD mapping fix is in effect

#### Scenario: Space key triggers action
- **WHEN** the Space key is pressed after game start
- **THEN** no console errors SHALL appear, and the game SHALL continue running without crashing
