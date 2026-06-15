use std::collections::HashMap;
use std::path::Path;

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use crate::error::{MmlError, XmlContextMessage};
use crate::tags::WadTag;
use crate::wad::WadEntry;

/// A single XML element with its name, attributes, child elements, and text content.
#[derive(Debug, Clone)]
pub struct MmlElement {
    pub name: String,
    pub attributes: HashMap<String, String>,
    pub children: Vec<MmlElement>,
    pub text: Option<String>,
}

/// A section within an MML document, containing the child elements
/// found under a recognized `<marathon>` child element.
#[derive(Debug, Clone, Default)]
pub struct MmlSection {
    /// Attributes on the section element itself — e.g. `name`/`id` on
    /// `<scenario name="..." id="...">` or `landscapes` on
    /// `<texture_loading landscapes="true">`. Section interpreters read these;
    /// child elements live in `elements`.
    pub attributes: HashMap<String, String>,
    pub elements: Vec<MmlElement>,
}

/// Parsed MML (Marathon Markup Language) configuration document.
///
/// Each field corresponds to a recognized child element of `<marathon>`.
/// Fields are `None` when the section was not present in the source XML.
/// MML parsing is deliberately shallow: we preserve the XML tree structure
/// but do not deeply interpret every possible override.
#[derive(Debug, Clone, Default)]
pub struct MmlDocument {
    pub stringset: Option<MmlSection>,
    pub interface: Option<MmlSection>,
    pub motion_sensor: Option<MmlSection>,
    pub overhead_map: Option<MmlSection>,
    pub infravision: Option<MmlSection>,
    pub animated_textures: Option<MmlSection>,
    pub control_panels: Option<MmlSection>,
    pub platforms: Option<MmlSection>,
    pub liquids: Option<MmlSection>,
    pub sounds: Option<MmlSection>,
    pub faders: Option<MmlSection>,
    pub player: Option<MmlSection>,
    pub weapons: Option<MmlSection>,
    pub items: Option<MmlSection>,
    pub monsters: Option<MmlSection>,
    pub scenery: Option<MmlSection>,
    pub landscapes: Option<MmlSection>,
    pub texture_loading: Option<MmlSection>,
    pub opengl: Option<MmlSection>,
    pub software: Option<MmlSection>,
    pub dynamic_limits: Option<MmlSection>,
    pub scenario: Option<MmlSection>,
    pub console: Option<MmlSection>,
    pub logging: Option<MmlSection>,
}

impl MmlElement {
    /// Copy `overlay`'s attributes into `self`. Overlay values win on conflict;
    /// attributes present only on the base element are preserved.
    pub fn merge_attributes(&mut self, overlay: &MmlElement) {
        for (k, v) in &overlay.attributes {
            self.attributes.insert(k.clone(), v.clone());
        }
    }

    /// Recursively merge `overlay`'s children into `self`. A child is matched to
    /// an existing child by `(name, index attribute)`: matched pairs merge their
    /// attributes and children recursively; unmatched overlay children — those
    /// with a new `index`, or with no `index` attribute at all — are appended.
    pub fn merge_children(&mut self, overlay: &MmlElement) {
        for oc in &overlay.children {
            match oc.attributes.get("index") {
                Some(idx) => {
                    match self
                        .children
                        .iter()
                        .position(|c| c.name == oc.name && c.attributes.get("index") == Some(idx))
                    {
                        Some(pos) => {
                            self.children[pos].merge_attributes(oc);
                            self.children[pos].merge_children(oc);
                        }
                        None => self.children.push(oc.clone()),
                    }
                }
                None => self.children.push(oc.clone()),
            }
        }
    }
}

impl MmlSection {
    /// Find the first element matching `name` and the given `index` attribute.
    pub fn find_element(&self, name: &str, index: &str) -> Option<&MmlElement> {
        self.elements.iter().find(|e| {
            e.name == name && e.attributes.get("index").map(String::as_str) == Some(index)
        })
    }

