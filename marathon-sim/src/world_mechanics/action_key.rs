use glam::Vec2;
use crate::world::MapGeometry;
use crate::world_mechanics::panels::ControlPanels;

/// Maximum distance for platform/door activation (3 world units).
const MAXIMUM_ACTIVATION_RANGE: f32 = 3.0;
/// Maximum distance for control panel activation (1.5 world units).
const MAXIMUM_CONTROL_ACTIVATION_RANGE: f32 = 1.5;
/// Polygon type for platforms.
const POLYGON_IS_PLATFORM: i16 = 5;

/// Target found by action key ray-cast.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionTarget {
    None,
    Platform(usize),
    Panel(usize),
}

/// Cast a ray from the player's position in their facing direction to find
/// an interaction target (door platform or control panel).
///
/// Traverses polygons along the ray using map adjacency data.
pub fn find_action_key_target(
    player_pos: Vec2,
    player_facing: f32,
    current_polygon: usize,
    geometry: &MapGeometry,
    panels: &ControlPanels,
) -> ActionTarget {
    let direction = Vec2::new(player_facing.cos(), player_facing.sin());

    let mut current_poly = current_polygon;
    // Track the line we entered through to avoid re-crossing it
    let mut entry_line: Option<usize> = None;

    // Walk through polygons along the ray, always casting from player_pos
    for step in 0..16 {
        let adjacency = match geometry.polygon_adjacency.get(current_poly) {
            Some(adj) => adj.clone(),
            None => return ActionTarget::None,
        };

        let vertices = match geometry.polygon_vertices.get(current_poly) {
            Some(v) => v.clone(),
            None => return ActionTarget::None,
        };

        let mut crossed_line = None;
        let mut crossed_adj = None;
        let mut best_t = f32::MAX;

        for (edge_idx, &(line_idx, adj_poly)) in adjacency.iter().enumerate() {
            // Skip the line we entered through to prevent back-crossing
            if entry_line == Some(line_idx) {
                continue;
            }

            let v0 = vertices[edge_idx];
            let v1 = vertices[(edge_idx + 1) % vertices.len()];

            // Always cast from player_pos to avoid floating point drift
            if let Some(t) = ray_segment_intersection(player_pos, direction, v0, v1) {
                if t > 0.001 && t < best_t && t <= MAXIMUM_ACTIVATION_RANGE {
                    best_t = t;
                    crossed_line = Some(line_idx);
                    crossed_adj = adj_poly;
                }
            }
        }

        let line_idx = match crossed_line {
            Some(idx) => idx,
            None => return ActionTarget::None,
        };

        // Check for control panel on this line (closer range)
        if best_t <= MAXIMUM_CONTROL_ACTIVATION_RANGE {
            if let Some(_side_indices) = geometry.line_side_indices.get(line_idx) {
                for (panel_idx, panel) in panels.0.iter().enumerate() {
                    if panel.line_index == line_idx {
                        return ActionTarget::Panel(panel_idx);
                    }
                }
            }
        }

        // Check if adjacent polygon is a platform (door)
        if let Some(adj_poly) = crossed_adj {
            if let Some(&poly_type) = geometry.polygon_types.get(adj_poly) {
                if poly_type == POLYGON_IS_PLATFORM {
                    return ActionTarget::Platform(adj_poly);
                }
            }
            // Continue into adjacent polygon
            current_poly = adj_poly;
            entry_line = Some(line_idx);
        } else {
            // Solid wall — no adjacent polygon
            return ActionTarget::None;
        }
    }

    ActionTarget::None
}

