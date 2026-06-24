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

use crate::mml::{MmlDocument, MmlSection};

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

/// Overrides for one `<projectile index="N">` element. Each field's inner type
/// matches the corresponding
/// [`ProjectileDefinition`](crate::physics::ProjectileDefinition) field so an
/// override can be applied directly; `None` means "leave the engine default in
/// place".
///
/// DEVIATION: `ProjectileDefinition::damage` is a
/// [`DamageDefinition`](crate::types::DamageDefinition) sub-struct, not a single
/// scalar attribute, so it is **not** modeled here — only the scalar fields the
/// spec lists are mapped. (AlephOne expresses projectile damage as a nested
/// `<damage>` element, which is a richer cascade-merge concern handled
/// elsewhere.) `media_projectile_promotion` exists on the definition but is not
/// in the spec's attribute list, so it is also omitted.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProjectileOverride {
    pub index: usize,
    pub collection: Option<i16>,
    pub shape: Option<i16>,
    pub detonation_effect: Option<i16>,
    pub media_detonation_effect: Option<i16>,
    pub contrail_effect: Option<i16>,
    pub ticks_between_contrails: Option<i16>,
    pub maximum_contrails: Option<i16>,
    pub radius: Option<i16>,
    pub area_of_effect: Option<i16>,
    pub flags: Option<u32>,
    pub speed: Option<i16>,
    pub maximum_range: Option<i16>,
    pub sound_pitch: Option<f32>,
    pub flyby_sound: Option<i16>,
    pub rebound_sound: Option<i16>,
}

/// Interpret a merged `<projectiles>` section into per-projectile overrides.
/// Each `<projectile>` element's `index` attribute selects which projectile
/// definition to override; elements without a parseable `index` are skipped with
/// a warning. Each recognized attribute is parsed into the corresponding typed
/// field; unrecognized attributes (and the non-scalar `damage`) are silently
/// ignored, and a malformed value warns and leaves that field `None` without
/// discarding the rest of the element.
pub fn interpret_projectiles(section: &MmlSection) -> Vec<ProjectileOverride> {
    let mut out = Vec::new();
    for el in &section.elements {
        if el.name != "projectile" {
            continue;
        }
        let index = match el.attributes.get("index") {
            Some(raw) => match parse_mml_u32(raw) {
                Some(i) => i as usize,
                None => continue, // parse_mml_u32 already warned
            },
            None => {
                eprintln!(
                    "[mml] warning: <projectile> element without an index attribute, skipping"
                );
                continue;
            }
        };
        out.push(ProjectileOverride {
            index,
            collection: el
                .attributes
                .get("collection")
                .and_then(|s| parse_mml_i16(s)),
            shape: el.attributes.get("shape").and_then(|s| parse_mml_i16(s)),
            detonation_effect: el
                .attributes
                .get("detonation_effect")
                .and_then(|s| parse_mml_i16(s)),
            media_detonation_effect: el
                .attributes
                .get("media_detonation_effect")
                .and_then(|s| parse_mml_i16(s)),
            contrail_effect: el
                .attributes
                .get("contrail_effect")
                .and_then(|s| parse_mml_i16(s)),
            ticks_between_contrails: el
                .attributes
                .get("ticks_between_contrails")
                .and_then(|s| parse_mml_i16(s)),
            maximum_contrails: el
                .attributes
                .get("maximum_contrails")
                .and_then(|s| parse_mml_i16(s)),
            radius: el.attributes.get("radius").and_then(|s| parse_mml_i16(s)),
            area_of_effect: el
                .attributes
                .get("area_of_effect")
                .and_then(|s| parse_mml_i16(s)),
            flags: el.attributes.get("flags").and_then(|s| parse_mml_u32(s)),
            speed: el.attributes.get("speed").and_then(|s| parse_mml_i16(s)),
            maximum_range: el
                .attributes
                .get("maximum_range")
                .and_then(|s| parse_mml_i16(s)),
            sound_pitch: el
                .attributes
                .get("sound_pitch")
                .and_then(|s| parse_mml_f32(s)),
            flyby_sound: el
                .attributes
                .get("flyby_sound")
                .and_then(|s| parse_mml_i16(s)),
            rebound_sound: el
                .attributes
                .get("rebound_sound")
                .and_then(|s| parse_mml_i16(s)),
        });
    }
    out
}

