//! Placement stage: the PURE mapping from a lane's lease stream onto the
//! monster's spatial home (box 5.5).
//!
//! A monster's body does NOT live wherever a per-sprite tint puts it; it lives in
//! a *place* derived from its lease, so spatial position itself carries meaning:
//! - a `changeId`/`lease_key` lease → a ROOM (room-per-change);
//! - a `keyType:path` lease → a CORRIDOR/ZONE per domain (path-leases group by the
//!   domain prefix of their stripped path);
//! - a lease COLLISION (`fleet.lease.collision`) → the blocked monster QUEUES at
//!   the occupied workbench rather than being tinted in place.
//!
//! The room/corridor LABEL is derived from the *stripped* lease attribute; the
//! lease `key` itself is treated as stable and OPAQUE — used only for grouping
//! (identity/equality), never parsed for human-facing text. Every function is
//! total, deterministic, and side-effect-free: no clock, no RNG, no I/O.

use crate::event::EntityDesc;

/// The kind of place a lease resolves to (box 5.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlaceKind {
    /// A room — one per `changeId`/`lease_key` (room-per-change).
    Room,
    /// A corridor/zone — one per domain, for a `keyType:path` lease.
    Corridor,
    /// A queue slot at an occupied workbench — for a blocked (collided) lease.
    QueueAtWorkbench,
}

/// A monster's resolved spatial home, derived from its lease (box 5.5).
///
/// `key` is the stable, OPAQUE grouping key (two monsters with the same `key`
/// share the same place); `label` is the human-facing room/corridor name derived
/// from the STRIPPED lease attribute — never the opaque key. `kind` distinguishes
/// a room, a per-domain corridor, and a queue slot.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Placement {
    /// What kind of place this is.
    pub kind: PlaceKind,
    /// The opaque, stable grouping key (used for equality/grouping only).
    pub key: String,
    /// The human-facing label, derived from the stripped lease attribute.
    pub label: String,
}

/// A captured lease for a lane, normalized from the lease stream (box 5.5).
///
/// The caller supplies the lease `key` (opaque, stable, used for grouping), an
/// optional `key_type` (`"path"` selects the corridor/zone-per-domain mapping),
/// and an optional `collision_workbench` — set to the OCCUPYING lane's workbench
/// key when a `fleet.lease.collision` blocks this lane.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Lease {
    /// The opaque, stable lease key (`changeId`/`lease_key`/path).
    pub key: String,
    /// The lease key type; `Some("path")` selects the corridor mapping.
    pub key_type: Option<String>,
    /// When set, this lane is BLOCKED at the named occupied workbench (collision).
    pub collision_workbench: Option<String>,
}

/// Strip a lease key down to its human-facing label (box 5.5).
///
/// The opaque key may carry a `keyType:` prefix and/or path segments; the label
/// is the last meaningful segment with the prefix removed, so a `changeId`
/// `change/add-foo` labels as `add-foo` and a path `path:src/net/mod.rs` labels
/// off its domain. The key itself is left untouched for grouping.
#[must_use]
fn strip_to_label(key: &str) -> &str {
    // Drop a leading `keyType:` prefix if present (e.g. `path:src/x` -> `src/x`).
    let after_prefix = key.split_once(':').map_or(key, |(_, rest)| rest);
    // The label is the trailing segment of a `/`-delimited key.
    after_prefix.rsplit('/').next().unwrap_or(after_prefix)
}

/// The domain a `keyType:path` lease groups under (box 5.5).
///
/// All paths under the same leading directory share one corridor/zone, so
/// `src/net/a.rs` and `src/net/b.rs` group under the `src` domain corridor (the
/// first path segment after any `keyType:` prefix).
#[must_use]
fn path_domain(key: &str) -> &str {
    let after_prefix = key.split_once(':').map_or(key, |(_, rest)| rest);
    after_prefix.split('/').next().unwrap_or(after_prefix)
}