/// Ray-segment intersection test. Returns parameter t along ray if intersection exists.
/// Ray: origin + direction * t
/// Segment: v0 to v1
fn ray_segment_intersection(
    origin: Vec2,
    direction: Vec2,
    v0: Vec2,
    v1: Vec2,
) -> Option<f32> {
    let edge = v1 - v0;
    let denom = direction.x * edge.y - direction.y * edge.x;

    if denom.abs() < 1e-10 {
        return None; // Parallel
    }

    let to_v0 = v0 - origin;
    let t = (to_v0.x * edge.y - to_v0.y * edge.x) / denom;
    let u = (to_v0.x * direction.y - to_v0.y * direction.x) / denom;

    if t >= 0.0 && u >= 0.0 && u <= 1.0 {
        Some(t)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ray_segment_basic_intersection() {
        let origin = Vec2::new(0.0, 0.0);
        let direction = Vec2::new(1.0, 0.0);
        let v0 = Vec2::new(2.0, -1.0);
        let v1 = Vec2::new(2.0, 1.0);
        let t = ray_segment_intersection(origin, direction, v0, v1);
        assert!(t.is_some());
        assert!((t.unwrap() - 2.0).abs() < 1e-6);
    }

    #[test]
    fn ray_segment_no_intersection_behind() {
        let origin = Vec2::new(0.0, 0.0);
        let direction = Vec2::new(1.0, 0.0);
        let v0 = Vec2::new(-2.0, -1.0);
        let v1 = Vec2::new(-2.0, 1.0);
        let t = ray_segment_intersection(origin, direction, v0, v1);
        assert!(t.is_none() || t.unwrap() < 0.0);
    }

    #[test]
    fn ray_segment_parallel() {
        let origin = Vec2::new(0.0, 0.0);
        let direction = Vec2::new(1.0, 0.0);
        let v0 = Vec2::new(0.0, 1.0);
        let v1 = Vec2::new(5.0, 1.0);
        assert!(ray_segment_intersection(origin, direction, v0, v1).is_none());
    }

    #[test]
    fn action_target_none_for_empty_geometry() {
        let geometry = MapGeometry {
            polygon_vertices: vec![vec![
                Vec2::new(-1.0, -1.0),
                Vec2::new(1.0, -1.0),
                Vec2::new(1.0, 1.0),
                Vec2::new(-1.0, 1.0),
            ]],
            floor_heights: vec![0.0],
            ceiling_heights: vec![3.0],
            polygon_adjacency: vec![vec![
                (0, None),
                (1, None),
                (2, None),
                (3, None),
            ]],
            line_endpoints: vec![
                (Vec2::new(-1.0, -1.0), Vec2::new(1.0, -1.0)),
                (Vec2::new(1.0, -1.0), Vec2::new(1.0, 1.0)),
                (Vec2::new(1.0, 1.0), Vec2::new(-1.0, 1.0)),
                (Vec2::new(-1.0, 1.0), Vec2::new(-1.0, -1.0)),
            ],
            line_solid: vec![true; 4],
            line_transparent: vec![false; 4],
            polygon_media_index: vec![-1],
            polygon_floor_light_index: vec![-1],
            polygon_ceiling_light_index: vec![-1],
            polygon_types: vec![0],
            polygon_permutations: vec![-1],
            line_side_indices: vec![
                (None, None),
                (None, None),
                (None, None),
                (None, None),
            ],
        };
        let panels = ControlPanels::default();
        let result = find_action_key_target(
            Vec2::new(0.0, 0.0),
            0.0,
            0,
            &geometry,
            &panels,
        );
        assert_eq!(result, ActionTarget::None);
    }

    #[test]
    fn action_target_finds_platform_door() {
        // Two polygons: player room (0) and door (1, type=5 platform)
        // Player at (0,0) facing east (+X), door polygon is adjacent via line 1
        let geometry = MapGeometry {
            polygon_vertices: vec![
                // Polygon 0: player room (-2,-1) to (1,1)
                vec![
                    Vec2::new(-2.0, -1.0),
                    Vec2::new(1.0, -1.0),
                    Vec2::new(1.0, 1.0),
                    Vec2::new(-2.0, 1.0),
                ],
                // Polygon 1: door room (1,-1) to (3,1)
                vec![
                    Vec2::new(1.0, -1.0),
                    Vec2::new(3.0, -1.0),
                    Vec2::new(3.0, 1.0),
                    Vec2::new(1.0, 1.0),
                ],
            ],
            floor_heights: vec![0.0, 0.0],
            ceiling_heights: vec![3.0, 3.0],
            polygon_adjacency: vec![
                // Poly 0 edges: bottom(0), right→poly1(1), top(2), left(3)
                vec![(0, None), (1, Some(1)), (2, None), (3, None)],
                // Poly 1 edges: bottom(4), right(5), top(6), left→poly0(1)
                vec![(4, None), (5, None), (6, None), (1, Some(0))],
            ],
            line_endpoints: vec![
                (Vec2::new(-2.0, -1.0), Vec2::new(1.0, -1.0)),  // 0: bottom of poly0
                (Vec2::new(1.0, -1.0), Vec2::new(1.0, 1.0)),    // 1: shared edge
                (Vec2::new(-2.0, 1.0), Vec2::new(1.0, 1.0)),    // 2: top of poly0
                (Vec2::new(-2.0, -1.0), Vec2::new(-2.0, 1.0)),  // 3: left of poly0
                (Vec2::new(1.0, -1.0), Vec2::new(3.0, -1.0)),   // 4: bottom of poly1
                (Vec2::new(3.0, -1.0), Vec2::new(3.0, 1.0)),    // 5: right of poly1
                (Vec2::new(1.0, 1.0), Vec2::new(3.0, 1.0)),     // 6: top of poly1
            ],
            line_solid: vec![true, false, true, true, true, true, true],
            line_transparent: vec![false, true, false, false, false, false, false],
            polygon_media_index: vec![-1, -1],
            polygon_floor_light_index: vec![-1, -1],
            polygon_ceiling_light_index: vec![-1, -1],
            polygon_types: vec![0, 5],          // poly 1 is a platform
            polygon_permutations: vec![-1, 0],   // platform index 0
            line_side_indices: vec![
                (None, None), (None, None), (None, None), (None, None),
                (None, None), (None, None), (None, None),
            ],
        };
        let panels = ControlPanels::default();

        // Player at origin facing east (0 rad) → should cross line 1 → find platform poly 1
        let result = find_action_key_target(
            Vec2::new(0.0, 0.0),
            0.0, // facing east
            0,   // in polygon 0
            &geometry,
            &panels,
        );
        assert_eq!(result, ActionTarget::Platform(1));
    }

    #[test]
    fn action_target_finds_panel() {
        use crate::world_mechanics::panels::{ControlPanel, PanelAction};

        // Single room, player facing east toward a wall with a control panel
        let geometry = MapGeometry {
            polygon_vertices: vec![vec![
                Vec2::new(-2.0, -1.0),
                Vec2::new(1.0, -1.0),
                Vec2::new(1.0, 1.0),
                Vec2::new(-2.0, 1.0),
            ]],
            floor_heights: vec![0.0],
            ceiling_heights: vec![3.0],
            polygon_adjacency: vec![vec![
                (0, None), // bottom
                (1, None), // right wall (has panel)
                (2, None), // top
                (3, None), // left
            ]],
            line_endpoints: vec![
                (Vec2::new(-2.0, -1.0), Vec2::new(1.0, -1.0)),
                (Vec2::new(1.0, -1.0), Vec2::new(1.0, 1.0)),  // line 1: right wall
                (Vec2::new(-2.0, 1.0), Vec2::new(1.0, 1.0)),
                (Vec2::new(-2.0, -1.0), Vec2::new(-2.0, 1.0)),
            ],
            line_solid: vec![true; 4],
            line_transparent: vec![false; 4],
            polygon_media_index: vec![-1],
            polygon_floor_light_index: vec![-1],
            polygon_ceiling_light_index: vec![-1],
            polygon_types: vec![0],
            polygon_permutations: vec![-1],
            line_side_indices: vec![
                (None, None), (Some(0), None), (None, None), (None, None),
            ],
        };

        let panels = ControlPanels(vec![
            ControlPanel {
                line_index: 1, // right wall
                side: 0,
                action: PanelAction::ToggleLight { light_index: 0 },
                max_distance: 1.5,
            },
        ]);

        // Player at origin facing east, wall at x=1 (distance 1.0 < 1.5)
        let result = find_action_key_target(
            Vec2::new(0.0, 0.0),
            0.0,
            0,
            &geometry,
            &panels,
        );
        assert_eq!(result, ActionTarget::Panel(0));
    }
}