/// Overrides for one `<effect index="N">` element. Each field's inner type
/// matches the corresponding
/// [`EffectDefinition`](crate::physics::EffectDefinition) field so an override
/// can be applied directly; `None` means "leave the engine default in place".
/// `flags` is a `u16` bitfield on the definition.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EffectOverride {
    pub index: usize,
    pub collection: Option<i16>,
    pub shape: Option<i16>,
    pub sound_pitch: Option<f32>,
    pub flags: Option<u16>,
    pub delay: Option<i16>,
    pub delay_sound: Option<i16>,
}

/// Interpret a merged `<effects>` section into per-effect overrides. Each
/// `<effect>` element's `index` attribute selects which effect definition to
/// override; elements without a parseable `index` are skipped with a warning.
/// Each recognized attribute is parsed into the corresponding typed field;
/// unrecognized attributes are silently ignored, and a malformed value warns and
/// leaves that field `None` without discarding the rest of the element.
pub fn interpret_effects(section: &MmlSection) -> Vec<EffectOverride> {
    let mut out = Vec::new();
    for el in &section.elements {
        if el.name != "effect" {
            continue;
        }
        let index = match el.attributes.get("index") {
            Some(raw) => match parse_mml_u32(raw) {
                Some(i) => i as usize,
                None => continue, // parse_mml_u32 already warned
            },
            None => {
                eprintln!("[mml] warning: <effect> element without an index attribute, skipping");
                continue;
            }
        };
        out.push(EffectOverride {
            index,
            collection: el
                .attributes
                .get("collection")
                .and_then(|s| parse_mml_i16(s)),
            shape: el.attributes.get("shape").and_then(|s| parse_mml_i16(s)),
            sound_pitch: el
                .attributes
                .get("sound_pitch")
                .and_then(|s| parse_mml_f32(s)),
            flags: el
                .attributes
                .get("flags")
                .and_then(|s| parse_mml_u32(s).and_then(|v| u16::try_from(v).ok())),
            delay: el.attributes.get("delay").and_then(|s| parse_mml_i16(s)),
            delay_sound: el
                .attributes
                .get("delay_sound")
                .and_then(|s| parse_mml_i16(s)),
        });
    }
    out
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

/// Overrides for one `<shell_casings index="N">` element under `<weapons>`.
///
/// AlephOne has no standalone shell-casing *definition* struct in this crate's
/// [`physics`](crate::physics) module (shell-casing state is engine-internal),
/// so every field is modeled as `Option<i16>` to match the integer values the
/// spec scenario uses. `collection` carries the `coll` attribute and `sequence`
/// carries `seq` (renamed for clarity); the remaining fields keep their MML
/// names. `x0`/`y0` are spawn offsets and `vx0`/`vy0`/`dvx`/`dvy` are velocity
/// and velocity-delta components (AlephOne fixed-point integers). `None` means
/// "leave the engine default in place".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShellCasingOverride {
    pub index: usize,
    pub collection: Option<i16>,
    pub sequence: Option<i16>,
    pub x0: Option<i16>,
    pub y0: Option<i16>,
    pub vx0: Option<i16>,
    pub vy0: Option<i16>,
    pub dvx: Option<i16>,
    pub dvy: Option<i16>,
}

/// One `<order index="S" weapon="W"/>` entry under `<weapons>`: it places weapon
/// `weapon` at cycling slot `index`. The spec scenario
/// (`<order index="0" weapon="3"/>`) maps a slot index to a weapon index, so
/// both values are captured. An `<order>` element without a parseable `index`
/// is skipped; a missing/malformed `weapon` leaves `weapon = None`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WeaponOrderEntry {
    pub index: usize,
    pub weapon: Option<usize>,
}

/// Result of interpreting a `<weapons>` section: the per-`<shell_casings>`
/// overrides plus the `<order>` (weapon cycling) entries.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WeaponOverrides {
    pub shell_casings: Vec<ShellCasingOverride>,
    pub order: Vec<WeaponOrderEntry>,
}

