use crate::player::movement::{
    apply_player_collision, compute_facing, compute_player_velocity, compute_vertical_look,
    velocity_local_to_world, velocity_world_to_local, PlayerPhysicsParams,
};
use crate::world::{MapGeometry, PhysicsTables, SimRng, SimWorld, TickCounter};

/// Check if a position is submerged in media at a given polygon.
fn is_submerged_at(
    polygon_media_index: &[i16],
    media_data: &std::collections::HashMap<usize, (f32, i16)>,
    polygon: usize,
    z: f32,
) -> bool {
    let media_idx = polygon_media_index.get(polygon).copied().unwrap_or(-1);
    if media_idx >= 0 {
        if let Some(&(media_height, _)) = media_data.get(&(media_idx as usize)) {
            return z < media_height;
        }
    }
    false
}

/// Action flags consumed by the simulation each tick.
/// Mirrors marathon-integration's ActionFlags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ActionFlags {
    bits: u32,
}

impl ActionFlags {
    pub const MOVE_FORWARD: u32 = 1 << 0;
    pub const MOVE_BACKWARD: u32 = 1 << 1;
    pub const STRAFE_LEFT: u32 = 1 << 2;
    pub const STRAFE_RIGHT: u32 = 1 << 3;
    pub const TURN_LEFT: u32 = 1 << 4;
    pub const TURN_RIGHT: u32 = 1 << 5;
    pub const LOOK_UP: u32 = 1 << 6;
    pub const LOOK_DOWN: u32 = 1 << 7;
    pub const FIRE_PRIMARY: u32 = 1 << 8;
    pub const FIRE_SECONDARY: u32 = 1 << 9;
    pub const ACTION: u32 = 1 << 10;
    pub const CYCLE_WEAPON_FWD: u32 = 1 << 11;
    pub const CYCLE_WEAPON_BACK: u32 = 1 << 12;
    pub const TOGGLE_MAP: u32 = 1 << 13;
    pub const MICROPHONE: u32 = 1 << 14;

    pub fn new(bits: u32) -> Self {
        Self { bits }
    }

    pub fn contains(&self, flag: u32) -> bool {
        self.bits & flag != 0
    }

    pub fn is_empty(&self) -> bool {
        self.bits == 0
    }
}

/// Per-tick input resource injected before systems run.
#[derive(Debug, Default, bevy_ecs::prelude::Resource)]
pub struct TickInput {
    pub action_flags: ActionFlags,
    /// Mouse yaw delta in radians (positive = turn left / counter-clockwise).
    pub mouse_yaw: f32,
    /// Mouse pitch delta in radians (positive = look up).
    pub mouse_pitch: f32,
}

/// Persistent edge-detection state for the ACTION key.
///
/// Door / control-panel activation is a one-shot event: a single press must
/// activate a target exactly once, no matter how many ticks the key is held.
/// We detect a rising edge (previous tick clear -> this tick set) by stashing
/// last tick's ACTION state here. Stored as a resource so it persists across
/// ticks alongside the rest of the sim state.
#[derive(Debug, Default, Clone, Copy, bevy_ecs::prelude::Resource)]
pub struct PrevActionKey(pub bool);

impl From<ActionFlags> for TickInput {
    fn from(action_flags: ActionFlags) -> Self {
        TickInput {
            action_flags,
            mouse_yaw: 0.0,
            mouse_pitch: 0.0,
        }
    }
}

impl SimWorld {
    /// Advance the simulation by one tick (1/30th of a second).
    ///
    /// Systems execute in order:
    /// 1. Input processing
    /// 2. Player physics
    /// 3. Monster AI
    /// 4. Weapon/combat
    /// 5. Projectile physics
    /// 6. Damage resolution
    /// 7. World mechanics (platforms, lights, media, items)
    /// 8. Cleanup
    pub fn tick(&mut self, input: TickInput) {
        // Store input for this tick
        self.world.insert_resource(input);

        // Systems execute in alephone's update_world() order:
        // 1. Update lights
        self.run_light_updates();
        // 2. Update media (depends on light intensities)
        self.update_media();
        // 3. Update platforms (before player physics so collision uses new heights)
        self.update_platforms();
        // 3b. Action key processing (doors and control panels)
        self.process_action_key();
        // 4. Player physics
        self.run_player_physics();
        // 5. Player weapons (depends on player position/facing)
        self.run_player_weapons();
        // 6. Update monsters (depends on player position)
        self.update_monsters();
        // 6b. Update agents (reconcile the daemon-fed desired-set, parallel to
        //     monsters; box 1.5 call seam — no agent behavior yet)
        self.update_agents();
        // 7. Update projectiles (processes monster-spawned projectiles)
        self.update_projectiles();
        // 8. Update effects (cleanup)
        self.update_effects();
        // 9. Update items (pickup check)
        self.update_items();

        // Advance tick counter
        self.world.resource_mut::<TickCounter>().0 += 1;
    }

    fn run_player_physics(&mut self) {
        let tick_input = self.world.resource::<TickInput>();
        let flags = tick_input.action_flags;
        let mouse_yaw = tick_input.mouse_yaw;
        let mouse_pitch = tick_input.mouse_pitch;

        let Some(params) = self.world.get_resource::<PlayerPhysicsParams>().cloned() else {
            return;
        };
        let geometry = self.world.resource::<MapGeometry>();
        // Clone what we need from geometry to avoid borrow conflicts
        let geo_clone = geometry.clone();

        // Collect media data for submersion checks
        let media_data: std::collections::HashMap<usize, (f32, i16)> = {
            let mut map = std::collections::HashMap::new();
            let mut q = self.world.query::<&crate::Media>();
            for media in q.iter(&self.world) {
                map.insert(media.index, (media.current_height, media.media_type));
            }
            map
        };

        let mut query = self.world.query_filtered::<(
            &mut crate::Position,
            &mut crate::Velocity,
            &mut crate::Facing,
            &mut crate::VerticalLook,
            &mut crate::AngularVelocity,
            &mut crate::PolygonIndex,
            &mut crate::Grounded,
            &mut crate::Oxygen,
            &mut crate::Health,
            &mut crate::Shield,
        ), bevy_ecs::prelude::With<crate::Player>>();

        for (
            mut pos,
            mut vel,
            mut facing,
            mut vlook,
            mut angular_vel,
            mut poly_idx,
            mut grounded,
            mut oxygen,
            mut health,
            mut shield,
        ) in query.iter_mut(&mut self.world)
        {
            // Velocity is stored in player-local frame: x=forward, y=perp, z=vert.
            // Compute the next tick's player-local velocity from input.
            let new_local_vel =
                compute_player_velocity(vel.0, facing.0, &flags, &params, grounded.0);

            // Compute facing (turning) — mouse yaw applied directly, keyboard via angular velocity
            let (new_facing, new_angular) =
                compute_facing(facing.0, angular_vel.0, &flags, &params, mouse_yaw);
            facing.0 = new_facing;
            angular_vel.0 = new_angular;

            // Compute vertical look — mouse pitch applied directly
            vlook.0 = compute_vertical_look(vlook.0, &flags, &params, mouse_pitch);

            // Project player-local velocity into world-space using the NEW facing.
            let world_vel = velocity_local_to_world(new_local_vel, new_facing);

            // Apply collision in world-space.
            let new_pos = pos.0 + world_vel;
            let result =
                apply_player_collision(pos.0, new_pos, world_vel, poly_idx.0, &params, &geo_clone);

            pos.0 = result.position;
            vel.0 = velocity_world_to_local(result.velocity, new_facing);
            poly_idx.0 = result.polygon_index;
            grounded.0 = result.grounded;

            // Media interaction: check if player is submerged
            let media_idx = geo_clone
                .polygon_media_index
                .get(poly_idx.0)
                .copied()
                .unwrap_or(-1);
            if media_idx >= 0 {
                if let Some(&(media_height, media_type)) = media_data.get(&(media_idx as usize)) {
                    if pos.0.z < media_height {
                        // Submerged: apply drag to velocity
                        let drag = crate::world_mechanics::media::media_drag_factor(media_type);
                        vel.0 *= drag;

                        // Decrement oxygen
                        oxygen.0 = (oxygen.0 - 1).max(0);

                        // Apply media damage if applicable
                        if crate::world_mechanics::media::media_deals_damage(media_type) {
                            let (new_health, new_shield, _) =
                                crate::combat::damage::apply_damage(1, health.0, shield.0);
                            health.0 = new_health;
                            shield.0 = new_shield;
                        }

                        // Drowning damage when oxygen is zero
                        if oxygen.0 <= 0 {
                            let (new_health, new_shield, _) =
                                crate::combat::damage::apply_damage(5, health.0, shield.0);
                            health.0 = new_health;
                            shield.0 = new_shield;
                        }
                    } else {
                        // Above surface: recharge oxygen
                        oxygen.0 = (oxygen.0 + 1).min(600);
                    }
                }
            }
        }
    }

    // ─── Helpers ──────────────────────────────────────────────────────────

    /// Apply damage to an entity that has Health and optionally Shield.
    /// Reads shield first (immutable) then writes both (avoids borrow conflicts).
    fn apply_damage_to_entity(&mut self, entity: bevy_ecs::entity::Entity, damage: i16) {
        let shield_val = self
            .world
            .get::<crate::Shield>(entity)
            .map(|s| s.0)
            .unwrap_or(0);
        let health_val = self.world.get::<crate::Health>(entity).map(|h| h.0);
        if let Some(hp) = health_val {
            let (new_h, new_s, _) = crate::combat::damage::apply_damage(damage, hp, shield_val);
            if let Some(mut health) = self.world.get_mut::<crate::Health>(entity) {
                health.0 = new_h;
            }
            if let Some(mut shield) = self.world.get_mut::<crate::Shield>(entity) {
                shield.0 = new_s;
            }
        }
    }

    // ─── Simulation Systems ─────────────────────────────────────────────────

    fn run_light_updates(&mut self) {
        self.world.resource_scope(
            |world: &mut bevy_ecs::prelude::World, mut sim_rng: bevy_ecs::prelude::Mut<SimRng>| {
                let mut query = world.query::<&mut crate::Light>();
                for mut light in query.iter_mut(world) {
                    crate::world_mechanics::lights::update_single_light(&mut light, &mut sim_rng.0);
                }
            },
        );
    }

    fn update_media(&mut self) {
        self.run_media_updates();
    }

    /// Query all `Media` entities, look up each one's associated `Light` by
    /// `light_index`, and recompute `current_height` via `compute_media_height()`.
    ///
    /// Media surfaces (water, lava, …) rise and fall in lockstep with the
    /// intensity of the light they are linked to, mirroring Alephone's
    /// `update_medias()` pass over the light table.
    fn run_media_updates(&mut self) {
        // Build light intensity lookup by light_index
        let light_intensities: std::collections::HashMap<usize, f32> = {
            let mut map = std::collections::HashMap::new();
            let mut query = self.world.query::<&crate::Light>();
            for light in query.iter(&self.world) {
                map.insert(light.light_index, light.current_intensity);
            }
            map
        };

        // Update media heights based on associated light intensity
        let mut query = self.world.query::<&mut crate::Media>();
        for mut media in query.iter_mut(&mut self.world) {
            if let Some(&intensity) = light_intensities.get(&media.light_index) {
                media.current_height =
                    crate::world_mechanics::media::compute_media_height(&media, intensity);
            }
        }
    }

    fn process_action_key(&mut self) {
        let tick_input = self.world.resource::<TickInput>();
        let action_now = tick_input.action_flags.contains(ActionFlags::ACTION);

        // Edge-detect: update the stored previous-ACTION state EVERY tick (even
        // on release) so the edge re-arms, then only act on a rising edge.
        let action_prev = self.world.resource::<PrevActionKey>().0;
        self.world.resource_mut::<PrevActionKey>().0 = action_now;
        let rising_edge = action_now && !action_prev;
        if !rising_edge {
            return;
        }

        // Get player position, facing, polygon
        let player_data: Option<(glam::Vec2, f32, usize)> = {
            let mut q = self.world.query_filtered::<(
                &crate::Position,
                &crate::Facing,
                &crate::PolygonIndex,
            ), bevy_ecs::prelude::With<crate::Player>>();
            q.iter(&self.world)
                .next()
                .map(|(pos, fac, poly)| (glam::Vec2::new(pos.0.x, pos.0.y), fac.0, poly.0))
        };

        let (player_pos, player_facing, player_poly) = match player_data {
            Some(data) => data,
            None => return,
        };

        // Get control panels resource
        let panels = self
            .world
            .get_resource::<crate::world_mechanics::panels::ControlPanels>()
            .cloned()
            .unwrap_or_default();
        let geometry = self.world.resource::<MapGeometry>().clone();

        let target = crate::world_mechanics::action_key::find_action_key_target(
            player_pos,
            player_facing,
            player_poly,
            &geometry,
            &panels,
        );

        match target {
            crate::world_mechanics::action_key::ActionTarget::Platform(platform_poly_idx) => {
                let mut query = self.world.query::<&mut crate::Platform>();
                for mut platform in query.iter_mut(&mut self.world) {
                    if platform.polygon_index == platform_poly_idx {
                        if crate::world_mechanics::platforms::should_activate(
                            &platform,
                            crate::world_mechanics::platforms::PlatformTrigger::ActionKey,
                        ) {
                            crate::world_mechanics::platforms::activate_platform(&mut platform);
                        } else if platform.state == crate::PlatformState::Extending {
                            platform.state = crate::PlatformState::Returning;
                        } else if platform.state == crate::PlatformState::Returning {
                            platform.state = crate::PlatformState::Extending;
                        }
                        break;
                    }
                }
            }
            crate::world_mechanics::action_key::ActionTarget::Panel(panel_idx) => {
                self.execute_panel_action(panel_idx, &panels);
            }
            crate::world_mechanics::action_key::ActionTarget::None => {}
        }
    }

