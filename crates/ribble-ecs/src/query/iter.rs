use crate::query::data::QueryData;
use crate::query::filtered::{FilteredEntityMut, FilteredEntityRef};
use crate::query::filter::QueryFilter;
use crate::query::state::QueryState;
use crate::world::World;

/// Immutable iterator driven by a cached [`QueryState`].
pub struct QueryIter<'w, D: QueryData, F: QueryFilter = crate::query::filter::Allow> {
    world: &'w World,
    state: &'w QueryState,
    matched_idx: usize,
    row: usize,
    _marker: std::marker::PhantomData<(D, F)>,
}

impl<'w, D: QueryData, F: QueryFilter> QueryIter<'w, D, F> {
    pub fn new(world: &'w World, state: &'w QueryState) -> Self {
        Self {
            world,
            state,
            matched_idx: 0,
            row: 0,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<'w, D: QueryData, F: QueryFilter> Iterator for QueryIter<'w, D, F> {
    type Item = D::Item<'w>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.matched_idx >= self.state.matched_archetypes().len() {
                return None;
            }

            let arch_idx = self.state.matched_archetypes()[self.matched_idx];
            let arch = &self.world.archetypes()[arch_idx];
            if !F::matches(arch) {
                self.matched_idx += 1;
                self.row = 0;
                continue;
            }

            if self.row >= arch.entities.len() {
                self.matched_idx += 1;
                self.row = 0;
                continue;
            }

            let entity = arch.entities[self.row];
            let item = D::fetch(self.world, entity, self.row, arch_idx);
            self.row += 1;
            return Some(item);
        }
    }
}

/// Iterator over dynamically filtered read-only entity references.
pub struct DynamicQueryIter<'w> {
    world: &'w World,
    state: &'w QueryState,
    matched_idx: usize,
    row: usize,
}

impl<'w> DynamicQueryIter<'w> {
    pub fn new(world: &'w World, state: &'w QueryState) -> Self {
        Self {
            world,
            state,
            matched_idx: 0,
            row: 0,
        }
    }
}

impl<'w> Iterator for DynamicQueryIter<'w> {
    type Item = FilteredEntityRef<'w>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.matched_idx >= self.state.matched_archetypes().len() {
                return None;
            }

            let arch_idx = self.state.matched_archetypes()[self.matched_idx];
            let arch = &self.world.archetypes()[arch_idx];
            if self.row >= arch.entities.len() {
                self.matched_idx += 1;
                self.row = 0;
                continue;
            }

            let entity = arch.entities[self.row];
            let item = FilteredEntityRef::with_access(
                self.world,
                entity,
                arch_idx,
                self.row,
                self.state.access.clone(),
            );
            self.row += 1;
            return Some(item);
        }
    }
}

/// Iterator over dynamically filtered mutable entity references.
pub struct DynamicQueryIterMut<'w> {
    world: *mut World,
    state: QueryState,
    matched_idx: usize,
    row: usize,
    _marker: std::marker::PhantomData<&'w mut World>,
}

impl<'w> DynamicQueryIterMut<'w> {
    pub fn new(world: &'w mut World, state: QueryState) -> Self {
        Self {
            world,
            state,
            matched_idx: 0,
            row: 0,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<'w> Iterator for DynamicQueryIterMut<'w> {
    type Item = FilteredEntityMut<'w>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.matched_idx >= self.state.matched_archetypes().len() {
                return None;
            }

            // SAFETY: each item accesses a distinct row; caller must not retain refs.
            let world = unsafe { &mut *self.world };
            let arch_idx = self.state.matched_archetypes()[self.matched_idx];
            let arch = &world.archetypes()[arch_idx];
            if self.row >= arch.entities.len() {
                self.matched_idx += 1;
                self.row = 0;
                continue;
            }

            let entity = arch.entities[self.row];
            let access = self.state.access.clone();
            let item = FilteredEntityMut::with_access(world, entity, arch_idx, self.row, access);
            self.row += 1;
            return Some(item);
        }
    }
}

/// Convenience wrapper pairing a world reference with a cached query state.
pub struct Query<'w, D: QueryData, F: QueryFilter = crate::query::filter::Allow> {
    world: &'w World,
    state: QueryState,
    _marker: std::marker::PhantomData<(D, F)>,
}

impl<'w, D: QueryData, F: QueryFilter> Query<'w, D, F> {
    pub fn new(world: &'w World, state: QueryState) -> Self {
        Self {
            world,
            state,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn iter(&self) -> QueryIter<'_, D, F> {
        QueryIter::new(self.world, &self.state)
    }

    pub fn len(&self) -> usize {
        self.state.len(self.world)
    }

    pub fn is_empty(&self) -> bool {
        self.state.is_empty(self.world)
    }

    pub fn state(&self) -> &QueryState {
        &self.state
    }

    pub fn single(&self) -> Option<D::Item<'_>> {
        self.iter().next()
    }
}

pub struct QueryMut<'w, D: QueryData, F: QueryFilter = crate::query::filter::Allow> {
    _world: &'w mut World,
    state: QueryState,
    _marker: std::marker::PhantomData<(D, F)>,
}

impl<'w, D: QueryData, F: QueryFilter> QueryMut<'w, D, F> {
    pub fn new(world: &'w mut World, state: QueryState) -> Self {
        Self {
            _world: world,
            state,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn state(&self) -> &QueryState {
        &self.state
    }
}