    /// Merge `overlay` into `base` at element granularity. Elements are matched
    /// by `(name, index attribute)`: matched pairs merge their attributes and
    /// children recursively; unmatched base elements are preserved; unmatched
    /// overlay elements (new `index`) and non-indexed overlay elements are
    /// appended. This lets an overlay tweak one element without dropping the
    /// base's siblings — the property `Option::or` could not provide.
    pub fn merge(base: Self, overlay: Self) -> Self {
        // Section-level attributes merge overlay-wins, base-only preserved.
        let mut attributes = base.attributes;
        for (k, v) in overlay.attributes {
            attributes.insert(k, v);
        }
        let mut elements = base.elements;
        for oe in overlay.elements {
            match oe.attributes.get("index") {
                Some(idx) => {
                    match elements
                        .iter()
                        .position(|e| e.name == oe.name && e.attributes.get("index") == Some(idx))
                    {
                        Some(pos) => {
                            elements[pos].merge_attributes(&oe);
                            elements[pos].merge_children(&oe);
                        }
                        None => elements.push(oe),
                    }
                }
                None => elements.push(oe),
            }
        }
        Self {
            attributes,
            elements,
        }
    }
}

impl MmlDocument {
    /// Parse an MML document from a byte slice.
    pub fn from_bytes(data: &[u8]) -> Result<Self, MmlError> {
        let data = strip_trailing_nulls(data);
        let mut reader = Reader::from_reader(data);

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) => {
                    let name = element_name(&e);
                    if name != "marathon" {
                        return Err(MmlError::InvalidRootElement(name));
                    }
                    return parse_marathon_body(&mut reader);
                }
                Ok(Event::Empty(e)) => {
                    let name = element_name(&e);
                    if name != "marathon" {
                        return Err(MmlError::InvalidRootElement(name));
                    }
                    return Ok(Self::default());
                }
                Ok(Event::Eof) => return Ok(Self::default()),
                Ok(_) => continue,
                Err(e) => return Err(MmlError::Xml(e)),
            }
        }
    }

    /// Parse an MML document from a byte slice, wrapping XML errors with source context.
    pub fn from_bytes_with_source(data: &[u8], source: &str) -> Result<Self, MmlError> {
        Self::from_bytes(data).map_err(|e| wrap_error_with_source(e, source))
    }

    /// Parse an MML document from a file.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, MmlError> {
        let path = path.as_ref();
        let data = std::fs::read(path).map_err(|e| MmlError::XmlWithContext {
            source: path.display().to_string(),
            message: XmlContextMessage(e.to_string()),
        })?;
        Self::from_bytes_with_source(&data, &path.display().to_string())
    }

    /// Extract and parse embedded MML from a WAD entry's MMLS tag.
    /// Returns `Ok(None)` if no MMLS tag is present.
    pub fn from_wad_entry(entry: &WadEntry) -> Result<Option<Self>, MmlError> {
        match entry.get_tag_data(WadTag::MmlScript) {
            Some(data) => Self::from_bytes_with_source(data, "embedded MMLS tag").map(Some),
            None => Ok(None),
        }
    }

    /// Layer two MML documents. Sections present in both are merged at element
    /// granularity (see [`MmlSection::merge`]); sections present in only one
    /// document are carried through unchanged.
    pub fn layer(base: Self, overlay: Self) -> Self {
        // When a section is present in both documents, merge it at element
        // granularity (overlay tweaks individual elements without dropping the
        // base's siblings); otherwise take whichever side has it.
        fn merge_opt(base: Option<MmlSection>, overlay: Option<MmlSection>) -> Option<MmlSection> {
            match (base, overlay) {
                (Some(b), Some(o)) => Some(MmlSection::merge(b, o)),
                (b, o) => o.or(b),
            }
        }
        Self {
            stringset: merge_opt(base.stringset, overlay.stringset),
            interface: merge_opt(base.interface, overlay.interface),
            motion_sensor: merge_opt(base.motion_sensor, overlay.motion_sensor),
            overhead_map: merge_opt(base.overhead_map, overlay.overhead_map),
            infravision: merge_opt(base.infravision, overlay.infravision),
            animated_textures: merge_opt(base.animated_textures, overlay.animated_textures),
            control_panels: merge_opt(base.control_panels, overlay.control_panels),
            platforms: merge_opt(base.platforms, overlay.platforms),
            liquids: merge_opt(base.liquids, overlay.liquids),
            sounds: merge_opt(base.sounds, overlay.sounds),
            faders: merge_opt(base.faders, overlay.faders),
            player: merge_opt(base.player, overlay.player),
            weapons: merge_opt(base.weapons, overlay.weapons),
            items: merge_opt(base.items, overlay.items),
            monsters: merge_opt(base.monsters, overlay.monsters),
            scenery: merge_opt(base.scenery, overlay.scenery),
            landscapes: merge_opt(base.landscapes, overlay.landscapes),
            texture_loading: merge_opt(base.texture_loading, overlay.texture_loading),
            opengl: merge_opt(base.opengl, overlay.opengl),
            software: merge_opt(base.software, overlay.software),
            dynamic_limits: merge_opt(base.dynamic_limits, overlay.dynamic_limits),
            scenario: merge_opt(base.scenario, overlay.scenario),
            console: merge_opt(base.console, overlay.console),
            logging: merge_opt(base.logging, overlay.logging),
        }
    }

    /// Returns `true` if this document has no sections.
    pub fn is_empty(&self) -> bool {
        self.stringset.is_none()
            && self.interface.is_none()
            && self.motion_sensor.is_none()
            && self.overhead_map.is_none()
            && self.infravision.is_none()
            && self.animated_textures.is_none()
            && self.control_panels.is_none()
            && self.platforms.is_none()
            && self.liquids.is_none()
            && self.sounds.is_none()
            && self.faders.is_none()
            && self.player.is_none()
            && self.weapons.is_none()
            && self.items.is_none()
            && self.monsters.is_none()
            && self.scenery.is_none()
            && self.landscapes.is_none()
            && self.texture_loading.is_none()
            && self.opengl.is_none()
            && self.software.is_none()
            && self.dynamic_limits.is_none()
            && self.scenario.is_none()
            && self.console.is_none()
            && self.logging.is_none()
    }
}