/// Interpret a merged `<weapons>` section into shell-casing overrides and the
/// weapon cycling order.
///
/// `<shell_casings>` elements are matched by `index` and parsed into typed
/// [`ShellCasingOverride`] structs; elements without a parseable `index` are
/// skipped with a warning, malformed attribute values warn and leave that field
/// `None`, and unrecognized attributes are ignored. `<order>` elements are
/// matched by `index` (the cycling slot) and produce [`WeaponOrderEntry`] values
/// carrying the `weapon` index; an `<order>` without a parseable `index` is
/// skipped. Any other child element is ignored.
pub fn interpret_weapons(section: &MmlSection) -> WeaponOverrides {
    let mut out = WeaponOverrides::default();
    for el in &section.elements {
        match el.name.as_str() {
            "shell_casings" => {
                let index = match el.attributes.get("index") {
                    Some(raw) => match parse_mml_u32(raw) {
                        Some(i) => i as usize,
                        None => continue, // parse_mml_u32 already warned
                    },
                    None => {
                        eprintln!(
                            "[mml] warning: <shell_casings> element without an index attribute, skipping"
                        );
                        continue;
                    }
                };
                out.shell_casings.push(ShellCasingOverride {
                    index,
                    collection: el.attributes.get("coll").and_then(|s| parse_mml_i16(s)),
                    sequence: el.attributes.get("seq").and_then(|s| parse_mml_i16(s)),
                    x0: el.attributes.get("x0").and_then(|s| parse_mml_i16(s)),
                    y0: el.attributes.get("y0").and_then(|s| parse_mml_i16(s)),
                    vx0: el.attributes.get("vx0").and_then(|s| parse_mml_i16(s)),
                    vy0: el.attributes.get("vy0").and_then(|s| parse_mml_i16(s)),
                    dvx: el.attributes.get("dvx").and_then(|s| parse_mml_i16(s)),
                    dvy: el.attributes.get("dvy").and_then(|s| parse_mml_i16(s)),
                });
            }
            "order" => {
                let index = match el.attributes.get("index") {
                    Some(raw) => match parse_mml_u32(raw) {
                        Some(i) => i as usize,
                        None => continue, // parse_mml_u32 already warned
                    },
                    None => {
                        eprintln!(
                            "[mml] warning: <order> element without an index attribute, skipping"
                        );
                        continue;
                    }
                };
                out.order.push(WeaponOrderEntry {
                    index,
                    weapon: el
                        .attributes
                        .get("weapon")
                        .and_then(|s| parse_mml_u32(s))
                        .map(|w| w as usize),
                });
            }
            _ => {}
        }
    }
    out
}

/// Placeholder value returned by the stub interpreters (box 1.14) for sections
/// that are parsed structurally but not yet interpreted into typed overrides
/// (`interface`, `motion_sensor`, `overhead_map`, `infravision`,
/// `animated_textures`, `control_panels`, `platforms`, `liquids`, `sounds`,
/// `faders`, `view`, `scenery`, `opengl`, `software`, `console`, `logging`).
///
/// Each stub interpreter logs a single "not yet implemented" warning and returns
/// a [`StubOverride`]. When real interpreters land they replace the stub and its
/// return type; until then [`StubOverride`] keeps the cascade-assembly code
/// (box 3.x) uniform without pretending these sections carry data.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StubOverride;

/// Emit the one-time "not yet implemented" notice for a stubbed section.
fn warn_stub(section_name: &str) {
    eprintln!("[mml] section <{section_name}> not yet implemented; ignoring overrides");
}

macro_rules! stub_interpreter {
    ($fn_name:ident, $section_name:literal) => {
        #[doc = concat!("Stub interpreter for the `<", $section_name, ">` section (box 1.14). ")]
        #[doc = "Logs a \"not yet implemented\" warning and returns an empty [`StubOverride`]."]
        pub fn $fn_name(_section: &MmlSection) -> StubOverride {
            warn_stub($section_name);
            StubOverride
        }
    };
}

