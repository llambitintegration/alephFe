use bevy_ecs::prelude::Resource;
use glam::Vec2;

/// Types of control panel actions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PanelAction {
    ActivatePlatform { platform_index: usize },
    ToggleLight { light_index: usize },
    ActivateTerminal { terminal_index: usize },
    ActivateTaggedPlatforms { tag: i16 },
}

/// Resource holding all control panels in the level.
#[derive(Resource, Default, Clone)]
pub struct ControlPanels(pub Vec<ControlPanel>);

/// A control panel on a wall side.
#[derive(Debug, Clone)]
pub struct ControlPanel {
    /// Line index of the side with the panel.
    pub line_index: usize,
    /// Which side of the line (0 = clockwise, 1 = counterclockwise).
    pub side: u8,
    /// Action triggered on activation.
    pub action: PanelAction,
    /// Maximum activation distance.
    pub max_distance: f32,
}

/// Check if a player can activate a control panel.
///
/// The player must be facing the panel's line, within range, and pressing action.
pub fn can_activate_panel(
    player_pos: Vec2,
    player_facing: f32,
    panel: &ControlPanel,
    line_endpoints: &[(Vec2, Vec2)],
) -> bool {
    let (la, lb) = line_endpoints[panel.line_index];
    let line_center = (la + lb) * 0.5;
    let to_panel = line_center - player_pos;
    let distance = to_panel.length();

    if distance > panel.max_distance || distance < 1e-6 {
        return false;
    }

    // Check if player is roughly facing the panel
    let angle_to_panel = to_panel.y.atan2(to_panel.x);
    let angle_diff = normalize_angle(angle_to_panel - player_facing);

    // Must be within ~60 degrees of facing
    angle_diff.abs() <= std::f32::consts::FRAC_PI_3
}

/// A debug-positioning target: where to stand, which way to face, and which
/// polygon the player ends up in so that the next ACTION-key press lands on an
/// activatable door/control-panel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DoorFacingPose {
    /// World-space 2D position to place the player at.
    pub position: Vec2,
    /// Facing angle (radians) toward the door/panel line.
    pub facing: f32,
    /// Polygon index the player ends up in (the room adjacent to the door).
    pub polygon: usize,
}