    fn execute_panel_action(
        &mut self,
        panel_idx: usize,
        panels: &crate::world_mechanics::panels::ControlPanels,
    ) {
        let panel = match panels.0.get(panel_idx) {
            Some(p) => p,
            None => return,
        };

        match &panel.action {
            crate::world_mechanics::panels::PanelAction::ActivatePlatform { platform_index } => {
                let target_idx = *platform_index;
                let mut query = self.world.query::<&mut crate::Platform>();
                for mut platform in query.iter_mut(&mut self.world) {
                    if platform.polygon_index == target_idx {
                        crate::world_mechanics::platforms::activate_platform(&mut platform);
                        break;
                    }
                }
            }
            crate::world_mechanics::panels::PanelAction::ToggleLight { light_index } => {
                let target_idx = *light_index;
                let mut query = self.world.query::<&mut crate::Light>();
                for mut light in query.iter_mut(&mut self.world) {
                    if light.light_index == target_idx {
                        // Flip the activation ramp: lit -> begin deactivating,
                        // dark -> begin activating. Snap to the target extreme so
                        // the toggle reads immediately; the state machine carries
                        // on from the new state on subsequent ticks.
                        let lit = light.current_intensity > 0.5;
                        light.state = if lit {
                            crate::components::LightState::BecomingInactive
                        } else {
                            crate::components::LightState::BecomingActive
                        };
                        light.initial_intensity = light.current_intensity;
                        light.final_intensity = if lit { 0.0 } else { 1.0 };
                        light.current_intensity = light.final_intensity;
                        light.phase = 0;
                        break;
                    }
                }
            }
            crate::world_mechanics::panels::PanelAction::ActivateTaggedPlatforms { tag } => {
                let tag_val = *tag;
                let geometry = self.world.resource::<MapGeometry>();
                let matching_polys: Vec<usize> = geometry
                    .polygon_types
                    .iter()
                    .zip(geometry.polygon_permutations.iter())
                    .enumerate()
                    .filter(|(_, (&ptype, &perm))| ptype == 5 && perm == tag_val)
                    .map(|(idx, _)| idx)
                    .collect();

                let mut query = self.world.query::<&mut crate::Platform>();
                for mut platform in query.iter_mut(&mut self.world) {
                    if matching_polys.contains(&platform.polygon_index) {
                        crate::world_mechanics::platforms::activate_platform(&mut platform);
                    }
                }
            }
            crate::world_mechanics::panels::PanelAction::ActivateTerminal { terminal_index } => {
                let idx = *terminal_index;
                self.world.resource_mut::<crate::world::SimEvents>().push(
                    crate::world::SimEvent::TerminalActivation {
                        terminal_index: idx,
                    },
                );
            }
        }
    }

    fn update_platforms(&mut self) {
        // Tick all platforms, collect height updates
        let mut height_updates: Vec<(usize, f32, f32)> = Vec::new();
        {
            let mut query = self.world.query::<&mut crate::Platform>();
            for mut platform in query.iter_mut(&mut self.world) {
                let (floor, ceiling) =
                    crate::world_mechanics::platforms::tick_platform(&mut platform);
                height_updates.push((platform.polygon_index, floor, ceiling));
            }
        }

        // Write back heights to MapGeometry
        {
            let mut geometry = self.world.resource_mut::<MapGeometry>();
            for &(poly_idx, floor, ceiling) in &height_updates {
                if poly_idx < geometry.floor_heights.len() {
                    geometry.floor_heights[poly_idx] = floor;
                }
                if poly_idx < geometry.ceiling_heights.len() {
                    geometry.ceiling_heights[poly_idx] = ceiling;
                }
            }
        }

        // Player-entry activation check
        let player_poly = {
            let mut q = self
                .world
                .query_filtered::<&crate::PolygonIndex, bevy_ecs::prelude::With<crate::Player>>();
            q.iter(&self.world).next().map(|p| p.0)
        };

        if let Some(player_poly) = player_poly {
            let mut query = self.world.query::<&mut crate::Platform>();
            for mut platform in query.iter_mut(&mut self.world) {
                if platform.polygon_index == player_poly {
                    use crate::world_mechanics::platforms::{
                        activate_platform, should_activate, PlatformTrigger,
                    };
                    if should_activate(&platform, PlatformTrigger::PlayerEntry) {
                        activate_platform(&mut platform);
                    }
                }
            }
        }

        // Crush check
        let player_data: Option<(f32, f32, usize)> = {
            let mut q = self.world.query_filtered::<(
                &crate::Position,
                &crate::EntityHeight,
                &crate::PolygonIndex,
            ), bevy_ecs::prelude::With<crate::Player>>();
            q.iter(&self.world)
                .next()
                .map(|(pos, h, poly)| (pos.0.z, h.0, poly.0))
        };

        if let Some((player_z, player_height, player_poly)) = player_data {
            let mut crush_damage: Option<i16> = None;
            let mut reverse_polys: Vec<usize> = Vec::new();

            {
                let mut query = self.world.query::<&crate::Platform>();
                for platform in query.iter(&self.world) {
                    if platform.polygon_index == player_poly {
                        use crate::world_mechanics::platforms::{
                            check_platform_crush, PlatformCrushResult,
                        };
                        match check_platform_crush(platform, player_z, player_height) {
                            PlatformCrushResult::Crush { damage } => {
                                crush_damage = Some(damage);
                            }
                            PlatformCrushResult::Reverse => {
                                reverse_polys.push(platform.polygon_index);
                            }
                            PlatformCrushResult::None => {}
                        }
                    }
                }
            }

            if let Some(damage) = crush_damage {
                let mut q = self.world.query_filtered::<(
                    &mut crate::Health,
                    &mut crate::Shield,
                ), bevy_ecs::prelude::With<crate::Player>>();
                if let Some((mut health, mut shield)) = q.iter_mut(&mut self.world).next() {
                    let (new_h, new_s, _) =
                        crate::combat::damage::apply_damage(damage, health.0, shield.0);
                    health.0 = new_h;
                    shield.0 = new_s;
                }
            }

            if !reverse_polys.is_empty() {
                let mut query = self.world.query::<&mut crate::Platform>();
                for mut platform in query.iter_mut(&mut self.world) {
                    if reverse_polys.contains(&platform.polygon_index) {
                        use crate::PlatformState;
                        if platform.state == PlatformState::Extending {
                            platform.state = PlatformState::Returning;
                        } else if platform.state == PlatformState::Returning {
                            platform.state = PlatformState::Extending;
                        }
                    }
                }
            }
        }
    }

    fn run_player_weapons(&mut self) {
        let tick_input = self.world.resource::<TickInput>();
        let flags = tick_input.action_flags;

        let fire_primary = flags.contains(ActionFlags::FIRE_PRIMARY);
        let fire_secondary = flags.contains(ActionFlags::FIRE_SECONDARY);
        let cycle_fwd = flags.contains(ActionFlags::CYCLE_WEAPON_FWD);
        let cycle_back = flags.contains(ActionFlags::CYCLE_WEAPON_BACK);

        // Get physics tables for weapon definitions
        let physics_tables = self
            .world
            .get_resource::<PhysicsTables>()
            .map(|pt| pt.data.clone());

        // Get player position and facing for projectile spawn
        let player_data: Option<(glam::Vec3, f32, usize, bevy_ecs::entity::Entity)> = {
            let mut q = self.world.query_filtered::<(
                bevy_ecs::entity::Entity,
                &crate::Position,
                &crate::Facing,
                &crate::PolygonIndex,
            ), bevy_ecs::prelude::With<crate::Player>>();
            q.iter(&self.world)
                .next()
                .map(|(e, pos, fac, poly)| (pos.0, fac.0, poly.0, e))
        };

        let Some((player_pos, player_facing, player_poly, player_entity)) = player_data else {
            return;
        };
        let Some(physics) = physics_tables else {
            return;
        };

        // Handle weapon cycling
        if cycle_fwd {
            if let Some(mut inv) = self
                .world
                .get_resource_mut::<crate::player::inventory::WeaponInventory>()
            {
                inv.cycle_forward(10);
            }
        }
        if cycle_back {
            if let Some(mut inv) = self
                .world
                .get_resource_mut::<crate::player::inventory::WeaponInventory>()
            {
                inv.cycle_backward(10);
            }
        }

        // Handle firing
        if !fire_primary && !fire_secondary {
            // Still need to tick weapon cooldowns
            if let Some(mut inv) = self
                .world
                .get_resource_mut::<crate::player::inventory::WeaponInventory>()
            {
                if let Some(weapon) = inv.current_mut() {
                    crate::combat::weapons::tick_weapon(weapon, false, 2, 3);
                }
            }
            return;
        }

        // Get weapon definition and tick
        let mut projectile_spawn: Option<(usize, glam::Vec3, glam::Vec3)> = None;

        if let Some(mut inv) = self
            .world
            .get_resource_mut::<crate::player::inventory::WeaponInventory>()
        {
            let weapon_def_idx = inv.current().map(|w| w.definition_index);
            if let Some(def_idx) = weapon_def_idx {
                // Look up weapon definition
                let weapon_def = physics.weapons.as_ref().and_then(|w| w.get(def_idx));
                if let Some(wdef) = weapon_def {
                    let ticks_per_round = wdef.primary_trigger.ticks_per_round as u16;
                    let recovery_ticks = wdef.primary_trigger.recovery_ticks as u16;
                    let proj_type = wdef.primary_trigger.projectile_type;

                    if let Some(weapon) = inv.current_mut() {
                        let fired = crate::combat::weapons::tick_weapon(
                            weapon,
                            fire_primary,
                            ticks_per_round,
                            recovery_ticks,
                        );

                        if fired && proj_type >= 0 {
                            // Spawn projectile
                            let speed = physics
                                .projectiles
                                .as_ref()
                                .and_then(|p| p.get(proj_type as usize))
                                .map(|p| p.speed as f32 / 1024.0)
                                .unwrap_or(0.5);

                            let dir =
                                glam::Vec3::new(player_facing.cos(), player_facing.sin(), 0.0);
                            let spawn_pos = player_pos + dir * 0.3 + glam::Vec3::new(0.0, 0.0, 0.4);
                            let velocity = dir * speed;

                            projectile_spawn = Some((proj_type as usize, spawn_pos, velocity));
                        }
                    }
                }
            }
        }

        // Spawn projectile entity outside the resource borrow
        if let Some((proj_type, spawn_pos, velocity)) = projectile_spawn {
            self.world.spawn((
                crate::Projectile {
                    definition_index: proj_type,
                    distance_traveled: 0.0,
                    contrails_spawned: 0,
                    ticks_alive: 0,
                    current_polygon: player_poly,
                },
                crate::Position(spawn_pos),
                crate::Velocity(velocity),
                crate::PolygonIndex(player_poly),
                crate::ProjectileSource(player_entity),
            ));
        }
    }

