use crate::player::movement::{
    apply_player_collision, compute_facing, compute_player_velocity, compute_vertical_look,
    velocity_local_to_world, velocity_world_to_local, PlayerPhysicsParams,
};
use crate::world::{MapGeometry, PhysicsTables, SimRng, SimWorld, TickCounter};

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
        self.update_lights();
        // 2. Update media (depends on light intensities)
        self.update_media();
        // 3. Update platforms (before player physics so collision uses new heights)
        self.update_platforms();
        // 4. Player physics
        self.run_player_physics();
        // 5. Player weapons (depends on player position/facing)
        self.run_player_weapons();
        // 6. Update monsters (depends on player position)
        self.update_monsters();
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
        let geo_clone = MapGeometry {
            polygon_vertices: geometry.polygon_vertices.clone(),
            floor_heights: geometry.floor_heights.clone(),
            ceiling_heights: geometry.ceiling_heights.clone(),
            polygon_adjacency: geometry.polygon_adjacency.clone(),
            line_endpoints: geometry.line_endpoints.clone(),
            line_solid: geometry.line_solid.clone(),
            line_transparent: geometry.line_transparent.clone(),
            polygon_media_index: geometry.polygon_media_index.clone(),
        };

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

        for (mut pos, mut vel, mut facing, mut vlook, mut angular_vel, mut poly_idx, mut grounded, mut oxygen, mut health, mut shield) in query.iter_mut(&mut self.world) {
            // Velocity is stored in player-local frame: x=forward, y=perp, z=vert.
            // Compute the next tick's player-local velocity from input.
            let new_local_vel =
                compute_player_velocity(vel.0, facing.0, &flags, &params, grounded.0);

            // Compute facing (turning) — mouse yaw applied directly, keyboard via angular velocity
            let (new_facing, new_angular) = compute_facing(facing.0, angular_vel.0, &flags, &params, mouse_yaw);
            facing.0 = new_facing;
            angular_vel.0 = new_angular;

            // Compute vertical look — mouse pitch applied directly
            vlook.0 = compute_vertical_look(vlook.0, &flags, &params, mouse_pitch);

            // Project player-local velocity into world-space using the NEW facing.
            let world_vel = velocity_local_to_world(new_local_vel, new_facing);

            // Apply collision in world-space.
            let new_pos = pos.0 + world_vel;
            let result = apply_player_collision(
                pos.0, new_pos, world_vel, poly_idx.0, &params, &geo_clone,
            );

            pos.0 = result.position;
            vel.0 = velocity_world_to_local(result.velocity, new_facing);
            poly_idx.0 = result.polygon_index;
            grounded.0 = result.grounded;

            // Media interaction: check if player is submerged
            let media_idx = geo_clone.polygon_media_index.get(poly_idx.0).copied().unwrap_or(-1);
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
        let health_val = self
            .world
            .get::<crate::Health>(entity)
            .map(|h| h.0);
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

    fn update_lights(&mut self) {
        let tick = self.world.resource::<TickCounter>().0;
        self.world
            .resource_scope(|world: &mut bevy_ecs::prelude::World, mut sim_rng: bevy_ecs::prelude::Mut<SimRng>| {
                let mut query = world.query::<&mut crate::Light>();
                for mut light in query.iter_mut(world) {
                    let intensity = crate::world_mechanics::lights::compute_light_intensity(
                        &*light,
                        tick,
                        &mut sim_rng.0,
                    );
                    light.current_intensity = intensity;
                }
            });
    }

    fn update_media(&mut self) {
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
                    crate::world_mechanics::media::compute_media_height(&*media, intensity);
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
                    crate::world_mechanics::platforms::tick_platform(&mut *platform);
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
            let mut q = self.world.query_filtered::<
                &crate::PolygonIndex,
                bevy_ecs::prelude::With<crate::Player>,
            >();
            q.iter(&self.world).next().map(|p| p.0)
        };

        if let Some(player_poly) = player_poly {
            let mut query = self.world.query::<&mut crate::Platform>();
            for mut platform in query.iter_mut(&mut self.world) {
                if platform.polygon_index == player_poly {
                    use crate::world_mechanics::platforms::{
                        activate_platform, should_activate, PlatformTrigger,
                    };
                    if should_activate(&*platform, PlatformTrigger::PlayerEntry) {
                        activate_platform(&mut *platform);
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
            if let Some(mut inv) = self.world.get_resource_mut::<crate::player::inventory::WeaponInventory>() {
                inv.cycle_forward(10);
            }
        }
        if cycle_back {
            if let Some(mut inv) = self.world.get_resource_mut::<crate::player::inventory::WeaponInventory>() {
                inv.cycle_backward(10);
            }
        }

        // Handle firing
        if !fire_primary && !fire_secondary {
            // Still need to tick weapon cooldowns
            if let Some(mut inv) = self.world.get_resource_mut::<crate::player::inventory::WeaponInventory>() {
                if let Some(weapon) = inv.current_mut() {
                    crate::combat::weapons::tick_weapon(weapon, false, 2, 3);
                }
            }
            return;
        }

        // Get weapon definition and tick
        let mut projectile_spawn: Option<(usize, glam::Vec3, glam::Vec3)> = None;

        if let Some(mut inv) = self.world.get_resource_mut::<crate::player::inventory::WeaponInventory>() {
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

                            let dir = glam::Vec3::new(player_facing.cos(), player_facing.sin(), 0.0);
                            let spawn_pos = player_pos + dir * 0.3 + glam::Vec3::new(0.0, 0.0, 0.4);
                            let velocity = dir * speed;

                            projectile_spawn =
                                Some((proj_type as usize, spawn_pos, velocity));
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
            q.iter(&self.world)
                .next()
                .map(|(e, pos)| (pos.0, e))
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

            for (entity, monster, state, pos, vel, facing, health, cooldown, poly_idx, _grounded, flying) in
                query.iter(&self.world)
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
                let terminal_vel = def.map(|d| d.terminal_velocity as f32 / 1024.0).unwrap_or(0.5);

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
                    let ranged_proj_type = def.map(|d| d.ranged_attack.attack_type as usize).unwrap_or(0);
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
                    target: if has_target { Some(player_entity) } else { None },
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
            let idle_monsters: Vec<(glam::Vec2, usize, u32, crate::MonsterState, bevy_ecs::entity::Entity)> = {
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
                },
                crate::Position(spawn_pos),
                crate::Velocity(velocity),
                crate::PolygonIndex(poly),
            ));
        }
    }

    fn update_projectiles(&mut self) {
        let physics_tables = match self.world.get_resource::<PhysicsTables>() {
            Some(pt) => pt.data.clone(),
            None => return,
        };

        // Clone geometry for wall collision
        let geometry = self.world.resource::<MapGeometry>();
        let polygon_adjacency = geometry.polygon_adjacency.clone();
        let line_endpoints = geometry.line_endpoints.clone();
        let line_solid = geometry.line_solid.clone();

        // Collect monster positions for entity collision
        let monster_data: Vec<(bevy_ecs::entity::Entity, glam::Vec2, f32, f32, f32)> = {
            let mut q = self.world.query::<(
                bevy_ecs::entity::Entity,
                &crate::Monster,
                &crate::Position,
                &crate::CollisionRadius,
                &crate::EntityHeight,
            )>();
            q.iter(&self.world)
                .map(|(e, _m, pos, r, h)| {
                    (e, glam::Vec2::new(pos.0.x, pos.0.y), r.0, pos.0.z, pos.0.z + h.0)
                })
                .collect()
        };

        // Get player data for monster-projectile collision
        let player_data: Option<(bevy_ecs::entity::Entity, glam::Vec2, f32, f32, f32)> = {
            let mut q = self.world.query_filtered::<(
                bevy_ecs::entity::Entity,
                &crate::Position,
                &crate::CollisionRadius,
                &crate::EntityHeight,
            ), bevy_ecs::prelude::With<crate::Player>>();
            q.iter(&self.world)
                .next()
                .map(|(e, pos, r, h)| {
                    (e, glam::Vec2::new(pos.0.x, pos.0.y), r.0, pos.0.z, pos.0.z + h.0)
                })
        };

        // Process each projectile
        struct ProjectileUpdate {
            entity: bevy_ecs::entity::Entity,
            despawn: bool,
            new_pos: glam::Vec3,
            new_vel: glam::Vec3,
            new_distance: f32,
            _new_poly: usize,
            detonation_point: Option<glam::Vec3>,
            hit_entity: Option<bevy_ecs::entity::Entity>,
            damage_amount: i16,
            aoe_radius: f32,
            effect_def: Option<usize>,
        }

        let mut proj_updates: Vec<ProjectileUpdate> = Vec::new();

        {
            let mut query = self.world.query::<(
                bevy_ecs::entity::Entity,
                &crate::Projectile,
                &crate::Position,
                &crate::Velocity,
                &crate::PolygonIndex,
                Option<&crate::ProjectileSource>,
            )>();

            for (entity, proj, pos, vel, poly_idx, source) in query.iter(&self.world) {
                let def = physics_tables
                    .projectiles
                    .as_ref()
                    .and_then(|p| p.get(proj.definition_index));

                let max_range = def.map(|d| d.maximum_range as f32 / 1024.0).unwrap_or(0.0);
                let is_gravity = def.map(|d| d.flags & 0x0010 != 0).unwrap_or(false);
                let is_homing = def.map(|d| d.flags & 0x0020 != 0).unwrap_or(false);
                let aoe = def.map(|d| d.area_of_effect as f32 / 1024.0).unwrap_or(0.0);
                let damage_base = def.map(|d| d.damage.base).unwrap_or(10);
                let damage_scale = def.map(|d| d.damage.scale).unwrap_or(1.0);
                let detonation_effect = def.map(|d| d.detonation_effect).unwrap_or(-1);

                // Apply gravity
                let mut current_vel = vel.0;
                if is_gravity {
                    current_vel =
                        crate::combat::projectiles::apply_projectile_gravity(current_vel, 0.01);
                }

                // Apply homing (toward closest monster or player)
                if is_homing {
                    // Find nearest target
                    let is_player_fired = source.is_some();
                    if is_player_fired {
                        // Home toward nearest monster
                        if let Some((_, nearest_pos, _, _, _)) = monster_data
                            .iter()
                            .min_by(|a, b| {
                                let da = a.1.distance(glam::Vec2::new(pos.0.x, pos.0.y));
                                let db = b.1.distance(glam::Vec2::new(pos.0.x, pos.0.y));
                                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                            })
                        {
                            let target = glam::Vec3::new(nearest_pos.x, nearest_pos.y, pos.0.z);
                            current_vel =
                                crate::combat::projectiles::apply_homing(current_vel, pos.0, target, 0.05);
                        }
                    } else if let Some((_, ppos, _, _, _)) = &player_data {
                        let target = glam::Vec3::new(ppos.x, ppos.y, pos.0.z);
                        current_vel =
                            crate::combat::projectiles::apply_homing(current_vel, pos.0, target, 0.05);
                    }
                }

                // Advance position
                let (new_pos, tick_dist) =
                    crate::combat::projectiles::advance_projectile(pos.0, current_vel);
                let new_distance = proj.distance_traveled + tick_dist;

                let mut despawn = false;
                let mut detonation_point = None;
                let mut hit_entity = None;
                let mut damage_amount = (damage_base as f32 * damage_scale) as i16;

                // Check range limit
                if crate::combat::projectiles::check_range_limit(new_distance, max_range) {
                    despawn = true;
                    detonation_point = Some(new_pos);
                }

                // Check wall collision
                if !despawn && poly_idx.0 < polygon_adjacency.len() {
                    let old_2d = glam::Vec2::new(pos.0.x, pos.0.y);
                    let new_2d = glam::Vec2::new(new_pos.x, new_pos.y);
                    match crate::combat::projectiles::check_projectile_wall_collision(
                        old_2d,
                        new_2d,
                        pos.0.z,
                        new_pos.z,
                        poly_idx.0,
                        &polygon_adjacency,
                        &line_endpoints,
                        &line_solid,
                    ) {
                        crate::combat::projectiles::WallHitResult::Hit { hit_point, .. } => {
                            despawn = true;
                            detonation_point = Some(hit_point);
                        }
                        crate::combat::projectiles::WallHitResult::Clear => {}
                    }
                }

                // Check entity collision
                if !despawn {
                    let is_player_fired = source.is_some();

                    // Build collision targets
                    let mut targets: Vec<(glam::Vec2, f32, f32, f32)> = Vec::new();
                    let mut target_entities: Vec<bevy_ecs::entity::Entity> = Vec::new();

                    if is_player_fired {
                        // Player projectile: check against monsters
                        for (e, center, radius, z_bot, z_top) in &monster_data {
                            targets.push((*center, *radius, *z_bot, *z_top));
                            target_entities.push(*e);
                        }
                    } else {
                        // Monster projectile: check against player
                        if let Some((e, center, radius, z_bot, z_top)) = &player_data {
                            targets.push((*center, *radius, *z_bot, *z_top));
                            target_entities.push(*e);
                        }
                    }

                    if let Some(hit) = crate::combat::projectiles::check_projectile_entity_collision(
                        pos.0,
                        new_pos,
                        &targets,
                    ) {
                        despawn = true;
                        detonation_point = Some(hit.hit_point);
                        hit_entity = Some(target_entities[hit.entity_index]);
                    }
                }

                let effect_def_idx = if detonation_effect >= 0 {
                    Some(detonation_effect as usize)
                } else {
                    None
                };

                // If not despawning but no damage, clear damage_amount
                if !despawn {
                    damage_amount = 0;
                }

                proj_updates.push(ProjectileUpdate {
                    entity,
                    despawn,
                    new_pos: if despawn { pos.0 } else { new_pos },
                    new_vel: current_vel,
                    new_distance,
                    _new_poly: poly_idx.0,
                    detonation_point,
                    hit_entity,
                    damage_amount,
                    aoe_radius: aoe,
                    effect_def: effect_def_idx,
                });
            }
        }

        // Apply projectile updates
        let mut to_despawn: Vec<bevy_ecs::entity::Entity> = Vec::new();
        let mut effects_to_spawn: Vec<(glam::Vec3, usize)> = Vec::new();

        for update in &proj_updates {
            if update.despawn {
                to_despawn.push(update.entity);

                // Apply direct hit damage
                if let Some(hit_entity) = update.hit_entity {
                    if update.damage_amount > 0 {
                        self.apply_damage_to_entity(hit_entity, update.damage_amount);
                    }
                }

                // Spawn detonation effect
                if let (Some(det_point), Some(effect_idx)) =
                    (update.detonation_point, update.effect_def)
                {
                    effects_to_spawn.push((det_point, effect_idx));
                }

                // AoE damage
                if update.aoe_radius > 0.0 {
                    if let Some(det_point) = update.detonation_point {
                        // Damage nearby entities
                        let det_2d = glam::Vec2::new(det_point.x, det_point.y);

                        // Damage monsters
                        for (entity, center, _, _, _) in &monster_data {
                            let dist = det_2d.distance(*center);
                            let aoe_dmg = crate::combat::damage::calculate_aoe_damage(
                                update.damage_amount,
                                dist,
                                update.aoe_radius,
                            );
                            if aoe_dmg > 0 {
                                self.apply_damage_to_entity(*entity, aoe_dmg);
                            }
                        }

                        // Damage player
                        if let Some((pe, pcenter, _, _, _)) = &player_data {
                            let dist = det_2d.distance(*pcenter);
                            let aoe_dmg = crate::combat::damage::calculate_aoe_damage(
                                update.damage_amount,
                                dist,
                                update.aoe_radius,
                            );
                            if aoe_dmg > 0 {
                                self.apply_damage_to_entity(*pe, aoe_dmg);
                            }
                        }
                    }
                }
            } else {
                // Update position/velocity
                if let Some(mut pos) = self.world.get_mut::<crate::Position>(update.entity) {
                    pos.0 = update.new_pos;
                }
                if let Some(mut vel) = self.world.get_mut::<crate::Velocity>(update.entity) {
                    vel.0 = update.new_vel;
                }
                if let Some(mut proj) = self.world.get_mut::<crate::Projectile>(update.entity) {
                    proj.distance_traveled = update.new_distance;
                }
            }
        }

        // Despawn projectiles
        for entity in to_despawn {
            self.world.despawn(entity);
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
                    if let Some(effect) = crate::world_mechanics::items::item_effect(item.item_type) {
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
                    // Inventory items tracked separately (keys, powerups) - stub for now
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
        let mut query = self.world.query_filtered::<&crate::Position, bevy_ecs::prelude::With<crate::Player>>();
        query.iter(&self.world).next().map(|p| p.0)
    }

    /// Query the player's facing angle.
    pub fn player_facing(&mut self) -> Option<f32> {
        let mut query = self.world.query_filtered::<&crate::Facing, bevy_ecs::prelude::With<crate::Player>>();
        query.iter(&self.world).next().map(|f| f.0)
    }

    /// Query the player's health.
    pub fn player_health(&mut self) -> Option<i16> {
        let mut query = self.world.query_filtered::<&crate::Health, bevy_ecs::prelude::With<crate::Player>>();
        query.iter(&self.world).next().map(|h| h.0)
    }

    /// Query the player's shield.
    pub fn player_shield(&mut self) -> Option<i16> {
        let mut query = self.world.query_filtered::<&crate::Shield, bevy_ecs::prelude::With<crate::Player>>();
        query.iter(&self.world).next().map(|s| s.0)
    }

    /// Query the player's oxygen.
    pub fn player_oxygen(&mut self) -> Option<i16> {
        let mut query = self.world.query_filtered::<&crate::Oxygen, bevy_ecs::prelude::With<crate::Player>>();
        query.iter(&self.world).next().map(|o| o.0)
    }

    /// Query the player's vertical look angle.
    pub fn player_vertical_look(&mut self) -> Option<f32> {
        let mut query = self.world.query_filtered::<&crate::VerticalLook, bevy_ecs::prelude::With<crate::Player>>();
        query.iter(&self.world).next().map(|v| v.0)
    }

    /// Query the player's current polygon index.
    pub fn player_polygon(&mut self) -> Option<usize> {
        let mut query = self.world.query_filtered::<&crate::PolygonIndex, bevy_ecs::prelude::With<crate::Player>>();
        query.iter(&self.world).next().map(|p| p.0)
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
            let mut query = self.world.query::<(
                &crate::Projectile,
                &crate::Position,
            )>();
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
            let mut query = self.world.query::<(
                &crate::Effect,
                &crate::Position,
            )>();
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
#[derive(Debug, Clone)]
pub struct EntityRenderState {
    pub entity_type: RenderEntityType,
    pub position: glam::Vec3,
    pub facing: f32,
    pub shape: u16,
    pub frame: u16,
}

/// Type of entity for rendering purposes.
#[derive(Debug, Clone)]
pub enum RenderEntityType {
    Monster { definition_index: usize },
    Projectile { definition_index: usize },
    Item { item_type: i16 },
    Effect { definition_index: usize },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_flags_contains() {
        let flags = ActionFlags::new(ActionFlags::MOVE_FORWARD | ActionFlags::FIRE_PRIMARY);
        assert!(flags.contains(ActionFlags::MOVE_FORWARD));
        assert!(flags.contains(ActionFlags::FIRE_PRIMARY));
        assert!(!flags.contains(ActionFlags::STRAFE_LEFT));
    }

    #[test]
    fn action_flags_empty() {
        let flags = ActionFlags::default();
        assert!(flags.is_empty());
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