fn strip_trailing_nulls(data: &[u8]) -> &[u8] {
    let end = data.iter().rposition(|&b| b != 0).map_or(0, |i| i + 1);
    &data[..end]
}

fn element_name(e: &BytesStart<'_>) -> String {
    String::from_utf8_lossy(e.name().as_ref()).into_owned()
}

fn parse_attributes(e: &BytesStart<'_>) -> Result<HashMap<String, String>, MmlError> {
    let mut attrs = HashMap::new();
    for attr_result in e.attributes() {
        let attr = attr_result.map_err(|e| MmlError::Xml(e.into()))?;
        let key = String::from_utf8_lossy(attr.key.as_ref()).into_owned();
        let value = String::from_utf8_lossy(&attr.value).into_owned();
        attrs.insert(key, value);
    }
    Ok(attrs)
}

fn parse_marathon_body(reader: &mut Reader<&[u8]>) -> Result<MmlDocument, MmlError> {
    let mut doc = MmlDocument::default();

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = element_name(&e);
                let attributes = parse_attributes(&e)?;
                let section = parse_section(reader, attributes)?;
                set_section(&mut doc, &name, section);
            }
            Ok(Event::Empty(e)) => {
                let name = element_name(&e);
                set_section(
                    &mut doc,
                    &name,
                    MmlSection {
                        attributes: parse_attributes(&e)?,
                        elements: Vec::new(),
                    },
                );
            }
            Ok(Event::End(_) | Event::Eof) => break,
            Ok(_) => continue,
            Err(e) => return Err(MmlError::Xml(e)),
        }
    }

    Ok(doc)
}

fn parse_section(
    reader: &mut Reader<&[u8]>,
    attributes: HashMap<String, String>,
) -> Result<MmlSection, MmlError> {
    let mut elements = Vec::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = element_name(&e);
                let attrs = parse_attributes(&e)?;
                let (children, text) = parse_element_children(reader)?;
                elements.push(MmlElement {
                    name,
                    attributes: attrs,
                    children,
                    text,
                });
            }
            Ok(Event::Empty(e)) => {
                elements.push(MmlElement {
                    name: element_name(&e),
                    attributes: parse_attributes(&e)?,
                    children: Vec::new(),
                    text: None,
                });
            }
            Ok(Event::End(_) | Event::Eof) => break,
            Ok(_) => continue,
            Err(e) => return Err(MmlError::Xml(e)),
        }
    }

    Ok(MmlSection {
        attributes,
        elements,
    })
}

