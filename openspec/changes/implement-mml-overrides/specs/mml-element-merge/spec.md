## ADDED Requirements

### Requirement: Merge MML sections by element index
When two `MmlSection` instances are merged, the system SHALL combine their child elements by matching on the `index` attribute. For each element in the overlay that has an `index` attribute matching an element in the base with the same element name and `index` value, the system SHALL merge the two elements' attributes (overlay attributes overwrite base attributes with the same key; base-only attributes are preserved). Child elements within the matched pair SHALL be merged recursively using the same index-based rules.

#### Scenario: Overlay modifies one monster among many
- **WHEN** a base section contains `<monster index="0" vitality="100"/>`, `<monster index="1" vitality="200"/>`, `<monster index="2" vitality="300"/>`
- **AND** an overlay section contains `<monster index="1" vitality="500"/>`
- **THEN** the merged section SHALL contain all three monsters, with monster 1's vitality changed to "500" and monsters 0 and 2 unchanged

#### Scenario: Overlay adds an element not in base
- **WHEN** a base section contains `<monster index="0" vitality="100"/>`
- **AND** an overlay section contains `<monster index="5" vitality="200"/>`
- **THEN** the merged section SHALL contain both monster 0 (from base) and monster 5 (from overlay)

#### Scenario: Attribute-level merge within a matched element
- **WHEN** a base element has `<monster index="3" vitality="100" speed="5" radius="200"/>`
- **AND** an overlay element has `<monster index="3" vitality="300"/>`
- **THEN** the merged element SHALL have `vitality="300"` (from overlay), `speed="5"` (preserved from base), and `radius="200"` (preserved from base)

#### Scenario: Recursive child merge
- **WHEN** a base section contains `<weapon index="0"><shell_casings index="1" coll="14"/></weapon>`
- **AND** an overlay contains `<weapon index="0"><shell_casings index="1" seq="3"/></weapon>`
- **THEN** the merged weapon 0 SHALL contain shell_casings 1 with both `coll="14"` (from base) and `seq="3"` (from overlay)

### Requirement: Preserve elements without index attribute
When merging two `MmlSection` instances, elements in the overlay that do not have an `index` attribute SHALL be appended to the merged section after all index-matched elements. Elements in the base that do not have an `index` attribute SHALL be preserved unless an overlay element with the same element name (but no index) replaces them.

#### Scenario: Non-indexed elements appended from overlay
- **WHEN** a base section contains `<monster index="0" vitality="100"/>`
- **AND** an overlay section contains `<clear/>` (no index attribute)
- **THEN** the merged section SHALL contain both the monster element and the clear element

### Requirement: MmlDocument layer uses element-level merge
The `MmlDocument::layer()` method SHALL use element-level merging for sections present in both the base and the overlay. When the overlay has a section that the base also has, the two sections SHALL be merged using element-level merge semantics. When the overlay has a section that the base does not, the overlay section SHALL be used as-is. When the base has a section that the overlay does not, the base section SHALL be preserved as-is.

#### Scenario: Two documents with overlapping monster sections
- **WHEN** a base document has `<monsters>` with monsters 0, 1, 2 and an overlay document has `<monsters>` with monster 1 (modified vitality)
- **THEN** `MmlDocument::layer(base, overlay)` SHALL produce a document whose `<monsters>` section contains all three monsters, with monster 1's vitality updated

#### Scenario: Overlay adds new section
- **WHEN** a base document has `<monsters>` and an overlay document has `<weapons>`
- **THEN** `MmlDocument::layer(base, overlay)` SHALL produce a document with both `<monsters>` (from base) and `<weapons>` (from overlay)

#### Scenario: Three-layer cascade preserves all changes
- **WHEN** document A defines monster 0, document B modifies monster 0's vitality and adds monster 5, and document C modifies monster 5's speed
- **THEN** layering A, then B, then C SHALL produce a section with monster 0 (vitality from B), monster 5 (from B with speed from C)

### Requirement: MmlSection provides index-based element lookup
`MmlSection` SHALL provide a method to find an element by name and `index` attribute value. This method SHALL return a reference to the matching `MmlElement` if found, or `None` if no element with that name and index exists in the section.

#### Scenario: Look up existing element by index
- **WHEN** a section contains `<monster index="3" vitality="100"/>`
- **AND** a lookup is performed for element name "monster" with index "3"
- **THEN** the method SHALL return a reference to the matching element

#### Scenario: Look up non-existent index
- **WHEN** a section contains `<monster index="3" vitality="100"/>`
- **AND** a lookup is performed for element name "monster" with index "7"
- **THEN** the method SHALL return `None`