stub_interpreter!(interpret_interface, "interface");
stub_interpreter!(interpret_motion_sensor, "motion_sensor");
stub_interpreter!(interpret_overhead_map, "overhead_map");
stub_interpreter!(interpret_infravision, "infravision");
stub_interpreter!(interpret_animated_textures, "animated_textures");
stub_interpreter!(interpret_control_panels, "control_panels");
stub_interpreter!(interpret_platforms, "platforms");
stub_interpreter!(interpret_liquids, "liquids");
stub_interpreter!(interpret_sounds, "sounds");
stub_interpreter!(interpret_faders, "faders");
stub_interpreter!(interpret_view, "view");
stub_interpreter!(interpret_scenery, "scenery");
stub_interpreter!(interpret_opengl, "opengl");
stub_interpreter!(interpret_software, "software");
stub_interpreter!(interpret_console, "console");
stub_interpreter!(interpret_logging, "logging");

/// The complete set of typed overrides interpreted from a single (already
/// layered) [`MmlDocument`] (boxes 3.1/3.2). Each field holds the result of the
/// matching `interpret_*` function for a populated section, or that override
/// type's default/empty value for an absent section.
///
/// Only sections with **implemented** interpreters get typed fields. The
/// remaining structurally-parsed sections (box 1.14 stubs) are tracked by
/// [`stub_sections`](Self::stub_sections): the list of section names that were
/// present on the document but only have stub interpreters.
///
/// `player` is intentionally **omitted**: box 1.7 (`PlayerOverride` /
/// `interpret_player`) is not yet implemented (no spec Requirement), so there is
/// no typed override to aggregate. When box 1.7 lands, add a `player:
/// PlayerOverride` field and wire it in [`from_document`](Self::from_document).
///
/// DEVIATION: `interpret_projectiles` and `interpret_effects` exist, but
/// `MmlDocument` has **no** `projectiles`/`effects` section field (projectile
/// and effect overrides are carried under other sections in the source MML), so
/// those fields can never be populated from a document today and stay at their
/// default empty `Vec`. They are kept here so the aggregate type is complete and
/// ready once a corresponding document section is added.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MmlOverrideSet {
    /// Per-monster overrides (`<monsters>` → [`interpret_monsters`]).
    pub monsters: Vec<MonsterOverride>,
    /// Shell-casing and weapon-order overrides (`<weapons>` → [`interpret_weapons`]).
    pub weapons: WeaponOverrides,
    /// Per-projectile overrides. No `MmlDocument` section field exists yet, so
    /// always empty (see type-level DEVIATION note).
    pub projectiles: Vec<ProjectileOverride>,
    /// Per-effect overrides. No `MmlDocument` section field exists yet, so always
    /// empty (see type-level DEVIATION note).
    pub effects: Vec<EffectOverride>,
    /// Dynamic-limit overrides (`<dynamic_limits>` → [`interpret_dynamic_limits`]).
    pub dynamic_limits: DynamicLimitsOverride,
    /// Per-item overrides (`<items>` → [`interpret_items`]).
    pub items: Vec<ItemOverride>,
    /// Landscape overrides and clears (`<landscapes>` → [`interpret_landscapes`]).
    pub landscapes: LandscapesOverride,
    /// Texture-loading overrides (`<texture_loading>` → [`interpret_texture_loading`]).
    pub texture_loading: TextureLoadingOverride,
    /// String-set overrides (`<stringset>` → [`interpret_stringset`]).
    pub stringset: StringSetOverride,
    /// Scenario-identity override (`<scenario>` → [`interpret_scenario`]).
    pub scenario: ScenarioIdOverride,
    /// Names of sections that were present on the document but only have stub
    /// interpreters (box 1.14). Each is logged once via its stub interpreter
    /// during [`from_document`](Self::from_document).
    pub stub_sections: Vec<String>,
}