/// DEBUG-ONLY. Compute a pose that places the player directly in front of the
/// nearest activatable door so that the action-key raycast
/// ([`crate::world_mechanics::action_key::find_action_key_target`]) will hit
/// it. Used only by the `__marathonDebug.faceNearestDoor()` web hook to make
/// door-interaction e2e tests deterministic — it is never wired into normal
/// gameplay.
///
/// Strategy: scan every line that borders a platform polygon (Marathon door
/// type 5) from a NON-platform neighbour room. For the nearest such line to
/// `player_pos`, stand a short standoff back from the line's midpoint, inside
/// the neighbour room, facing the line. That satisfies both the distance and
/// the facing checks used during activation, and the raycast crosses the
/// shared line into the platform polygon.
///
/// Falls back to control-panel lines (so panels that *drive* a door also
/// work) when no directly-adjacent platform door is found. Returns `None`
/// only when the map has no activatable door or panel at all.
pub fn debug_pose_facing_nearest_door(
    player_pos: Vec2,
    polygon_vertices: &[Vec<Vec2>],
    polygon_adjacency: &[Vec<(usize, Option<usize>)>],
    polygon_types: &[i16],
    line_endpoints: &[(Vec2, Vec2)],
    panels: &[ControlPanel],
) -> Option<DoorFacingPose> {
    /// Platform polygon type in the Marathon map format.
    const POLYGON_IS_PLATFORM: i16 = 5;
    /// How far back from the line to stand. Must be < the activation ranges
    /// (control panels 1.5, platforms 3.0) yet > the raycast's near epsilon.
    const STANDOFF: f32 = 0.75;

    let mut best: Option<(f32, DoorFacingPose)> = None;

    let consider = |neighbour_poly: usize,
                    line_idx: usize,
                    best: &mut Option<(f32, DoorFacingPose)>| {
        let (la, lb) = match line_endpoints.get(line_idx) {
            Some(&pair) => pair,
            None => return,
        };
        let line_center = (la + lb) * 0.5;

        // Inward normal: point from the line toward the neighbour room's
        // centroid so the standoff position lands inside that room.
        let verts = match polygon_vertices.get(neighbour_poly) {
            Some(v) if !v.is_empty() => v,
            _ => return,
        };
        let centroid = verts.iter().copied().fold(Vec2::ZERO, |a, b| a + b) / verts.len() as f32;
        let mut inward = centroid - line_center;
        if inward.length() < 1e-4 {
            // Degenerate: fall back to the line's perpendicular.
            let edge = lb - la;
            inward = Vec2::new(-edge.y, edge.x);
        }
        let inward = inward.normalize_or_zero();
        if inward == Vec2::ZERO {
            return;
        }

        let position = line_center + inward * STANDOFF;
        let to_line = line_center - position;
        let facing = to_line.y.atan2(to_line.x);
        let dist = (line_center - player_pos).length();

        let pose = DoorFacingPose {
            position,
            facing,
            polygon: neighbour_poly,
        };
        if best.as_ref().map(|(d, _)| dist < *d).unwrap_or(true) {
            *best = Some((dist, pose));
        }
    };

    // 1. Platform doors reachable from a neighbouring room.
    for (poly, adj) in polygon_adjacency.iter().enumerate() {
        let is_platform = polygon_types.get(poly).copied() == Some(POLYGON_IS_PLATFORM);
        if is_platform {
            continue;
        }
        for &(line_idx, neighbour) in adj.iter() {
            if let Some(adj_poly) = neighbour {
                if polygon_types.get(adj_poly).copied() == Some(POLYGON_IS_PLATFORM) {
                    consider(poly, line_idx, &mut best);
                }
            }
        }
    }

    // 2. Fall back to control-panel lines if no platform door was adjacent.
    if best.is_none() {
        for panel in panels {
            // Find the polygon that borders this panel's line.
            let neighbour = polygon_adjacency
                .iter()
                .position(|adj| adj.iter().any(|&(l, _)| l == panel.line_index));
            if let Some(poly) = neighbour {
                consider(poly, panel.line_index, &mut best);
            }
        }
    }

    best.map(|(_, pose)| pose)
}

/// A debug-positioning result for a light-switch panel: where to stand/face so
/// the action key activates the switch, plus the `light_index` that switch
/// toggles (so an e2e test can read that specific light's intensity before and
/// after the press).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LightSwitchPose {
    /// Where to stand and which way to face to activate the switch.
    pub pose: DoorFacingPose,
    /// The light index the switch's [`PanelAction::ToggleLight`] flips.
    pub light_index: usize,
}

/// DEBUG-ONLY. Compute a pose that places the player in front of the nearest
/// *light-switch* control panel (a panel whose action is
/// [`PanelAction::ToggleLight`]) so that the action-key raycast activates it.
///
/// Unlike [`debug_pose_facing_nearest_door`] (which prefers platform doors and
/// only falls back to panels), this deliberately targets light switches so an
/// e2e test can verify the light-toggle path. Returns the standoff pose plus
/// the `light_index` the switch controls, or `None` when the level has no
/// light-switch panel.
pub fn debug_pose_facing_nearest_light_switch(
    player_pos: Vec2,
    polygon_vertices: &[Vec<Vec2>],
    polygon_adjacency: &[Vec<(usize, Option<usize>)>],
    line_endpoints: &[(Vec2, Vec2)],
    panels: &[ControlPanel],
) -> Option<LightSwitchPose> {
    debug_poses_facing_light_switches(
        player_pos,
        polygon_vertices,
        polygon_adjacency,
        line_endpoints,
        panels,
    )
    .into_iter()
    .next()
}