    fn update_monsters(&mut self) {
        // Get player position and entity
        let player_data: Option<(glam::Vec3, bevy_ecs::entity::Entity)> = {
            let mut q = self.world.query_filtered::<(
                bevy_ecs::entity::Entity,
                &crate::Position,
            ), bevy_ecs::prelude::With<crate::Player>>();
            q.iter(&self.world).next().map(|(e, pos)| (pos.0, e))
        };

        let Some((player_pos, player_entity)) = player_data else {
            return;
        };

        // Clone physics tables
        let physics_tables = match self.world.get_resource::<PhysicsTables>() {
            Some(pt) => pt.data.clone(),
            None => return,
        };

        // Clone geometry for floor heights
        let floor_heights = self.world.resource::<MapGeometry>().floor_heights.clone();

        // Collect monster data for processing
        struct MonsterUpdate {
            entity: bevy_ecs::entity::Entity,
            new_state: crate::MonsterState,
            new_pos: Option<glam::Vec3>,
            new_vel: Option<glam::Vec3>,
            new_facing: Option<f32>,
            new_poly: Option<usize>,
            new_grounded: Option<bool>,
            attack_cooldown: Option<u16>,
            target: Option<bevy_ecs::entity::Entity>,
            // For cascade alerts
            cascade_source: bool,
            cascade_pos: glam::Vec2,
            cascade_class: usize,
            cascade_enemies: u32,
        }

        let mut updates: Vec<MonsterUpdate> = Vec::new();
        let mut projectile_spawns: Vec<(usize, glam::Vec3, glam::Vec3, usize)> = Vec::new();
        let mut damage_to_player: i16 = 0;

        // Collect all monster data
        {
            let mut query = self.world.query::<(
                bevy_ecs::entity::Entity,
                &crate::Monster,
                &crate::MonsterState,
                &crate::Position,
                &crate::Velocity,
                &crate::Facing,
                &crate::Health,
                &crate::AttackCooldown,
                &crate::PolygonIndex,
                &crate::Grounded,
                Option<&crate::Flying>,
            )>();

            for (
                entity,
                monster,
                state,
                pos,
                vel,
                facing,
                health,
                cooldown,
                poly_idx,
                _grounded,
                flying,
            ) in query.iter(&self.world)
            {
                if *state == crate::MonsterState::Dead {
                    updates.push(MonsterUpdate {
                        entity,
                        new_state: crate::MonsterState::Dead,
                        new_pos: None,
                        new_vel: None,
                        new_facing: None,
                        new_poly: None,
                        new_grounded: None,
                        attack_cooldown: None,
                        target: None,
                        cascade_source: false,
                        cascade_pos: glam::Vec2::ZERO,
                        cascade_class: 0,
                        cascade_enemies: 0,
                    });
                    continue;
                }

                let monster_pos_2d = glam::Vec2::new(pos.0.x, pos.0.y);
                let player_pos_2d = glam::Vec2::new(player_pos.x, player_pos.y);

                // Get monster definition
                let def = physics_tables
                    .monsters
                    .as_ref()
                    .and_then(|m| m.get(monster.definition_index));

                let visual_range = def.map(|d| d.visual_range as f32 / 1024.0).unwrap_or(10.0);
                let half_arc = def
                    .map(|d| (d.half_visual_arc as f32) * std::f32::consts::TAU / 512.0)
                    .unwrap_or(std::f32::consts::FRAC_PI_4);
                let speed = def.map(|d| d.speed as f32 / 1024.0).unwrap_or(0.02);
                let melee_range = def
                    .map(|d| d.melee_attack.range as f32 / 1024.0)
                    .unwrap_or(0.5);
                let ranged_range = def
                    .map(|d| d.ranged_attack.range as f32 / 1024.0)
                    .unwrap_or(5.0);
                let monster_class = def.map(|d| d.monster_class as usize).unwrap_or(0);
                let monster_enemies = def.map(|d| d.enemies as u32).unwrap_or(0);
                let gravity = def.map(|d| d.gravity as f32 / 1024.0).unwrap_or(0.01);
                let terminal_vel = def
                    .map(|d| d.terminal_velocity as f32 / 1024.0)
                    .unwrap_or(0.5);

                // Vision check
                let can_see = crate::monster::ai::can_see_target(
                    monster_pos_2d,
                    facing.0,
                    player_pos_2d,
                    visual_range,
                    half_arc,
                );

                let distance = monster_pos_2d.distance(player_pos_2d);
                let in_melee = distance <= melee_range;
                let in_ranged = distance <= ranged_range;
                let vitality_zero = health.0 <= 0;

                // Determine next state
                let has_target = can_see || *state != crate::MonsterState::Idle;
                let new_state = crate::monster::ai::next_state(
                    *state,
                    can_see,
                    has_target,
                    in_melee,
                    in_ranged,
                    vitality_zero,
                );

                let was_idle = *state == crate::MonsterState::Idle;
                let now_alerted = new_state == crate::MonsterState::Alerted;
                let cascade = was_idle && now_alerted;

                // Movement
                let mut new_pos_val = None;
                let mut new_vel_val = None;
                let mut new_facing_val = None;
                let new_poly_val = None;
                let mut new_grounded_val = None;

                if new_state == crate::MonsterState::Moving {
                    let dir_to_player = player_pos_2d - monster_pos_2d;
                    let angle_to_player = dir_to_player.y.atan2(dir_to_player.x);
                    new_facing_val = Some(angle_to_player);

                    if let Some(fly) = flying {
                        let new_vel = crate::monster::ai::compute_flying_movement(
                            pos.0,
                            player_pos,
                            speed,
                            fly.preferred_hover_height,
                            floor_heights.get(poly_idx.0).copied().unwrap_or(0.0),
                        );
                        let next_pos = pos.0 + new_vel;
                        new_pos_val = Some(next_pos);
                        new_vel_val = Some(new_vel);
                    } else {
                        // Ground movement: move toward player in XY
                        let move_dir = dir_to_player.normalize_or_zero() * speed;
                        let next_xy = glam::Vec2::new(pos.0.x + move_dir.x, pos.0.y + move_dir.y);

                        // Apply gravity
                        let floor_h = floor_heights.get(poly_idx.0).copied().unwrap_or(0.0);
                        let (new_z, new_vel_z, is_grounded) =
                            crate::monster::ai::apply_monster_gravity(
                                pos.0.z,
                                vel.0.z,
                                floor_h,
                                gravity,
                                terminal_vel,
                            );

                        new_pos_val = Some(glam::Vec3::new(next_xy.x, next_xy.y, new_z));
                        new_vel_val = Some(glam::Vec3::new(move_dir.x, move_dir.y, new_vel_z));
                        new_grounded_val = Some(is_grounded);
                    }
                }

                // Attack handling
                let mut new_cooldown = None;
                if new_state == crate::MonsterState::Attacking {
                    let cd = if cooldown.0 > 0 { cooldown.0 - 1 } else { 0 };

                    let melee_damage_base = def.map(|d| d.shrapnel_damage.base).unwrap_or(10);
                    let melee_damage_random = def.map(|d| d.shrapnel_damage.random).unwrap_or(5);
                    let melee_damage_type = def.map(|d| d.shrapnel_damage.damage_type).unwrap_or(0);
                    let melee_damage_scale = def.map(|d| d.shrapnel_damage.scale).unwrap_or(1.0);
                    let ranged_proj_type = def
                        .map(|d| d.ranged_attack.attack_type as usize)
                        .unwrap_or(0);
                    let attack_frequency = def.map(|d| d.attack_frequency as u16).unwrap_or(30);

                    let attack_result = crate::monster::ai::compute_monster_attack(
                        new_state,
                        distance,
                        cd,
                        melee_range,
                        melee_damage_base,
                        melee_damage_random,
                        melee_damage_type,
                        melee_damage_scale,
                        ranged_range,
                        ranged_proj_type,
                        glam::Vec3::new(0.0, 0.0, 0.3),
                        0.05,
                    );

                    match attack_result {
                        crate::monster::ai::AttackResult::Melee {
                            damage_base,
                            damage_random: _,
                            damage_type: _,
                            damage_scale,
                            ..
                        } => {
                            damage_to_player += (damage_base as f32 * damage_scale) as i16;
                            new_cooldown = Some(attack_frequency);
                        }
                        crate::monster::ai::AttackResult::Ranged {
                            projectile_type,
                            offset,
                            ..
                        } => {
                            let spawn_pos = pos.0 + offset;
                            let dir = (player_pos - spawn_pos).normalize_or_zero();
                            let proj_speed = physics_tables
                                .projectiles
                                .as_ref()
                                .and_then(|p| p.get(projectile_type))
                                .map(|p| p.speed as f32 / 1024.0)
                                .unwrap_or(0.3);
                            projectile_spawns.push((
                                projectile_type,
                                spawn_pos,
                                dir * proj_speed,
                                poly_idx.0,
                            ));
                            new_cooldown = Some(attack_frequency);
                        }
                        crate::monster::ai::AttackResult::None => {
                            new_cooldown = Some(cd);
                        }
                    }
                }

                updates.push(MonsterUpdate {
                    entity,
                    new_state,
                    new_pos: new_pos_val,
                    new_vel: new_vel_val,
                    new_facing: new_facing_val,
                    new_poly: new_poly_val,
                    new_grounded: new_grounded_val,
                    attack_cooldown: new_cooldown,
                    target: if has_target {
                        Some(player_entity)
                    } else {
                        None
                    },
                    cascade_source: cascade,
                    cascade_pos: monster_pos_2d,
                    cascade_class: monster_class,
                    cascade_enemies: monster_enemies,
                });
            }
        }

        // Apply updates
        for update in &updates {
            let mut entity_ref = self.world.entity_mut(update.entity);
            if let Some(mut state) = entity_ref.get_mut::<crate::MonsterState>() {
                *state = update.new_state;
            }
            if let Some(new_pos) = update.new_pos {
                if let Some(mut pos) = entity_ref.get_mut::<crate::Position>() {
                    pos.0 = new_pos;
                }
            }
            if let Some(new_vel) = update.new_vel {
                if let Some(mut vel) = entity_ref.get_mut::<crate::Velocity>() {
                    vel.0 = new_vel;
                }
            }
            if let Some(new_facing) = update.new_facing {
                if let Some(mut fac) = entity_ref.get_mut::<crate::Facing>() {
                    fac.0 = new_facing;
                }
            }
            if let Some(new_poly) = update.new_poly {
                if let Some(mut poly) = entity_ref.get_mut::<crate::PolygonIndex>() {
                    poly.0 = new_poly;
                }
            }
            if let Some(new_grounded) = update.new_grounded {
                if let Some(mut gr) = entity_ref.get_mut::<crate::Grounded>() {
                    gr.0 = new_grounded;
                }
            }
            if let Some(cd) = update.attack_cooldown {
                if let Some(mut cooldown) = entity_ref.get_mut::<crate::AttackCooldown>() {
                    cooldown.0 = cd;
                }
            }
            if let Some(target) = update.target {
                if let Some(mut t) = entity_ref.get_mut::<crate::Target>() {
                    t.0 = Some(target);
                }
            }
        }

        // Apply cascade alerts
        let cascade_sources: Vec<(glam::Vec2, usize, u32)> = updates
            .iter()
            .filter(|u| u.cascade_source)
            .map(|u| (u.cascade_pos, u.cascade_class, u.cascade_enemies))
            .collect();

        if !cascade_sources.is_empty() {
            // Collect idle monster data for cascade
            let idle_monsters: Vec<(
                glam::Vec2,
                usize,
                u32,
                crate::MonsterState,
                bevy_ecs::entity::Entity,
            )> = {
                let mut q = self.world.query::<(
                    bevy_ecs::entity::Entity,
                    &crate::Monster,
                    &crate::MonsterState,
                    &crate::Position,
                )>();
                q.iter(&self.world)
                    .map(|(e, m, s, p)| {
                        let class = physics_tables
                            .monsters
                            .as_ref()
                            .and_then(|ms| ms.get(m.definition_index))
                            .map(|d| d.monster_class as usize)
                            .unwrap_or(0);
                        let enemies = physics_tables
                            .monsters
                            .as_ref()
                            .and_then(|ms| ms.get(m.definition_index))
                            .map(|d| d.enemies as u32)
                            .unwrap_or(0);
                        (glam::Vec2::new(p.0.x, p.0.y), class, enemies, *s, e)
                    })
                    .collect()
            };

            let cascade_data: Vec<(glam::Vec2, usize, u32, crate::MonsterState)> = idle_monsters
                .iter()
                .map(|(pos, class, enemies, state, _)| (*pos, *class, *enemies, *state))
                .collect();

            for (source_pos, source_class, source_enemies) in &cascade_sources {
                let targets = crate::monster::ai::find_cascade_targets(
                    *source_pos,
                    *source_class,
                    *source_enemies,
                    &cascade_data,
                    10.0, // cascade radius
                );
                for idx in targets {
                    let entity = idle_monsters[idx].4;
                    if let Some(mut state) = self.world.get_mut::<crate::MonsterState>(entity) {
                        if *state == crate::MonsterState::Idle {
                            *state = crate::MonsterState::Alerted;
                        }
                    }
                }
            }
        }

        // Apply damage to player from melee attacks
        if damage_to_player > 0 {
            let mut q = self.world.query_filtered::<(
                &mut crate::Health,
                &mut crate::Shield,
            ), bevy_ecs::prelude::With<crate::Player>>();
            if let Some((mut health, mut shield)) = q.iter_mut(&mut self.world).next() {
                let (new_h, new_s, _) =
                    crate::combat::damage::apply_damage(damage_to_player, health.0, shield.0);
                health.0 = new_h;
                shield.0 = new_s;
            }
        }

        // Spawn monster projectiles
        for (proj_type, spawn_pos, velocity, poly) in projectile_spawns {
            self.world.spawn((
                crate::Projectile {
                    definition_index: proj_type,
                    distance_traveled: 0.0,
                    contrails_spawned: 0,
                    ticks_alive: 0,
                    current_polygon: poly,
                },
                crate::Position(spawn_pos),
                crate::Velocity(velocity),
                crate::PolygonIndex(poly),
            ));
        }
    }

    /// Per-tick agent reconcile seam (box 1.5), called from `tick()` after
    /// [`Self::update_monsters`].
    ///
    /// This is the call seam parallel to `update_monsters()` that the agent
    /// reconciler (boxes 4.x) and the interaction surface (boxes 6.x) hang off.
    /// Each tick it reads the latest desired-set off the installed
    /// [`crate::fleet_bridge::SimBridge`] (latest-wins `watch`, so N publishes
    /// between ticks coalesce into one snapshot) and emits agent
    /// [`marathon_fleet::event::GameAction`]s onto the bridge's outbound sender.
    ///
    /// Box 1.5 deliberately ships ONLY the seam: no diff, no spawn/update/despawn,
    /// no `GameAction` emission. With no bridge installed, or with the seeded empty
    /// desired-set a dead/absent daemon leaves, this is a no-op — the sim keeps
    /// ticking with nothing to reconcile. Real agent behavior lands in later boxes.
    fn update_agents(&mut self) {
        // No bridge installed → nothing to reconcile; the sim ticks on.
        let Some(bridge) = self.fleet_bridge.as_ref() else {
            return;
        };

        // Read the latest desired-set (latest-wins; coalesces intervening
        // publishes). An empty set is the dead/absent-daemon case and is tolerated.
        let desired = bridge.desired.borrow();
        if desired.is_empty() {
            return;
        }

        // Box 1.5 is the call seam only: the diff/spawn/despawn reconcile and the
        // outbound `GameAction` emission land in later boxes (4.x/6.x). For now a
        // non-empty desired-set is observed but drives no behavior.
        let _ = &*desired;
        let _outbound = &bridge.actions;
    }