/// Map a lease onto the monster's spatial home (box 5.5).
///
/// - A collision lease (`collision_workbench` set) → a [`PlaceKind::QueueAtWorkbench`]
///   keyed on the OCCUPYING workbench, so the blocked monster queues there.
/// - A `keyType:path` lease → a [`PlaceKind::Corridor`] keyed on the path DOMAIN,
///   so a whole domain shares one corridor/zone.
/// - Any other lease (a `changeId`/`lease_key`) → a [`PlaceKind::Room`] keyed on
///   the opaque lease key (room-per-change).
///
/// The `label` is always the STRIPPED attribute; the `key` is the opaque grouping
/// key. Pure: depends only on its argument.
#[must_use]
pub fn placement_for(lease: &Lease) -> Placement {
    // A collision takes precedence: the blocked monster queues at the occupied
    // workbench regardless of its own lease key type.
    if let Some(workbench) = &lease.collision_workbench {
        return Placement {
            kind: PlaceKind::QueueAtWorkbench,
            // Grouping key is the OCCUPIED workbench, so all blocked monsters
            // queue at the same place.
            key: workbench.clone(),
            label: strip_to_label(workbench).to_string(),
        };
    }

    // A path lease maps to a per-domain corridor/zone.
    if lease.key_type.as_deref() == Some("path") {
        let domain = path_domain(&lease.key);
        return Placement {
            kind: PlaceKind::Corridor,
            key: domain.to_string(),
            label: domain.to_string(),
        };
    }

    // Default: a room per change/lease_key. The key stays opaque for grouping;
    // the label is the stripped attribute.
    Placement {
        kind: PlaceKind::Room,
        key: lease.key.clone(),
        label: strip_to_label(&lease.key).to_string(),
    }
}

/// Read a lease off an [`EntityDesc`]'s `meta` axes, if present (box 5.5).
///
/// Reads `lease_key`/`changeId` as the key, `keyType` as the optional key type,
/// and `lease_collision_workbench` as the occupying workbench on a collision. A
/// desc carrying no lease key resolves to `None` (no spatial home from a lease).
#[must_use]
pub fn lease_of(desc: &EntityDesc) -> Option<Lease> {
    let key = desc
        .meta
        .get("lease_key")
        .or_else(|| desc.meta.get("changeId"))?
        .clone();
    Some(Lease {
        key,
        key_type: desc.meta.get("keyType").cloned(),
        collision_workbench: desc.meta.get("lease_collision_workbench").cloned(),
    })
}

