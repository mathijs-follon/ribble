use std::any::{Any, TypeId};
use std::collections::HashMap;

use crate::allocator::EntityAllocator;
use crate::archetype::{Archetype, ArchetypeId};
use crate::component::Component;
use crate::entity::Entity;
use crate::error::EcsError;
use crate::event::{Event, EventCursor, EventReader, EventStorage, EventWriter, Events};
use crate::query::{
    DynamicQueryIter, DynamicQueryIterMut, EntityData, Query, QueryBuilder, QueryData, QueryFilter,
    QueryState, ReadData,
};
use crate::resource::{Resource, ResourceStorage};
use crate::storage::ComponentRegistry;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EntityLocation {
    pub archetype_id: ArchetypeId,
    pub row: usize,
}

pub struct World {
    allocator: EntityAllocator,
    archetypes: Vec<Archetype>,
    archetype_index: HashMap<Vec<TypeId>, ArchetypeId>,
    locations: HashMap<Entity, EntityLocation>,
    resources: ResourceStorage,
    events: EventStorage,
    registry: ComponentRegistry,
}

impl World {
    pub fn new() -> Self {
        let mut world = Self {
            allocator: EntityAllocator::new(),
            archetypes: Vec::new(),
            archetype_index: HashMap::new(),
            locations: HashMap::new(),
            resources: ResourceStorage::new(),
            events: EventStorage::new(),
            registry: ComponentRegistry::new(),
        };

        world.get_or_create_archetype(vec![]);
        world
    }

