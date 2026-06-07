use std::any::{Any, TypeId};

use crate::component::Component;
use crate::entity::Entity;
use crate::query::access::QueryAccess;
use crate::world::World;

/// Runtime-checked read access to the components declared on a query.
pub struct FilteredEntityRef<'w> {
    world: &'w World,
    entity: Entity,
    arch_idx: usize,
    row: usize,
    access: QueryAccess,
}

impl<'w> FilteredEntityRef<'w> {
    pub(crate) fn new(world: &'w World, entity: Entity, arch_idx: usize, row: usize) -> Self {
        Self {
            world,
            entity,
            arch_idx,
            row,
            access: QueryAccess::new(),
        }
    }

    pub(crate) fn with_access(
        world: &'w World,
        entity: Entity,
        arch_idx: usize,
        row: usize,
        access: QueryAccess,
    ) -> Self {
        Self {
            world,
            entity,
            arch_idx,
            row,
            access,
        }
    }

    pub fn entity(&self) -> Entity {
        self.entity
    }

    pub fn get<T: Component>(&self) -> Option<&T> {
        let type_id = TypeId::of::<T>();
        if !self.access.can_read(type_id) {
            return None;
        }
        self.world
            .get_component_at(self.entity, type_id, self.arch_idx, self.row)?
            .downcast_ref::<T>()
    }

    pub fn get_by_id(&self, type_id: TypeId) -> Option<&dyn Any> {
        if !self.access.can_read(type_id) {
            return None;
        }
        self.world
            .get_component_at(self.entity, type_id, self.arch_idx, self.row)
    }
}

/// Runtime-checked mutable access to the components declared on a query.
pub struct FilteredEntityMut<'w> {
    world: *mut World,
    entity: Entity,
    arch_idx: usize,
    row: usize,
    access: QueryAccess,
    _marker: std::marker::PhantomData<&'w mut World>,
}

impl<'w> FilteredEntityMut<'w> {
    pub(crate) fn new(world: &'w World, entity: Entity, arch_idx: usize, row: usize) -> Self {
        Self {
            world: world as *const World as *mut World,
            entity,
            arch_idx,
            row,
            access: QueryAccess::new(),
            _marker: std::marker::PhantomData,
        }
    }

    pub(crate) fn with_access(
        world: &'w mut World,
        entity: Entity,
        arch_idx: usize,
        row: usize,
        access: QueryAccess,
    ) -> Self {
        Self {
            world,
            entity,
            arch_idx,
            row,
            access,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn entity(&self) -> Entity {
        self.entity
    }

    pub fn get<T: Component>(&self) -> Option<&T> {
        let type_id = TypeId::of::<T>();
        if !self.access.can_read(type_id) {
            return None;
        }
        // SAFETY: query iteration ensures disjoint archetype row access.
        let world = unsafe { &*self.world };
        world
            .get_component_at(self.entity, type_id, self.arch_idx, self.row)?
            .downcast_ref::<T>()
    }

    pub fn get_mut<T: Component>(&mut self) -> Option<&mut T> {
        let type_id = TypeId::of::<T>();
        if !self.access.can_write(type_id) {
            return None;
        }
        // SAFETY: query iteration ensures disjoint archetype row access.
        let world = unsafe { &*self.world };
        let ptr = unsafe {
            world.get_component_at_mut_ptr::<T>(type_id, self.arch_idx, self.row)?
        };
        Some(unsafe { &mut *ptr })
    }

    pub fn get_by_id(&self, type_id: TypeId) -> Option<&dyn Any> {
        if !self.access.can_read(type_id) {
            return None;
        }
        let world = unsafe { &*self.world };
        world.get_component_at(self.entity, type_id, self.arch_idx, self.row)
    }

    pub fn get_mut_by_id(&mut self, type_id: TypeId) -> Option<&mut dyn Any> {
        if !self.access.can_write(type_id) {
            return None;
        }
        let world = unsafe { &mut *self.world };
        world.get_component_at_mut(self.entity, type_id, self.arch_idx, self.row)
    }
}