/// The placement for an [`EntityDesc`], reading its lease axes (box 5.5).
///
/// Convenience over [`placement_for`]: a desc carrying no lease key resolves to
/// `None` (no lease → no lease-derived home) with no error.
#[must_use]
pub fn placement_of(desc: &EntityDesc) -> Option<Placement> {
    lease_of(desc).as_ref().map(placement_for)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{EntityKind, EntityState};
    use std::collections::HashMap;

    fn desc(lane_id: &str) -> EntityDesc {
        EntityDesc {
            lane_id: lane_id.to_string(),
            kind: EntityKind::Agent,
            label: "label".to_string(),
            state: EntityState::Active,
            meta: HashMap::new(),
        }
    }

    fn lease(key: &str) -> Lease {
        Lease {
            key: key.to_string(),
            key_type: None,
            collision_workbench: None,
        }
    }

    // Scenario: A change-lease places the monster in its room.
    #[test]
    fn change_lease_places_monster_in_its_room() {
        let l = lease("change/add-platform-mechanics");
        let p = placement_for(&l);
        assert_eq!(p.kind, PlaceKind::Room, "a change lease yields a room");
        // The room label is the STRIPPED attribute, not the opaque key.
        assert_eq!(p.label, "add-platform-mechanics");
        assert_ne!(p.label, l.key, "label is stripped, not the opaque key");
        // The key stays opaque for grouping.
        assert_eq!(p.key, "change/add-platform-mechanics");
    }

    #[test]
    fn same_change_key_groups_into_the_same_room() {
        // Two lanes on the same changeId share one opaque grouping key (room).
        let a = placement_for(&lease("change/foo"));
        let b = placement_for(&lease("change/foo"));
        assert_eq!(a.key, b.key, "same change -> same opaque room key");
        assert_eq!(a, b);
        // A different change is a different room.
        let c = placement_for(&lease("change/bar"));
        assert_ne!(a.key, c.key);
    }

    // keyType:path lease -> corridor/zone-per-domain.
    #[test]
    fn path_lease_maps_to_corridor_per_domain() {
        let l = Lease {
            key: "src/net/transfer.rs".to_string(),
            key_type: Some("path".to_string()),
            collision_workbench: None,
        };
        let p = placement_for(&l);
        assert_eq!(
            p.kind,
            PlaceKind::Corridor,
            "a path lease yields a corridor"
        );
        assert_eq!(p.label, "src", "corridor groups by the domain prefix");

        // Two distinct paths under the same domain share one corridor.
        let q = placement_for(&Lease {
            key: "src/sim/tick.rs".to_string(),
            key_type: Some("path".to_string()),
            collision_workbench: None,
        });
        assert_eq!(p.key, q.key, "same domain -> same corridor key");

        // A different domain is a different corridor.
        let r = placement_for(&Lease {
            key: "docs/readme.md".to_string(),
            key_type: Some("path".to_string()),
            collision_workbench: None,
        });
        assert_ne!(p.key, r.key);
    }

    // Scenario: A lease collision queues the blocked monster.
    #[test]
    fn lease_collision_queues_the_blocked_monster() {
        let l = Lease {
            key: "change/mine".to_string(),
            key_type: None,
            collision_workbench: Some("workbench/change/theirs".to_string()),
        };
        let p = placement_for(&l);
        assert_eq!(
            p.kind,
            PlaceKind::QueueAtWorkbench,
            "a collision queues the blocked monster at the occupied workbench"
        );
        // Queued at the OCCUPYING workbench, not its own lease room.
        assert_eq!(p.key, "workbench/change/theirs");
        assert_eq!(p.label, "theirs");

        // Two lanes blocked on the same workbench queue at the same place.
        let other = Lease {
            key: "change/other".to_string(),
            key_type: None,
            collision_workbench: Some("workbench/change/theirs".to_string()),
        };
        assert_eq!(placement_for(&other).key, p.key);
    }

    #[test]
    fn collision_takes_precedence_over_path_corridor() {
        // A path lease that is ALSO blocked queues at the workbench, not a corridor.
        let l = Lease {
            key: "src/net/a.rs".to_string(),
            key_type: Some("path".to_string()),
            collision_workbench: Some("bench/x".to_string()),
        };
        assert_eq!(placement_for(&l).kind, PlaceKind::QueueAtWorkbench);
    }

    #[test]
    fn entity_desc_lease_axes_drive_placement() {
        let mut d = desc("lane-1");
        d.meta
            .insert("lease_key".to_string(), "change/add-foo".to_string());
        let p = placement_of(&d).expect("a lease-bearing desc resolves a place");
        assert_eq!(p.kind, PlaceKind::Room);
        assert_eq!(p.label, "add-foo");
    }

    #[test]
    fn entity_desc_path_lease_drives_corridor() {
        let mut d = desc("lane-1");
        d.meta
            .insert("lease_key".to_string(), "src/net/mod.rs".to_string());
        d.meta.insert("keyType".to_string(), "path".to_string());
        let p = placement_of(&d).expect("path lease resolves");
        assert_eq!(p.kind, PlaceKind::Corridor);
        assert_eq!(p.label, "src");
    }

    #[test]
    fn no_lease_axis_resolves_to_no_placement_no_error() {
        let d = desc("lane-1");
        assert_eq!(placement_of(&d), None);
    }
}