    fn update_projectiles(&mut self) {
        use crate::combat::projectiles::ProjectileFlags;
        use rand::Rng;

        let physics_tables = match self.world.get_resource::<PhysicsTables>() {
            Some(pt) => pt.data.clone(),
            None => return,
        };

        // Clone geometry for collision checks
        let geometry = self.world.resource::<MapGeometry>();
        let polygon_adjacency = geometry.polygon_adjacency.clone();
        let line_endpoints = geometry.line_endpoints.clone();
        let line_solid = geometry.line_solid.clone();
        let line_transparent = geometry.line_transparent.clone();
        let floor_heights = geometry.floor_heights.clone();
        let ceiling_heights = geometry.ceiling_heights.clone();
        let polygon_media_index = geometry.polygon_media_index.clone();

        // Collect media data for submersion checks
        let media_data: std::collections::HashMap<usize, (f32, i16)> = {
            let mut map = std::collections::HashMap::new();
            let mut q = self.world.query::<&crate::Media>();
            for media in q.iter(&self.world) {
                map.insert(media.index, (media.current_height, media.media_type));
            }
            map
        };

        // Collect monster positions for entity collision
        #[allow(clippy::type_complexity)]
        let monster_data: Vec<(
            bevy_ecs::entity::Entity,
            glam::Vec2,
            f32,
            f32,
            f32,
            glam::Vec3,
            u32,
            u32,
        )> = {
            let mut q = self.world.query::<(
                bevy_ecs::entity::Entity,
                &crate::Monster,
                &crate::Position,
                &crate::CollisionRadius,
                &crate::EntityHeight,
                Option<&crate::Immunities>,
                Option<&crate::Weaknesses>,
            )>();
            q.iter(&self.world)
                .map(|(e, _m, pos, r, h, imm, weak)| {
                    (
                        e,
                        glam::Vec2::new(pos.0.x, pos.0.y),
                        r.0,
                        pos.0.z,
                        pos.0.z + h.0,
                        pos.0,
                        imm.map(|i| i.0).unwrap_or(0),
                        weak.map(|w| w.0).unwrap_or(0),
                    )
                })
                .collect()
        };

        // Get player data for collision and homing
        let player_data: Option<(
            bevy_ecs::entity::Entity,
            glam::Vec2,
            f32,
            f32,
            f32,
            glam::Vec3,
        )> = {
            let mut q = self.world.query_filtered::<(
                bevy_ecs::entity::Entity,
                &crate::Position,
                &crate::CollisionRadius,
                &crate::EntityHeight,
            ), bevy_ecs::prelude::With<crate::Player>>();
            q.iter(&self.world).next().map(|(e, pos, r, h)| {
                (
                    e,
                    glam::Vec2::new(pos.0.x, pos.0.y),
                    r.0,
                    pos.0.z,
                    pos.0.z + h.0,
                    pos.0,
                )
            })
        };

        // Collect projectile data (collect-then-process pattern)
        struct ProjData {
            entity: bevy_ecs::entity::Entity,
            proj: crate::Projectile,
            pos: glam::Vec3,
            vel: glam::Vec3,
            poly: usize,
            source: Option<bevy_ecs::entity::Entity>,
            homing_target: Option<glam::Vec3>,
        }

        let proj_data: Vec<ProjData> = {
            let mut query = self.world.query::<(
                bevy_ecs::entity::Entity,
                &crate::Projectile,
                &crate::Position,
                &crate::Velocity,
                &crate::PolygonIndex,
                Option<&crate::ProjectileSource>,
                Option<&crate::HomingTarget>,
            )>();
            query
                .iter(&self.world)
                .map(|(entity, proj, pos, vel, poly, source, homing)| ProjData {
                    entity,
                    proj: *proj,
                    pos: pos.0,
                    vel: vel.0,
                    poly: poly.0,
                    source: source.map(|s| s.0),
                    homing_target: homing.map(|h| h.0),
                })
                .collect()
        };

        // Get SimRng for randomness
        let mut rng_vals: Vec<(f32, f32, bool)> = Vec::new(); // pre-roll random values
        {
            let mut sim_rng = self.world.resource_mut::<SimRng>();
            for _ in 0..proj_data.len() {
                let h_wander: f32 = sim_rng.0.gen_range(-0.02..0.02);
                let v_wander: f32 = sim_rng.0.gen_range(-0.02..0.02);
                let pass_transparent: bool = sim_rng.0.gen_bool(0.5);
                rng_vals.push((h_wander, v_wander, pass_transparent));
            }
        }

        // Process each projectile
        #[derive(Debug)]
        enum ProjAction {
            /// Continue flying — update position, velocity, and projectile fields.
            Continue {
                entity: bevy_ecs::entity::Entity,
                new_pos: glam::Vec3,
                new_vel: glam::Vec3,
                new_distance: f32,
                new_ticks_alive: u16,
                new_contrails_spawned: u16,
                new_polygon: usize,
                contrail_effect: Option<(glam::Vec3, usize)>,
            },
            /// Detonate — despawn projectile, apply damage, spawn effects.
            Detonate {
                entity: bevy_ecs::entity::Entity,
                hit_point: glam::Vec3,
                hit_entity: Option<bevy_ecs::entity::Entity>,
                damage_amount: i16,
                /// Base damage for AoE calculation (from projectile definition).
                aoe_damage_base: i16,
                aoe_radius: f32,
                effect_def: Option<usize>,
                is_submerged: bool,
                media_effect_def: Option<usize>,
                #[allow(dead_code)]
                rebound_sound: i16,
            },
            /// Despawn without effect (range exceeded).
            DespawnSilent { entity: bevy_ecs::entity::Entity },
            /// Rebound from wall — update position and velocity.
            ReboundWall {
                entity: bevy_ecs::entity::Entity,
                hit_point: glam::Vec3,
                new_vel: glam::Vec3,
                new_distance: f32,
                new_ticks_alive: u16,
                new_polygon: usize,
                rebound_sound: i16,
            },
            /// Rebound from floor — update position and velocity.
            ReboundFloor {
                entity: bevy_ecs::entity::Entity,
                new_pos: glam::Vec3,
                new_vel: glam::Vec3,
                new_distance: f32,
                new_ticks_alive: u16,
                new_polygon: usize,
                rebound_sound: i16,
            },
            /// Persistent projectile hit — damage but keep going.
            PersistentHit {
                entity: bevy_ecs::entity::Entity,
                hit_entity: bevy_ecs::entity::Entity,
                damage_amount: i16,
                new_pos: glam::Vec3,
                new_vel: glam::Vec3,
                new_distance: f32,
                new_ticks_alive: u16,
                new_polygon: usize,
            },
            /// Promote to different projectile type (media interaction).
            Promote {
                entity: bevy_ecs::entity::Entity,
                new_def_index: usize,
                position: glam::Vec3,
                velocity: glam::Vec3,
                polygon: usize,
                source: Option<bevy_ecs::entity::Entity>,
            },
        }

        let mut actions: Vec<ProjAction> = Vec::new();

        for (idx, pd) in proj_data.iter().enumerate() {
            let def = physics_tables
                .projectiles
                .as_ref()
                .and_then(|p| p.get(pd.proj.definition_index));

            let Some(def) = def else {
                actions.push(ProjAction::DespawnSilent { entity: pd.entity });
                continue;
            };

            let flags = def.flags;
            let max_range = def.maximum_range as f32 / 1024.0;
            let rebound_sound = def.rebound_sound;
            let proj_damage_base = (def.damage.base as f32 * def.damage.scale) as i16;

            // 1. Apply gravity
            let mut current_vel = pd.vel;
            if flags & ProjectileFlags::AFFECTED_BY_GRAVITY != 0 {
                let gravity_mul = if flags & ProjectileFlags::DOUBLE_GRAVITY != 0 {
                    2.0
                } else if flags & ProjectileFlags::HALF_GRAVITY != 0 {
                    0.5
                } else {
                    1.0
                };
                let gravity = (1.0 / 120.0) * gravity_mul;
                current_vel =
                    crate::combat::projectiles::apply_projectile_gravity(current_vel, gravity);
            }

            // 2. Apply homing
            if flags & ProjectileFlags::GUIDED != 0 {
                if let Some(target) = pd.homing_target {
                    current_vel =
                        crate::combat::projectiles::apply_homing(current_vel, pd.pos, target, 0.05);
                } else {
                    // Fallback: home toward nearest valid target
                    let is_player_fired = pd.source.is_some();
                    let target_pos = if is_player_fired {
                        monster_data
                            .iter()
                            .min_by(|a, b| {
                                let da = a.1.distance(glam::Vec2::new(pd.pos.x, pd.pos.y));
                                let db = b.1.distance(glam::Vec2::new(pd.pos.x, pd.pos.y));
                                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                            })
                            .map(|(_, _, _, _, _, pos3, _, _)| *pos3)
                    } else {
                        player_data.map(|(_, _, _, _, _, pos3)| pos3)
                    };
                    if let Some(target) = target_pos {
                        current_vel = crate::combat::projectiles::apply_homing(
                            current_vel,
                            pd.pos,
                            target,
                            0.05,
                        );
                    }
                }
            }

            // 3. Apply wander
            let (h_wander, v_wander, pass_transparent) = rng_vals[idx];
            if flags & ProjectileFlags::HORIZONTAL_WANDER != 0 {
                let speed = current_vel.length();
                if speed > 1e-6 {
                    let angle = current_vel.y.atan2(current_vel.x) + h_wander;
                    current_vel.x = angle.cos() * speed;
                    current_vel.y = angle.sin() * speed;
                }
            }
            if flags & ProjectileFlags::VERTICAL_WANDER != 0 {
                current_vel.z += v_wander * current_vel.length() * 0.1;
            }

            // 4. Advance position
            let (new_pos, tick_dist) =
                crate::combat::projectiles::advance_projectile(pd.pos, current_vel);
            let new_distance = pd.proj.distance_traveled + tick_dist;
            let new_ticks = pd.proj.ticks_alive.saturating_add(1);
            let mut current_polygon = pd.poly;

            // 5. Check range limit (despawn silently)
            if crate::combat::projectiles::check_range_limit(new_distance, max_range) {
                actions.push(ProjAction::DespawnSilent { entity: pd.entity });
                continue;
            }

            // 6. Check animation-based detonation
            if flags & ProjectileFlags::STOP_WHEN_ANIMATION_LOOPS != 0 && new_ticks >= 15 {
                let is_sub = is_submerged_at(
                    &polygon_media_index,
                    &media_data,
                    current_polygon,
                    new_pos.z,
                );
                actions.push(ProjAction::Detonate {
                    entity: pd.entity,
                    hit_point: new_pos,
                    hit_entity: None,
                    damage_amount: 0,
                    aoe_damage_base: proj_damage_base,
                    aoe_radius: def.area_of_effect as f32 / 1024.0,
                    effect_def: if def.detonation_effect >= 0 {
                        Some(def.detonation_effect as usize)
                    } else {
                        None
                    },
                    is_submerged: is_sub,
                    media_effect_def: if def.media_detonation_effect >= 0 {
                        Some(def.media_detonation_effect as usize)
                    } else {
                        None
                    },
                    rebound_sound,
                });
                continue;
            }

            // 7. Check wall collision
            let mut wall_handled = false;
            if current_polygon < polygon_adjacency.len() {
                let old_2d = glam::Vec2::new(pd.pos.x, pd.pos.y);
                let new_2d = glam::Vec2::new(new_pos.x, new_pos.y);

                // Check all polygon edges for crossing
                for &(line_idx, adj) in &polygon_adjacency[current_polygon] {
                    let (la, lb) = line_endpoints[line_idx];
                    if let Some(hit) =
                        crate::collision::segment_intersection(old_2d, new_2d, la, lb)
                    {
                        let is_passable = adj.is_some() && !line_solid[line_idx];
                        let is_transparent =
                            line_transparent.get(line_idx).copied().unwrap_or(false);

                        if is_passable {
                            // Cross into adjacent polygon
                            if let Some(adj_poly) = adj {
                                current_polygon = adj_poly;
                            }
                        } else if is_transparent {
                            // Transparent wall: check pass-through flags
                            let passes = if flags & ProjectileFlags::USUALLY_PASS_TRANSPARENT_SIDE
                                != 0
                            {
                                true
                            } else if flags & ProjectileFlags::SOMETIMES_PASS_TRANSPARENT_SIDE != 0
                            {
                                pass_transparent
                            } else {
                                false
                            };
                            if passes {
                                if let Some(adj_poly) = adj {
                                    current_polygon = adj_poly;
                                }
                            } else {
                                // Detonate at transparent wall
                                let hit_z = pd.pos.z + (new_pos.z - pd.pos.z) * hit.t;
                                let hit_point = glam::Vec3::new(hit.point.x, hit.point.y, hit_z);
                                let is_sub = is_submerged_at(
                                    &polygon_media_index,
                                    &media_data,
                                    current_polygon,
                                    hit_point.z,
                                );
                                actions.push(ProjAction::Detonate {
                                    entity: pd.entity,
                                    hit_point,
                                    hit_entity: None,
                                    damage_amount: 0,
                                    aoe_damage_base: proj_damage_base,
                                    aoe_radius: def.area_of_effect as f32 / 1024.0,
                                    effect_def: if def.detonation_effect >= 0 {
                                        Some(def.detonation_effect as usize)
                                    } else {
                                        None
                                    },
                                    is_submerged: is_sub,
                                    media_effect_def: if def.media_detonation_effect >= 0 {
                                        Some(def.media_detonation_effect as usize)
                                    } else {
                                        None
                                    },
                                    rebound_sound,
                                });
                                wall_handled = true;
                                break;
                            }
                        } else {
                            // Solid wall hit
                            let hit_z = pd.pos.z + (new_pos.z - pd.pos.z) * hit.t;
                            let hit_point = glam::Vec3::new(hit.point.x, hit.point.y, hit_z);

                            if flags & ProjectileFlags::REBOUNDS_FROM_WALLS != 0 {
                                let reflected = crate::combat::projectiles::reflect_velocity_wall(
                                    current_vel,
                                    la,
                                    lb,
                                );
                                actions.push(ProjAction::ReboundWall {
                                    entity: pd.entity,
                                    hit_point,
                                    new_vel: reflected,
                                    new_distance,
                                    new_ticks_alive: new_ticks,
                                    new_polygon: current_polygon,
                                    rebound_sound,
                                });
                            } else {
                                let is_sub = is_submerged_at(
                                    &polygon_media_index,
                                    &media_data,
                                    current_polygon,
                                    hit_point.z,
                                );
                                actions.push(ProjAction::Detonate {
                                    entity: pd.entity,
                                    hit_point,
                                    hit_entity: None,
                                    damage_amount: 0,
                                    aoe_damage_base: proj_damage_base,
                                    aoe_radius: def.area_of_effect as f32 / 1024.0,
                                    effect_def: if def.detonation_effect >= 0 {
                                        Some(def.detonation_effect as usize)
                                    } else {
                                        None
                                    },
                                    is_submerged: is_sub,
                                    media_effect_def: if def.media_detonation_effect >= 0 {
                                        Some(def.media_detonation_effect as usize)
                                    } else {
                                        None
                                    },
                                    rebound_sound,
                                });
                            }
                            wall_handled = true;
                            break;
                        }
                    }
                }
            }
            if wall_handled {
                continue;
            }

            // 8. Check floor/ceiling collision
            let floor_h = floor_heights.get(current_polygon).copied().unwrap_or(0.0);
            let ceil_h = ceiling_heights
                .get(current_polygon)
                .copied()
                .unwrap_or(100.0);

            if new_pos.z <= floor_h {
                if flags & ProjectileFlags::REBOUNDS_FROM_FLOOR != 0 {
                    let reflected =
                        crate::combat::projectiles::reflect_velocity_floor(current_vel, 0.2);
                    let clamped_pos = glam::Vec3::new(new_pos.x, new_pos.y, floor_h + 0.01);
                    actions.push(ProjAction::ReboundFloor {
                        entity: pd.entity,
                        new_pos: clamped_pos,
                        new_vel: reflected,
                        new_distance,
                        new_ticks_alive: new_ticks,
                        new_polygon: current_polygon,
                        rebound_sound,
                    });
                    continue;
                } else {
                    let hit_point = glam::Vec3::new(new_pos.x, new_pos.y, floor_h);
                    let is_sub = is_submerged_at(
                        &polygon_media_index,
                        &media_data,
                        current_polygon,
                        hit_point.z,
                    );
                    actions.push(ProjAction::Detonate {
                        entity: pd.entity,
                        hit_point,
                        hit_entity: None,
                        damage_amount: 0,
                        aoe_damage_base: proj_damage_base,
                        aoe_radius: def.area_of_effect as f32 / 1024.0,
                        effect_def: if def.detonation_effect >= 0 {
                            Some(def.detonation_effect as usize)
                        } else {
                            None
                        },
                        is_submerged: is_sub,
                        media_effect_def: if def.media_detonation_effect >= 0 {
                            Some(def.media_detonation_effect as usize)
                        } else {
                            None
                        },
                        rebound_sound,
                    });
                    continue;
                }
            }

            if new_pos.z >= ceil_h {
                let hit_point = glam::Vec3::new(new_pos.x, new_pos.y, ceil_h);
                let is_sub = is_submerged_at(
                    &polygon_media_index,
                    &media_data,
                    current_polygon,
                    hit_point.z,
                );
                actions.push(ProjAction::Detonate {
                    entity: pd.entity,
                    hit_point,
                    hit_entity: None,
                    damage_amount: 0,
                    aoe_damage_base: proj_damage_base,
                    aoe_radius: def.area_of_effect as f32 / 1024.0,
                    effect_def: if def.detonation_effect >= 0 {
                        Some(def.detonation_effect as usize)
                    } else {
                        None
                    },
                    is_submerged: is_sub,
                    media_effect_def: if def.media_detonation_effect >= 0 {
                        Some(def.media_detonation_effect as usize)
                    } else {
                        None
                    },
                    rebound_sound,
                });
                continue;
            }

            // 9. Check entity collision
            let is_player_fired = pd.source.is_some();
            let mut targets: Vec<(glam::Vec2, f32, f32, f32)> = Vec::new();
            let mut target_entities: Vec<bevy_ecs::entity::Entity> = Vec::new();

            if is_player_fired {
                for (e, center, radius, z_bot, z_top, _, _, _) in &monster_data {
                    targets.push((*center, *radius, *z_bot, *z_top));
                    target_entities.push(*e);
                }
            } else {
                if let Some((e, center, radius, z_bot, z_top, _)) = &player_data {
                    targets.push((*center, *radius, *z_bot, *z_top));
                    target_entities.push(*e);
                }
            }

            if let Some(hit) = crate::combat::projectiles::check_projectile_entity_collision(
                pd.pos, new_pos, &targets,
            ) {
                let hit_entity = target_entities[hit.entity_index];
                let damage_base = def.damage.base;
                let damage_scale = def.damage.scale;
                let damage_amount = (damage_base as f32 * damage_scale) as i16;

                let is_persistent = flags & ProjectileFlags::PERSISTENT != 0
                    || flags & ProjectileFlags::PERSISTENT_AND_VIRULENT != 0;

                if is_persistent {
                    actions.push(ProjAction::PersistentHit {
                        entity: pd.entity,
                        hit_entity,
                        damage_amount,
                        new_pos,
                        new_vel: current_vel,
                        new_distance,
                        new_ticks_alive: new_ticks,
                        new_polygon: current_polygon,
                    });
                    continue;
                } else {
                    let is_sub = is_submerged_at(
                        &polygon_media_index,
                        &media_data,
                        current_polygon,
                        hit.hit_point.z,
                    );
                    actions.push(ProjAction::Detonate {
                        entity: pd.entity,
                        hit_point: hit.hit_point,
                        hit_entity: Some(hit_entity),
                        damage_amount,
                        aoe_damage_base: proj_damage_base,
                        aoe_radius: def.area_of_effect as f32 / 1024.0,
                        effect_def: if def.detonation_effect >= 0 {
                            Some(def.detonation_effect as usize)
                        } else {
                            None
                        },
                        is_submerged: is_sub,
                        media_effect_def: if def.media_detonation_effect >= 0 {
                            Some(def.media_detonation_effect as usize)
                        } else {
                            None
                        },
                        rebound_sound,
                    });
                    continue;
                }
            }

            // 10. Check media boundary interaction
            let media_idx = polygon_media_index
                .get(current_polygon)
                .copied()
                .unwrap_or(-1);
            if media_idx >= 0 {
                if let Some(&(media_height, _media_type)) = media_data.get(&(media_idx as usize)) {
                    // Check if projectile crossed the media surface
                    let was_above = pd.pos.z >= media_height;
                    let now_below = new_pos.z < media_height;
                    let was_below = pd.pos.z < media_height;
                    let now_above = new_pos.z >= media_height;

                    if (was_above && now_below) || (was_below && now_above) {
                        if def.media_projectile_promotion >= 0 {
                            actions.push(ProjAction::Promote {
                                entity: pd.entity,
                                new_def_index: def.media_projectile_promotion as usize,
                                position: new_pos,
                                velocity: current_vel,
                                polygon: current_polygon,
                                source: pd.source,
                            });
                            continue;
                        } else if flags & ProjectileFlags::PASSES_MEDIA_BOUNDARY == 0 {
                            let hit_point = glam::Vec3::new(new_pos.x, new_pos.y, media_height);
                            actions.push(ProjAction::Detonate {
                                entity: pd.entity,
                                hit_point,
                                hit_entity: None,
                                damage_amount: 0,
                                aoe_damage_base: proj_damage_base,
                                aoe_radius: def.area_of_effect as f32 / 1024.0,
                                effect_def: if def.media_detonation_effect >= 0 {
                                    Some(def.media_detonation_effect as usize)
                                } else {
                                    None
                                },
                                is_submerged: true,
                                media_effect_def: if def.media_detonation_effect >= 0 {
                                    Some(def.media_detonation_effect as usize)
                                } else {
                                    None
                                },
                                rebound_sound,
                            });
                            continue;
                        }
                        // PASSES_MEDIA_BOUNDARY: fall through to continue
                    }
                }
            }

            // 11. Spawn contrails
            let mut new_contrails = pd.proj.contrails_spawned;
            let contrail_effect = if def.contrail_effect >= 0
                && def.ticks_between_contrails > 0
                && new_contrails < def.maximum_contrails as u16
                && new_ticks > 0
                && new_ticks % (def.ticks_between_contrails as u16) == 0
            {
                new_contrails += 1;
                Some((new_pos, def.contrail_effect as usize))
            } else {
                None
            };

            // 12. Continue flying
            actions.push(ProjAction::Continue {
                entity: pd.entity,
                new_pos,
                new_vel: current_vel,
                new_distance,
                new_ticks_alive: new_ticks,
                new_contrails_spawned: new_contrails,
                new_polygon: current_polygon,
                contrail_effect,
            });
        }

        // Apply all actions
        let mut to_despawn: Vec<bevy_ecs::entity::Entity> = Vec::new();
        let mut effects_to_spawn: Vec<(glam::Vec3, usize)> = Vec::new();
        let mut sound_events: Vec<(i16, glam::Vec3)> = Vec::new();
        let mut promotions: Vec<(
            bevy_ecs::entity::Entity,
            usize,
            glam::Vec3,
            glam::Vec3,
            usize,
            Option<bevy_ecs::entity::Entity>,
        )> = Vec::new();

        for action in &actions {
            match action {
                ProjAction::Continue {
                    entity,
                    new_pos,
                    new_vel,
                    new_distance,
                    new_ticks_alive,
                    new_contrails_spawned,
                    new_polygon,
                    contrail_effect,
                } => {
                    if let Some(mut pos) = self.world.get_mut::<crate::Position>(*entity) {
                        pos.0 = *new_pos;
                    }
                    if let Some(mut vel) = self.world.get_mut::<crate::Velocity>(*entity) {
                        vel.0 = *new_vel;
                    }
                    if let Some(mut proj) = self.world.get_mut::<crate::Projectile>(*entity) {
                        proj.distance_traveled = *new_distance;
                        proj.ticks_alive = *new_ticks_alive;
                        proj.contrails_spawned = *new_contrails_spawned;
                        proj.current_polygon = *new_polygon;
                    }
                    if let Some(mut poly) = self.world.get_mut::<crate::PolygonIndex>(*entity) {
                        poly.0 = *new_polygon;
                    }
                    if let Some((pos, eff_idx)) = contrail_effect {
                        effects_to_spawn.push((*pos, *eff_idx));
                    }
                }
                ProjAction::Detonate {
                    entity,
                    hit_point,
                    hit_entity,
                    damage_amount,
                    aoe_damage_base,
                    aoe_radius,
                    effect_def,
                    is_submerged,
                    media_effect_def,
                    ..
                } => {
                    to_despawn.push(*entity);

                    // Direct hit damage
                    if let Some(hit_ent) = hit_entity {
                        if *damage_amount > 0 {
                            self.apply_damage_to_entity(*hit_ent, *damage_amount);
                            self.world.resource_mut::<crate::world::SimEvents>().push(
                                crate::world::SimEvent::EntityDamaged {
                                    entity: *hit_ent,
                                    amount: *damage_amount,
                                    damage_type: 0,
                                },
                            );
                        }
                    }

                    // AoE damage (uses projectile definition base damage, not direct hit amount)
                    if *aoe_radius > 0.0 {
                        let det_2d = glam::Vec2::new(hit_point.x, hit_point.y);
                        for (ent, center, _, _, _, _, _, _) in &monster_data {
                            let dist = det_2d.distance(*center);
                            let aoe_dmg = crate::combat::damage::calculate_aoe_damage(
                                *aoe_damage_base,
                                dist,
                                *aoe_radius,
                            );
                            if aoe_dmg > 0 {
                                self.apply_damage_to_entity(*ent, aoe_dmg);
                            }
                        }
                        if let Some((pe, pcenter, _, _, _, _)) = &player_data {
                            let dist = det_2d.distance(*pcenter);
                            let aoe_dmg = crate::combat::damage::calculate_aoe_damage(
                                *aoe_damage_base,
                                dist,
                                *aoe_radius,
                            );
                            if aoe_dmg > 0 {
                                self.apply_damage_to_entity(*pe, aoe_dmg);
                            }
                        }
                    }

                    // Detonation effect
                    let eff = if *is_submerged {
                        media_effect_def.or(*effect_def)
                    } else {
                        *effect_def
                    };
                    if let Some(eff_idx) = eff {
                        effects_to_spawn.push((*hit_point, eff_idx));
                    }
                }
                ProjAction::DespawnSilent { entity } => {
                    to_despawn.push(*entity);
                }
                ProjAction::ReboundWall {
                    entity,
                    hit_point,
                    new_vel,
                    new_distance,
                    new_ticks_alive,
                    new_polygon,
                    rebound_sound,
                }
                | ProjAction::ReboundFloor {
                    entity,
                    new_pos: hit_point,
                    new_vel,
                    new_distance,
                    new_ticks_alive,
                    new_polygon,
                    rebound_sound,
                } => {
                    if let Some(mut pos) = self.world.get_mut::<crate::Position>(*entity) {
                        pos.0 = *hit_point;
                    }
                    if let Some(mut vel) = self.world.get_mut::<crate::Velocity>(*entity) {
                        vel.0 = *new_vel;
                    }
                    if let Some(mut proj) = self.world.get_mut::<crate::Projectile>(*entity) {
                        proj.distance_traveled = *new_distance;
                        proj.ticks_alive = *new_ticks_alive;
                        proj.current_polygon = *new_polygon;
                    }
                    if *rebound_sound >= 0 {
                        sound_events.push((*rebound_sound, *hit_point));
                    }
                }
                ProjAction::PersistentHit {
                    entity,
                    hit_entity,
                    damage_amount,
                    new_pos,
                    new_vel,
                    new_distance,
                    new_ticks_alive,
                    new_polygon,
                } => {
                    // Apply damage but keep projectile alive
                    if *damage_amount > 0 {
                        self.apply_damage_to_entity(*hit_entity, *damage_amount);
                        self.world.resource_mut::<crate::world::SimEvents>().push(
                            crate::world::SimEvent::EntityDamaged {
                                entity: *hit_entity,
                                amount: *damage_amount,
                                damage_type: 0,
                            },
                        );
                    }
                    if let Some(mut pos) = self.world.get_mut::<crate::Position>(*entity) {
                        pos.0 = *new_pos;
                    }
                    if let Some(mut vel) = self.world.get_mut::<crate::Velocity>(*entity) {
                        vel.0 = *new_vel;
                    }
                    if let Some(mut proj) = self.world.get_mut::<crate::Projectile>(*entity) {
                        proj.distance_traveled = *new_distance;
                        proj.ticks_alive = *new_ticks_alive;
                        proj.current_polygon = *new_polygon;
                    }
                }
                ProjAction::Promote {
                    entity,
                    new_def_index,
                    position,
                    velocity,
                    polygon,
                    source,
                } => {
                    to_despawn.push(*entity);
                    promotions.push((
                        *entity,
                        *new_def_index,
                        *position,
                        *velocity,
                        *polygon,
                        *source,
                    ));
                }
            }
        }

        // Despawn projectiles
        for entity in to_despawn {
            self.world.despawn(entity);
        }

        // Spawn promoted projectiles
        for (_old_entity, def_index, position, velocity, polygon, source) in promotions {
            let new_speed = physics_tables
                .projectiles
                .as_ref()
                .and_then(|p| p.get(def_index))
                .map(|d| d.speed as f32 / 1024.0)
                .unwrap_or(velocity.length());
            let dir = velocity.normalize_or_zero();
            let mut spawned = self.world.spawn((
                crate::Projectile {
                    definition_index: def_index,
                    distance_traveled: 0.0,
                    contrails_spawned: 0,
                    ticks_alive: 0,
                    current_polygon: polygon,
                },
                crate::Position(position),
                crate::Velocity(dir * new_speed),
                crate::PolygonIndex(polygon),
            ));
            if let Some(src) = source {
                spawned.insert(crate::ProjectileSource(src));
            }
        }

        // Spawn effects
        for (pos, effect_idx) in effects_to_spawn {
            let ticks = physics_tables
                .effects
                .as_ref()
                .and_then(|e| e.get(effect_idx))
                .map(|e| (e.delay.max(1) as u16).max(3))
                .unwrap_or(10);
            self.world.spawn((
                crate::Effect {
                    definition_index: effect_idx,
                    ticks_remaining: ticks,
                },
                crate::Position(pos),
            ));
        }

        // Emit sound events
        for (sound_idx, pos) in sound_events {
            self.world.resource_mut::<crate::world::SimEvents>().push(
                crate::world::SimEvent::SoundTrigger {
                    sound_index: sound_idx as usize,
                    position: pos,
                },
            );
        }
    }

