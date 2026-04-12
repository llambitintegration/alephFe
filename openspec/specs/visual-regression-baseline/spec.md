### Requirement: Canvas renders a non-trivial scene
The Playwright e2e suite SHALL include a visual regression test that verifies the rendered canvas contains a structurally valid scene, not just non-blank pixels.

#### Scenario: Rendered scene has sufficient non-black pixel coverage
- **WHEN** the game has started and at least 2 seconds have elapsed for rendering to settle
- **THEN** more than 20% of sampled canvas pixels SHALL have at least one non-zero RGB channel

#### Scenario: Rendered scene has color variety
- **WHEN** the game canvas is sampled after rendering settles
- **THEN** the number of unique RGB colors (quantized to 6-bit per channel) SHALL exceed 50

#### Scenario: Rendered scene covers multiple screen regions
- **WHEN** the canvas is divided into four quadrants (top-left, top-right, bottom-left, bottom-right)
- **THEN** at least 3 of the 4 quadrants SHALL contain non-black pixels, indicating geometry covers most of the viewport
