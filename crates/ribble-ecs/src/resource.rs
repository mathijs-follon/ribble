use std::any::{Any, TypeId};
use std::collections::HashMap;

/// Singleton data stored on the world, not attached to entities.
///
/// Implement manually or with `#[derive(Resource)]`.
pub trait Resource: Send + Sync + 'static {}

pub(crate) struct ResourceStorage {
    resources: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl ResourceStorage {
    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
        }
    }

    pub fn insert<T: Resource>(&mut self, resource: T) {
        self.resources.insert(TypeId::of::<T>(), Box::new(resource));
    }

    pub fn init<T: Resource + Default>(&mut self) -> &mut T {
        self.resources
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(T::default()))
            .downcast_mut::<T>()
            .expect("resource type mismatch")
    }

    pub fn init_with<T: Resource, F: FnOnce() -> T>(&mut self, f: F) -> &mut T {
        self.resources
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(f()))
            .downcast_mut::<T>()
            .expect("resource type mismatch")
    }

    pub fn remove<T: Resource>(&mut self) -> Option<T> {
        self.resources
            .remove(&TypeId::of::<T>())
            .and_then(|boxed| boxed.downcast::<T>().ok().map(|b| *b))
    }

    pub fn contains<T: Resource>(&self) -> bool {
        self.resources.contains_key(&TypeId::of::<T>())
    }

    pub fn get<T: Resource>(&self) -> Option<&T> {
        self.resources
            .get(&TypeId::of::<T>())?
            .downcast_ref::<T>()
    }

    pub fn get_mut<T: Resource>(&mut self) -> Option<&mut T> {
        self.resources
            .get_mut(&TypeId::of::<T>())?
            .downcast_mut::<T>()
    }
}

impl Default for ResourceStorage {
    fn default() -> Self {
        Self::new()
    }
}