    fn update_effects(&mut self) {
        let mut to_despawn: Vec<bevy_ecs::entity::Entity> = Vec::new();

        {
            let mut query = self
                .world
                .query::<(bevy_ecs::entity::Entity, &mut crate::Effect)>();
            for (entity, mut effect) in query.iter_mut(&mut self.world) {
                if effect.ticks_remaining > 0 {
                    effect.ticks_remaining -= 1;
                }
                if effect.ticks_remaining == 0 {
                    to_despawn.push(entity);
                }
            }
        }

        for entity in to_despawn {
            self.world.despawn(entity);
        }
    }

    fn update_items(&mut self) {
        // Get player data
        let player_data: Option<(bevy_ecs::entity::Entity, glam::Vec2, f32)> = {
            let mut q = self.world.query_filtered::<(
                bevy_ecs::entity::Entity,
                &crate::Position,
                &crate::CollisionRadius,
            ), bevy_ecs::prelude::With<crate::Player>>();
            q.iter(&self.world)
                .next()
                .map(|(e, pos, r)| (e, glam::Vec2::new(pos.0.x, pos.0.y), r.0))
        };

        let Some((player_entity, player_pos, player_radius)) = player_data else {
            return;
        };

        // Get player vitals for pickup checks
        let player_health = self
            .world
            .get::<crate::Health>(player_entity)
            .map(|h| h.0)
            .unwrap_or(0);
        let player_shield = self
            .world
            .get::<crate::Shield>(player_entity)
            .map(|s| s.0)
            .unwrap_or(0);
        let player_oxygen = self
            .world
            .get::<crate::Oxygen>(player_entity)
            .map(|o| o.0)
            .unwrap_or(0);

        // Check each item for overlap
        struct ItemPickup {
            entity: bevy_ecs::entity::Entity,
            effect: crate::world_mechanics::items::ItemEffect,
        }

        let mut pickups: Vec<ItemPickup> = Vec::new();

        {
            let mut query = self.world.query::<(
                bevy_ecs::entity::Entity,
                &crate::Item,
                &crate::Position,
                &crate::CollisionRadius,
            )>();
            for (entity, item, pos, radius) in query.iter(&self.world) {
                let item_pos = glam::Vec2::new(pos.0.x, pos.0.y);
                let dist = player_pos.distance(item_pos);

                if dist <= player_radius + radius.0 {
                    if let Some(effect) = crate::world_mechanics::items::item_effect(item.item_type)
                    {
                        // Check if pickup can be applied
                        let can_pickup = match &effect {
                            crate::world_mechanics::items::ItemEffect::RestoreHealth { .. } => {
                                player_health < 150
                            }
                            crate::world_mechanics::items::ItemEffect::RestoreShield { .. } => {
                                player_shield < 150
                            }
                            crate::world_mechanics::items::ItemEffect::RestoreOxygen { .. } => {
                                player_oxygen < 600
                            }
                            _ => true, // weapons, ammo, inventory always pickupable
                        };

                        if can_pickup {
                            pickups.push(ItemPickup { entity, effect });
                        }
                    }
                }
            }
        }

        // Apply pickups
        for pickup in &pickups {
            use crate::world_mechanics::items::ItemEffect;
            match &pickup.effect {
                ItemEffect::RestoreHealth { amount } => {
                    if let Some(mut health) = self.world.get_mut::<crate::Health>(player_entity) {
                        health.0 = (health.0 + amount).min(150);
                    }
                }
                ItemEffect::RestoreShield { amount } => {
                    if let Some(mut shield) = self.world.get_mut::<crate::Shield>(player_entity) {
                        shield.0 = (shield.0 + amount).min(150);
                    }
                }
                ItemEffect::RestoreOxygen { amount } => {
                    if let Some(mut oxygen) = self.world.get_mut::<crate::Oxygen>(player_entity) {
                        oxygen.0 = (oxygen.0 + amount).min(600);
                    }
                }
                ItemEffect::AddWeapon {
                    weapon_definition_index,
                } => {
                    if let Some(mut inv) = self
                        .world
                        .get_resource_mut::<crate::player::inventory::WeaponInventory>()
                    {
                        let idx = *weapon_definition_index;
                        if idx < inv.weapons.len() && inv.weapons[idx].is_none() {
                            inv.weapons[idx] = Some(crate::player::inventory::WeaponSlot {
                                definition_index: idx,
                                primary_magazine: 8,
                                primary_reserve: 0,
                                secondary_magazine: 0,
                                secondary_reserve: 0,
                                state: crate::player::inventory::WeaponState::Idle,
                                cooldown_ticks: 0,
                            });
                        }
                    }
                }
                ItemEffect::AddAmmo {
                    weapon_definition_index,
                    is_primary,
                    amount,
                } => {
                    if let Some(mut inv) = self
                        .world
                        .get_resource_mut::<crate::player::inventory::WeaponInventory>()
                    {
                        let idx = *weapon_definition_index;
                        if idx < inv.weapons.len() {
                            if let Some(ref mut weapon) = inv.weapons[idx] {
                                if *is_primary {
                                    weapon.primary_reserve += amount;
                                } else {
                                    weapon.secondary_reserve += amount;
                                }
                            }
                        }
                    }
                }
                ItemEffect::AddInventoryItem { .. } => {
                    // Inventory items tracked separately (keys, balls) - stub for now
                }
                ItemEffect::ActivatePowerup { .. } => {
                    // Powerup timer activation wired in a later task (PowerupTimers) - stub for now
                }
            }
        }

        // Despawn picked-up items
        for pickup in &pickups {
            self.world.despawn(pickup.entity);
        }
    }

