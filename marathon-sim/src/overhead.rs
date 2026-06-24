//! Overhead (automap) view data model and classification (boxes 3.1-3.3).
//!
//! This module holds the *presentation* model for the overhead map: the
//! categories used to colour lines, the per-polygon fill colours, and the
//! entity markers drawn on top. The classification functions consume the
//! static [`MapMetadata`] resource (built in [`crate::world`]) and are pure
//! free functions — they need no `SimWorld` and no ECS access, so the renderer
//! / integration layer can call them directly with a borrowed `&MapMetadata`.

use crate::world::MapMetadata;
use crate::world_mechanics::media::{
    MEDIA_GOO, MEDIA_JJARO, MEDIA_LAVA, MEDIA_SEWAGE, MEDIA_WATER,
};
use glam::Vec2;
use marathon_formats::map::LineFlags;

/// Marathon polygon type for a platform polygon.
const POLYGON_TYPE_PLATFORM: i16 = 5;

/// How a line should be drawn on the overhead map (box 3.1).
///
/// Classification priority (highest first) is encoded in [`classify_line`]:
/// `ControlPanel` > `Platform` > `Landscape` > `ElevationChange` >
/// `Transparent` > `SolidWall` (the default).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineCategory {
    /// An opaque, impassable wall line (the default when nothing else matches).
    SolidWall,
    /// A line where the floor/ceiling elevation changes between its two sides.
    ElevationChange,
    /// A transparent (see-through) line.
    Transparent,
    /// A line that borders a landscape (sky / texture-mapped horizon) polygon.
    Landscape,
    /// A line bordering a platform (door/elevator) polygon.
    Platform,
    /// A line carrying a control panel (switch / terminal / recharger).
    ControlPanel,
}

/// A line as presented on the overhead map (box 3.1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExploredLine {
    /// The two world-space endpoints of the line segment.
    pub endpoints: (Vec2, Vec2),
    /// How the line should be drawn.
    pub category: LineCategory,
    /// Whether the player has uncovered (explored) this line yet.
    pub explored: bool,
}

/// A polygon as presented on the overhead map (box 3.1).
#[derive(Debug, Clone, PartialEq)]
pub struct ExploredPolygon {
    /// World-space vertices, in order, forming the polygon's outline.
    pub vertices: Vec<Vec2>,
    /// RGBA fill colour for the polygon interior.
    pub fill_color: [u8; 4],
    /// Whether the player has uncovered (explored) this polygon yet.
    pub explored: bool,
}

/// The kind of marker drawn for an [`OverheadEntity`] (box 3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverheadEntityKind {
    /// The local player.
    Player,
    /// A monster / AI actor.
    Monster,
    /// A pickup item.
    Item,
}

/// An entity marker drawn on top of the overhead map (box 3.1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OverheadEntity {
    /// World-space position of the marker.
    pub position: Vec2,
    /// Facing angle in radians (used to draw a direction arrow for the player).
    pub facing: f32,
    /// What kind of marker this is.
    pub kind: OverheadEntityKind,
}

/// Classify a line for the overhead map (box 3.2).
///
/// Priority (highest first): control panel > platform > landscape > elevation
/// > transparent > solid (the default). The first matching condition wins.
///
/// * **ControlPanel** — `metadata.line_has_control_panel[line_index]` is set
///   (takes precedence even over a SOLID flag).
/// * **Platform** — either adjacent polygon owner (skipping `-1`) is a platform
///   polygon (`polygon_types[poly] == 5`).
/// * Otherwise the line's [`LineFlags`] are consulted via `.contains()`:
///   `LANDSCAPE` → `Landscape`, `ELEVATION` → `ElevationChange`,
///   `TRANSPARENT` → `Transparent`.
/// * **SolidWall** — the default when nothing above matches.
pub fn classify_line(line_index: usize, metadata: &MapMetadata) -> LineCategory {
    // Highest priority: control panels override everything (incl. SOLID).
    if metadata
        .line_has_control_panel
        .get(line_index)
        .copied()
        .unwrap_or(false)
    {
        return LineCategory::ControlPanel;
    }

    // Next: a line bordering a platform polygon.
    if let Some(&(cw, ccw)) = metadata.line_adjacent_polygons.get(line_index) {
        let is_platform = |owner: i16| -> bool {
            if owner < 0 {
                return false;
            }
            metadata
                .polygon_types
                .get(owner as usize)
                .copied()
                .map(|t| t == POLYGON_TYPE_PLATFORM)
                .unwrap_or(false)
        };
        if is_platform(cw) || is_platform(ccw) {
            return LineCategory::Platform;
        }
    }

    // Finally, fall back to the line's own flags by descending priority.
    let flags = metadata
        .line_flags
        .get(line_index)
        .copied()
        .unwrap_or_else(LineFlags::empty);

    if flags.contains(LineFlags::LANDSCAPE) {
        LineCategory::Landscape
    } else if flags.contains(LineFlags::ELEVATION) {
        LineCategory::ElevationChange
    } else if flags.contains(LineFlags::TRANSPARENT) {
        LineCategory::Transparent
    } else {
        LineCategory::SolidWall
    }
}

// ----- Overhead fill colours (RGBA bytes) -----------------------------------