impl MmlOverrideSet {
    /// Aggregate every implemented section override from `doc` into one
    /// [`MmlOverrideSet`] (box 3.2). Each populated `doc.<section>` calls the
    /// matching `interpret_*` function; absent sections leave that field at its
    /// default/empty value.
    ///
    /// Sections that only have stub interpreters (box 1.14) are handled by
    /// calling their stub (so the one-time "not yet implemented" warning fires)
    /// and recording the section name in [`stub_sections`](Self::stub_sections);
    /// the returned [`StubOverride`] carries no data and is discarded.
    ///
    /// `projectiles`/`effects` are left default because `MmlDocument` has no
    /// matching section field (see the type-level DEVIATION note). `player` is
    /// omitted entirely pending box 1.7.
    pub fn from_document(doc: &MmlDocument) -> Self {
        let mut out = MmlOverrideSet::default();

        if let Some(section) = &doc.monsters {
            out.monsters = interpret_monsters(section);
        }
        if let Some(section) = &doc.weapons {
            out.weapons = interpret_weapons(section);
        }
        if let Some(section) = &doc.dynamic_limits {
            out.dynamic_limits = interpret_dynamic_limits(section);
        }
        if let Some(section) = &doc.items {
            out.items = interpret_items(section);
        }
        if let Some(section) = &doc.landscapes {
            out.landscapes = interpret_landscapes(section);
        }
        if let Some(section) = &doc.texture_loading {
            out.texture_loading = interpret_texture_loading(section);
        }
        if let Some(section) = &doc.stringset {
            out.stringset = interpret_stringset(section);
        }
        if let Some(section) = &doc.scenario {
            out.scenario = interpret_scenario(section);
        }

        // Stub sections (box 1.14): call the stub to emit the notice, record the
        // name. `view` has no dedicated MmlDocument field, so it is not checked.
        let stubs: [(&Option<MmlSection>, &str, fn(&MmlSection) -> StubOverride); 15] = [
            (&doc.interface, "interface", interpret_interface),
            (&doc.motion_sensor, "motion_sensor", interpret_motion_sensor),
            (&doc.overhead_map, "overhead_map", interpret_overhead_map),
            (&doc.infravision, "infravision", interpret_infravision),
            (
                &doc.animated_textures,
                "animated_textures",
                interpret_animated_textures,
            ),
            (
                &doc.control_panels,
                "control_panels",
                interpret_control_panels,
            ),
            (&doc.platforms, "platforms", interpret_platforms),
            (&doc.liquids, "liquids", interpret_liquids),
            (&doc.sounds, "sounds", interpret_sounds),
            (&doc.faders, "faders", interpret_faders),
            (&doc.scenery, "scenery", interpret_scenery),
            (&doc.opengl, "opengl", interpret_opengl),
            (&doc.software, "software", interpret_software),
            (&doc.console, "console", interpret_console),
            (&doc.logging, "logging", interpret_logging),
        ];
        for (slot, name, interp) in stubs {
            if let Some(section) = slot {
                let _ = interp(section);
                out.stub_sections.push(name.to_string());
            }
        }

        out
    }
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

    // ── test helpers: build a section of one element with the given attrs ──

    fn section_with(elem_name: &str, attrs: &[(&str, &str)]) -> MmlSection {
        let mut attributes = std::collections::HashMap::new();
        for (k, v) in attrs {
            attributes.insert((*k).to_string(), (*v).to_string());
        }
        MmlSection {
            attributes: std::collections::HashMap::new(),
            elements: vec![crate::mml::MmlElement {
                name: elem_name.to_string(),
                attributes,
                children: Vec::new(),
                text: None,
            }],
        }
    }

    // ── box 1.5: projectiles interpreter ──

    #[test]
    fn projectile_override_subset_of_attributes() {
        let section = section_with(
            "projectile",
            &[("index", "5"), ("radius", "128"), ("speed", "20")],
        );
        let projectiles = interpret_projectiles(&section);
        assert_eq!(projectiles.len(), 1);
        assert_eq!(
            projectiles[0],
            ProjectileOverride {
                index: 5,
                radius: Some(128),
                speed: Some(20),
                ..Default::default()
            }
        );
    }

    #[test]
    fn projectile_override_without_index_is_skipped() {
        let section = section_with("projectile", &[("radius", "100")]);
        let projectiles = interpret_projectiles(&section);
        assert!(projectiles.is_empty(), "index-less <projectile> is skipped");
    }

