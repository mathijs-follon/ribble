use std::any::TypeId;
use std::marker::PhantomData;

use crate::component::Component;
use crate::entity::Entity;
use crate::query::filtered::{FilteredEntityMut, FilteredEntityRef};
use crate::world::World;

/// Describes what a query yields per matching entity.
pub trait QueryData: Send + Sync + 'static {
    type Item<'a>;

    fn access() -> crate::query::access::QueryAccess;
    fn fetch(world: &World, entity: Entity, row: usize, arch_idx: usize) -> Self::Item<'_>;
}

/// Query over entity ids only.
pub struct EntityData;

impl QueryData for EntityData {
    type Item<'a> = Entity;

    fn access() -> crate::query::access::QueryAccess {
        crate::query::access::QueryAccess::new()
    }

    fn fetch(_world: &World, entity: Entity, _row: usize, _arch_idx: usize) -> Entity {
        entity
    }
}

/// Read-only component access.
pub struct ReadData<T: Component>(PhantomData<T>);

impl<T: Component> QueryData for ReadData<T> {
    type Item<'a> = &'a T;

    fn access() -> crate::query::access::QueryAccess {
        let mut access = crate::query::access::QueryAccess::new();
        access.read(TypeId::of::<T>());
        access
    }

    fn fetch(world: &World, entity: Entity, row: usize, arch_idx: usize) -> &T {
        world
            .get_component_at(entity, TypeId::of::<T>(), arch_idx, row)
            .expect("query data missing component")
            .downcast_ref::<T>()
            .expect("query data type mismatch")
    }
}

/// Mutable component access.
pub struct WriteData<T: Component>(PhantomData<T>);

impl<T: Component> QueryData for WriteData<T> {
    type Item<'a> = &'a mut T;

    fn access() -> crate::query::access::QueryAccess {
        let mut access = crate::query::access::QueryAccess::new();
        access.write(TypeId::of::<T>());
        access
    }

    fn fetch(world: &World, _entity: Entity, row: usize, arch_idx: usize) -> &mut T {
        // SAFETY: mutable query iterators must not alias the same row.
        let ptr = unsafe {
            world
                .get_component_at_mut_ptr::<T>(TypeId::of::<T>(), arch_idx, row)
                .expect("query data missing component")
        };
        unsafe { &mut *ptr }
    }
}

/// Dynamically filtered entity reference.
pub struct FilteredEntityRefData;

impl QueryData for FilteredEntityRefData {
    type Item<'a> = FilteredEntityRef<'a>;

    fn access() -> crate::query::access::QueryAccess {
        crate::query::access::QueryAccess::new()
    }

    fn fetch(world: &World, entity: Entity, row: usize, arch_idx: usize) -> FilteredEntityRef<'_> {
        FilteredEntityRef::new(world, entity, arch_idx, row)
    }
}

/// Dynamically filtered mutable entity reference.
pub struct FilteredEntityMutData;

impl QueryData for FilteredEntityMutData {
    type Item<'a> = FilteredEntityMut<'a>;

    fn access() -> crate::query::access::QueryAccess {
        crate::query::access::QueryAccess::new()
    }

    fn fetch(world: &World, entity: Entity, row: usize, arch_idx: usize) -> FilteredEntityMut<'_> {
        FilteredEntityMut::new(world, entity, arch_idx, row)
    }
}

macro_rules! impl_query_data_tuple {
    ($($name:ident $(: $bound:ident)?),+) => {
        impl<$($name: QueryData),+> QueryData for ($($name,)+) {
            type Item<'a> = ($($name::Item<'a>,)+);

            fn access() -> crate::query::access::QueryAccess {
                let mut access = crate::query::access::QueryAccess::new();
                $(
                    access.merge(&<$name as QueryData>::access());
                )+
                access
            }

            fn fetch(world: &World, entity: Entity, row: usize, arch_idx: usize) -> Self::Item<'_> {
                ($(
                    <$name as QueryData>::fetch(world, entity, row, arch_idx),
                )+)
            }
        }
    };
}

impl_query_data_tuple!(A);
impl_query_data_tuple!(A, B);
impl_query_data_tuple!(A, B, C);
impl_query_data_tuple!(A, B, C, D);

/// Shorthand for `ReadData<T>`.
pub type Read<T> = ReadData<T>;

/// Shorthand for `WriteData<T>`.
pub type Write<T> = WriteData<T>;