    /// Query the player's position.
    pub fn player_position(&mut self) -> Option<glam::Vec3> {
        let mut query = self
            .world
            .query_filtered::<&crate::Position, bevy_ecs::prelude::With<crate::Player>>();
        query.iter(&self.world).next().map(|p| p.0)
    }

    /// Query the player's facing angle.
    pub fn player_facing(&mut self) -> Option<f32> {
        let mut query = self
            .world
            .query_filtered::<&crate::Facing, bevy_ecs::prelude::With<crate::Player>>();
        query.iter(&self.world).next().map(|f| f.0)
    }

    /// Query the player's health.
    pub fn player_health(&mut self) -> Option<i16> {
        let mut query = self
            .world
            .query_filtered::<&crate::Health, bevy_ecs::prelude::With<crate::Player>>();
        query.iter(&self.world).next().map(|h| h.0)
    }

    /// Query the player's shield.
    pub fn player_shield(&mut self) -> Option<i16> {
        let mut query = self
            .world
            .query_filtered::<&crate::Shield, bevy_ecs::prelude::With<crate::Player>>();
        query.iter(&self.world).next().map(|s| s.0)
    }

    /// Query the player's oxygen.
    pub fn player_oxygen(&mut self) -> Option<i16> {
        let mut query = self
            .world
            .query_filtered::<&crate::Oxygen, bevy_ecs::prelude::With<crate::Player>>();
        query.iter(&self.world).next().map(|o| o.0)
    }

    /// Query the player's vertical look angle.
    pub fn player_vertical_look(&mut self) -> Option<f32> {
        let mut query = self
            .world
            .query_filtered::<&crate::VerticalLook, bevy_ecs::prelude::With<crate::Player>>();
        query.iter(&self.world).next().map(|v| v.0)
    }

    /// Query the player's current polygon index.
    pub fn player_polygon(&mut self) -> Option<usize> {
        let mut query = self
            .world
            .query_filtered::<&crate::PolygonIndex, bevy_ecs::prelude::With<crate::Player>>();
        query.iter(&self.world).next().map(|p| p.0)
    }

    /// Query the player's current weapon rendering state.
    ///
    /// Returns the shape collection, high-level shape index (based on weapon
    /// operational state), and animation frame for the currently equipped weapon.
    pub fn player_weapon_state(&mut self) -> Option<WeaponRenderState> {
        let inv = self
            .world
            .get_resource::<crate::player::inventory::WeaponInventory>()?;
        let slot = inv.current()?;
        let def_idx = slot.definition_index;
        let weapon_state = slot.state;

        let tables = self.world.get_resource::<PhysicsTables>()?;
        let weapons = tables.data.weapons.as_ref()?;
        let def = weapons.get(def_idx)?;

        if def.collection < 0 {
            return None;
        }

        let shape = match weapon_state {
            crate::player::inventory::WeaponState::Idle => def.idle_shape,
            crate::player::inventory::WeaponState::Firing => {
                if def.firing_shape >= 0 {
                    def.firing_shape
                } else {
                    def.idle_shape
                }
            }
            crate::player::inventory::WeaponState::Recovering => def.idle_shape,
            crate::player::inventory::WeaponState::Reloading => {
                if def.reloading_shape >= 0 {
                    def.reloading_shape
                } else {
                    def.idle_shape
                }
            }
            crate::player::inventory::WeaponState::Switching => def.idle_shape,
        };

        if shape < 0 {
            return None;
        }

        Some(WeaponRenderState {
            collection: def.collection as u16,
            shape: shape as u16,
            frame: 0,
            vertical_position: def.idle_height,
            horizontal_position: def.idle_width,
        })
    }

    /// Query the player's current weapon info for HUD display.
    ///
    /// Returns (definition_index, primary_ammo, secondary_ammo).
    pub fn player_weapon_info(&mut self) -> Option<(usize, u16, u16)> {
        let inv = self
            .world
            .get_resource::<crate::player::inventory::WeaponInventory>()?;
        let slot = inv.current()?;
        Some((
            slot.definition_index,
            slot.primary_magazine,
            slot.secondary_magazine,
        ))
    }