    #[test]
    fn projectile_override_malformed_value_stays_none() {
        let section = section_with("projectile", &[("index", "5"), ("radius", "not_a_number")]);
        let projectiles = interpret_projectiles(&section);
        assert_eq!(projectiles.len(), 1);
        assert_eq!(projectiles[0].index, 5);
        assert_eq!(
            projectiles[0].radius, None,
            "malformed value -> None, element still produced"
        );
    }

    #[test]
    fn projectile_override_full_attributes_and_ignores_unknown() {
        let section = section_with(
            "projectile",
            &[
                ("index", "3"),
                ("collection", "7"),
                ("shape", "2"),
                ("detonation_effect", "10"),
                ("media_detonation_effect", "11"),
                ("contrail_effect", "12"),
                ("ticks_between_contrails", "4"),
                ("maximum_contrails", "8"),
                ("radius", "256"),
                ("area_of_effect", "512"),
                ("flags", "0x1F"),
                ("speed", "30"),
                ("maximum_range", "1024"),
                ("sound_pitch", "1.25"),
                ("flyby_sound", "5"),
                ("rebound_sound", "6"),
                ("bogus_attr", "99"),
            ],
        );
        let projectiles = interpret_projectiles(&section);
        assert_eq!(projectiles.len(), 1);
        let p = &projectiles[0];
        assert_eq!(p.index, 3);
        assert_eq!(p.collection, Some(7));
        assert_eq!(p.shape, Some(2));
        assert_eq!(p.detonation_effect, Some(10));
        assert_eq!(p.media_detonation_effect, Some(11));
        assert_eq!(p.contrail_effect, Some(12));
        assert_eq!(p.ticks_between_contrails, Some(4));
        assert_eq!(p.maximum_contrails, Some(8));
        assert_eq!(p.radius, Some(256));
        assert_eq!(p.area_of_effect, Some(512));
        assert_eq!(p.flags, Some(31)); // 0x1F
        assert_eq!(p.speed, Some(30));
        assert_eq!(p.maximum_range, Some(1024));
        assert_eq!(p.sound_pitch, Some(1.25));
        assert_eq!(p.flyby_sound, Some(5));
        assert_eq!(p.rebound_sound, Some(6));
        // `bogus_attr` and the non-scalar `damage` are not modeled here.
    }

    // ── box 1.6: effects interpreter ──

    #[test]
    fn effect_override_subset_of_attributes() {
        let section = section_with("effect", &[("index", "4"), ("delay", "15")]);
        let effects = interpret_effects(&section);
        assert_eq!(effects.len(), 1);
        assert_eq!(
            effects[0],
            EffectOverride {
                index: 4,
                delay: Some(15),
                ..Default::default()
            }
        );
    }

    #[test]
    fn effect_override_without_index_is_skipped() {
        let section = section_with("effect", &[("delay", "10")]);
        let effects = interpret_effects(&section);
        assert!(effects.is_empty(), "index-less <effect> is skipped");
    }

    #[test]
    fn effect_override_malformed_value_stays_none() {
        let section = section_with("effect", &[("index", "4"), ("delay", "oops")]);
        let effects = interpret_effects(&section);
        assert_eq!(effects.len(), 1);
        assert_eq!(effects[0].index, 4);
        assert_eq!(
            effects[0].delay, None,
            "malformed value -> None, element still produced"
        );
    }

    #[test]
    fn effect_override_full_attributes_and_ignores_unknown() {
        let section = section_with(
            "effect",
            &[
                ("index", "2"),
                ("collection", "9"),
                ("shape", "1"),
                ("sound_pitch", "0.75"),
                ("flags", "0x3"),
                ("delay", "20"),
                ("delay_sound", "7"),
                ("bogus_attr", "99"),
            ],
        );
        let effects = interpret_effects(&section);
        assert_eq!(effects.len(), 1);
        let e = &effects[0];
        assert_eq!(e.index, 2);
        assert_eq!(e.collection, Some(9));
        assert_eq!(e.shape, Some(1));
        assert_eq!(e.sound_pitch, Some(0.75));
        assert_eq!(e.flags, Some(3)); // 0x3 — u16 bitfield
        assert_eq!(e.delay, Some(20));
        assert_eq!(e.delay_sound, Some(7));
        // `bogus_attr` is unrecognized and silently ignored.
    }

