use crate::event::{Event, EventCursor, EventReader, EventWriter, Events};
use crate::resource::Resource;
use crate::world::World;

/// Read-only resource access (Bevy `Res<T>`).
pub struct Res<'w, T: Resource> {
    value: &'w T,
}

impl<'w, T: Resource> Res<'w, T> {
    pub fn new(value: &'w T) -> Self {
        Self { value }
    }
}

impl<T: Resource> std::ops::Deref for Res<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

/// Mutable resource access (Bevy `ResMut<T>`).
pub struct ResMut<'w, T: Resource> {
    value: &'w mut T,
}

impl<'w, T: Resource> ResMut<'w, T> {
    pub fn new(value: &'w mut T) -> Self {
        Self { value }
    }
}

impl<T: Resource> std::ops::Deref for ResMut<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<T: Resource> std::ops::DerefMut for ResMut<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.value
    }
}

impl World {
    pub fn res<T: Resource>(&self) -> Option<Res<'_, T>> {
        self.resource::<T>().map(Res::new)
    }

    pub fn res_mut<T: Resource>(&mut self) -> Option<ResMut<'_, T>> {
        self.resource_mut::<T>().map(ResMut::new)
    }

    pub fn event_reader_param<E: Event>(&self) -> Option<EventReader<'_, E>> {
        self.event_reader::<E>()
    }

    pub fn event_reader_param_with_cursor<E: Event>(
        &self,
        cursor: EventCursor,
    ) -> Option<EventReader<'_, E>> {
        self.event_reader_with_cursor::<E>(cursor)
    }

    pub fn event_writer_param<E: Event>(&mut self) -> Option<EventWriter<'_, E>> {
        self.event_writer::<E>()
    }

    pub fn init_events<E: Event>(&mut self) -> &mut Events<E> {
        self.add_event::<E>();
        self.events_mut::<E>().expect("event just registered")
    }
}