    /// Query nearby entities for the motion sensor HUD.
    ///
    /// Returns up to 16 entities as (relative_x, relative_z, entity_type)
    /// where positions are relative to the player and entity_type is
    /// 0=enemy, 1=ally, 2=item.
    pub fn nearby_entities(&mut self, range: f32) -> Vec<(f32, f32, u8)> {
        let player_pos = match self.player_position() {
            Some(p) => p,
            None => return Vec::new(),
        };
        let range_sq = range * range;
        let mut results: Vec<(f32, f32, u8, f32)> = Vec::new();

        // Monsters
        {
            let mut query = self
                .world
                .query::<(&crate::Position, &crate::MonsterState)>();
            for (pos, state) in query.iter(&self.world) {
                if *state == crate::MonsterState::Dead {
                    continue;
                }
                let dx = pos.0.x - player_pos.x;
                let dy = pos.0.y - player_pos.y;
                let dist_sq = dx * dx + dy * dy;
                if dist_sq < range_sq {
                    results.push((dx, dy, 0u8, dist_sq));
                }
            }
        }

        // Items
        {
            let mut query = self
                .world
                .query_filtered::<&crate::Position, bevy_ecs::prelude::With<crate::Item>>();
            for pos in query.iter(&self.world) {
                let dx = pos.0.x - player_pos.x;
                let dy = pos.0.y - player_pos.y;
                let dist_sq = dx * dx + dy * dy;
                if dist_sq < range_sq {
                    results.push((dx, dy, 2u8, dist_sq));
                }
            }
        }

        results.sort_by(|a, b| a.3.partial_cmp(&b.3).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(16);
        results.into_iter().map(|(x, z, t, _)| (x, z, t)).collect()
    }

    /// Return all active entities with their rendering state.
    ///
    /// This includes monsters (not Dead), projectiles, items, and effects.
    /// Does not include the player entity (queried separately).
    pub fn entities(&mut self) -> Vec<EntityRenderState> {
        let mut result = Vec::new();

        // Monsters (exclude Dead)
        {
            let mut query = self.world.query::<(
                &crate::Monster,
                &crate::Position,
                &crate::Facing,
                &crate::MonsterState,
                &crate::SpriteShape,
                &crate::AnimationFrame,
            )>();
            for (monster, pos, facing, state, shape, frame) in query.iter(&self.world) {
                if *state == crate::MonsterState::Dead {
                    continue;
                }
                result.push(EntityRenderState {
                    entity_type: RenderEntityType::Monster {
                        definition_index: monster.definition_index,
                    },
                    position: pos.0,
                    facing: facing.0,
                    shape: shape.0,
                    frame: frame.0,
                });
            }
        }

        // Projectiles
        {
            let mut query = self.world.query::<(&crate::Projectile, &crate::Position)>();
            for (proj, pos) in query.iter(&self.world) {
                result.push(EntityRenderState {
                    entity_type: RenderEntityType::Projectile {
                        definition_index: proj.definition_index,
                    },
                    position: pos.0,
                    facing: 0.0,
                    shape: 0,
                    frame: 0,
                });
            }
        }

        // Items
        {
            let mut query = self.world.query::<(
                &crate::Item,
                &crate::Position,
                &crate::SpriteShape,
                &crate::AnimationFrame,
            )>();
            for (item, pos, shape, frame) in query.iter(&self.world) {
                result.push(EntityRenderState {
                    entity_type: RenderEntityType::Item {
                        item_type: item.item_type,
                    },
                    position: pos.0,
                    facing: 0.0,
                    shape: shape.0,
                    frame: frame.0,
                });
            }
        }

        // Effects
        {
            let mut query = self.world.query::<(&crate::Effect, &crate::Position)>();
            for (effect, pos) in query.iter(&self.world) {
                result.push(EntityRenderState {
                    entity_type: RenderEntityType::Effect {
                        definition_index: effect.definition_index,
                    },
                    position: pos.0,
                    facing: 0.0,
                    shape: 0,
                    frame: 0,
                });
            }
        }

        result
    }
}

/// Rendering data for an active entity.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EntityRenderState {
    pub entity_type: RenderEntityType,
    pub position: glam::Vec3,
    pub facing: f32,
    pub shape: u16,
    pub frame: u16,
}

/// Type of entity for rendering purposes.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum RenderEntityType {
    Monster { definition_index: usize },
    Projectile { definition_index: usize },
    Item { item_type: i16 },
    Effect { definition_index: usize },
}

