use std::any::{Any, TypeId};
use std::collections::HashMap;

/// A buffered message that systems can send and read.
///
/// Implement manually or with `#[derive(Event)]`.
pub trait Event: Send + Sync + 'static {}

/// Registers a derived event type with a world.
///
/// Implemented automatically by `#[derive(Event)]`.
pub trait RegisterEvent: Event {
    fn register(world: &mut crate::world::World);
}

/// Double-buffered event queue stored as a world resource.
#[derive(Debug)]
pub struct Events<E: Event> {
    buffers: [Vec<E>; 2],
    write_index: usize,
}

impl<E: Event> Default for Events<E> {
    fn default() -> Self {
        Self {
            buffers: [Vec::new(), Vec::new()],
            write_index: 0,
        }
    }
}

impl<E: Event> Events<E> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn send(&mut self, event: E) {
        self.buffers[self.write_index].push(event);
    }

    pub fn send_batch(&mut self, events: impl IntoIterator<Item = E>) {
        self.buffers[self.write_index].extend(events);
    }

    pub fn len(&self) -> usize {
        self.buffers[self.write_index].len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffers[self.write_index].is_empty()
    }

    pub fn update(&mut self) {
        let old_write = self.write_index;
        self.write_index = 1 - self.write_index;
        self.buffers[self.write_index].clear();
        self.buffers[old_write].clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = &E> {
        self.buffers[self.write_index].iter()
    }

    pub fn drain(&mut self) -> impl Iterator<Item = E> + use<E> {
        std::mem::take(&mut self.buffers[self.write_index]).into_iter()
    }

    pub fn clear(&mut self) {
        self.buffers[self.write_index].clear();
    }
}

/// Tracks how far an event reader has consumed the current buffer.
#[derive(Debug, Default, Clone, Copy)]
pub struct EventCursor {
    last: usize,
}

impl EventCursor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.last = 0;
    }
}

/// Read-only view into an event buffer.
pub struct EventReader<'w, E: Event> {
    events: &'w Events<E>,
    cursor: EventCursor,
}

impl<'w, E: Event> EventReader<'w, E> {
    pub fn new(events: &'w Events<E>) -> Self {
        Self {
            events,
            cursor: EventCursor::new(),
        }
    }

    pub fn with_cursor(events: &'w Events<E>, cursor: EventCursor) -> Self {
        Self { events, cursor }
    }

    pub fn cursor(&self) -> EventCursor {
        self.cursor
    }

    pub fn read(&mut self) -> &[E] {
        let buffer = &self.events.buffers[self.events.write_index];
        if self.cursor.last > buffer.len() {
            self.cursor.last = buffer.len();
        }
        &buffer[self.cursor.last..]
    }

    pub fn read_iter(&mut self) -> impl Iterator<Item = &E> {
        let buffer = &self.events.buffers[self.events.write_index];
        if self.cursor.last > buffer.len() {
            self.cursor.last = buffer.len();
        }
        let start = self.cursor.last;
        self.cursor.last = buffer.len();
        buffer[start..].iter()
    }

    pub fn is_empty(&self) -> bool {
        let buffer = &self.events.buffers[self.events.write_index];
        self.cursor.last >= buffer.len()
    }
}

/// Mutable handle for sending events.
pub struct EventWriter<'w, E: Event> {
    events: &'w mut Events<E>,
}

impl<'w, E: Event> EventWriter<'w, E> {
    pub fn new(events: &'w mut Events<E>) -> Self {
        Self { events }
    }

    pub fn send(&mut self, event: E) {
        self.events.send(event);
    }

    pub fn send_batch(&mut self, events: impl IntoIterator<Item = E>) {
        self.events.send_batch(events);
    }
}

pub(crate) struct EventRegistry {
    registered: Vec<TypeId>,
}

impl EventRegistry {
    pub fn new() -> Self {
        Self {
            registered: Vec::new(),
        }
    }

    pub fn register<E: Event>(&mut self) {
        let id = TypeId::of::<E>();
        if !self.registered.contains(&id) {
            self.registered.push(id);
        }
    }

    pub fn is_registered<E: Event>(&self) -> bool {
        self.registered.contains(&TypeId::of::<E>())
    }
}

impl Default for EventRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Type-erased event buffer for dynamic event dispatch.
pub(crate) trait EventBuffer: Send + Sync {
    fn update(&mut self);
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<E: Event> EventBuffer for Events<E> {
    fn update(&mut self) {
        Events::update(self);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub(crate) struct EventStorage {
    registry: EventRegistry,
    buffers: HashMap<TypeId, Box<dyn EventBuffer>>,
}

impl EventStorage {
    pub fn new() -> Self {
        Self {
            registry: EventRegistry::new(),
            buffers: HashMap::new(),
        }
    }

    pub fn register<E: Event>(&mut self) {
        self.registry.register::<E>();
        self.buffers
            .entry(TypeId::of::<E>())
            .or_insert_with(|| Box::new(Events::<E>::new()));
    }

    pub fn send<E: Event>(&mut self, event: E) -> bool {
        let Some(buffer) = self.buffers.get_mut(&TypeId::of::<E>()) else {
            return false;
        };
        buffer
            .as_any_mut()
            .downcast_mut::<Events<E>>()
            .expect("event buffer type mismatch")
            .send(event);
        true
    }

    pub fn events<E: Event>(&self) -> Option<&Events<E>> {
        self.buffers
            .get(&TypeId::of::<E>())?
            .as_any()
            .downcast_ref::<Events<E>>()
    }

    pub fn events_mut<E: Event>(&mut self) -> Option<&mut Events<E>> {
        self.buffers
            .get_mut(&TypeId::of::<E>())?
            .as_any_mut()
            .downcast_mut::<Events<E>>()
    }

    pub fn update_all(&mut self) {
        for buffer in self.buffers.values_mut() {
            buffer.update();
        }
    }

    pub fn is_registered<E: Event>(&self) -> bool {
        self.registry.is_registered::<E>()
    }
}

impl Default for EventStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    struct Damage {
        amount: u32,
    }
    impl Event for Damage {}

    #[test]
    fn events_send_and_read() {
        let mut events = Events::<Damage>::new();
        events.send(Damage { amount: 10 });
        events.send(Damage { amount: 5 });

        let mut reader = EventReader::new(&events);
        let batch: Vec<_> = reader.read_iter().cloned().collect();
        assert_eq!(
            batch,
            vec![Damage { amount: 10 }, Damage { amount: 5 }]
        );
        assert!(reader.is_empty());
    }

    #[test]
    fn events_update_clears_buffers() {
        let mut events = Events::<Damage>::new();
        events.send(Damage { amount: 1 });
        events.update();
        assert!(events.is_empty());
    }
}
