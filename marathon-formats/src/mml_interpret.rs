//! MML interpretation layer.
//!
//! The parser in [`crate::mml`] is purely structural: it turns MML/XML bytes
//! into [`MmlElement`](crate::mml::MmlElement) trees without interpreting any
//! attribute values. This module reads those trees and produces typed override
//! structs for each recognized section.
//!
//! Attribute parsing follows AlephOne's lenient conventions: integers may be
//! decimal or hex (`0x` prefix), booleans accept `1`/`t`/`true` and
//! `0`/`f`/`false`, and a malformed value logs a warning and yields `None`
//! rather than failing the whole document — matching decades of community MML
//! written against AlephOne's forgiving parser.

use crate::mml::MmlSection;

/// Emit a non-fatal warning for a malformed attribute value.
///
/// `marathon-formats` has no `log`/`tracing` dependency, so warnings go to
/// stderr. Interpretation never fails on a bad value; it returns `None` and
/// lets the caller fall back to the engine default.
fn warn_malformed(kind: &str, raw: &str) {
    eprintln!("[mml] warning: malformed {kind} attribute value: {raw:?}");
}

/// Split a trimmed integer literal into `(radix, digits)`, honoring an optional
/// sign and an AlephOne-style `0x`/`0X` hex prefix. The returned `digits` string
/// is suitable for `from_str_radix` (sign preserved, prefix stripped).
fn normalize_int(s: &str) -> (u32, String) {
    let t = s.trim();
    let (sign, rest) = match t.strip_prefix('-') {
        Some(r) => ("-", r),
        None => match t.strip_prefix('+') {
            Some(r) => ("", r),
            None => ("", t),
        },
    };
    match rest.strip_prefix("0x").or_else(|| rest.strip_prefix("0X")) {
        Some(hex) => (16, format!("{sign}{hex}")),
        None => (10, t.to_string()),
    }
}

/// Parse an MML attribute as `i16` (decimal or `0x` hex). Returns `None` and
/// warns on a malformed or out-of-range value.
pub fn parse_mml_i16(s: &str) -> Option<i16> {
    let (radix, digits) = normalize_int(s);
    match i16::from_str_radix(&digits, radix) {
        Ok(v) => Some(v),
        Err(_) => {
            warn_malformed("i16", s);
            None
        }
    }
}

/// Parse an MML attribute as `i32` (decimal or `0x` hex). Returns `None` and
/// warns on a malformed or out-of-range value.
pub fn parse_mml_i32(s: &str) -> Option<i32> {
    let (radix, digits) = normalize_int(s);
    match i32::from_str_radix(&digits, radix) {
        Ok(v) => Some(v),
        Err(_) => {
            warn_malformed("i32", s);
            None
        }
    }
}

/// Parse an MML attribute as `u32` (decimal or `0x` hex). Negative values are
/// rejected. Returns `None` and warns on a malformed or out-of-range value.
pub fn parse_mml_u32(s: &str) -> Option<u32> {
    let (radix, digits) = normalize_int(s);
    match u32::from_str_radix(&digits, radix) {
        Ok(v) => Some(v),
        Err(_) => {
            warn_malformed("u32", s);
            None
        }
    }
}

/// Parse an MML attribute as `f32` (decimal). Returns `None` and warns on a
/// malformed value.
pub fn parse_mml_f32(s: &str) -> Option<f32> {
    match s.trim().parse::<f32>() {
        Ok(v) => Some(v),
        Err(_) => {
            warn_malformed("f32", s);
            None
        }
    }
}

/// Parse an MML attribute as `bool`. Accepts `1`/`t`/`true` (case-insensitive)
/// for true and `0`/`f`/`false` for false. Returns `None` and warns otherwise.
pub fn parse_mml_bool(s: &str) -> Option<bool> {
    match s.trim().to_ascii_lowercase().as_str() {
        "1" | "t" | "true" => Some(true),
        "0" | "f" | "false" => Some(false),
        _ => {
            warn_malformed("bool", s);
            None
        }
    }
}