fn parse_element_children(
    reader: &mut Reader<&[u8]>,
) -> Result<(Vec<MmlElement>, Option<String>), MmlError> {
    let mut children = Vec::new();
    let mut text_parts = Vec::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = element_name(&e);
                let attrs = parse_attributes(&e)?;
                let (nested, text) = parse_element_children(reader)?;
                children.push(MmlElement {
                    name,
                    attributes: attrs,
                    children: nested,
                    text,
                });
            }
            Ok(Event::Empty(e)) => {
                children.push(MmlElement {
                    name: element_name(&e),
                    attributes: parse_attributes(&e)?,
                    children: Vec::new(),
                    text: None,
                });
            }
            Ok(Event::Text(t)) => {
                if let Ok(s) = t.unescape() {
                    let s = s.trim();
                    if !s.is_empty() {
                        text_parts.push(s.to_string());
                    }
                }
            }
            Ok(Event::CData(t)) => {
                let s = String::from_utf8_lossy(&t).trim().to_string();
                if !s.is_empty() {
                    text_parts.push(s);
                }
            }
            Ok(Event::End(_) | Event::Eof) => break,
            Ok(_) => continue,
            Err(e) => return Err(MmlError::Xml(e)),
        }
    }

    let text = if text_parts.is_empty() {
        None
    } else {
        Some(text_parts.join(""))
    };
    Ok((children, text))
}

fn set_section(doc: &mut MmlDocument, name: &str, section: MmlSection) {
    match name {
        "stringset" => doc.stringset = Some(section),
        "interface" => doc.interface = Some(section),
        "motion_sensor" => doc.motion_sensor = Some(section),
        "overhead_map" => doc.overhead_map = Some(section),
        "infravision" => doc.infravision = Some(section),
        "animated_textures" => doc.animated_textures = Some(section),
        "control_panels" => doc.control_panels = Some(section),
        "platforms" => doc.platforms = Some(section),
        "liquids" => doc.liquids = Some(section),
        "sounds" => doc.sounds = Some(section),
        "faders" => doc.faders = Some(section),
        "player" => doc.player = Some(section),
        "weapons" => doc.weapons = Some(section),
        "items" => doc.items = Some(section),
        "monsters" => doc.monsters = Some(section),
        "scenery" => doc.scenery = Some(section),
        "landscapes" => doc.landscapes = Some(section),
        "texture_loading" => doc.texture_loading = Some(section),
        "opengl" => doc.opengl = Some(section),
        "software" => doc.software = Some(section),
        "dynamic_limits" => doc.dynamic_limits = Some(section),
        "scenario" => doc.scenario = Some(section),
        "console" => doc.console = Some(section),
        "logging" => doc.logging = Some(section),
        _ => {} // silently ignore unrecognized sections
    }
}