    pub fn register_component<T: Component + Send + Sync + 'static>(&mut self) {
        self.registry.register::<T>();
    }

    pub fn is_alive(&self, entity: Entity) -> bool {
        self.allocator.is_alive(entity)
    }

    pub fn spawn(&mut self) -> Entity {
        let entity = self.allocator.alloc();
        let archetype = &mut self.archetypes[0];
        let row = archetype.allocate_row(entity);
        self.locations
            .insert(entity, EntityLocation { archetype_id: 0, row });
        self.debug_assert_invariants();
        entity
    }

    pub fn despawn(&mut self, entity: Entity) -> Result<(), EcsError> {
        if !self.allocator.is_alive(entity) {
            return Err(EcsError::DeadEntity);
        }

        let Some(loc) = self.locations.remove(&entity) else {
            return Err(EcsError::MissingLocation);
        };

        let src_id = loc.archetype_id as usize;
        let src_row = loc.row;
        let src_type_ids = self.archetypes[src_id].type_signature.clone();

        for tid in &src_type_ids {
            self.archetypes[src_id]
                .columns
                .get_mut(tid)
                .unwrap()
                .swap_remove_any(src_row);
        }

        let displaced = self.archetypes[src_id].free_row(src_row);
        if let Some(displaced_entity) = displaced {
            self.locations.get_mut(&displaced_entity).unwrap().row = src_row;
        }

        self.allocator.free(entity);
        self.debug_assert_invariants();
        Ok(())
    }

    pub fn insert_component<T: Component + Send + Sync + 'static>(
        &mut self,
        entity: Entity,
        component: T,
    ) -> Result<(), EcsError> {
        if !self.allocator.is_alive(entity) {
            return Err(EcsError::DeadEntity);
        }

        let type_id = TypeId::of::<T>();
        if !self.registry.is_registered(type_id) {
            return Err(EcsError::UnregisteredComponent);
        }

        let loc = self
            .locations
            .get(&entity)
            .copied()
            .ok_or(EcsError::MissingLocation)?;

        let src_id = loc.archetype_id;
        let dst_id = self.resolve_add_edge(src_id, type_id)?;

        if src_id == dst_id {
            let arch = &mut self.archetypes[src_id as usize];
            let col = arch.columns.get_mut(&type_id).unwrap();
            *col.get_any_mut(loc.row)
                .ok_or(EcsError::MissingComponent)?
                .downcast_mut::<T>()
                .ok_or(EcsError::TypeMismatch)? = component;
            self.debug_assert_invariants();
            return Ok(());
        }

        let src_type_ids = self.archetypes[src_id as usize].type_signature.clone();
        let src_row = loc.row;

        for tid in &src_type_ids {
            let value = self.archetypes[src_id as usize]
                .columns
                .get_mut(tid)
                .unwrap()
                .swap_remove_any(src_row);

            self.archetypes[dst_id as usize]
                .columns
                .get_mut(tid)
                .unwrap()
                .push_any(value)?;
        }

        self.archetypes[dst_id as usize]
            .columns
            .get_mut(&type_id)
            .unwrap()
            .push_any(Box::new(component))?;

        let displaced = self.archetypes[src_id as usize].free_row(src_row);
        let dst_row = self.archetypes[dst_id as usize].allocate_row(entity);

        if let Some(displaced_entity) = displaced {
            self.locations.get_mut(&displaced_entity).unwrap().row = src_row;
        }

        *self.locations.get_mut(&entity).unwrap() =
            EntityLocation { archetype_id: dst_id, row: dst_row };

        self.debug_assert_invariants();
        Ok(())
    }

    pub fn remove_component<T: Component + Send + Sync + 'static>(
        &mut self,
        entity: Entity,
    ) -> Result<T, EcsError> {
        if !self.allocator.is_alive(entity) {
            return Err(EcsError::DeadEntity);
        }

        let type_id = TypeId::of::<T>();
        let loc = self
            .locations
            .get(&entity)
            .copied()
            .ok_or(EcsError::MissingLocation)?;

        let src_id = loc.archetype_id;
        if !self.archetypes[src_id as usize].has_component(type_id) {
            return Err(EcsError::MissingComponent);
        }

        let dst_id = self.resolve_remove_edge(src_id, type_id)?;
        let src_row = loc.row;

        if src_id == dst_id {
            return Err(EcsError::MissingComponent);
        }

        let src_type_ids = self.archetypes[src_id as usize].type_signature.clone();
        let mut removed: Option<T> = None;

        for tid in &src_type_ids {
            let value = self.archetypes[src_id as usize]
                .columns
                .get_mut(tid)
                .unwrap()
                .swap_remove_any(src_row);

            if *tid == type_id {
                removed = Some(*value.downcast::<T>().map_err(|_| EcsError::TypeMismatch)?);
            } else {
                self.archetypes[dst_id as usize]
                    .columns
                    .get_mut(tid)
                    .unwrap()
                    .push_any(value)?;
            }
        }

        let displaced = self.archetypes[src_id as usize].free_row(src_row);
        let dst_row = self.archetypes[dst_id as usize].allocate_row(entity);

        if let Some(displaced_entity) = displaced {
            self.locations.get_mut(&displaced_entity).unwrap().row = src_row;
        }

        *self.locations.get_mut(&entity).unwrap() =
            EntityLocation { archetype_id: dst_id, row: dst_row };

        self.debug_assert_invariants();
        removed.ok_or(EcsError::MissingComponent)
    }

    pub fn get<T: Component + Send + Sync + 'static>(&self, entity: Entity) -> Option<&T> {
        if !self.allocator.is_alive(entity) {
            return None;
        }

        let loc = self.locations.get(&entity)?;
        let type_id = TypeId::of::<T>();
        let arch = &self.archetypes[loc.archetype_id as usize];
        if !arch.has_component(type_id) {
            return None;
        }
        arch.columns
            .get(&type_id)?
            .get_any(loc.row)?
            .downcast_ref::<T>()
    }

    pub fn get_mut<T: Component + Send + Sync + 'static>(
        &mut self,
        entity: Entity,
    ) -> Option<&mut T> {
        if !self.allocator.is_alive(entity) {
            return None;
        }

        let loc = self.locations.get(&entity).copied()?;
        let type_id = TypeId::of::<T>();
        let arch = &mut self.archetypes[loc.archetype_id as usize];
        if !arch.has_component(type_id) {
            return None;
        }
        arch.columns
            .get_mut(&type_id)?
            .get_any_mut(loc.row)?
            .downcast_mut::<T>()
    }

    // --- Query API ---

    pub fn query_builder(&self) -> QueryBuilder {
        QueryBuilder::new()
    }

    pub fn query<D: QueryData, F: QueryFilter>(&self) -> Query<'_, D, F> {
        let mut builder = QueryBuilder::new();
        let access = D::access();
        for id in &access.reads {
            builder.read_id(*id);
        }
        for id in &access.writes {
            builder.write_id(*id);
        }
        builder.filter(F::to_desc());
        let state = builder.build(self);
        Query::new(self, state)
    }

    pub fn query_dynamic<'a>(&'a self, state: &'a QueryState) -> DynamicQueryIter<'a> {
        DynamicQueryIter::new(self, state)
    }

    pub fn query_dynamic_mut(&mut self, state: QueryState) -> DynamicQueryIterMut<'_> {
        DynamicQueryIterMut::new(self, state)
    }

    /// Typed read query over one component (backward-compatible helper).
    pub fn query_single<T: Component>(&self) -> Query<'_, (EntityData, ReadData<T>)> {
        self.query::<(EntityData, ReadData<T>), crate::query::Allow>()
    }

    /// Typed read query over two components (backward-compatible helper).
    pub fn query_pair<A: Component, B: Component>(
        &self,
    ) -> Query<'_, (EntityData, ReadData<A>, ReadData<B>)> {
        self.query::<(EntityData, ReadData<A>, ReadData<B>), crate::query::Allow>()
    }

    /// Typed mutable query over one component (backward-compatible helper).
    pub fn query_mut<T: Component>(&mut self) -> DynamicQueryIterMut<'_> {
        let state = QueryBuilder::new().write::<T>().build(self);
        DynamicQueryIterMut::new(self, state)
    }

    // --- Resource API ---

    pub fn insert_resource<T: Resource>(&mut self, resource: T) {
        self.resources.insert(resource);
    }

    pub fn init_resource<T: Resource + Default>(&mut self) -> &mut T {
        self.resources.init::<T>()
    }

    pub fn init_resource_with<T: Resource, F: FnOnce() -> T>(&mut self, f: F) -> &mut T {
        self.resources.init_with(f)
    }

    pub fn remove_resource<T: Resource>(&mut self) -> Option<T> {
        self.resources.remove::<T>()
    }

    pub fn contains_resource<T: Resource>(&self) -> bool {
        self.resources.contains::<T>()
    }

    pub fn resource<T: Resource>(&self) -> Option<&T> {
        self.resources.get::<T>()
    }

    pub fn resource_mut<T: Resource>(&mut self) -> Option<&mut T> {
        self.resources.get_mut::<T>()
    }

    /// Backward-compatible alias.
    pub fn get_resource<T: Resource>(&self) -> Option<&T> {
        self.resource::<T>()
    }

    /// Backward-compatible alias.
    pub fn get_resource_mut<T: Resource>(&mut self) -> Option<&mut T> {
        self.resource_mut::<T>()
    }

    // --- Event API ---

    pub fn add_event<E: Event>(&mut self) {
        self.events.register::<E>();
    }

    pub fn send_event<E: Event>(&mut self, event: E) -> bool {
        if !self.events.is_registered::<E>() {
            self.add_event::<E>();
        }
        self.events.send(event)
    }

    pub fn events<E: Event>(&self) -> Option<&Events<E>> {
        self.events.events::<E>()
    }

    pub fn events_mut<E: Event>(&mut self) -> Option<&mut Events<E>> {
        self.events.events_mut::<E>()
    }

    pub fn event_reader<E: Event>(&self) -> Option<EventReader<'_, E>> {
        self.events().map(EventReader::new)
    }

    pub fn event_reader_with_cursor<E: Event>(
        &self,
        cursor: EventCursor,
    ) -> Option<EventReader<'_, E>> {
        self.events()
            .map(|events| EventReader::with_cursor(events, cursor))
    }

    pub fn event_writer<E: Event>(&mut self) -> Option<EventWriter<'_, E>> {
        if !self.events.is_registered::<E>() {
            self.add_event::<E>();
        }
        self.events_mut::<E>().map(EventWriter::new)
    }

    /// Advance all event buffers. Call once per frame after systems run.
    pub fn update(&mut self) {
        self.events.update_all();
    }

    // --- Internal accessors for query module ---

    pub(crate) fn archetypes(&self) -> &[Archetype] {
        &self.archetypes
    }

    pub(crate) fn get_component_at(
        &self,
        _entity: Entity,
        type_id: TypeId,
        arch_idx: usize,
        row: usize,
    ) -> Option<&dyn Any> {
        let arch = self.archetypes.get(arch_idx)?;
        arch.columns.get(&type_id)?.get_any(row)
    }

    pub(crate) fn get_component_at_mut(
        &mut self,
        _entity: Entity,
        type_id: TypeId,
        arch_idx: usize,
        row: usize,
    ) -> Option<&mut dyn Any> {
        let arch = self.archetypes.get_mut(arch_idx)?;
        arch.columns.get_mut(&type_id)?.get_any_mut(row)
    }

    /// # Safety
    /// Caller must ensure no aliasing mutable references to the same component slot.
    pub(crate) unsafe fn get_component_at_mut_ptr<T: Component>(
        &self,
        type_id: TypeId,
        arch_idx: usize,
        row: usize,
    ) -> Option<*mut T> {
        use crate::storage::typed_column;
        let arch = self.archetypes.get(arch_idx)?;
        let col = arch.columns.get(&type_id)?;
        let typed = typed_column::<T>(col.as_ref());
        if row >= typed.data.len() {
            return None;
        }
        Some(unsafe { typed.data.as_ptr().add(row).cast_mut() })
    }

    fn resolve_add_edge(
        &mut self,
        src_id: ArchetypeId,
        type_id: TypeId,
    ) -> Result<ArchetypeId, EcsError> {
        if let Some(&cached) = self.archetypes[src_id as usize].add_edges.get(&type_id) {
            return Ok(cached);
        }

        let mut new_sig = self.archetypes[src_id as usize].type_signature.clone();
        if !new_sig.contains(&type_id) {
            new_sig.push(type_id);
            new_sig.sort_unstable();
        }

        let dst = self.get_or_create_archetype(new_sig);
        self.archetypes[src_id as usize]
            .add_edges
            .insert(type_id, dst);
        Ok(dst)
    }

    fn resolve_remove_edge(
        &mut self,
        src_id: ArchetypeId,
        type_id: TypeId,
    ) -> Result<ArchetypeId, EcsError> {
        if let Some(&cached) = self.archetypes[src_id as usize].remove_edges.get(&type_id) {
            return Ok(cached);
        }

        let mut new_sig = self.archetypes[src_id as usize]
            .type_signature
            .clone();
        if let Ok(pos) = new_sig.binary_search(&type_id) {
            new_sig.remove(pos);
        }

        let dst = self.get_or_create_archetype(new_sig);
        self.archetypes[src_id as usize]
            .remove_edges
            .insert(type_id, dst);
        Ok(dst)
    }

    fn get_or_create_archetype(&mut self, mut signature: Vec<TypeId>) -> ArchetypeId {
        signature.sort_unstable();
        signature.dedup();

        if let Some(&id) = self.archetype_index.get(&signature) {
            return id;
        }

        let id = self.archetypes.len() as ArchetypeId;
        let mut columns = HashMap::new();
        for &type_id in &signature {
            let column = self
                .registry
                .create_column(type_id)
                .unwrap_or_else(|| panic!("unregistered component type: {type_id:?}"));
            columns.insert(type_id, column);
        }

        self.archetypes
            .push(Archetype::new(id, signature.clone(), columns));
        self.archetype_index.insert(signature, id);
        id
    }

    #[cfg(debug_assertions)]
    fn debug_assert_invariants(&self) {
        for arch in &self.archetypes {
            let len = arch.entities.len();
            for col in arch.columns.values() {
                assert_eq!(
                    col.len(),
                    len,
                    "column length mismatch in archetype {}",
                    arch.id
                );
            }

            let mut sorted = arch.type_signature.clone();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(
                arch.type_signature, sorted,
                "archetype {} signature is not sorted/deduped",
                arch.id
            );
        }

        for (entity, loc) in &self.locations {
            assert!(
                self.allocator.is_alive(*entity),
                "dead entity {:?} in locations",
                entity
            );
            let arch = &self.archetypes[loc.archetype_id as usize];
            assert!(
                loc.row < arch.entities.len(),
                "row out of bounds for entity {:?}",
                entity
            );
            assert_eq!(
                arch.entities[loc.row], *entity,
                "entity at row does not match location"
            );
        }
    }

    #[cfg(not(debug_assertions))]
    fn debug_assert_invariants(&self) {}
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::With;
    use crate::register_events;
    use std::any::TypeId;

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Position {
        x: f32,
        y: f32,
    }
    impl Component for Position {}

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Velocity {
        dx: f32,
        dy: f32,
    }
    impl Component for Velocity {}

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Health(u32);
    impl Component for Health {}

    #[derive(Clone, Debug, PartialEq)]
    struct Damaged {
        entity: Entity,
        amount: u32,
    }
    impl crate::event::Event for Damaged {}
    impl crate::event::RegisterEvent for Damaged {
        fn register(world: &mut World) {
            world.add_event::<Self>();
        }
    }

    #[derive(Default, Debug, PartialEq, Eq)]
    struct Tick(u32);
    impl crate::resource::Resource for Tick {}

    fn test_world() -> World {
        let mut world = World::new();
        world.register_component::<Position>();
        world.register_component::<Velocity>();
        world.register_component::<Health>();
        world
    }

    #[test]
    fn spawn_despawn_respawn_no_collisions() {
        let mut world = test_world();
        let mut spawned = Vec::new();
        for _ in 0..100 {
            spawned.push(world.spawn());
        }

        let to_despawn: Vec<_> = spawned.drain(50..).collect();
        for entity in to_despawn {
            world.despawn(entity).unwrap();
            assert!(!world.is_alive(entity));
        }

        for _ in 0..50 {
            let e = world.spawn();
            assert!(world.is_alive(e));
            spawned.push(e);
        }

        let mut indices = spawned.iter().map(|e| e.index).collect::<Vec<_>>();
        indices.sort_unstable();
        indices.dedup();
        assert_eq!(indices.len(), spawned.len());
    }

    #[test]
    fn get_or_create_archetype_reuses_signature() {
        let mut world = test_world();
        let entities: Vec<_> = (0..3).map(|_| world.spawn()).collect();
        for (i, &e) in entities.iter().enumerate() {
            world
                .insert_component(e, Position { x: i as f32, y: 0.0 })
                .unwrap();
        }
        assert_eq!(world.query_single::<Position>().len(), 3);
    }

    #[test]
    fn insert_and_remove_migrate_correctly() {
        let mut world = test_world();
        let e = world.spawn();

        world
            .insert_component(e, Position { x: 1.0, y: 2.0 })
            .unwrap();
        assert_eq!(world.get::<Position>(e), Some(&Position { x: 1.0, y: 2.0 }));

        world
            .insert_component(e, Velocity { dx: 0.5, dy: -1.0 })
            .unwrap();
        assert_eq!(world.get::<Velocity>(e), Some(&Velocity { dx: 0.5, dy: -1.0 }));
        assert_eq!(world.get::<Position>(e), Some(&Position { x: 1.0, y: 2.0 }));

        let pos = world.remove_component::<Position>(e).unwrap();
        assert_eq!(pos, Position { x: 1.0, y: 2.0 });
        assert!(world.get::<Position>(e).is_none());
        assert_eq!(world.get::<Velocity>(e), Some(&Velocity { dx: 0.5, dy: -1.0 }));
    }

    #[test]
    fn query_returns_matching_entities_only() {
        let mut world = test_world();

        for i in 0..10 {
            let e = world.spawn();
            world
                .insert_component(e, Position { x: i as f32, y: 0.0 })
                .unwrap();
            world
                .insert_component(e, Velocity { dx: 1.0, dy: 0.0 })
                .unwrap();
        }

        for i in 0..5 {
            let e = world.spawn();
            world
                .insert_component(e, Position { x: 100.0 + i as f32, y: 0.0 })
                .unwrap();
        }

        let count = world
            .query::<(EntityData, ReadData<Position>, ReadData<Velocity>), crate::query::Allow>()
            .len();
        assert_eq!(count, 10);

        let pos_only = world.query_single::<Position>().len();
        assert_eq!(pos_only, 15);
    }

    #[test]
    fn swap_remove_updates_displaced_location() {
        let mut world = test_world();
        let e0 = world.spawn();
        let e1 = world.spawn();
        let e2 = world.spawn();

        for e in [e0, e1, e2] {
            world
                .insert_component(e, Position { x: e.index as f32, y: 0.0 })
                .unwrap();
        }

        world.despawn(e1).unwrap();
        assert!(!world.is_alive(e1));
        assert_eq!(world.get::<Position>(e2), Some(&Position { x: 2.0, y: 0.0 }));
    }

    #[test]
    fn dynamic_query_builder_with_without() {
        let mut world = test_world();
        let with_vel = world.spawn();
        world
            .insert_component(with_vel, Position { x: 1.0, y: 0.0 })
            .unwrap();
        world
            .insert_component(with_vel, Velocity { dx: 1.0, dy: 0.0 })
            .unwrap();

        let without_vel = world.spawn();
        world
            .insert_component(without_vel, Position { x: 2.0, y: 0.0 })
            .unwrap();

        let state = world
            .query_builder()
            .read::<Position>()
            .with::<Velocity>()
            .build(&world);
        assert_eq!(world.query_dynamic(&state).count(), 1);

        let state = world
            .query_builder()
            .read::<Position>()
            .without::<Velocity>()
            .build(&world);
        assert_eq!(world.query_dynamic(&state).count(), 1);
    }

    #[test]
    fn filtered_entity_ref_dynamic_access() {
        let mut world = test_world();
        let e = world.spawn();
        world
            .insert_component(e, Position { x: 3.0, y: 4.0 })
            .unwrap();

        let state = world.query_builder().read::<Position>().build(&world);
        let entity_ref = world.query_dynamic(&state).next().unwrap();
        assert_eq!(entity_ref.get::<Position>(), Some(&Position { x: 3.0, y: 4.0 }));
        assert!(entity_ref.get::<Velocity>().is_none());
        assert!(entity_ref.get_by_id(TypeId::of::<Velocity>()).is_none());
    }

    #[test]
    fn typed_query_with_filter() {
        let mut world = test_world();
        let e = world.spawn();
        world
            .insert_component(e, Position { x: 1.0, y: 0.0 })
            .unwrap();
        world
            .insert_component(e, Velocity { dx: 1.0, dy: 0.0 })
            .unwrap();

        let query = world.query::<(EntityData, ReadData<Position>), With<Velocity>>();
        assert_eq!(query.len(), 1);
    }

    #[test]
    fn resources_and_events() {
        let mut world = World::new();
        world.insert_resource(Tick(42));
        assert_eq!(world.resource::<Tick>(), Some(&Tick(42)));

        register_events!(&mut world, Damaged);
        world.send_event(Damaged {
            entity: Entity { index: 0, generation: 0 },
            amount: 10,
        });

        let mut reader = world.event_reader::<Damaged>().unwrap();
        let events: Vec<_> = reader.read_iter().cloned().collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].amount, 10);

        world.update();
        let reader = world.event_reader::<Damaged>().unwrap();
        assert!(reader.is_empty());
    }
}
