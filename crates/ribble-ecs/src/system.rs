use std::any::TypeId;

use crate::shared::SharedWorld;

#[derive(Default, Clone, Debug)]
pub struct AccessDescriptor {
    pub reads: Vec<TypeId>,
    pub writes: Vec<TypeId>,
    pub reads_world: bool,
    pub writes_world: bool,
}

pub trait System: Send + Sync {
    fn run(&mut self, world: &SharedWorld);
    fn access(&self) -> AccessDescriptor;
    fn name(&self) -> &str {
        "unnamed"
    }
}

struct FunctionSystem<F>(F);

impl<F> System for FunctionSystem<F>
where
    F: Fn(&SharedWorld) + Send + Sync,
{
    fn run(&mut self, world: &SharedWorld) {
        (self.0)(world);
    }

    fn access(&self) -> AccessDescriptor {
        AccessDescriptor::default()
    }
}

pub fn conflicts(a: &AccessDescriptor, b: &AccessDescriptor) -> bool {
    if a.writes_world || b.writes_world {
        return true;
    }

    for w in &a.writes {
        if b.reads.contains(w) || b.writes.contains(w) {
            return true;
        }
    }

    for w in &b.writes {
        if a.reads.contains(w) || a.writes.contains(w) {
            return true;
        }
    }

    false
}

pub struct Schedule {
    systems: Vec<Box<dyn System>>,
}

impl Schedule {
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
        }
    }

    pub fn add_system<S>(&mut self, system: S)
    where
        S: System + 'static,
    {
        self.systems.push(Box::new(system));
    }

    pub fn add_fn<F>(&mut self, f: F)
    where
        F: Fn(&SharedWorld) + Send + Sync + 'static,
    {
        self.systems.push(Box::new(FunctionSystem(f)));
    }

    pub fn run_sequential(&mut self, world: &SharedWorld) {
        for system in &mut self.systems {
            system.run(world);
        }
        world.write().update();
    }

    pub fn run_parallel(&mut self, world: &SharedWorld) {
        let _ = world;
        todo!("parallel scheduling requires a thread pool")
    }
}

impl Default for Schedule {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component::Component;
    use crate::shared::SharedWorld;
    use std::any::TypeId;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[derive(Clone, Copy)]
    struct Position {
        x: f32,
        y: f32,
    }
    impl Component for Position {}

    #[derive(Clone, Copy)]
    struct Velocity {
        dx: f32,
        dy: f32,
    }
    impl Component for Velocity {}

    #[test]
    fn conflict_detection() {
        let read_pos = AccessDescriptor {
            reads: vec![TypeId::of::<Position>()],
            ..Default::default()
        };
        let write_pos = AccessDescriptor {
            writes: vec![TypeId::of::<Position>()],
            ..Default::default()
        };
        let write_world = AccessDescriptor {
            writes_world: true,
            ..Default::default()
        };

        assert!(conflicts(&read_pos, &write_pos));
        assert!(!conflicts(&read_pos, &read_pos));
        assert!(conflicts(&read_pos, &write_world));
    }

    #[test]
    fn schedule_runs_systems_sequentially() {
        let counter = Arc::new(AtomicU32::new(0));
        let counter2 = counter.clone();

        let mut schedule = Schedule::new();
        schedule.add_fn(move |_| {
            counter2.fetch_add(1, Ordering::SeqCst);
        });
        let counter3 = counter.clone();
        schedule.add_fn(move |_| {
            counter3.fetch_add(10, Ordering::SeqCst);
        });

        let world = SharedWorld::new();
        schedule.run_sequential(&world);
        assert_eq!(counter.load(Ordering::SeqCst), 11);
    }

    #[test]
    fn movement_integration() {
        let world = SharedWorld::new();
        {
            let mut w = world.write();
            w.register_component::<Position>();
            w.register_component::<Velocity>();
            for _ in 0..3 {
                let e = w.spawn();
                w.insert_component(e, Position { x: 0.0, y: 0.0 }).unwrap();
                w.insert_component(e, Velocity { dx: 1.0, dy: 2.0 })
                    .unwrap();
            }
        }

        fn movement(world: &SharedWorld) {
            let mut w = world.write();
            let updates: Vec<_> = w
                .query_single::<Velocity>()
                .iter()
                .map(|(entity, vel)| (entity, *vel))
                .collect();
            for (entity, vel) in updates {
                if let Some(pos) = w.get_mut::<Position>(entity) {
                    pos.x += vel.dx;
                    pos.y += vel.dy;
                }
            }
        }

        let mut schedule = Schedule::new();
        schedule.add_fn(movement);
        for _ in 0..5 {
            schedule.run_sequential(&world);
        }

        let w = world.read();
        for (_, pos) in w.query_single::<Position>().iter() {
            assert_eq!(pos.x, 5.0);
            assert_eq!(pos.y, 10.0);
        }
    }
}