fn wrap_error_with_source(err: MmlError, source: &str) -> MmlError {
    match err {
        MmlError::Xml(xml_err) => MmlError::XmlWithContext {
            source: source.to_string(),
            message: XmlContextMessage(xml_err.to_string()),
        },
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_marathon_root() {
        let xml = b"<marathon><weapons></weapons></marathon>";
        let doc = MmlDocument::from_bytes(xml).unwrap();
        assert!(doc.weapons.is_some());
        assert!(doc.monsters.is_none());
    }

    #[test]
    fn test_wrong_root_element() {
        let xml = b"<config><weapons/></config>";
        let err = MmlDocument::from_bytes(xml).unwrap_err();
        match err {
            MmlError::InvalidRootElement(name) => assert_eq!(name, "config"),
            other => panic!("expected InvalidRootElement, got {:?}", other),
        }
    }

    #[test]
    fn test_empty_marathon_element() {
        let doc = MmlDocument::from_bytes(b"<marathon></marathon>").unwrap();
        assert!(doc.is_empty());
    }

    #[test]
    fn test_self_closing_marathon() {
        let doc = MmlDocument::from_bytes(b"<marathon/>").unwrap();
        assert!(doc.is_empty());
    }

    #[test]
    fn test_single_section_with_attributes() {
        let xml = b"<marathon><weapons><weapon index=\"0\" speed=\"100\"/></weapons></marathon>";
        let doc = MmlDocument::from_bytes(xml).unwrap();
        let section = doc.weapons.unwrap();
        assert_eq!(section.elements.len(), 1);
        assert_eq!(section.elements[0].name, "weapon");
        assert_eq!(section.elements[0].attributes.get("index").unwrap(), "0");
        assert_eq!(section.elements[0].attributes.get("speed").unwrap(), "100");
    }

    #[test]
    fn test_multiple_sections() {
        let xml = b"<marathon><monsters/><weapons/><dynamic_limits/></marathon>";
        let doc = MmlDocument::from_bytes(xml).unwrap();
        assert!(doc.monsters.is_some());
        assert!(doc.weapons.is_some());
        assert!(doc.dynamic_limits.is_some());
        assert!(doc.sounds.is_none());
    }

    #[test]
    fn test_unrecognized_elements_ignored() {
        let xml =
            b"<marathon><weapons/><custom_extension foo=\"bar\"><child/></custom_extension></marathon>";
        let doc = MmlDocument::from_bytes(xml).unwrap();
        assert!(doc.weapons.is_some());
    }

    #[test]
    fn test_nested_elements() {
        let xml = br#"<marathon><console><carnage_message projectile_type="0"><detail>test</detail></carnage_message></console></marathon>"#;
        let doc = MmlDocument::from_bytes(xml).unwrap();
        let console = doc.console.unwrap();
        assert_eq!(console.elements.len(), 1);
        let msg = &console.elements[0];
        assert_eq!(msg.name, "carnage_message");
        assert_eq!(msg.attributes.get("projectile_type").unwrap(), "0");
        assert_eq!(msg.children.len(), 1);
        assert_eq!(msg.children[0].name, "detail");
        assert_eq!(msg.children[0].text.as_deref(), Some("test"));
    }

    #[test]
    fn test_layer_override() {
        let base = MmlDocument::from_bytes(
            b"<marathon><weapons><weapon index=\"0\"/></weapons><monsters><monster index=\"0\"/></monsters></marathon>",
        )
        .unwrap();
        let overlay = MmlDocument::from_bytes(
            b"<marathon><weapons><weapon index=\"1\"/></weapons></marathon>",
        )
        .unwrap();

        let result = MmlDocument::layer(base, overlay);
        // Element-level merge: the overlay adds weapon index="1" without
        // dropping the base's weapon index="0" — both are present now.
        let weapons = result.weapons.unwrap();
        assert!(
            weapons.find_element("weapon", "0").is_some(),
            "base weapon index=0 should be preserved"
        );
        assert!(
            weapons.find_element("weapon", "1").is_some(),
            "overlay weapon index=1 should be added"
        );
        assert_eq!(weapons.elements.len(), 2);
        // The base's monsters section (absent from the overlay) is carried over.
        let monsters = result.monsters.unwrap();
        assert_eq!(monsters.elements[0].attributes.get("index").unwrap(), "0");
    }

    #[test]
    fn test_layer_add_new_section() {
        let base = MmlDocument::from_bytes(
            b"<marathon><monsters><monster index=\"0\"/></monsters></marathon>",
        )
        .unwrap();
        let overlay = MmlDocument::from_bytes(
            b"<marathon><weapons><weapon index=\"0\"/></weapons></marathon>",
        )
        .unwrap();

        let result = MmlDocument::layer(base, overlay);
        assert!(result.monsters.is_some());
        assert!(result.weapons.is_some());
    }

    #[test]
    fn test_layer_preserve_absent_overlay() {
        let base =
            MmlDocument::from_bytes(b"<marathon><monsters/><weapons/><dynamic_limits/></marathon>")
                .unwrap();
        let overlay = MmlDocument::from_bytes(b"<marathon><weapons/></marathon>").unwrap();

        let result = MmlDocument::layer(base, overlay);
        assert!(result.monsters.is_some());
        assert!(result.weapons.is_some());
        assert!(result.dynamic_limits.is_some());
    }

    #[test]
    fn test_layer_three_documents() {
        let base = MmlDocument::from_bytes(b"<marathon><monsters/></marathon>").unwrap();
        let scenario = MmlDocument::from_bytes(b"<marathon><weapons/></marathon>").unwrap();
        let plugin = MmlDocument::from_bytes(b"<marathon><sounds/></marathon>").unwrap();

        let result = MmlDocument::layer(MmlDocument::layer(base, scenario), plugin);
        assert!(result.monsters.is_some());
        assert!(result.weapons.is_some());
        assert!(result.sounds.is_some());
    }

    #[test]
    fn test_merge_modifies_one_element_among_many() {
        let base = MmlDocument::from_bytes(
            b"<marathon><monsters><monster index=\"0\" vitality=\"10\"/><monster index=\"1\" vitality=\"20\"/><monster index=\"2\" vitality=\"30\"/></monsters></marathon>",
        )
        .unwrap();
        let overlay = MmlDocument::from_bytes(
            b"<marathon><monsters><monster index=\"1\" vitality=\"99\"/></monsters></marathon>",
        )
        .unwrap();

        let monsters = MmlDocument::layer(base, overlay).monsters.unwrap();
        assert_eq!(monsters.elements.len(), 3, "siblings preserved");
        assert_eq!(
            monsters.find_element("monster", "0").unwrap().attributes["vitality"],
            "10"
        );
        assert_eq!(
            monsters.find_element("monster", "1").unwrap().attributes["vitality"],
            "99"
        );
        assert_eq!(
            monsters.find_element("monster", "2").unwrap().attributes["vitality"],
            "30"
        );
    }

    #[test]
    fn test_merge_adds_new_indexed_element() {
        let base = MmlDocument::from_bytes(
            b"<marathon><monsters><monster index=\"0\"/></monsters></marathon>",
        )
        .unwrap();
        let overlay = MmlDocument::from_bytes(
            b"<marathon><monsters><monster index=\"5\"/></monsters></marathon>",
        )
        .unwrap();

        let monsters = MmlDocument::layer(base, overlay).monsters.unwrap();
        assert_eq!(monsters.elements.len(), 2);
        assert!(monsters.find_element("monster", "0").is_some());
        assert!(monsters.find_element("monster", "5").is_some());
    }

    #[test]
    fn test_merge_attribute_level_preserves_unmentioned() {
        let base = MmlDocument::from_bytes(
            b"<marathon><monsters><monster index=\"0\" vitality=\"100\" speed=\"5\"/></monsters></marathon>",
        )
        .unwrap();
        let overlay = MmlDocument::from_bytes(
            b"<marathon><monsters><monster index=\"0\" vitality=\"200\"/></monsters></marathon>",
        )
        .unwrap();

        let monsters = MmlDocument::layer(base, overlay).monsters.unwrap();
        let m = monsters.find_element("monster", "0").unwrap();
        assert_eq!(m.attributes["vitality"], "200", "overlay wins");
        assert_eq!(m.attributes["speed"], "5", "base-only attribute preserved");
    }

    #[test]
    fn test_merge_recursive_child_merge() {
        let base = MmlDocument::from_bytes(
            b"<marathon><weapons><weapon index=\"0\"><trigger index=\"0\" rounds=\"10\"/></weapon></weapons></marathon>",
        )
        .unwrap();
        let overlay = MmlDocument::from_bytes(
            b"<marathon><weapons><weapon index=\"0\"><trigger index=\"0\" rounds=\"20\"/><trigger index=\"1\" rounds=\"5\"/></weapon></weapons></marathon>",
        )
        .unwrap();

        let weapons = MmlDocument::layer(base, overlay).weapons.unwrap();
        let weapon = weapons.find_element("weapon", "0").unwrap();
        assert_eq!(weapon.children.len(), 2, "child trigger index=1 appended");
        let t0 = weapon
            .children
            .iter()
            .find(|c| c.attributes.get("index").map(String::as_str) == Some("0"))
            .unwrap();
        assert_eq!(
            t0.attributes["rounds"], "20",
            "matched child merged recursively"
        );
    }

    #[test]
    fn test_merge_three_layer_cascade_accumulates_attributes() {
        let base = MmlDocument::from_bytes(
            b"<marathon><player><item index=\"0\" amount=\"1\"/></player></marathon>",
        )
        .unwrap();
        let o1 = MmlDocument::from_bytes(
            b"<marathon><player><item index=\"0\" kind=\"pistol\"/></player></marathon>",
        )
        .unwrap();
        let o2 = MmlDocument::from_bytes(
            b"<marathon><player><item index=\"0\" count=\"3\"/></player></marathon>",
        )
        .unwrap();

        let player = MmlDocument::layer(MmlDocument::layer(base, o1), o2)
            .player
            .unwrap();
        let item = player.find_element("item", "0").unwrap();
        assert_eq!(item.attributes["amount"], "1");
        assert_eq!(item.attributes["kind"], "pistol");
        assert_eq!(item.attributes["count"], "3");
    }

    #[test]
    fn test_null_terminated_data() {
        let mut data = b"<marathon><weapons/></marathon>".to_vec();
        data.extend_from_slice(&[0, 0, 0]);
        let doc = MmlDocument::from_bytes(&data).unwrap();
        assert!(doc.weapons.is_some());
    }

    #[test]
    fn test_xml_declaration_handled() {
        let xml = b"<?xml version=\"1.0\"?><marathon><weapons/></marathon>";
        let doc = MmlDocument::from_bytes(xml).unwrap();
        assert!(doc.weapons.is_some());
    }

    #[test]
    fn test_comment_only_document() {
        let xml = b"<marathon><!-- just a comment --></marathon>";
        let doc = MmlDocument::from_bytes(xml).unwrap();
        assert!(doc.is_empty());
    }

    #[test]
    fn test_malformed_xml_error() {
        let xml = b"<marathon><weapons <<broken></marathon>";
        assert!(MmlDocument::from_bytes(xml).is_err());
    }

    #[test]
    fn test_unclosed_tag() {
        let xml = b"<marathon><weapons>";
        // Should not panic; may produce error or partial result
        let _ = MmlDocument::from_bytes(xml);
    }

    #[test]
    fn test_error_with_source_context() {
        let xml = b"<marathon><weapons <<broken></marathon>";
        let result = MmlDocument::from_bytes_with_source(xml, "test.mml");
        match result {
            Err(MmlError::XmlWithContext { source, .. }) => {
                assert_eq!(source, "test.mml");
            }
            Err(MmlError::InvalidRootElement(_)) => {}
            other => panic!("expected error, got {:?}", other),
        }
    }

    #[test]
    fn test_all_recognized_sections() {
        let sections = [
            "stringset",
            "interface",
            "motion_sensor",
            "overhead_map",
            "infravision",
            "animated_textures",
            "control_panels",
            "platforms",
            "liquids",
            "sounds",
            "faders",
            "player",
            "weapons",
            "items",
            "monsters",
            "scenery",
            "landscapes",
            "texture_loading",
            "opengl",
            "software",
            "dynamic_limits",
            "scenario",
            "console",
            "logging",
        ];
        for section_name in sections {
            let xml = format!("<marathon><{}/></marathon>", section_name);
            let doc = MmlDocument::from_bytes(xml.as_bytes()).unwrap();
            assert!(
                !doc.is_empty(),
                "section '{}' should be recognized",
                section_name
            );
        }
    }

    #[test]
    fn test_embedded_mmls_extraction() {
        // Test the from_bytes path with data that simulates WAD MMLS content
        let mut data = b"<marathon><weapons><weapon index=\"5\"/></weapons></marathon>".to_vec();
        data.push(0); // trailing null as in WAD chunks
        let doc = MmlDocument::from_bytes(&data).unwrap();
        let weapons = doc.weapons.unwrap();
        assert_eq!(weapons.elements[0].attributes.get("index").unwrap(), "5");
    }
}
