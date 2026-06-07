use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EcsError {
    #[error("Component type mismatch during downcast")]
    TypeMismatch,

    #[error("Entity is not alive")]
    DeadEntity,

    #[error("Entity has no location in the world")]
    MissingLocation,

    #[error("Component type is not registered")]
    UnregisteredComponent,

    #[error("Entity does not have the requested component")]
    MissingComponent,
}
