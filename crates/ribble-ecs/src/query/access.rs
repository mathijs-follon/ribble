use std::any::TypeId;

/// Describes which component columns a query may read or write.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QueryAccess {
    pub reads: Vec<TypeId>,
    pub writes: Vec<TypeId>,
}

impl QueryAccess {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read(&mut self, type_id: TypeId) {
        if !self.reads.contains(&type_id) && !self.writes.contains(&type_id) {
            self.reads.push(type_id);
        }
    }

    pub fn write(&mut self, type_id: TypeId) {
        if self.reads.contains(&type_id) {
            self.reads.retain(|id| *id != type_id);
        }
        if !self.writes.contains(&type_id) {
            self.writes.push(type_id);
        }
    }

    pub fn can_read(&self, type_id: TypeId) -> bool {
        self.reads.contains(&type_id) || self.writes.contains(&type_id)
    }

    pub fn can_write(&self, type_id: TypeId) -> bool {
        self.writes.contains(&type_id)
    }

    pub fn merge(&mut self, other: &QueryAccess) {
        for &id in &other.reads {
            self.read(id);
        }
        for &id in &other.writes {
            self.write(id);
        }
    }
}

/// Archetype filter expression evaluated at runtime.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum FilterDesc {
    #[default]
    All,
    With(TypeId),
    Without(TypeId),
    And(Vec<FilterDesc>),
    Or(Vec<FilterDesc>),
}

impl FilterDesc {
    pub fn with<T: crate::component::Component>() -> Self {
        Self::With(TypeId::of::<T>())
    }

    pub fn without<T: crate::component::Component>() -> Self {
        Self::Without(TypeId::of::<T>())
    }

    pub fn and(filters: impl IntoIterator<Item = FilterDesc>) -> Self {
        Self::And(filters.into_iter().collect())
    }

    pub fn or(filters: impl IntoIterator<Item = FilterDesc>) -> Self {
        Self::Or(filters.into_iter().collect())
    }
}

pub fn matches_filter(signature: &[TypeId], filter: &FilterDesc) -> bool {
    match filter {
        FilterDesc::All => true,
        FilterDesc::With(type_id) => signature.binary_search(type_id).is_ok(),
        FilterDesc::Without(type_id) => signature.binary_search(type_id).is_err(),
        FilterDesc::And(filters) => filters
            .iter()
            .all(|f| matches_filter(signature, f)),
        FilterDesc::Or(filters) => filters
            .iter()
            .any(|f| matches_filter(signature, f)),
    }
}

pub fn matches_access(signature: &[TypeId], access: &QueryAccess) -> bool {
    access
        .reads
        .iter()
        .chain(access.writes.iter())
        .all(|type_id| signature.binary_search(type_id).is_ok())
}
