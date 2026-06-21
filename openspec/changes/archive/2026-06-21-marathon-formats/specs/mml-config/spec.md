## ADDED Requirements

### Requirement: Parse MML documents with marathon root element
The parser SHALL accept XML documents with a `<marathon>` root element and produce a structured MML configuration result. The parser SHALL use the `quick-xml` crate for XML processing. Documents without a `<marathon>` root element SHALL be rejected with an error.

#### Scenario: Valid MML document with marathon root
- **WHEN** the parser receives a well-formed XML byte slice containing `<marathon>` as the root element
- **THEN** the parser SHALL return a successfully parsed MML configuration structure

#### Scenario: XML document with wrong root element
- **WHEN** the parser receives a well-formed XML document whose root element is not `<marathon>`
- **THEN** the parser SHALL return an error indicating an unexpected root element, including the element name that was found

#### Scenario: Empty marathon element
- **WHEN** the parser receives `<marathon></marathon>` with no child elements
- **THEN** the parser SHALL return a valid MML configuration structure with all sections empty or at their defaults

### Requirement: Parse individual configuration sections
The parser SHALL recognize and parse each of the following sub-elements within `<marathon>`: `<stringset>`, `<interface>`, `<motion_sensor>`, `<overhead_map>`, `<infravision>`, `<animated_textures>`, `<control_panels>`, `<platforms>`, `<liquids>`, `<sounds>`, `<faders>`, `<player>`, `<weapons>`, `<items>`, `<monsters>`, `<scenery>`, `<landscapes>`, `<texture_loading>`, `<opengl>`, `<software>`, `<dynamic_limits>`, `<scenario>`, `<console>`, and `<logging>`. Each recognized section SHALL be parsed into a corresponding typed Rust structure. Unrecognized top-level child elements within `<marathon>` SHALL be ignored without error.

#### Scenario: Document with a single known section
- **WHEN** the parser receives a `<marathon>` document containing only a `<weapons>` child element with valid weapon configuration attributes
- **THEN** the parser SHALL return an MML configuration with the weapons section populated and all other sections empty

#### Scenario: Document with multiple known sections
- **WHEN** the parser receives a `<marathon>` document containing `<monsters>`, `<weapons>`, and `<dynamic_limits>` child elements
- **THEN** the parser SHALL return an MML configuration with all three sections populated with their respective parsed data

#### Scenario: Document with unrecognized child elements
- **WHEN** the parser receives a `<marathon>` document containing `<weapons>` and an unrecognized element `<custom_extension>`
- **THEN** the parser SHALL parse the `<weapons>` section successfully and silently ignore `<custom_extension>` without producing an error

### Requirement: Support partial MML files
The parser SHALL treat all configuration sections as optional. An MML file is valid as long as it has a `<marathon>` root element, regardless of which sections are present. The parsed result SHALL clearly indicate which sections were present and which were absent.

#### Scenario: MML file with only dynamic_limits
- **WHEN** the parser receives an MML file containing only `<marathon><dynamic_limits>...</dynamic_limits></marathon>`
- **THEN** the parser SHALL return a result where the dynamic_limits section contains the parsed values and all other sections indicate absence

#### Scenario: MML file with no configuration sections
- **WHEN** the parser receives an MML file containing `<marathon><!-- just a comment --></marathon>`
- **THEN** the parser SHALL return a valid result with all sections absent

### Requirement: Support MML layering
The system SHALL support applying multiple MML configurations in sequence where later configurations override values set by earlier ones. When two MML configurations are layered, sections present in the later configuration SHALL replace or merge with the corresponding sections from the earlier configuration. Sections not present in the later configuration SHALL retain their values from the earlier configuration.

#### Scenario: Later MML overrides a section from earlier MML
- **WHEN** a base MML configuration defines weapon settings and a second MML configuration also defines weapon settings
- **AND** the second configuration is layered on top of the first
- **THEN** the weapon settings from the second configuration SHALL replace the weapon settings from the first configuration

#### Scenario: Later MML adds a section not in earlier MML
- **WHEN** a base MML configuration defines only monster settings and a second MML configuration defines only weapon settings
- **AND** the second configuration is layered on top of the first
- **THEN** the resulting configuration SHALL contain both the monster settings from the first and the weapon settings from the second

#### Scenario: Later MML leaves sections from earlier MML intact
- **WHEN** a base MML configuration defines monster, weapon, and dynamic_limits settings and a second MML configuration defines only weapon settings
- **AND** the second configuration is layered on top of the first
- **THEN** the monster and dynamic_limits settings from the base SHALL be preserved unchanged, and only the weapon settings SHALL reflect the second configuration

#### Scenario: Three or more MML files layered in order
- **WHEN** three MML configurations are layered in order (base, scenario override, plugin override)
- **AND** each defines overlapping and non-overlapping sections
- **THEN** the final result SHALL reflect the last configuration to define each section, with sections defined only in earlier configurations preserved

### Requirement: Extract embedded MML from WAD MMLS tags
The parser SHALL extract MML XML content embedded within WAD file entries tagged with the four-character code `MMLS`. The extracted bytes SHALL be treated as MML XML and parsed using the same logic as standalone MML files. The parser SHALL handle the raw byte content from the WAD chunk, including stripping any null terminators if present.

#### Scenario: WAD entry contains MMLS tag with valid MML
- **WHEN** a WAD file entry contains a chunk with tag `MMLS` whose data is valid MML XML with a `<marathon>` root
- **THEN** the parser SHALL extract the chunk data and return a successfully parsed MML configuration

#### Scenario: WAD entry contains MMLS tag with null-terminated MML
- **WHEN** a WAD file entry contains a chunk with tag `MMLS` whose data is valid MML XML followed by one or more null bytes
- **THEN** the parser SHALL strip trailing null bytes before parsing and return a successfully parsed MML configuration

#### Scenario: WAD entry contains no MMLS tag
- **WHEN** a WAD file entry is examined for embedded MML but contains no chunk with tag `MMLS`
- **THEN** the parser SHALL indicate that no embedded MML was found without producing an error

### Requirement: Report errors for malformed XML with file context
The parser SHALL produce clear, actionable error messages when MML parsing fails. Error reports SHALL include the nature of the XML error (e.g., unclosed tag, invalid attribute, encoding issue). When available, error reports SHALL include the byte offset or line number where the error was detected. When parsing from a named source (file path or WAD entry identifier), the error SHALL include that source identifier so users can locate the problem.

#### Scenario: Unclosed XML tag
- **WHEN** the parser receives MML content containing `<marathon><weapons>` with no closing `</weapons>` or `</marathon>` tags
- **THEN** the parser SHALL return an error that describes the unclosed tag issue and includes the byte offset or line number of the error

#### Scenario: Invalid XML syntax
- **WHEN** the parser receives MML content containing `<marathon><weapons <<broken></marathon>`
- **THEN** the parser SHALL return an error describing the XML syntax problem with position information

#### Scenario: Error includes source file path
- **WHEN** the parser attempts to parse a file at path `mods/scenario/MML/weapons.mml` and the file contains malformed XML
- **THEN** the error message SHALL include the path `mods/scenario/MML/weapons.mml` so the user can identify which file is problematic

#### Scenario: Error from embedded WAD MML includes WAD context
- **WHEN** the parser extracts MML from a WAD entry's MMLS chunk and the extracted XML is malformed
- **THEN** the error message SHALL indicate that the MML originated from an embedded WAD source, including the WAD entry identifier or index
