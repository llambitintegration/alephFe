## MODIFIED Requirements

### Requirement: Light animation
The system SHALL animate lights using a 6-state machine parsed from `StaticLightData`. Each light SHALL cycle through six states: becoming_active, primary_active, secondary_active, becoming_inactive, primary_inactive, secondary_inactive. Each state SHALL have its own lighting function specification containing: function type (one of 6 types), period in ticks, delta_period for random variation, target intensity, and delta_intensity for random variation. The system SHALL evaluate each light's current intensity every tick based on the current state's function, advancing phase and transitioning states when the phase reaches the period. The system SHALL support all 6 lighting function types: constant, linear, smooth, flicker, random, and fluorescent.

#### Scenario: Constant light
- **WHEN** a light's current state has the constant function type
- **THEN** its intensity SHALL equal the final_intensity for the duration of that state, regardless of phase

#### Scenario: Linear light ramp
- **WHEN** a light's current state has the linear function type with a given period
- **THEN** its intensity SHALL linearly interpolate from initial_intensity to final_intensity over the period:
  `intensity = initial + (final - initial) * phase / period`

#### Scenario: Smooth cycling light
- **WHEN** a light's current state has the smooth function type
- **THEN** its intensity SHALL interpolate from initial_intensity to final_intensity using a cosine curve:
  `intensity = initial + (final - initial) * (cos(phase * PI / period + PI) + 1) / 2`

#### Scenario: Flickering light
- **WHEN** a light's current state has the flicker function type
- **THEN** its intensity SHALL combine a smooth base oscillation with random variation:
  `smooth_value = smooth(phase, period, initial, final)`
  `intensity = smooth_value + random() * (final - smooth_value)`

#### Scenario: Random light
- **WHEN** a light's current state has the random function type
- **THEN** its intensity SHALL be a purely random value between initial_intensity and final_intensity each tick:
  `intensity = initial + random() * (final - initial)`

#### Scenario: Fluorescent light
- **WHEN** a light's current state has the fluorescent function type
- **THEN** its intensity SHALL randomly toggle between initial_intensity and final_intensity each tick with 50% probability:
  `intensity = (random() > 0.5) ? final : initial`

#### Scenario: State transition on period expiry
- **WHEN** a light's phase reaches or exceeds its current period
- **THEN** the light SHALL advance to the next state in the cycle (becoming_active -> primary_active -> secondary_active -> becoming_inactive -> primary_inactive -> secondary_inactive -> becoming_active), the initial_intensity SHALL be set to the current intensity, the new period SHALL be the state's base period plus a random value in [0, delta_period], and the new final_intensity SHALL be the state's base intensity plus a random fraction of delta_intensity

#### Scenario: Initially active light
- **WHEN** a light has the initially_active flag set
- **THEN** it SHALL start in the becoming_active state on spawn

#### Scenario: Initially inactive light
- **WHEN** a light does not have the initially_active flag
- **THEN** it SHALL start in the becoming_inactive state on spawn

#### Scenario: Slaved intensities
- **WHEN** a light has the slaved_intensities flag set
- **THEN** the secondary_active and secondary_inactive states SHALL use the primary_active and primary_inactive intensity values respectively, instead of their own

#### Scenario: Light phase offset
- **WHEN** a light has a non-zero initial phase value from the map data
- **THEN** the light SHALL start with its phase counter set to that value, offsetting its animation relative to other lights

#### Scenario: Light types
- **WHEN** a light has type normal (0), strobe (1), or media (2)
- **THEN** the light_type SHALL be stored on the Light component for use by media height coupling and potential future strobe behavior

### Requirement: Media simulation
The system SHALL simulate liquid media (water, lava, goo, sewage, jjaro) in polygons that reference a `MediaData` entry. Media height SHALL be derived from the light intensity of the media's associated light each tick: as the light intensity varies, the liquid surface rises and falls between the media's defined low and high heights. Media SHALL apply current flow forces to entities standing in the liquid.

#### Scenario: Rising water
- **WHEN** the media's associated light intensity increases over successive ticks
- **THEN** the water surface height SHALL rise proportionally between the low and high bounds:
  `height = low + (high - low) * light_intensity`

#### Scenario: Falling water
- **WHEN** the media's associated light intensity decreases over successive ticks
- **THEN** the water surface height SHALL fall proportionally between the low and high bounds

#### Scenario: Media height updated each tick
- **WHEN** the simulation advances one tick
- **THEN** each media entity's current_height SHALL be recomputed from its associated light's current_intensity

#### Scenario: Lava damage
- **WHEN** the player is submerged in lava media
- **THEN** the player SHALL take environmental damage each tick based on the lava damage definition

#### Scenario: Media current pushes entity
- **WHEN** a player or monster stands in media with a defined current direction and magnitude
- **THEN** an external velocity SHALL be applied to the entity in the current direction
