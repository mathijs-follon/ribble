mod allocator;
mod archetype;
mod component;
mod entity;
mod error;
mod event;
mod query;
mod resource;
mod shared;
mod storage;
mod system;
mod system_param;
mod world;
mod derive;

pub use allocator::EntityAllocator;
pub use archetype::{Archetype, ArchetypeId};
pub use component::{Component, RegisterComponent};
pub use entity::Entity;
pub use error::EcsError;
pub use event::{Event, EventCursor, EventReader, EventWriter, Events, RegisterEvent};
pub use query::{
    Allow, And, DynamicQueryIter, DynamicQueryIterMut, EntityData, FilterDesc, FilteredEntityMut,
    FilteredEntityRef, FilteredEntityMutData, FilteredEntityRefData, Or, Query, QueryAccess,
    QueryBuilder, QueryData, QueryFilter, QueryIter, QueryMut, QueryState, Read, ReadData, With,
    Without, Write, WriteData,
};
pub use resource::Resource;
pub use shared::SharedWorld;
pub use storage::{typed_column, typed_column_mut, Column, ComponentRegistry, TypedColumn};
pub use system::{conflicts, AccessDescriptor, Schedule, System};
pub use system_param::{Res, ResMut};
pub use world::{EntityLocation, World};

// Proc-macros must be exported for `#[derive(...)]`. They share names with the traits above;
// use fully-qualified paths such as `#[derive(ribble_ecs::Component)]` when both are needed.
pub use ribble_ecs_derive::{Component, Event, Resource};

/// Register multiple derived component types at once.
#[macro_export]
macro_rules! register_components {
    ($world:expr $(, $component:ty)*) => {
        $(<$component as $crate::RegisterComponent>::register($world);)*
    };
}

/// Register multiple derived event types at once.
#[macro_export]
macro_rules! register_events {
    ($world:expr $(, $event:ty)*) => {
        $(<$event as $crate::RegisterEvent>::register($world);)*
    };
}