    // ── box 1.4: weapons interpreter (shell_casings + order) ──

    #[test]
    fn shell_casing_override_scenario() {
        // Spec scenario: <weapons><shell_casings index="0" coll="14" seq="2"/></weapons>
        let doc = MmlDocument::from_bytes(
            b"<marathon><weapons><shell_casings index=\"0\" coll=\"14\" seq=\"2\"/></weapons></marathon>",
        )
        .unwrap();
        let out = interpret_weapons(&doc.weapons.unwrap());
        assert_eq!(out.shell_casings.len(), 1);
        assert_eq!(
            out.shell_casings[0],
            ShellCasingOverride {
                index: 0,
                collection: Some(14),
                sequence: Some(2),
                ..Default::default()
            }
        );
        // position/velocity fields are None
        let sc = &out.shell_casings[0];
        assert_eq!(sc.x0, None);
        assert_eq!(sc.y0, None);
        assert_eq!(sc.vx0, None);
        assert_eq!(sc.vy0, None);
        assert_eq!(sc.dvx, None);
        assert_eq!(sc.dvy, None);
        assert!(out.order.is_empty());
    }

    #[test]
    fn shell_casing_without_index_is_skipped() {
        let doc = MmlDocument::from_bytes(
            b"<marathon><weapons><shell_casings coll=\"14\" seq=\"2\"/></weapons></marathon>",
        )
        .unwrap();
        let out = interpret_weapons(&doc.weapons.unwrap());
        assert!(
            out.shell_casings.is_empty(),
            "index-less <shell_casings> is skipped"
        );
    }

    #[test]
    fn shell_casing_malformed_value_stays_none() {
        let doc = MmlDocument::from_bytes(
            b"<marathon><weapons><shell_casings index=\"0\" coll=\"not_a_number\" seq=\"2\"/></weapons></marathon>",
        )
        .unwrap();
        let out = interpret_weapons(&doc.weapons.unwrap());
        assert_eq!(out.shell_casings.len(), 1);
        assert_eq!(out.shell_casings[0].index, 0);
        assert_eq!(
            out.shell_casings[0].collection, None,
            "malformed value -> None, element still produced"
        );
        assert_eq!(out.shell_casings[0].sequence, Some(2));
    }

    #[test]
    fn shell_casing_full_attributes_and_ignores_unknown() {
        let doc = MmlDocument::from_bytes(
            b"<marathon><weapons><shell_casings index=\"1\" coll=\"14\" seq=\"2\" x0=\"10\" y0=\"20\" vx0=\"-5\" vy0=\"6\" dvx=\"1\" dvy=\"-1\" bogus_attr=\"99\"/></weapons></marathon>",
        )
        .unwrap();
        let out = interpret_weapons(&doc.weapons.unwrap());
        assert_eq!(out.shell_casings.len(), 1);
        assert_eq!(
            out.shell_casings[0],
            ShellCasingOverride {
                index: 1,
                collection: Some(14),
                sequence: Some(2),
                x0: Some(10),
                y0: Some(20),
                vx0: Some(-5),
                vy0: Some(6),
                dvx: Some(1),
                dvy: Some(-1),
            }
        );
        // `bogus_attr` is unrecognized and silently ignored.
    }

    #[test]
    fn weapon_order_definition() {
        // Spec scenario: <order index="0" weapon="3"/><order index="1" weapon="0"/>
        let doc = MmlDocument::from_bytes(
            b"<marathon><weapons><order index=\"0\" weapon=\"3\"/><order index=\"1\" weapon=\"0\"/></weapons></marathon>",
        )
        .unwrap();
        let out = interpret_weapons(&doc.weapons.unwrap());
        assert!(out.shell_casings.is_empty());
        assert_eq!(
            out.order,
            vec![
                WeaponOrderEntry {
                    index: 0,
                    weapon: Some(3)
                },
                WeaponOrderEntry {
                    index: 1,
                    weapon: Some(0)
                },
            ]
        );
    }