/// Rendering data for the player's current weapon.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WeaponRenderState {
    /// Shape collection index (references a collection in the shapes file).
    pub collection: u16,
    /// High-level shape index within the collection (idle, firing, etc.).
    pub shape: u16,
    /// Animation frame within the shape sequence.
    pub frame: u16,
    /// Vertical position of the weapon sprite origin (normalized, from idle_height).
    pub vertical_position: f32,
    /// Horizontal position of the weapon sprite origin (normalized, from idle_width).
    pub horizontal_position: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Box 1.5: the per-tick `update_agents()` seam exists parallel to
    /// `update_monsters()`, is wired into `tick()` after the monster pass, reads
    /// the latest desired-set off the installed [`crate::fleet_bridge::SimBridge`],
    /// and — with an EMPTY desired-set (the seeded dead/absent-daemon case) —
    /// emits ZERO `GameAction`s and never panics. No agent behavior yet; this only
    /// proves the call seam is present and plumbed.
    #[test]
    fn update_agents_seam_empty_desired_set_emits_nothing() {
        let (sim_bridge, mut daemon_bridge) = crate::fleet_bridge::channel();
        let mut world = minimal_sim_world();
        world.set_fleet_bridge(sim_bridge);

        // Drive a full tick: `tick()` must invoke `update_agents()` after the
        // monster pass. With the seeded empty desired-set, the seam is a no-op.
        world.tick(TickInput::default());

        // The seam emitted no outbound actions for an empty desired-set.
        assert!(
            daemon_bridge.actions.try_recv().is_err(),
            "empty desired-set must drive zero agent GameActions through the seam"
        );

        // Calling the seam directly is also a no-op and does not panic.
        world.update_agents();
        assert!(
            daemon_bridge.actions.try_recv().is_err(),
            "direct update_agents() call on empty desired-set must emit nothing"
        );
    }

    #[test]
    fn action_flags_contains() {
        let flags = ActionFlags::new(ActionFlags::MOVE_FORWARD | ActionFlags::FIRE_PRIMARY);
        assert!(flags.contains(ActionFlags::MOVE_FORWARD));
        assert!(flags.contains(ActionFlags::FIRE_PRIMARY));
        assert!(!flags.contains(ActionFlags::STRAFE_LEFT));
    }

    /// Build a minimal single-square SimWorld (no lights/media in the map) so we
    /// can spawn `Light`/`Media` entities directly and exercise a single system.
    fn minimal_sim_world() -> SimWorld {
        use marathon_formats::map::LightData;
        use marathon_formats::physics::PhysicsData;
        use marathon_formats::{Endpoint, Line, MapData, Polygon, ShapeDescriptor, WorldPoint2d};

        let mk_endpoint = |x: i16, y: i16| Endpoint {
            flags: 0,
            highest_adjacent_floor_height: 0,
            lowest_adjacent_ceiling_height: 2048,
            vertex: WorldPoint2d { x, y },
            transformed: WorldPoint2d { x, y },
            supporting_polygon_index: 0,
        };
        let mk_line = |a: i16, b: i16| Line {
            endpoint_indexes: [a, b],
            flags: 0x4000, // SOLID
            length: 1024,
            highest_adjacent_floor: 0,
            lowest_adjacent_ceiling: 2048,
            clockwise_polygon_side_index: -1,
            counterclockwise_polygon_side_index: -1,
            clockwise_polygon_owner: -1,
            counterclockwise_polygon_owner: -1,
        };
        let wp_zero = WorldPoint2d { x: 0, y: 0 };
        let poly = Polygon {
            polygon_type: 0,
            flags: 0,
            permutation: 0,
            vertex_count: 4,
            endpoint_indexes: [0, 1, 2, 3, -1, -1, -1, -1],
            line_indexes: [0, 1, 2, 3, -1, -1, -1, -1],
            floor_texture: ShapeDescriptor(0xFFFF),
            ceiling_texture: ShapeDescriptor(0xFFFF),
            floor_height: 0,
            ceiling_height: 2048,
            floor_lightsource_index: 0,
            ceiling_lightsource_index: 0,
            area: 1024 * 1024,
            floor_transfer_mode: 0,
            ceiling_transfer_mode: 0,
            adjacent_polygon_indexes: [-1; 8],
            center: wp_zero,
            side_indexes: [-1; 8],
            floor_origin: wp_zero,
            ceiling_origin: wp_zero,
            media_index: -1,
            media_lightsource_index: -1,
            sound_source_indexes: -1,
            ambient_sound_image_index: -1,
            random_sound_image_index: -1,
        };
        let map = MapData {
            endpoints: vec![
                mk_endpoint(0, 0),
                mk_endpoint(1024, 0),
                mk_endpoint(1024, 1024),
                mk_endpoint(0, 1024),
            ],
            lines: vec![mk_line(0, 1), mk_line(1, 2), mk_line(2, 3), mk_line(3, 0)],
            sides: vec![],
            polygons: vec![poly],
            objects: vec![],
            lights: LightData::Static(vec![]),
            platforms: vec![],
            media: vec![],
            annotations: vec![],
            terminals: vec![],
            ambient_sounds: vec![],
            random_sounds: vec![],
            map_info: None,
            item_placement: vec![],
            guard_paths: None,
        };
        let physics = PhysicsData {
            monsters: None,
            effects: None,
            projectiles: None,
            physics: None,
            weapons: None,
        };
        SimWorld::new(&map, &physics, &crate::world::SimConfig::default())
            .expect("minimal world construction")
    }

    /// Build a two-polygon SimWorld: a player room (poly 0) and an adjacent
    /// door polygon (poly 1, `polygon_type == 5`) sharing one line. The loader's
    /// implicit-door pass spawns a player-controllable `Platform` for poly 1, so
    /// this is the real geometry a normal Space press must activate.
    ///
    /// Layout (map units, 1024 = 1 WU):
    ///   poly 0 occupies x∈[0,2048], y∈[0,1024]  (room, world x∈[0,2])
    ///   poly 1 occupies x∈[2048,4096], y∈[0,1024] (door, world x∈[2,4])
    ///   shared line at x = 2048 (world x = 2).
    fn door_sim_world() -> SimWorld {
        use marathon_formats::map::LightData;
        use marathon_formats::physics::PhysicsData;
        use marathon_formats::{Endpoint, Line, MapData, Polygon, ShapeDescriptor, WorldPoint2d};

        let mk_endpoint = |x: i16, y: i16| Endpoint {
            flags: 0,
            highest_adjacent_floor_height: 0,
            lowest_adjacent_ceiling_height: 2048,
            vertex: WorldPoint2d { x, y },
            transformed: WorldPoint2d { x, y },
            supporting_polygon_index: 0,
        };
        // Lines carry adjacency via the polygon's adjacent_polygon_indexes, but
        // mark the shared line non-solid so it reads like a doorway.
        let mk_line = |a: i16, b: i16, solid: bool| Line {
            endpoint_indexes: [a, b],
            flags: if solid { 0x4000 } else { 0 },
            length: 1024,
            highest_adjacent_floor: 0,
            lowest_adjacent_ceiling: 2048,
            clockwise_polygon_side_index: -1,
            counterclockwise_polygon_side_index: -1,
            clockwise_polygon_owner: -1,
            counterclockwise_polygon_owner: -1,
        };
        let wp_zero = WorldPoint2d { x: 0, y: 0 };
        let mk_poly =
            |poly_type: i16, endpoints: [i16; 8], lines: [i16; 8], adj: [i16; 8]| -> Polygon {
                Polygon {
                    polygon_type: poly_type,
                    flags: 0,
                    permutation: 0,
                    vertex_count: 4,
                    endpoint_indexes: endpoints,
                    line_indexes: lines,
                    floor_texture: ShapeDescriptor(0xFFFF),
                    ceiling_texture: ShapeDescriptor(0xFFFF),
                    floor_height: 0,
                    ceiling_height: 2048,
                    floor_lightsource_index: 0,
                    ceiling_lightsource_index: 0,
                    area: 2048 * 1024,
                    floor_transfer_mode: 0,
                    ceiling_transfer_mode: 0,
                    adjacent_polygon_indexes: adj,
                    center: wp_zero,
                    side_indexes: [-1; 8],
                    floor_origin: wp_zero,
                    ceiling_origin: wp_zero,
                    media_index: -1,
                    media_lightsource_index: -1,
                    sound_source_indexes: -1,
                    ambient_sound_image_index: -1,
                    random_sound_image_index: -1,
                }
            };

        // Endpoints: room corners 0..3, plus door's far corners 4,5.
        //   0:(0,0) 1:(2048,0) 2:(2048,1024) 3:(0,1024)   (room)
        //   4:(4096,0) 5:(4096,1024)                       (door far side)
        let endpoints = vec![
            mk_endpoint(0, 0),       // 0
            mk_endpoint(2048, 0),    // 1
            mk_endpoint(2048, 1024), // 2
            mk_endpoint(0, 1024),    // 3
            mk_endpoint(4096, 0),    // 4
            mk_endpoint(4096, 1024), // 5
        ];
        // Lines (edge order matches each polygon's CCW winding):
        //   poly 0 edges: 0(bottom 0→1), 1(shared 1→2), 2(top 2→3), 3(left 3→0)
        //   poly 1 edges: 4(bottom 1→4), 5(right 4→5), 6(top 5→2), 1(shared 2→1)
        let lines = vec![
            mk_line(0, 1, true),  // 0
            mk_line(1, 2, false), // 1 shared doorway
            mk_line(2, 3, true),  // 2
            mk_line(3, 0, true),  // 3
            mk_line(1, 4, true),  // 4
            mk_line(4, 5, true),  // 5
            mk_line(5, 2, true),  // 6
        ];

        // poly 0: room, adjacent to poly 1 across edge index 1 (line 1).
        let poly0 = mk_poly(
            0,
            [0, 1, 2, 3, -1, -1, -1, -1],
            [0, 1, 2, 3, -1, -1, -1, -1],
            [-1, 1, -1, -1, -1, -1, -1, -1],
        );
        // poly 1: door (type 5), adjacent to poly 0 across its edge index 3.
        let poly1 = mk_poly(
            5,
            [1, 4, 5, 2, -1, -1, -1, -1],
            [4, 5, 6, 1, -1, -1, -1, -1],
            [-1, -1, -1, 0, -1, -1, -1, -1],
        );

        let map = MapData {
            endpoints,
            lines,
            sides: vec![],
            polygons: vec![poly0, poly1],
            objects: vec![],
            lights: LightData::Static(vec![]),
            platforms: vec![],
            media: vec![],
            annotations: vec![],
            terminals: vec![],
            ambient_sounds: vec![],
            random_sounds: vec![],
            map_info: None,
            item_placement: vec![],
            guard_paths: None,
        };
        let physics = PhysicsData {
            monsters: None,
            effects: None,
            projectiles: None,
            physics: None,
            weapons: None,
        };
        SimWorld::new(&map, &physics, &crate::world::SimConfig::default())
            .expect("door world construction")
    }

    /// REGRESSION (operator bug "a player cannot open doors"): a normal Space
    /// press must open a reachable door through the REAL input path only — i.e.
    /// `tick(TickInput { action_flags: ACTION, .. })` → `process_action_key` →
    /// `find_action_key_target` → `activate_platform`. This deliberately does
    /// NOT call `debug_face_nearest_door()` or any `__marathonDebug` hook (that
    /// teleport hook is what masked the bug in the e2e suite). The player is
    /// placed by hand within range and facing the door, exactly as a WASD
    /// walk-up would leave them, and we assert the door platform actually leaves
    /// `AtRest`.
    #[test]
    fn space_press_opens_reachable_door_via_real_input() {
        use crate::components::{
            AngularVelocity, Facing, Grounded, Health, Oxygen, PlatformState, Position, Shield,
            VerticalLook,
        };
        use crate::{Platform, Player, PolygonIndex, Velocity};
        use glam::Vec3;

        let mut world = door_sim_world();

        // Sanity: the loader spawned an at-rest, player-controllable door for the
        // type-5 polygon (poly 1). If this regresses, the rest is moot.
        let door_poly = {
            let ecs = world.ecs_world_mut();
            let mut q = ecs.query::<&Platform>();
            let p = q
                .iter(ecs)
                .find(|p| p.polygon_index == 1)
                .expect("a door platform must exist for the type-5 polygon");
            assert_eq!(
                p.state,
                PlatformState::AtRest,
                "door must start at rest so a press has an effect"
            );
            assert!(
                crate::world_mechanics::platforms::should_activate(
                    p,
                    crate::world_mechanics::platforms::PlatformTrigger::ActionKey,
                ),
                "the implicit door must be activatable by the action key"
            );
            p.polygon_index
        };
        assert_eq!(door_poly, 1);

        // Place the player INSIDE the room (poly 0), ~0.75 WU back from the
        // shared door line at world x = 2.0, facing +X straight at the door.
        // This is a position a player walking east with W would reach; no hook.
        {
            let ecs = world.ecs_world_mut();
            ecs.spawn((
                Player,
                Position(Vec3::new(1.25, 0.5, 0.0)),
                Velocity::default(),
                Facing(0.0), // +X, toward the door line
                VerticalLook::default(),
                AngularVelocity::default(),
                PolygonIndex(0),
                Grounded(true),
                Oxygen(600),
                Health(150),
                Shield(150),
            ));
        }

        // One no-action tick disarms the ACTION edge (rising-edge detector), the
        // same priming a fresh frame does before the first press.
        world.tick(TickInput::default());

        // Door is still closed/at-rest before the press.
        {
            let ecs = world.ecs_world_mut();
            let mut q = ecs.query::<&Platform>();
            let p = q.iter(ecs).find(|p| p.polygon_index == 1).unwrap();
            assert_eq!(
                p.state,
                PlatformState::AtRest,
                "no press yet: the door must remain at rest"
            );
        }

        // THE REAL PRESS: ACTION flag set, exactly what `Space` produces via
        // `Input::to_action_flags()` in the web build.
        world.tick(TickInput::from(ActionFlags::new(ActionFlags::ACTION)));

        // The door must have been activated: it left AtRest (opening).
        let ecs = world.ecs_world_mut();
        let mut q = ecs.query::<&Platform>();
        let p = q.iter(ecs).find(|p| p.polygon_index == 1).unwrap();
        assert_ne!(
            p.state,
            PlatformState::AtRest,
            "a Space press while facing a reachable door must activate it \
             (door state stayed AtRest — the action key did not open the door)"
        );
    }

    /// Box 4.3: `run_media_updates()` must query every `Media`, look up its
    /// linked `Light` by `light_index`, and recompute `current_height` from the
    /// light's `current_intensity` via `compute_media_height()`.
    #[test]
    fn run_media_updates_tracks_linked_light_intensity() {
        use crate::components::{
            Light, LightFunction, LightFunctionSpec, LightState, LightType, Media,
        };

        let mut world = minimal_sim_world();

        // Spawn a light (index 7) sitting at half intensity, and a media entity
        // (range 0..=2 WU) linked to it whose current_height starts stale.
        let spec = LightFunctionSpec {
            function: LightFunction::Constant,
            period: 1,
            delta_period: 0,
            intensity: 0.5,
            delta_intensity: 0.0,
        };
        {
            let ecs = world.ecs_world_mut();
            ecs.spawn(Light {
                light_index: 7,
                light_type: LightType::Normal,
                state: LightState::PrimaryActive,
                flags: 0,
                phase: 0,
                period: 1,
                current_intensity: 0.5,
                initial_intensity: 0.5,
                final_intensity: 0.5,
                functions: [spec; 6],
                tag: 0,
            });
            ecs.spawn(Media {
                index: 0,
                polygon_index: 0,
                media_type: 0,
                height_low: 0.0,
                height_high: 2.0,
                light_index: 7,
                current_height: 99.0, // stale sentinel; must be overwritten
                current_direction: 0.0,
                current_magnitude: 0.0,
            });
        }

        world.run_media_updates();

        // height_low + (height_high - height_low) * intensity = 0 + 2 * 0.5 = 1.0
        let ecs = world.ecs_world_mut();
        let mut q = ecs.query::<&Media>();
        let media = q.iter(ecs).next().expect("media entity present");
        assert!(
            (media.current_height - 1.0).abs() < 1e-6,
            "media height should track linked light (expected 1.0, got {})",
            media.current_height
        );
    }

    /// Box 5.4 infra: `debug_toggle_nearest_light_switch()` must drive the REAL
    /// action-key path (face the switch → one ACTION rising-edge tick →
    /// `find_action_key_target` → `ToggleLight`) and report the controlled
    /// light's intensity straddling the toggle, crossing the lit/dark boundary.
    #[test]
    fn debug_toggle_nearest_light_switch_flips_the_controlled_light() {
        use crate::components::{Light, LightFunction, LightFunctionSpec, LightState, LightType};
        use crate::world_mechanics::panels::{ControlPanel, ControlPanels, PanelAction};

        let mut world = minimal_sim_world();

        // A steady light (index 3) currently fully lit, sitting in PrimaryActive.
        let lit_spec = LightFunctionSpec {
            function: LightFunction::Constant,
            period: 100,
            delta_period: 0,
            intensity: 1.0,
            delta_intensity: 0.0,
        };
        let dark_spec = LightFunctionSpec {
            function: LightFunction::Constant,
            period: 100,
            delta_period: 0,
            intensity: 0.0,
            delta_intensity: 0.0,
        };
        {
            use crate::components::{Facing, Position, VerticalLook};
            use crate::{Player, PolygonIndex};
            use glam::Vec3;

            let ecs = world.ecs_world_mut();
            // The minimal map has no player object, so spawn one (the debug hook
            // repositions it onto the switch). Placed at the square's center.
            ecs.spawn((
                Player,
                Position(Vec3::new(0.5, 0.5, 0.0)),
                Facing(0.0),
                VerticalLook::default(),
                PolygonIndex(0),
            ));
            ecs.spawn(Light {
                light_index: 3,
                light_type: LightType::Normal,
                state: LightState::PrimaryActive,
                flags: 0,
                phase: 0,
                period: 100,
                current_intensity: 1.0,
                initial_intensity: 1.0,
                final_intensity: 1.0,
                // active hold = lit, inactive hold = dark.
                functions: [
                    lit_spec, lit_spec, lit_spec, dark_spec, dark_spec, dark_spec,
                ],
                tag: 0,
            });

            // A light switch on line 1 (the east wall of the unit square) driving
            // light index 3. The minimal world's MapGeometry already carries this
            // line's endpoints/adjacency, so the debug pose + raycast can use it.
            ecs.insert_resource(ControlPanels(vec![ControlPanel {
                line_index: 1,
                side: 0,
                action: PanelAction::ToggleLight { light_index: 3 },
                max_distance: 1.5,
            }]));
        }

        let (idx, before, after) = world
            .debug_toggle_nearest_light_switch()
            .expect("the level has a light switch, so a toggle result is expected");

        assert_eq!(idx, 3, "the reported light is the one the switch controls");
        assert!(
            before > 0.5,
            "the light started lit (before={before}), so the toggle must darken it"
        );
        assert!(
            after < 0.5,
            "after activating the switch the light must be dark (after={after})"
        );
        assert!(
            (before - after).abs() > 0.4,
            "the toggle must move the intensity substantially (before={before}, after={after})"
        );
    }

    #[test]
    fn action_flags_empty() {
        let flags = ActionFlags::default();
        assert!(flags.is_empty());
    }

    #[test]
    fn entity_render_state_bincode_round_trip() {
        // box 1.2: a populated EntityRenderState (and its RenderEntityType)
        // round-trips through bincode unchanged.
        let state = EntityRenderState {
            entity_type: RenderEntityType::Monster {
                definition_index: 4,
            },
            position: glam::Vec3::new(1.5, -2.0, 3.25),
            facing: 0.7,
            shape: 12,
            frame: 6,
        };
        let bytes = bincode::serialize(&state).expect("serialize");
        let back: EntityRenderState = bincode::deserialize(&bytes).expect("deserialize");
        assert_eq!(back.position, state.position);
        assert_eq!(back.facing, state.facing);
        assert_eq!(back.shape, state.shape);
        assert_eq!(back.frame, state.frame);
        match back.entity_type {
            RenderEntityType::Monster { definition_index } => assert_eq!(definition_index, 4),
            _ => panic!("wrong RenderEntityType variant"),
        }
    }

    #[test]
    fn render_entity_type_variants_round_trip() {
        // box 1.2: each RenderEntityType variant survives bincode.
        for variant in [
            RenderEntityType::Monster {
                definition_index: 1,
            },
            RenderEntityType::Projectile {
                definition_index: 2,
            },
            RenderEntityType::Item { item_type: 3 },
            RenderEntityType::Effect {
                definition_index: 4,
            },
        ] {
            let bytes = bincode::serialize(&variant).expect("serialize");
            let back: RenderEntityType = bincode::deserialize(&bytes).expect("deserialize");
            assert_eq!(format!("{:?}", variant), format!("{:?}", back));
        }
    }

    #[test]
    fn weapon_render_state_bincode_round_trip() {
        // box 1.3: WeaponRenderState round-trips through bincode unchanged.
        let state = WeaponRenderState {
            collection: 3,
            shape: 9,
            frame: 1,
            vertical_position: 0.4,
            horizontal_position: 0.6,
        };
        let bytes = bincode::serialize(&state).expect("serialize");
        let back: WeaponRenderState = bincode::deserialize(&bytes).expect("deserialize");
        assert_eq!(back.collection, state.collection);
        assert_eq!(back.shape, state.shape);
        assert_eq!(back.frame, state.frame);
        assert_eq!(back.vertical_position, state.vertical_position);
        assert_eq!(back.horizontal_position, state.horizontal_position);
    }

    #[test]
    fn tick_input_mouse_deltas_round_trip() {
        let input = TickInput {
            action_flags: ActionFlags::new(ActionFlags::MOVE_FORWARD),
            mouse_yaw: 0.1,
            mouse_pitch: -0.05,
        };
        assert_eq!(input.mouse_yaw, 0.1);
        assert_eq!(input.mouse_pitch, -0.05);
        assert!(input.action_flags.contains(ActionFlags::MOVE_FORWARD));

        // Verify it round-trips through bevy_ecs resource insertion
        let mut world = bevy_ecs::prelude::World::new();
        world.insert_resource(TickInput {
            action_flags: ActionFlags::new(ActionFlags::MOVE_FORWARD),
            mouse_yaw: 0.1,
            mouse_pitch: -0.05,
        });
        let retrieved = world.resource::<TickInput>();
        assert_eq!(retrieved.mouse_yaw, 0.1);
        assert_eq!(retrieved.mouse_pitch, -0.05);
        assert!(retrieved.action_flags.contains(ActionFlags::MOVE_FORWARD));
    }
}