/// DEBUG-ONLY. Like [`debug_pose_facing_nearest_light_switch`] but returns
/// EVERY light-switch panel's standoff pose, sorted nearest-first. The caller
/// (which has ECS access to the actual `Light` components) can then pick the
/// nearest switch whose controlled light is *observably* togglable — some
/// lights run a continuously-oscillating function in their hold states, so the
/// action-key snap is immediately re-animated away and no steady change is
/// visible. The geometry-only layer here cannot see that, so it offers all
/// candidates and lets the sim layer choose.
pub fn debug_poses_facing_light_switches(
    player_pos: Vec2,
    polygon_vertices: &[Vec<Vec2>],
    polygon_adjacency: &[Vec<(usize, Option<usize>)>],
    line_endpoints: &[(Vec2, Vec2)],
    panels: &[ControlPanel],
) -> Vec<LightSwitchPose> {
    /// Standoff distance back from the panel line; must be < the panel
    /// activation range (1.5 WU) and > the raycast near epsilon.
    const STANDOFF: f32 = 0.75;

    let mut candidates: Vec<(f32, LightSwitchPose)> = Vec::new();

    for panel in panels {
        let light_index = match panel.action {
            PanelAction::ToggleLight { light_index } => light_index,
            _ => continue,
        };

        let (la, lb) = match line_endpoints.get(panel.line_index) {
            Some(&pair) => pair,
            None => continue,
        };
        let line_center = (la + lb) * 0.5;

        // Find the polygon that borders this panel's line so the standoff lands
        // inside a real room.
        let neighbour_poly = match polygon_adjacency
            .iter()
            .position(|adj| adj.iter().any(|&(l, _)| l == panel.line_index))
        {
            Some(p) => p,
            None => continue,
        };
        let verts = match polygon_vertices.get(neighbour_poly) {
            Some(v) if !v.is_empty() => v,
            _ => continue,
        };
        let centroid = verts.iter().copied().fold(Vec2::ZERO, |a, b| a + b) / verts.len() as f32;
        let mut inward = centroid - line_center;
        if inward.length() < 1e-4 {
            let edge = lb - la;
            inward = Vec2::new(-edge.y, edge.x);
        }
        let inward = inward.normalize_or_zero();
        if inward == Vec2::ZERO {
            continue;
        }

        let position = line_center + inward * STANDOFF;
        let to_line = line_center - position;
        let facing = to_line.y.atan2(to_line.x);
        let dist = (line_center - player_pos).length();

        candidates.push((
            dist,
            LightSwitchPose {
                pose: DoorFacingPose {
                    position,
                    facing,
                    polygon: neighbour_poly,
                },
                light_index,
            },
        ));
    }

    candidates.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    candidates.into_iter().map(|(_, r)| r).collect()
}