/// Overrides for one `<monster index="N">` element. Each field's inner type
/// matches the corresponding [`MonsterDefinition`](crate::physics::MonsterDefinition)
/// field so an override can be applied directly; `None` means "leave the engine
/// default in place".
///
/// `class` maps to `MonsterDefinition::monster_class` (renamed because `class`
/// is the MML attribute name). `immunities`/`weaknesses`/`flags` are bitfields
/// (`u32`). `must_be_exterminated` has no dedicated `MonsterDefinition` field —
/// it is a placement/objective attribute carried alongside the definition — so
/// it is modeled as a standalone `Option<bool>`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MonsterOverride {
    pub index: usize,
    pub vitality: Option<i16>,
    pub immunities: Option<u32>,
    pub weaknesses: Option<u32>,
    pub flags: Option<u32>,
    pub class: Option<i32>,
    pub friends: Option<i32>,
    pub enemies: Option<i32>,
    pub sound_pitch: Option<f32>,
    pub speed: Option<i16>,
    pub radius: Option<i16>,
    pub height: Option<i16>,
    pub visual_range: Option<i16>,
    pub dark_visual_range: Option<i16>,
    pub half_visual_arc: Option<i16>,
    pub half_vertical_visual_arc: Option<i16>,
    pub intelligence: Option<i16>,
    pub carrying_item_type: Option<i16>,
    pub must_be_exterminated: Option<bool>,
}

/// Interpret a merged `<monsters>` section into per-monster overrides. Each
/// `<monster>` element's `index` attribute selects which monster definition to
/// override; elements without a parseable `index` are skipped with a warning.
/// Each recognized attribute is parsed into the corresponding typed field;
/// unrecognized attributes are silently ignored, and a malformed value warns
/// and leaves that field `None` without discarding the rest of the element.
pub fn interpret_monsters(section: &MmlSection) -> Vec<MonsterOverride> {
    let mut out = Vec::new();
    for el in &section.elements {
        if el.name != "monster" {
            continue;
        }
        let index = match el.attributes.get("index") {
            Some(raw) => match parse_mml_u32(raw) {
                Some(i) => i as usize,
                None => continue, // parse_mml_u32 already warned
            },
            None => {
                eprintln!("[mml] warning: <monster> element without an index attribute, skipping");
                continue;
            }
        };
        out.push(MonsterOverride {
            index,
            vitality: el.attributes.get("vitality").and_then(|s| parse_mml_i16(s)),
            immunities: el
                .attributes
                .get("immunities")
                .and_then(|s| parse_mml_u32(s)),
            weaknesses: el
                .attributes
                .get("weaknesses")
                .and_then(|s| parse_mml_u32(s)),
            flags: el.attributes.get("flags").and_then(|s| parse_mml_u32(s)),
            class: el.attributes.get("class").and_then(|s| parse_mml_i32(s)),
            friends: el.attributes.get("friends").and_then(|s| parse_mml_i32(s)),
            enemies: el.attributes.get("enemies").and_then(|s| parse_mml_i32(s)),
            sound_pitch: el
                .attributes
                .get("sound_pitch")
                .and_then(|s| parse_mml_f32(s)),
            speed: el.attributes.get("speed").and_then(|s| parse_mml_i16(s)),
            radius: el.attributes.get("radius").and_then(|s| parse_mml_i16(s)),
            height: el.attributes.get("height").and_then(|s| parse_mml_i16(s)),
            visual_range: el
                .attributes
                .get("visual_range")
                .and_then(|s| parse_mml_i16(s)),
            dark_visual_range: el
                .attributes
                .get("dark_visual_range")
                .and_then(|s| parse_mml_i16(s)),
            half_visual_arc: el
                .attributes
                .get("half_visual_arc")
                .and_then(|s| parse_mml_i16(s)),
            half_vertical_visual_arc: el
                .attributes
                .get("half_vertical_visual_arc")
                .and_then(|s| parse_mml_i16(s)),
            intelligence: el
                .attributes
                .get("intelligence")
                .and_then(|s| parse_mml_i16(s)),
            carrying_item_type: el
                .attributes
                .get("carrying_item_type")
                .and_then(|s| parse_mml_i16(s)),
            must_be_exterminated: el
                .attributes
                .get("must_be_exterminated")
                .and_then(|s| parse_mml_bool(s)),
        });
    }
    out
}

