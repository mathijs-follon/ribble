use crate::archetype::ArchetypeId;
use crate::query::access::{matches_access, matches_filter, FilterDesc, QueryAccess};
use crate::world::World;

/// Cached query match list. Recompute with [`QueryState::update`] when archetypes change.
#[derive(Debug, Clone)]
pub struct QueryState {
    pub access: QueryAccess,
    pub filter: FilterDesc,
    matched_archetypes: Vec<usize>,
    archetype_count: usize,
}

impl QueryState {
    pub fn new(access: QueryAccess, filter: FilterDesc) -> Self {
        Self {
            access,
            filter,
            matched_archetypes: Vec::new(),
            archetype_count: 0,
        }
    }

    pub fn update(&mut self, world: &World) {
        let archetypes = world.archetypes();
        if self.archetype_count == archetypes.len() && !self.matched_archetypes.is_empty() {
            return;
        }

        self.archetype_count = archetypes.len();
        self.matched_archetypes.clear();

        for (idx, arch) in archetypes.iter().enumerate() {
            if matches_access(&arch.type_signature, &self.access)
                && matches_filter(&arch.type_signature, &self.filter)
            {
                self.matched_archetypes.push(idx);
            }
        }
    }

    pub fn force_update(&mut self, world: &World) {
        self.archetype_count = 0;
        self.update(world);
    }

    pub fn matched_archetypes(&self) -> &[usize] {
        &self.matched_archetypes
    }

    pub fn is_empty(&self, world: &World) -> bool {
        self.matched_archetypes
            .iter()
            .all(|&idx| world.archetypes()[idx].entities.is_empty())
    }

    pub fn len(&self, world: &World) -> usize {
        self.matched_archetypes
            .iter()
            .map(|&idx| world.archetypes()[idx].entities.len())
            .sum()
    }

    pub fn archetype_id(&self, world: &World, matched_idx: usize) -> ArchetypeId {
        world.archetypes()[self.matched_archetypes[matched_idx]].id
    }
}