    #[test]
    fn weapon_order_without_index_is_skipped() {
        let doc = MmlDocument::from_bytes(
            b"<marathon><weapons><order weapon=\"3\"/></weapons></marathon>",
        )
        .unwrap();
        let out = interpret_weapons(&doc.weapons.unwrap());
        assert!(out.order.is_empty(), "index-less <order> is skipped");
    }

    #[test]
    fn weapons_section_mixes_shell_casings_and_order() {
        let doc = MmlDocument::from_bytes(
            b"<marathon><weapons><shell_casings index=\"0\" coll=\"14\"/><order index=\"0\" weapon=\"3\"/></weapons></marathon>",
        )
        .unwrap();
        let out = interpret_weapons(&doc.weapons.unwrap());
        assert_eq!(out.shell_casings.len(), 1);
        assert_eq!(out.shell_casings[0].index, 0);
        assert_eq!(out.shell_casings[0].collection, Some(14));
        assert_eq!(out.order.len(), 1);
        assert_eq!(out.order[0].index, 0);
        assert_eq!(out.order[0].weapon, Some(3));
    }

    // ── boxes 3.1/3.2: MmlOverrideSet aggregation ──

    #[test]
    fn override_set_from_empty_document_is_all_defaults() {
        let doc = MmlDocument::from_bytes(b"<marathon></marathon>").unwrap();
        let set = MmlOverrideSet::from_document(&doc);
        assert_eq!(set, MmlOverrideSet::default());
        assert!(set.monsters.is_empty());
        assert!(set.items.is_empty());
        assert!(set.projectiles.is_empty());
        assert!(set.effects.is_empty());
        assert!(set.stub_sections.is_empty());
        assert_eq!(set.dynamic_limits, DynamicLimitsOverride::default());
        assert_eq!(set.scenario, ScenarioIdOverride::default());
    }

    #[test]
    fn override_set_wires_populated_sections() {
        let doc = MmlDocument::from_bytes(
            b"<marathon><monsters><monster index=\"5\" vitality=\"300\"/></monsters><items><item index=\"7\" maximum=\"5\"/></items></marathon>",
        )
        .unwrap();
        let set = MmlOverrideSet::from_document(&doc);

        // monsters interpreted into the monsters field
        assert_eq!(set.monsters.len(), 1);
        assert_eq!(
            set.monsters[0],
            MonsterOverride {
                index: 5,
                vitality: Some(300),
                ..Default::default()
            }
        );
        // items interpreted into the items field
        assert_eq!(set.items.len(), 1);
        assert_eq!(
            set.items[0],
            ItemOverride {
                index: 7,
                maximum: Some(5),
                ..Default::default()
            }
        );
        // untouched sections stay at default/empty
        assert_eq!(set.dynamic_limits, DynamicLimitsOverride::default());
        assert!(set.landscapes.landscapes.is_empty());
        assert!(set.stub_sections.is_empty());
    }

    #[test]
    fn override_set_wires_scenario_and_dynamic_limits() {
        let doc = MmlDocument::from_bytes(
            b"<marathon><scenario name=\"Marathon 2\" version=\"1\" id=\"M2\"/><dynamic_limits><monsters>1024</monsters></dynamic_limits></marathon>",
        )
        .unwrap();
        let set = MmlOverrideSet::from_document(&doc);
        assert_eq!(set.scenario.name.as_deref(), Some("Marathon 2"));
        assert_eq!(set.scenario.version, Some(1));
        assert_eq!(set.dynamic_limits.monsters, Some(1024));
    }

    #[test]
    fn override_set_records_stub_sections() {
        let doc = MmlDocument::from_bytes(
            b"<marathon><platforms><platform index=\"0\"/></platforms><liquids/></marathon>",
        )
        .unwrap();
        let set = MmlOverrideSet::from_document(&doc);
        assert!(set.stub_sections.iter().any(|s| s == "platforms"));
        assert!(set.stub_sections.iter().any(|s| s == "liquids"));
        // stub sections carry no typed data
        assert!(set.monsters.is_empty());
    }
}