/// Overrides for the `<dynamic_limits>` section. Each field mirrors an AlephOne
/// dynamic-limit slot; `None` means "leave the engine default in place".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DynamicLimitsOverride {
    pub objects: Option<i32>,
    pub monsters: Option<i32>,
    pub paths: Option<i32>,
    pub projectiles: Option<i32>,
    pub effects: Option<i32>,
    pub rendered: Option<i32>,
    pub local_collision: Option<i32>,
    pub global_collision: Option<i32>,
}

/// Interpret a merged `<dynamic_limits>` section. Each recognized child
/// element's text content is parsed as an integer (`<monsters>1024</monsters>`);
/// unrecognized elements are ignored, malformed values warn and yield `None`.
pub fn interpret_dynamic_limits(section: &MmlSection) -> DynamicLimitsOverride {
    let mut out = DynamicLimitsOverride::default();
    for el in &section.elements {
        let Some(text) = el.text.as_deref() else {
            continue;
        };
        let value = parse_mml_i32(text);
        match el.name.as_str() {
            "objects" => out.objects = value,
            "monsters" => out.monsters = value,
            "paths" => out.paths = value,
            "projectiles" => out.projectiles = value,
            "effects" => out.effects = value,
            "rendered" => out.rendered = value,
            "local_collision" => out.local_collision = value,
            "global_collision" => out.global_collision = value,
            _ => {}
        }
    }
    out
}

/// Overrides for one `<item index="N">` element. `item_type` carries the `type`
/// attribute (renamed to avoid the Rust keyword); `singular`/`plural` are the
/// display names. `None` fields leave the engine default.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ItemOverride {
    pub index: usize,
    pub item_type: Option<i32>,
    pub singular: Option<String>,
    pub plural: Option<String>,
    pub maximum: Option<i32>,
    pub invalid: Option<bool>,
}

/// Interpret a merged `<items>` section into per-item overrides. `<item>`
/// elements without a parseable `index` attribute are skipped with a warning.
pub fn interpret_items(section: &MmlSection) -> Vec<ItemOverride> {
    let mut out = Vec::new();
    for el in &section.elements {
        if el.name != "item" {
            continue;
        }
        let index = match el.attributes.get("index") {
            Some(raw) => match parse_mml_u32(raw) {
                Some(i) => i as usize,
                None => continue, // parse_mml_u32 already warned
            },
            None => {
                eprintln!("[mml] warning: <item> element without an index attribute, skipping");
                continue;
            }
        };
        out.push(ItemOverride {
            index,
            item_type: el.attributes.get("type").and_then(|s| parse_mml_i32(s)),
            singular: el.attributes.get("singular").cloned(),
            plural: el.attributes.get("plural").cloned(),
            maximum: el.attributes.get("maximum").and_then(|s| parse_mml_i32(s)),
            invalid: el.attributes.get("invalid").and_then(|s| parse_mml_bool(s)),
        });
    }
    out
}

/// Overrides for one `<landscape>` element. `collection` carries the `coll`
/// attribute; the `*_exp` fields are integer exponents. `vert_repeat` parses
/// as a bool (accepting `true`/`false` or `1`/`0`) and `azimuth` as a float
/// (accepting integer or fractional notation). `None` leaves the default.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LandscapeOverride {
    pub collection: Option<i32>,
    pub frame: Option<i32>,
    pub horiz_exp: Option<i32>,
    pub vert_exp: Option<i32>,
    pub ogl_asprat_exp: Option<i32>,
    pub vert_repeat: Option<bool>,
    pub azimuth: Option<f32>,
}

