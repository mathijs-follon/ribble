use ribble_ecs::{register_components, register_events, Entity, RegisterComponent, World};
use ribble_ecs_derive::{Component, Event, Resource};

#[derive(Component, Clone, Copy, Debug, PartialEq)]
struct Position {
    x: f32,
    y: f32,
}

#[derive(Component, Clone, Copy, Debug, PartialEq)]
#[component(no_register)]
struct Hidden;

#[derive(Event, Clone, Debug, PartialEq)]
struct Damaged {
    entity: Entity,
    amount: u32,
}

#[derive(Resource, Default, Debug, PartialEq)]
struct GameTime {
    seconds: f32,
}

#[test]
fn derive_component_registers_and_inserts() {
    let mut world = World::new();
    Position::register(&mut world);

    let entity = world.spawn();
    world
        .insert_component(entity, Position { x: 1.0, y: 2.0 })
        .unwrap();

    assert_eq!(world.get::<Position>(entity), Some(&Position { x: 1.0, y: 2.0 }));
}

#[test]
fn register_components_macro() {
    let mut world = World::new();
    register_components!(&mut world, Position);

    let entity = world.spawn();
    world
        .insert_component(entity, Position { x: 0.0, y: 0.0 })
        .unwrap();
    assert!(world.get::<Position>(entity).is_some());
}

#[test]
fn derive_event_registers_and_sends() {
    let mut world = World::new();
    register_events!(&mut world, Damaged);

    let entity = world.spawn();
    world.send_event(Damaged {
        entity,
        amount: 5,
    });

    let mut reader = world.event_reader::<Damaged>().unwrap();
    let events: Vec<_> = reader.read_iter().cloned().collect();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].amount, 5);
}

#[test]
fn derive_resource() {
    let mut world = World::new();
    world.insert_resource(GameTime { seconds: 1.5 });
    assert_eq!(
        world.resource::<GameTime>(),
        Some(&GameTime { seconds: 1.5 })
    );
}

#[test]
fn hidden_component_does_not_auto_register() {
    let mut world = World::new();
    let entity = world.spawn();
    assert!(world.insert_component(entity, Hidden).is_err());
}