/// Dark blue — water media.
pub const COLOR_WATER: [u8; 4] = [0x20, 0x40, 0x80, 0xFF];
/// Dark red / orange — lava media.
pub const COLOR_LAVA: [u8; 4] = [0x80, 0x28, 0x10, 0xFF];
/// Dark green — goo media (a greenish sludge).
pub const COLOR_GOO: [u8; 4] = [0x30, 0x60, 0x20, 0xFF];
/// Murky olive — sewage media.
pub const COLOR_SEWAGE: [u8; 4] = [0x50, 0x50, 0x20, 0xFF];
/// Cyan-ish — Jjaro media.
pub const COLOR_JJARO: [u8; 4] = [0x30, 0x60, 0x70, 0xFF];
/// Fallback for an unknown media type.
pub const COLOR_MEDIA_DEFAULT: [u8; 4] = [0x40, 0x40, 0x50, 0xFF];
/// Dark red — platform polygon (no media).
pub const COLOR_PLATFORM: [u8; 4] = [0x60, 0x10, 0x10, 0xFF];
/// Neutral dark gray — ordinary polygon (no media, not a platform).
pub const COLOR_DEFAULT: [u8; 4] = [0x28, 0x28, 0x28, 0xFF];

/// Compute the overhead-map fill colour (RGBA) for a polygon (box 3.3).
///
/// Media takes precedence: if `polygon_media_index[poly] >= 0` the referenced
/// `media_types[media_index]` selects a per-media colour. Otherwise the polygon
/// type is consulted: a platform (`5`) is dark red, anything else is a neutral
/// dark gray.
pub fn polygon_fill_color(poly_index: usize, metadata: &MapMetadata) -> [u8; 4] {
    // Media first.
    if let Some(&media_index) = metadata.polygon_media_index.get(poly_index) {
        if media_index >= 0 {
            if let Some(&media_type) = metadata.media_types.get(media_index as usize) {
                return match media_type {
                    MEDIA_WATER => COLOR_WATER,
                    MEDIA_LAVA => COLOR_LAVA,
                    MEDIA_GOO => COLOR_GOO,
                    MEDIA_SEWAGE => COLOR_SEWAGE,
                    MEDIA_JJARO => COLOR_JJARO,
                    _ => COLOR_MEDIA_DEFAULT,
                };
            }
        }
    }

    // No media: classify by polygon type.
    match metadata.polygon_types.get(poly_index).copied() {
        Some(POLYGON_TYPE_PLATFORM) => COLOR_PLATFORM,
        _ => COLOR_DEFAULT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal [`MapMetadata`] with the given per-collection vectors.
    /// Plain struct literal — no `SimWorld` needed (keeps these fast & focused).
    fn metadata(
        line_flags: Vec<LineFlags>,
        line_has_control_panel: Vec<bool>,
        line_adjacent_polygons: Vec<(i16, i16)>,
        polygon_types: Vec<i16>,
        polygon_media_index: Vec<i16>,
        media_types: Vec<i16>,
    ) -> MapMetadata {
        MapMetadata {
            line_flags,
            line_has_control_panel,
            line_adjacent_polygons,
            polygon_types,
            polygon_media_index,
            media_types,
        }
    }

    /// Box 3.9: a control-panel line classifies as `ControlPanel` even when its
    /// `LineFlags` has SOLID set.
    #[test]
    fn control_panel_overrides_solid() {
        let md = metadata(
            vec![LineFlags::SOLID],
            vec![true],
            vec![(-1, -1)],
            vec![],
            vec![],
            vec![],
        );
        assert_eq!(classify_line(0, &md), LineCategory::ControlPanel);
    }

    /// Box 3.10: a SOLID line adjacent to a platform-type (5) polygon
    /// classifies as `Platform`.
    #[test]
    fn solid_line_next_to_platform_is_platform() {
        let md = metadata(
            vec![LineFlags::SOLID],
            vec![false],
            // ccw owner -1 (void), cw owner = polygon 0 (a platform).
            vec![(0, -1)],
            vec![POLYGON_TYPE_PLATFORM],
            vec![-1],
            vec![],
        );
        assert_eq!(classify_line(0, &md), LineCategory::Platform);
    }

    /// Box 3.11: a line with the ELEVATION flag (and no higher-priority
    /// condition) classifies as `ElevationChange`.
    #[test]
    fn elevation_flag_is_elevation_change() {
        let md = metadata(
            vec![LineFlags::ELEVATION],
            vec![false],
            vec![(-1, -1)],
            vec![],
            vec![],
            vec![],
        );
        assert_eq!(classify_line(0, &md), LineCategory::ElevationChange);
    }

    /// Box 3.12: `polygon_fill_color` for a polygon with WATER media
    /// (media_type 0) returns the dark-blue RGBA.
    #[test]
    fn water_media_polygon_is_dark_blue() {
        let md = metadata(
            vec![],
            vec![],
            vec![],
            vec![0],           // poly 0 type (irrelevant — media wins)
            vec![0],           // poly 0 -> media index 0
            vec![MEDIA_WATER], // media 0 is water (type 0)
        );
        assert_eq!(polygon_fill_color(0, &md), COLOR_WATER);
        assert_eq!(COLOR_WATER, [0x20, 0x40, 0x80, 0xFF]);
    }

    /// Box 3.13: `polygon_fill_color` for a platform polygon (type 5) with NO
    /// media returns the dark-red RGBA.
    #[test]
    fn platform_polygon_no_media_is_dark_red() {
        let md = metadata(
            vec![],
            vec![],
            vec![],
            vec![POLYGON_TYPE_PLATFORM],
            vec![-1], // no media
            vec![],
        );
        assert_eq!(polygon_fill_color(0, &md), COLOR_PLATFORM);
        assert_eq!(COLOR_PLATFORM, [0x60, 0x10, 0x10, 0xFF]);
    }
}