/// Result of interpreting a `<landscapes>` section: the per-`<landscape>`
/// overrides plus the collection indices named by `<clear>` directives.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LandscapesOverride {
    pub landscapes: Vec<LandscapeOverride>,
    pub clears: Vec<i32>,
}

/// Interpret a merged `<landscapes>` section into landscape overrides and
/// `<clear>` directives. `<landscape>` elements produce a [`LandscapeOverride`];
/// `<clear coll="N"/>` elements add `N` to the clear list; other elements are
/// ignored.
pub fn interpret_landscapes(section: &MmlSection) -> LandscapesOverride {
    let mut out = LandscapesOverride::default();
    for el in &section.elements {
        match el.name.as_str() {
            "landscape" => out.landscapes.push(LandscapeOverride {
                collection: el.attributes.get("coll").and_then(|s| parse_mml_i32(s)),
                frame: el.attributes.get("frame").and_then(|s| parse_mml_i32(s)),
                horiz_exp: el
                    .attributes
                    .get("horiz_exp")
                    .and_then(|s| parse_mml_i32(s)),
                vert_exp: el.attributes.get("vert_exp").and_then(|s| parse_mml_i32(s)),
                ogl_asprat_exp: el
                    .attributes
                    .get("ogl_asprat_exp")
                    .and_then(|s| parse_mml_i32(s)),
                vert_repeat: el
                    .attributes
                    .get("vert_repeat")
                    .and_then(|s| parse_mml_bool(s)),
                azimuth: el.attributes.get("azimuth").and_then(|s| parse_mml_f32(s)),
            }),
            "clear" => {
                if let Some(coll) = el.attributes.get("coll").and_then(|s| parse_mml_i32(s)) {
                    out.clears.push(coll);
                }
            }
            _ => {}
        }
    }
    out
}

/// One `<texture_env index="N" which="W" coll="C"/>` entry under
/// `<texture_loading>`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextureEnvOverride {
    pub index: Option<i32>,
    pub which: Option<i32>,
    pub coll: Option<i32>,
}

/// Overrides for the `<texture_loading>` section: the section-level
/// `landscapes` boolean plus the list of `<texture_env>` entries.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextureLoadingOverride {
    pub landscapes: Option<bool>,
    pub texture_envs: Vec<TextureEnvOverride>,
}

/// Interpret a merged `<texture_loading>` section. The `landscapes` flag is read
/// from the section element's own attributes; each `<texture_env>` child becomes
/// a [`TextureEnvOverride`].
pub fn interpret_texture_loading(section: &MmlSection) -> TextureLoadingOverride {
    let landscapes = section
        .attributes
        .get("landscapes")
        .and_then(|s| parse_mml_bool(s));
    let mut texture_envs = Vec::new();
    for el in &section.elements {
        if el.name != "texture_env" {
            continue;
        }
        texture_envs.push(TextureEnvOverride {
            index: el.attributes.get("index").and_then(|s| parse_mml_i32(s)),
            which: el.attributes.get("which").and_then(|s| parse_mml_i32(s)),
            coll: el.attributes.get("coll").and_then(|s| parse_mml_i32(s)),
        });
    }
    TextureLoadingOverride {
        landscapes,
        texture_envs,
    }
}

/// Overrides for the `<scenario>` section identity. `name`/`id` are free-form
/// strings (AlephOne treats the scenario id as an opaque identifier); `version`
/// is an integer. `None` leaves the engine default.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScenarioIdOverride {
    pub name: Option<String>,
    pub version: Option<i32>,
    pub id: Option<String>,
}

/// Interpret a `<scenario name="..." version="..." id="...">` section. All three
/// values live on the section element's own attributes.
pub fn interpret_scenario(section: &MmlSection) -> ScenarioIdOverride {
    ScenarioIdOverride {
        name: section.attributes.get("name").cloned(),
        version: section
            .attributes
            .get("version")
            .and_then(|s| parse_mml_i32(s)),
        id: section.attributes.get("id").cloned(),
    }
}

