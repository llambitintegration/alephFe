## ADDED Requirements

### Requirement: Activate terminal from gameplay
The system SHALL detect when the player activates a terminal polygon via marathon-sim. Upon activation, the system SHALL look up the terminal data for that polygon from the loaded map's terminal definitions (parsed by marathon-formats). The system SHALL transition the game state to `Terminal` and initialize the terminal renderer with the terminal's text groups.

#### Scenario: Player activates a terminal
- **WHEN** the player presses the action key while facing a terminal polygon and within activation range
- **THEN** the game state SHALL transition to `Terminal` and the terminal renderer SHALL display the first page of the terminal's content

#### Scenario: Terminal with no data
- **WHEN** the player activates a terminal polygon that has no associated terminal data
- **THEN** the system SHALL not transition to `Terminal` state and gameplay SHALL continue

### Requirement: Display styled terminal text pages
The system SHALL render terminal content as styled text pages using glyphon for wgpu text rendering. The system SHALL support the following terminal text styles from marathon-formats terminal data: plain text, logon screen (computer identification header), logoff screen, information text, checkpoint text, and chapter headers. Each style SHALL have distinct visual formatting (font size, color, alignment).

#### Scenario: Information text display
- **WHEN** a terminal page contains information-style text
- **THEN** the text SHALL render in Marathon's terminal font with green text on a dark background

#### Scenario: Logon screen display
- **WHEN** a terminal page is a logon group
- **THEN** the system SHALL display the computer name header and authentication sequence before showing content

#### Scenario: Chapter header display
- **WHEN** a terminal page contains a chapter header
- **THEN** the header text SHALL render centered in a larger font size

### Requirement: Display terminal images
The system SHALL render inline images within terminal pages. Images SHALL be referenced by PICT resource ID from the terminal data and rendered at their specified position within the terminal view. Images SHALL be loaded from the scenario's resource fork data via marathon-formats.

#### Scenario: Terminal page with image
- **WHEN** a terminal page references PICT resource 1200
- **THEN** the system SHALL load and render that image within the terminal view at the specified position

#### Scenario: Missing image resource
- **WHEN** a terminal page references a PICT resource that cannot be found
- **THEN** the system SHALL display a placeholder or skip the image area and continue rendering text

### Requirement: Page-based navigation with scrolling
The system SHALL organize terminal content into pages that fit within the terminal display area. The system SHALL support scrolling within a page (line by line via up/down) and paging between pages (via page up/page down or reaching the end of scroll). The current page indicator SHALL be displayed.

#### Scenario: Scroll down within a page
- **WHEN** the player presses the scroll down key and there is more content below the visible area
- **THEN** the terminal view SHALL scroll down by one line

#### Scenario: Advance to next page
- **WHEN** the player presses page down or scrolls past the end of the current page
- **THEN** the terminal SHALL advance to the next page

#### Scenario: At last page
- **WHEN** the player is on the last page and presses page down
- **THEN** the terminal SHALL not advance further (stay on the last page)

#### Scenario: Page indicator
- **WHEN** a terminal has 5 pages and the player is viewing page 3
- **THEN** the terminal SHALL display a page indicator showing "3/5" or equivalent

### Requirement: Terminal exit returns to gameplay
The system SHALL exit the terminal view and return to the `Playing` state when the player presses the exit key (Escape or equivalent). The system SHALL also exit the terminal automatically after the player views the last page and presses a continue key.

#### Scenario: Exit via escape key
- **WHEN** the player presses Escape while in Terminal state
- **THEN** the game state SHALL transition from `Terminal` to `Playing`

#### Scenario: Exit after reading all pages
- **WHEN** the player is on the last page and presses the continue key
- **THEN** the game state SHALL transition from `Terminal` to `Playing`

### Requirement: Terminal teleport on exit
The system SHALL check if the current terminal's exit action specifies a level teleport. If a teleport target level is specified, exiting the terminal SHALL trigger a level transition to the target level instead of returning to normal gameplay.

#### Scenario: Terminal with teleport
- **WHEN** the player exits a terminal that specifies teleport to level 7
- **THEN** the system SHALL transition from `Terminal` to `Intermission` and then load level 7

#### Scenario: Terminal without teleport
- **WHEN** the player exits a terminal that has no teleport action
- **THEN** the system SHALL transition from `Terminal` to `Playing` and gameplay SHALL resume normally

### Requirement: Conditional text groups based on game state
The system SHALL evaluate terminal text group conditions to determine which content to display. Terminal groups MAY specify conditions based on mission state (success/failure of objectives). The system SHALL query marathon-sim for the current mission state and display only the groups whose conditions are satisfied.

#### Scenario: Success condition met
- **WHEN** a terminal has a success text group and the current level's success objective has been completed
- **THEN** the system SHALL display the success text group content

#### Scenario: Failure condition
- **WHEN** a terminal has both success and failure text groups and the success objective has NOT been completed
- **THEN** the system SHALL display the failure text group content

#### Scenario: Unconditional group
- **WHEN** a terminal text group has no condition
- **THEN** the system SHALL always display that group's content regardless of mission state

### Requirement: Terminal pauses simulation
The system SHALL pause the marathon-sim simulation while the terminal is active. No simulation ticks SHALL advance during terminal viewing. Audio ambient sounds MAY continue but spatial sounds tied to simulation events SHALL not trigger new playback.

#### Scenario: Simulation paused during terminal
- **WHEN** the game state is `Terminal`
- **THEN** marathon-sim SHALL receive no tick advances and all entity positions SHALL remain frozen

#### Scenario: Resume after terminal exit
- **WHEN** the player exits a terminal and returns to `Playing` state
- **THEN** marathon-sim SHALL resume receiving tick advances from the frame after exit

### Requirement: Track terminal read status
The system SHALL track which terminals the player has read (activated and viewed at least the first page). Terminal read status SHALL be included in save game data. Terminal read status SHALL be available to marathon-sim for mission state queries.

#### Scenario: Terminal marked as read
- **WHEN** the player activates terminal 3 and views at least the first page
- **THEN** terminal 3 SHALL be marked as read in the terminal status tracker

#### Scenario: Read status persists in save
- **WHEN** the player saves the game after reading terminals 1 and 3
- **THEN** the save file SHALL include that terminals 1 and 3 have been read

#### Scenario: Read status restored on load
- **WHEN** the player loads a save where terminals 1 and 3 were marked as read
- **THEN** the terminal status tracker SHALL reflect terminals 1 and 3 as read