fn normalize_angle(angle: f32) -> f32 {
    let mut a = angle % std::f32::consts::TAU;
    if a > std::f32::consts::PI {
        a -= std::f32::consts::TAU;
    } else if a < -std::f32::consts::PI {
        a += std::f32::consts::TAU;
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activate_panel_facing_it() {
        let panel = ControlPanel {
            line_index: 0,
            side: 0,
            action: PanelAction::ActivatePlatform { platform_index: 0 },
            max_distance: 2.0,
        };
        let endpoints = vec![(Vec2::new(1.0, -0.5), Vec2::new(1.0, 0.5))];

        // Player at origin, facing east (toward the panel)
        assert!(can_activate_panel(Vec2::ZERO, 0.0, &panel, &endpoints));
    }

    #[test]
    fn cant_activate_panel_facing_away() {
        let panel = ControlPanel {
            line_index: 0,
            side: 0,
            action: PanelAction::ActivatePlatform { platform_index: 0 },
            max_distance: 2.0,
        };
        let endpoints = vec![(Vec2::new(1.0, -0.5), Vec2::new(1.0, 0.5))];

        // Player facing west (away from panel)
        assert!(!can_activate_panel(
            Vec2::ZERO,
            std::f32::consts::PI,
            &panel,
            &endpoints,
        ));
    }

    #[test]
    fn debug_pose_faces_panel_so_activation_succeeds() {
        // Single room, a control panel on the east wall (line 1).
        let polygon_vertices = vec![vec![
            Vec2::new(-2.0, -1.0),
            Vec2::new(1.0, -1.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(-2.0, 1.0),
        ]];
        let polygon_adjacency = vec![vec![(0, None), (1, None), (2, None), (3, None)]];
        let polygon_types = vec![0i16];
        let line_endpoints = vec![
            (Vec2::new(-2.0, -1.0), Vec2::new(1.0, -1.0)),
            (Vec2::new(1.0, -1.0), Vec2::new(1.0, 1.0)), // line 1: east wall
            (Vec2::new(-2.0, 1.0), Vec2::new(1.0, 1.0)),
            (Vec2::new(-2.0, -1.0), Vec2::new(-2.0, 1.0)),
        ];
        let panels = vec![ControlPanel {
            line_index: 1,
            side: 0,
            action: PanelAction::ToggleLight { light_index: 0 },
            max_distance: 1.5,
        }];

        let pose = debug_pose_facing_nearest_door(
            Vec2::new(-1.0, 0.0),
            &polygon_vertices,
            &polygon_adjacency,
            &polygon_types,
            &line_endpoints,
            &panels,
        )
        .expect("a panel pose should be found");

        // The computed pose must satisfy can_activate_panel for that panel.
        assert!(
            can_activate_panel(pose.position, pose.facing, &panels[0], &line_endpoints),
            "debug pose at {:?} facing {} should activate the panel",
            pose.position,
            pose.facing,
        );
        assert_eq!(pose.polygon, 0);
    }

    #[test]
    fn debug_pose_faces_platform_door_for_raycast() {
        // Player room (poly 0) with an adjacent platform door (poly 1, type 5)
        // sharing line 1. The pose should let the action-key raycast hit the door.
        use crate::world::MapGeometry;
        use crate::world_mechanics::action_key::{find_action_key_target, ActionTarget};

        let polygon_vertices = vec![
            vec![
                Vec2::new(-2.0, -1.0),
                Vec2::new(1.0, -1.0),
                Vec2::new(1.0, 1.0),
                Vec2::new(-2.0, 1.0),
            ],
            vec![
                Vec2::new(1.0, -1.0),
                Vec2::new(3.0, -1.0),
                Vec2::new(3.0, 1.0),
                Vec2::new(1.0, 1.0),
            ],
        ];
        let polygon_adjacency = vec![
            vec![(0, None), (1, Some(1)), (2, None), (3, None)],
            vec![(4, None), (5, None), (6, None), (1, Some(0))],
        ];
        let polygon_types = vec![0i16, 5i16];
        let line_endpoints = vec![
            (Vec2::new(-2.0, -1.0), Vec2::new(1.0, -1.0)),
            (Vec2::new(1.0, -1.0), Vec2::new(1.0, 1.0)),
            (Vec2::new(-2.0, 1.0), Vec2::new(1.0, 1.0)),
            (Vec2::new(-2.0, -1.0), Vec2::new(-2.0, 1.0)),
            (Vec2::new(1.0, -1.0), Vec2::new(3.0, -1.0)),
            (Vec2::new(3.0, -1.0), Vec2::new(3.0, 1.0)),
            (Vec2::new(1.0, 1.0), Vec2::new(3.0, 1.0)),
        ];

        let pose = debug_pose_facing_nearest_door(
            Vec2::new(-1.0, 0.0),
            &polygon_vertices,
            &polygon_adjacency,
            &polygon_types,
            &line_endpoints,
            &[],
        )
        .expect("a door pose should be found");

        assert_eq!(
            pose.polygon, 0,
            "player should stand in the non-platform room"
        );

        let geometry = MapGeometry {
            polygon_vertices: polygon_vertices.clone(),
            floor_heights: vec![0.0, 0.0],
            ceiling_heights: vec![3.0, 3.0],
            polygon_adjacency: polygon_adjacency.clone(),
            line_endpoints: line_endpoints.clone(),
            line_solid: vec![true, false, true, true, true, true, true],
            line_transparent: vec![false, true, false, false, false, false, false],
            polygon_media_index: vec![-1, -1],
            polygon_floor_light_index: vec![-1, -1],
            polygon_ceiling_light_index: vec![-1, -1],
            polygon_types: polygon_types.clone(),
            polygon_permutations: vec![-1, 0],
            line_side_indices: vec![(None, None); 7],
            changed_polygons: vec![false; 2],
            has_changes: false,
        };

        let target = find_action_key_target(
            pose.position,
            pose.facing,
            pose.polygon,
            &geometry,
            &ControlPanels::default(),
        );
        assert_eq!(
            target,
            ActionTarget::Platform(1),
            "debug pose at {:?} facing {} should raycast onto the door platform",
            pose.position,
            pose.facing,
        );
    }

    #[test]
    fn debug_light_switch_pose_activates_a_toggle_light_panel() {
        // Single room; a light switch (ToggleLight) on the east wall (line 1),
        // plus a non-light panel that must NOT be selected.
        let polygon_vertices = vec![vec![
            Vec2::new(-2.0, -1.0),
            Vec2::new(1.0, -1.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(-2.0, 1.0),
        ]];
        let polygon_adjacency = vec![vec![(0, None), (1, None), (2, None), (3, None)]];
        let line_endpoints = vec![
            (Vec2::new(-2.0, -1.0), Vec2::new(1.0, -1.0)),
            (Vec2::new(1.0, -1.0), Vec2::new(1.0, 1.0)), // line 1: east wall
            (Vec2::new(-2.0, 1.0), Vec2::new(1.0, 1.0)),
            (Vec2::new(-2.0, -1.0), Vec2::new(-2.0, 1.0)),
        ];
        let panels = vec![
            ControlPanel {
                line_index: 3, // a non-light panel (west wall) — must be ignored
                side: 0,
                action: PanelAction::ActivateTerminal { terminal_index: 2 },
                max_distance: 1.5,
            },
            ControlPanel {
                line_index: 1,
                side: 0,
                action: PanelAction::ToggleLight { light_index: 7 },
                max_distance: 1.5,
            },
        ];

        let result = debug_pose_facing_nearest_light_switch(
            Vec2::new(-1.0, 0.0),
            &polygon_vertices,
            &polygon_adjacency,
            &line_endpoints,
            &panels,
        )
        .expect("a light-switch pose should be found");

        // It must select the light-switch panel, reporting its light index.
        assert_eq!(result.light_index, 7);
        assert_eq!(result.pose.polygon, 0);

        // The computed pose must satisfy can_activate_panel for that switch.
        let switch = &panels[1];
        assert!(
            can_activate_panel(
                result.pose.position,
                result.pose.facing,
                switch,
                &line_endpoints
            ),
            "light-switch debug pose at {:?} facing {} should activate the switch",
            result.pose.position,
            result.pose.facing,
        );
    }

    #[test]
    fn debug_light_switch_pose_none_when_no_light_panel() {
        let polygon_vertices = vec![vec![
            Vec2::new(-1.0, -1.0),
            Vec2::new(1.0, -1.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(-1.0, 1.0),
        ]];
        let polygon_adjacency = vec![vec![(0, None)]];
        let line_endpoints = vec![(Vec2::new(1.0, -1.0), Vec2::new(1.0, 1.0))];
        let panels = vec![ControlPanel {
            line_index: 0,
            side: 0,
            action: PanelAction::ActivatePlatform { platform_index: 0 },
            max_distance: 1.5,
        }];

        assert!(debug_pose_facing_nearest_light_switch(
            Vec2::ZERO,
            &polygon_vertices,
            &polygon_adjacency,
            &line_endpoints,
            &panels,
        )
        .is_none());
    }

    #[test]
    fn cant_activate_panel_too_far() {
        let panel = ControlPanel {
            line_index: 0,
            side: 0,
            action: PanelAction::ActivateTerminal { terminal_index: 5 },
            max_distance: 1.0,
        };
        let endpoints = vec![(Vec2::new(5.0, -0.5), Vec2::new(5.0, 0.5))];

        assert!(!can_activate_panel(Vec2::ZERO, 0.0, &panel, &endpoints));
    }
}
