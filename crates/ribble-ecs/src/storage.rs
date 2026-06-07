use std::any::{Any, TypeId};
use std::collections::HashMap;

use crate::component::Component;
use crate::error::EcsError;

/// A type-erased, owned, contiguous array of one component type.
pub trait Column: Send + Sync {
    fn push_any(&mut self, value: Box<dyn Any>) -> Result<(), EcsError>;

    fn swap_remove_any(&mut self, row: usize) -> Box<dyn Any>;

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn get_any(&self, row: usize) -> Option<&dyn Any>;

    fn get_any_mut(&mut self, row: usize) -> Option<&mut dyn Any>;

    fn as_any_ref(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;
}

type ColumnFactory = Box<dyn Fn() -> Box<dyn Column> + Send + Sync>;

pub struct ComponentRegistry {
    factories: HashMap<TypeId, ColumnFactory>,
}

impl Default for ComponentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ComponentRegistry {
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
        }
    }

    pub fn register<T: Component + Send + Sync + 'static>(&mut self) {
        self.factories.insert(
            TypeId::of::<T>(),
            Box::new(|| Box::new(TypedColumn::<T> { data: Vec::new() })),
        );
    }

    pub fn create_column(&self, type_id: TypeId) -> Option<Box<dyn Column>> {
        self.factories.get(&type_id).map(|factory| factory())
    }

    pub fn is_registered(&self, type_id: TypeId) -> bool {
        self.factories.contains_key(&type_id)
    }
}

pub struct TypedColumn<T: Component> {
    pub data: Vec<T>,
}

impl<T: Component + 'static> TypedColumn<T> {
    pub fn as_slice(&self) -> &[T] {
        &self.data
    }

    pub fn as_slice_mut(&mut self) -> &mut [T] {
        &mut self.data
    }
}

impl<T: Component + Send + Sync + 'static> Column for TypedColumn<T> {
    fn push_any(&mut self, value: Box<dyn Any>) -> Result<(), EcsError> {
        self.data
            .push(*value.downcast::<T>().map_err(|_| EcsError::TypeMismatch)?);
        Ok(())
    }

    fn swap_remove_any(&mut self, row: usize) -> Box<dyn Any> {
        Box::new(self.data.swap_remove(row))
    }

    fn len(&self) -> usize {
        self.data.len()
    }

    fn get_any(&self, row: usize) -> Option<&dyn Any> {
        self.data.get(row).map(|component| component as &dyn Any)
    }

    fn get_any_mut(&mut self, row: usize) -> Option<&mut dyn Any> {
        self.data
            .get_mut(row)
            .map(|component| component as &mut dyn Any)
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub fn typed_column<T: Component + 'static>(col: &dyn Column) -> &TypedColumn<T> {
    col.as_any_ref()
        .downcast_ref::<TypedColumn<T>>()
        .expect("column type mismatch, registry bug")
}

pub fn typed_column_mut<T: Component + 'static>(col: &mut dyn Column) -> &mut TypedColumn<T> {
    col.as_any_mut()
        .downcast_mut::<TypedColumn<T>>()
        .expect("column type mismatch, registry bug")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Pos {
        x: f32,
    }
    impl Component for Pos {}

    #[test]
    fn typed_column_push_swap_remove_len() {
        let mut col = TypedColumn::<Pos> { data: Vec::new() };
        col.push_any(Box::new(Pos { x: 1.0 })).unwrap();
        col.push_any(Box::new(Pos { x: 2.0 })).unwrap();
        col.push_any(Box::new(Pos { x: 3.0 })).unwrap();
        assert_eq!(col.len(), 3);

        let removed = col.swap_remove_any(1);
        assert_eq!(*removed.downcast::<Pos>().unwrap(), Pos { x: 2.0 });
        assert_eq!(col.len(), 2);
        assert_eq!(col.as_slice(), &[Pos { x: 1.0 }, Pos { x: 3.0 }]);
    }
}