/// A `<stringset index="R">` override: each entry maps a
/// `(resource_id, string_index)` pair to its replacement text. One
/// [`StringSetOverride`] corresponds to a single `<stringset>` section (one
/// resource id, conventionally 128–149).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StringSetOverride {
    pub entries: Vec<((i32, i32), String)>,
}

/// Interpret a merged `<stringset>` section. The resource id is read from the
/// section element's own `index` attribute; each child `<string index="N">text
/// </string>` contributes a `((resource_id, N), text)` entry. A section without
/// a parseable `index` yields no entries (warned). Interpreting multiple
/// `<stringset>` sections from one document is cascade-level work handled
/// elsewhere; this operates on a single section per its signature.
pub fn interpret_stringset(section: &MmlSection) -> StringSetOverride {
    let resource_id = match section.attributes.get("index") {
        Some(raw) => match parse_mml_i32(raw) {
            Some(id) => id,
            None => return StringSetOverride::default(), // parse_mml_i32 warned
        },
        None => {
            eprintln!("[mml] warning: <stringset> without an index attribute, skipping");
            return StringSetOverride::default();
        }
    };
    let mut entries = Vec::new();
    for el in &section.elements {
        if el.name != "string" {
            continue;
        }
        let Some(idx) = el.attributes.get("index").and_then(|s| parse_mml_i32(s)) else {
            continue;
        };
        let text = el.text.clone().unwrap_or_default();
        entries.push(((resource_id, idx), text));
    }
    StringSetOverride { entries }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mml::MmlDocument;

    #[test]
    fn i16_decimal_and_hex() {
        assert_eq!(parse_mml_i16("100"), Some(100));
        assert_eq!(parse_mml_i16("-100"), Some(-100));
        assert_eq!(parse_mml_i16("  42 "), Some(42)); // whitespace tolerated
        assert_eq!(parse_mml_i16("0x10"), Some(16));
        assert_eq!(parse_mml_i16("0XFF"), Some(255));
        assert_eq!(parse_mml_i16("-0x10"), Some(-16));
    }

    #[test]
    fn i16_rejects_malformed_and_overflow() {
        assert_eq!(parse_mml_i16("abc"), None);
        assert_eq!(parse_mml_i16(""), None);
        assert_eq!(parse_mml_i16("70000"), None); // > i16::MAX
        assert_eq!(parse_mml_i16("0xZZ"), None);
    }

    #[test]
    fn i32_decimal_and_hex() {
        assert_eq!(parse_mml_i32("2147483647"), Some(i32::MAX));
        assert_eq!(parse_mml_i32("-5"), Some(-5));
        assert_eq!(parse_mml_i32("0x7FFFFFFF"), Some(i32::MAX));
        assert_eq!(parse_mml_i32("nope"), None);
    }

    #[test]
    fn u32_decimal_hex_and_sign_rejection() {
        assert_eq!(parse_mml_u32("0"), Some(0));
        assert_eq!(parse_mml_u32("4294967295"), Some(u32::MAX));
        assert_eq!(parse_mml_u32("0xDEADBEEF"), Some(0xDEAD_BEEF));
        assert_eq!(parse_mml_u32("-1"), None); // unsigned rejects negative
        assert_eq!(parse_mml_u32("-0x1"), None);
    }

    #[test]
    fn f32_decimal() {
        assert_eq!(parse_mml_f32("1.5"), Some(1.5));
        assert_eq!(parse_mml_f32("-0.25"), Some(-0.25));
        assert_eq!(parse_mml_f32("  3 "), Some(3.0));
        assert_eq!(parse_mml_f32("0x1"), None); // no hex floats
        assert_eq!(parse_mml_f32("bad"), None);
    }

    #[test]
    fn bool_accepts_alephone_forms() {
        for t in ["1", "t", "true", "TRUE", "True", " t "] {
            assert_eq!(parse_mml_bool(t), Some(true), "{t:?} should be true");
        }
        for f in ["0", "f", "false", "FALSE", "False", " f "] {
            assert_eq!(parse_mml_bool(f), Some(false), "{f:?} should be false");
        }
        for bad in ["2", "yes", "no", "", "tru"] {
            assert_eq!(parse_mml_bool(bad), None, "{bad:?} should be None");
        }
    }

    // ── boxes 1.2/1.3: monsters interpreter ──

    #[test]
    fn monster_override_subset_of_attributes() {
        let doc = MmlDocument::from_bytes(
            b"<marathon><monsters><monster index=\"5\" vitality=\"300\" speed=\"10\"/></monsters></marathon>",
        )
        .unwrap();
        let monsters = interpret_monsters(&doc.monsters.unwrap());
        assert_eq!(monsters.len(), 1);
        assert_eq!(
            monsters[0],
            MonsterOverride {
                index: 5,
                vitality: Some(300),
                speed: Some(10),
                ..Default::default()
            }
        );
    }

    #[test]
    fn monster_override_without_index_is_skipped() {
        let doc = MmlDocument::from_bytes(
            b"<marathon><monsters><monster vitality=\"100\"/></monsters></marathon>",
        )
        .unwrap();
        let monsters = interpret_monsters(&doc.monsters.unwrap());
        assert!(monsters.is_empty(), "index-less <monster> is skipped");
    }

    #[test]
    fn monster_override_malformed_value_stays_none() {
        let doc = MmlDocument::from_bytes(
            b"<marathon><monsters><monster index=\"5\" vitality=\"not_a_number\"/></monsters></marathon>",
        )
        .unwrap();
        let monsters = interpret_monsters(&doc.monsters.unwrap());
        assert_eq!(monsters.len(), 1);
        assert_eq!(monsters[0].index, 5);
        assert_eq!(
            monsters[0].vitality, None,
            "malformed value -> None, element still produced"
        );
    }

    #[test]
    fn monster_override_full_attributes_and_ignores_unknown() {
        let doc = MmlDocument::from_bytes(
            b"<marathon><monsters><monster index=\"2\" vitality=\"250\" immunities=\"0x1F\" weaknesses=\"4\" flags=\"8\" class=\"16\" friends=\"32\" enemies=\"64\" sound_pitch=\"1.5\" speed=\"12\" radius=\"256\" height=\"128\" visual_range=\"30\" dark_visual_range=\"10\" half_visual_arc=\"60\" half_vertical_visual_arc=\"45\" intelligence=\"5\" carrying_item_type=\"3\" must_be_exterminated=\"true\" bogus_attr=\"99\"/></monsters></marathon>",
        )
        .unwrap();
        let monsters = interpret_monsters(&doc.monsters.unwrap());
        assert_eq!(monsters.len(), 1);
        let m = &monsters[0];
        assert_eq!(m.index, 2);
        assert_eq!(m.vitality, Some(250));
        assert_eq!(m.immunities, Some(31)); // 0x1F
        assert_eq!(m.weaknesses, Some(4));
        assert_eq!(m.flags, Some(8));
        assert_eq!(m.class, Some(16));
        assert_eq!(m.friends, Some(32));
        assert_eq!(m.enemies, Some(64));
        assert_eq!(m.sound_pitch, Some(1.5));
        assert_eq!(m.speed, Some(12));
        assert_eq!(m.radius, Some(256));
        assert_eq!(m.height, Some(128));
        assert_eq!(m.visual_range, Some(30));
        assert_eq!(m.dark_visual_range, Some(10));
        assert_eq!(m.half_visual_arc, Some(60));
        assert_eq!(m.half_vertical_visual_arc, Some(45));
        assert_eq!(m.intelligence, Some(5));
        assert_eq!(m.carrying_item_type, Some(3));
        assert_eq!(m.must_be_exterminated, Some(true));
        // `bogus_attr` is unrecognized and silently ignored — no panic, no field.
    }

    // ── box 1.8: dynamic_limits interpreter ──

    #[test]
    fn dynamic_limits_parses_child_text() {
        let doc = MmlDocument::from_bytes(
            b"<marathon><dynamic_limits><monsters>1024</monsters><projectiles>256</projectiles></dynamic_limits></marathon>",
        )
        .unwrap();
        let limits = interpret_dynamic_limits(&doc.dynamic_limits.unwrap());
        assert_eq!(limits.monsters, Some(1024));
        assert_eq!(limits.projectiles, Some(256));
        assert_eq!(limits.objects, None);
        assert_eq!(limits.global_collision, None);
    }

    #[test]
    fn dynamic_limits_ignores_unknown_and_malformed() {
        let doc = MmlDocument::from_bytes(
            b"<marathon><dynamic_limits><objects>900</objects><bogus>5</bogus><effects>oops</effects></dynamic_limits></marathon>",
        )
        .unwrap();
        let limits = interpret_dynamic_limits(&doc.dynamic_limits.unwrap());
        assert_eq!(limits.objects, Some(900));
        assert_eq!(limits.effects, None, "malformed value -> None");
        // `<bogus>` is simply not a recognized slot; no panic, no field set.
        assert_eq!(
            limits,
            DynamicLimitsOverride {
                objects: Some(900),
                ..Default::default()
            }
        );
    }

    // ── box 1.9: items interpreter ──

    #[test]
    fn item_maximum_override() {
        let doc = MmlDocument::from_bytes(
            b"<marathon><items><item index=\"7\" maximum=\"5\"/></items></marathon>",
        )
        .unwrap();
        let items = interpret_items(&doc.items.unwrap());
        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0],
            ItemOverride {
                index: 7,
                maximum: Some(5),
                ..Default::default()
            }
        );
    }

    #[test]
    fn item_full_attributes_and_skips_unindexed() {
        let doc = MmlDocument::from_bytes(
            b"<marathon><items><item index=\"3\" type=\"2\" singular=\"Magnum\" plural=\"Magnums\" maximum=\"9\" invalid=\"true\"/><item maximum=\"4\"/></items></marathon>",
        )
        .unwrap();
        let items = interpret_items(&doc.items.unwrap());
        assert_eq!(items.len(), 1, "the index-less <item> is skipped");
        let it = &items[0];
        assert_eq!(it.index, 3);
        assert_eq!(it.item_type, Some(2));
        assert_eq!(it.singular.as_deref(), Some("Magnum"));
        assert_eq!(it.plural.as_deref(), Some("Magnums"));
        assert_eq!(it.maximum, Some(9));
        assert_eq!(it.invalid, Some(true));
    }

    // ── box 1.10: landscapes interpreter ──

    #[test]
    fn landscape_assignment_override() {
        let doc = MmlDocument::from_bytes(
            b"<marathon><landscapes><landscape coll=\"27\" frame=\"0\" horiz_exp=\"1\"/></landscapes></marathon>",
        )
        .unwrap();
        let out = interpret_landscapes(&doc.landscapes.unwrap());
        assert_eq!(out.landscapes.len(), 1);
        assert_eq!(
            out.landscapes[0],
            LandscapeOverride {
                collection: Some(27),
                frame: Some(0),
                horiz_exp: Some(1),
                ..Default::default()
            }
        );
        assert!(out.clears.is_empty());
    }

    #[test]
    fn landscape_clear_directive() {
        let doc = MmlDocument::from_bytes(
            b"<marathon><landscapes><clear coll=\"5\"/><clear coll=\"9\"/></landscapes></marathon>",
        )
        .unwrap();
        let out = interpret_landscapes(&doc.landscapes.unwrap());
        assert!(out.landscapes.is_empty());
        assert_eq!(out.clears, vec![5, 9]);
    }

    #[test]
    fn landscape_vert_repeat_and_azimuth_accept_flexible_forms() {
        // vert_repeat accepts 1/0 or true/false; azimuth accepts int or float.
        let doc = MmlDocument::from_bytes(
            b"<marathon><landscapes><landscape coll=\"3\" vert_repeat=\"1\" azimuth=\"90\"/></landscapes></marathon>",
        )
        .unwrap();
        let out = interpret_landscapes(&doc.landscapes.unwrap());
        assert_eq!(out.landscapes[0].vert_repeat, Some(true));
        assert_eq!(out.landscapes[0].azimuth, Some(90.0));
    }

    // ── box 1.11: texture_loading interpreter (reads section attribute) ──

    #[test]
    fn texture_environment_override() {
        let doc = MmlDocument::from_bytes(
            b"<marathon><texture_loading landscapes=\"true\"><texture_env index=\"0\" which=\"1\" coll=\"17\"/></texture_loading></marathon>",
        )
        .unwrap();
        let out = interpret_texture_loading(&doc.texture_loading.unwrap());
        assert_eq!(
            out.landscapes,
            Some(true),
            "section-level attribute captured"
        );
        assert_eq!(out.texture_envs.len(), 1);
        assert_eq!(
            out.texture_envs[0],
            TextureEnvOverride {
                index: Some(0),
                which: Some(1),
                coll: Some(17)
            }
        );
    }

    // ── box 1.13: scenario interpreter (reads section attributes) ──

    #[test]
    fn scenario_identity_override() {
        let doc = MmlDocument::from_bytes(
            b"<marathon><scenario name=\"Marathon 2\" version=\"1\" id=\"M2\"/></marathon>",
        )
        .unwrap();
        let out = interpret_scenario(&doc.scenario.unwrap());
        assert_eq!(out.name.as_deref(), Some("Marathon 2"));
        assert_eq!(out.version, Some(1));
        assert_eq!(out.id.as_deref(), Some("M2"));
    }

    #[test]
    fn scenario_section_attributes_survive_layering() {
        // Confirms the parser now captures section-element attributes and that
        // layering merges them (overlay wins, base-only preserved).
        let base =
            MmlDocument::from_bytes(b"<marathon><scenario name=\"Base\" id=\"B\"/></marathon>")
                .unwrap();
        let overlay =
            MmlDocument::from_bytes(b"<marathon><scenario name=\"Override\"/></marathon>").unwrap();
        let merged = MmlDocument::layer(base, overlay);
        let out = interpret_scenario(&merged.scenario.unwrap());
        assert_eq!(out.name.as_deref(), Some("Override"), "overlay wins");
        assert_eq!(
            out.id.as_deref(),
            Some("B"),
            "base-only attribute preserved"
        );
    }

    // ── box 1.12: stringset interpreter (resource id from section attr) ──

    #[test]
    fn stringset_override_maps_resource_and_index() {
        let doc = MmlDocument::from_bytes(
            b"<marathon><stringset index=\"128\"><string index=\"0\">Custom error</string></stringset></marathon>",
        )
        .unwrap();
        let out = interpret_stringset(&doc.stringset.unwrap());
        assert_eq!(out.entries, vec![((128, 0), "Custom error".to_string())]);
    }

    #[test]
    fn stringset_multiple_strings_share_resource_id() {
        let doc = MmlDocument::from_bytes(
            b"<marathon><stringset index=\"131\"><string index=\"0\">Zero</string><string index=\"5\">Five</string></stringset></marathon>",
        )
        .unwrap();
        let out = interpret_stringset(&doc.stringset.unwrap());
        assert_eq!(
            out.entries,
            vec![
                ((131, 0), "Zero".to_string()),
                ((131, 5), "Five".to_string()),
            ]
        );
    }

    #[test]
    fn stringset_without_resource_id_is_empty() {
        let doc = MmlDocument::from_bytes(
            b"<marathon><stringset><string index=\"0\">orphan</string></stringset></marathon>",
        )
        .unwrap();
        let out = interpret_stringset(&doc.stringset.unwrap());
        assert!(out.entries.is_empty(), "no resource id -> no entries");
    }
}
