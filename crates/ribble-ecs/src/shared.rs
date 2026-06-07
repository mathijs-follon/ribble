use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::world::World;

#[derive(Clone)]
pub struct SharedWorld(pub Arc<RwLock<World>>);

impl SharedWorld {
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(World::new())))
    }

    pub fn read(&self) -> RwLockReadGuard<'_, World> {
        self.0.read().expect("world read lock poisoned")
    }

    pub fn write(&self) -> RwLockWriteGuard<'_, World> {
        self.0.write().expect("world write lock poisoned")
    }
}

impl Default for SharedWorld {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component::Component;
    use crate::entity::Entity;
    use std::sync::Barrier;
    use std::thread;

    #[derive(Clone, Copy)]
    struct Counter;
    impl Component for Counter {}

    #[test]
    fn concurrent_read_and_write() {
        let world = SharedWorld::new();
        {
            let mut w = world.write();
            w.register_component::<Counter>();
            let e = w.spawn();
            w.insert_component(e, Counter).unwrap();
        }

        let barrier = Arc::new(Barrier::new(3));
        let world_read = world.clone();
        let b1 = barrier.clone();
        let t1 = thread::spawn(move || {
            b1.wait();
            let w = world_read.read();
            assert!(w.is_alive(Entity {
                index: 0,
                generation: 0
            }));
        });

        let world_read2 = world.clone();
        let b2 = barrier.clone();
        let t2 = thread::spawn(move || {
            b2.wait();
            let w = world_read2.read();
            assert_eq!(w.query_single::<Counter>().len(), 1);
        });

        barrier.wait();
        t1.join().unwrap();
        t2.join().unwrap();

        {
            let mut w = world.write();
            let e = w.spawn();
            w.insert_component(e, Counter).unwrap();
        }

        let w = world.read();
        assert_eq!(w.query_single::<Counter>().len(), 2);
    }
}
