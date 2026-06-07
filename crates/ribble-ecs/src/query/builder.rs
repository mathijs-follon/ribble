use std::any::TypeId;

use crate::component::Component;
use crate::query::access::{FilterDesc, QueryAccess};
use crate::query::state::QueryState;
use crate::world::World;

/// Build runtime queries with arbitrary component access and filter expressions.
#[derive(Debug, Clone)]
pub struct QueryBuilder {
    access: QueryAccess,
    filter: FilterDesc,
}

impl QueryBuilder {
    pub fn new() -> Self {
        Self {
            access: QueryAccess::new(),
            filter: FilterDesc::All,
        }
    }

    pub fn read<T: Component>(&mut self) -> &mut Self {
        self.access.read(TypeId::of::<T>());
        self
    }

    pub fn read_id(&mut self, type_id: TypeId) -> &mut Self {
        self.access.read(type_id);
        self
    }

    pub fn write<T: Component>(&mut self) -> &mut Self {
        self.access.write(TypeId::of::<T>());
        self
    }

    pub fn write_id(&mut self, type_id: TypeId) -> &mut Self {
        self.access.write(type_id);
        self
    }

    pub fn with<T: Component>(&mut self) -> &mut Self {
        self.merge_filter(FilterDesc::with::<T>());
        self
    }

    pub fn with_id(&mut self, type_id: TypeId) -> &mut Self {
        self.merge_filter(FilterDesc::With(type_id));
        self
    }

    pub fn without<T: Component>(&mut self) -> &mut Self {
        self.merge_filter(FilterDesc::without::<T>());
        self
    }

    pub fn without_id(&mut self, type_id: TypeId) -> &mut Self {
        self.merge_filter(FilterDesc::Without(type_id));
        self
    }

    pub fn filter(&mut self, filter: FilterDesc) -> &mut Self {
        self.merge_filter(filter);
        self
    }

    pub fn and(&mut self, f: impl FnOnce(&mut QueryBuilder)) -> &mut Self {
        let mut nested = QueryBuilder::new();
        f(&mut nested);
        self.merge_filter(FilterDesc::And(vec![self.filter.clone(), nested.filter]));
        self.access.merge(&nested.access);
        self
    }

    pub fn or(&mut self, f: impl FnOnce(&mut QueryBuilder)) -> &mut Self {
        let mut nested = QueryBuilder::new();
        f(&mut nested);
        self.merge_filter(FilterDesc::Or(vec![nested.filter]));
        self
    }

    pub fn optional(&mut self, f: impl FnOnce(&mut QueryBuilder)) -> &mut Self {
        let mut nested = QueryBuilder::new();
        f(&mut nested);
        // Optional terms are read-only extras; keep them in access but do not
        // require them for archetype matching (handled by not adding to required reads).
        self.access.merge(&nested.access);
        self
    }

    pub fn access(&self) -> &QueryAccess {
        &self.access
    }

    pub fn build(&self, world: &World) -> QueryState {
        let mut state = QueryState::new(self.access.clone(), self.filter.clone());
        state.update(world);
        state
    }

    pub fn build_fresh(&self, world: &World) -> QueryState {
        let mut state = QueryState::new(self.access.clone(), self.filter.clone());
        state.force_update(world);
        state
    }

    fn merge_filter(&mut self, filter: FilterDesc) {
        self.filter = match (&self.filter, filter) {
            (FilterDesc::All, new) => new,
            (existing, FilterDesc::All) => existing.clone(),
            (existing, new) => FilterDesc::And(vec![existing.clone(), new]),
        };
    }
}

impl Default for QueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}
