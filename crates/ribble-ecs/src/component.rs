pub trait Component: Send + Sync + 'static {}

/// Registers a derived component type with a world.
///
/// Implemented automatically by `#[derive(Component)]`.
pub trait RegisterComponent: Component {
    fn register(world: &mut crate::world::World);
}
