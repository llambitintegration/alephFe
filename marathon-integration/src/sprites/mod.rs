use marathon_formats::ShapeDescriptor;

/// Entity type for sprite rendering categorization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpriteEntityType {
    Monster,
    Item,
    Projectile,
    Effect,
    Player,
}

/// Sprite state extracted from the simulation for a single entity.
#[derive(Debug, Clone)]
pub struct EntitySpriteState {
    /// Unique entity identifier.
    pub entity_id: u32,
    /// Entity type for categorization.
    pub entity_type: SpriteEntityType,
    /// World position (x, y, z).
    pub position: (i32, i32, i32),
    /// Facing angle (0..65536 fixed-point).
    pub facing: u16,
    /// Shape descriptor (collection, clut, shape index).
    pub shape: ShapeDescriptor,
    /// Current animation frame index within the sequence.
    pub frame: u16,
    /// Whether the entity is visible (not hidden/inactive).
    pub visible: bool,
}

/// A sprite render command to pass to marathon-viewer.
#[derive(Debug, Clone)]
pub struct SpriteRenderCommand {
    /// World position for the billboarded sprite.
    pub position: (i32, i32, i32),
    /// Which shape collection and frame to render.
    pub shape: ShapeDescriptor,
    /// Animation frame within the sequence.
    pub frame: u16,
    /// Facing angle for multi-angle sprites.
    pub facing: u16,
}

/// Bridge that converts simulation entity state into sprite render commands.
pub struct SpriteBridge {
    /// Previous frame's entity set for lifecycle tracking.
    previous_entities: Vec<u32>,
}

impl SpriteBridge {
    pub fn new() -> Self {
        Self {
            previous_entities: Vec::new(),
        }
    }

    /// Process entity states from the simulation and produce render commands.
    ///
    /// Returns (commands, newly_added_ids, removed_ids) for lifecycle tracking.
    pub fn update(
        &mut self,
        entities: &[EntitySpriteState],
    ) -> (Vec<SpriteRenderCommand>, Vec<u32>, Vec<u32>) {
        let current_ids: Vec<u32> = entities
            .iter()
            .filter(|e| e.visible)
            .map(|e| e.entity_id)
            .collect();

        let added: Vec<u32> = current_ids
            .iter()
            .filter(|id| !self.previous_entities.contains(id))
            .copied()
            .collect();

        let removed: Vec<u32> = self
            .previous_entities
            .iter()
            .filter(|id| !current_ids.contains(id))
            .copied()
            .collect();

        let commands: Vec<SpriteRenderCommand> = entities
            .iter()
            .filter(|e| e.visible)
            .map(|e| SpriteRenderCommand {
                position: e.position,
                shape: e.shape,
                frame: e.frame,
                facing: e.facing,
            })
            .collect();

        self.previous_entities = current_ids;

        (commands, added, removed)
    }
}

impl Default for SpriteBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entity(id: u32, visible: bool) -> EntitySpriteState {
        EntitySpriteState {
            entity_id: id,
            entity_type: SpriteEntityType::Monster,
            position: (100, 200, 0),
            facing: 0,
            shape: ShapeDescriptor(0),
            frame: 0,
            visible,
        }
    }

    #[test]
    fn initial_entities_are_added() {
        let mut bridge = SpriteBridge::new();
        let entities = vec![make_entity(1, true), make_entity(2, true)];

        let (commands, added, removed) = bridge.update(&entities);
        assert_eq!(commands.len(), 2);
        assert_eq!(added, vec![1, 2]);
        assert!(removed.is_empty());
    }

    #[test]
    fn removed_entities_detected() {
        let mut bridge = SpriteBridge::new();

        // Frame 1: entities 1, 2
        bridge.update(&[make_entity(1, true), make_entity(2, true)]);

        // Frame 2: only entity 1
        let (commands, added, removed) = bridge.update(&[make_entity(1, true)]);
        assert_eq!(commands.len(), 1);
        assert!(added.is_empty());
        assert_eq!(removed, vec![2]);
    }

    #[test]
    fn invisible_entities_excluded() {
        let mut bridge = SpriteBridge::new();
        let entities = vec![make_entity(1, true), make_entity(2, false)];

        let (commands, added, _) = bridge.update(&entities);
        assert_eq!(commands.len(), 1);
        assert_eq!(added, vec![1]);
    }
}
