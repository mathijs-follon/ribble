use std::marker::PhantomData;

use crate::archetype::Archetype;
use crate::component::Component;

/// Marker filter: archetype must contain `T`.
pub struct With<T: Component>(PhantomData<T>);

/// Marker filter: archetype must not contain `T`.
pub struct Without<T: Component>(PhantomData<T>);

/// Marker filter: no additional constraints.
pub struct Allow;

pub trait QueryFilter: Send + Sync + 'static {
    fn to_desc() -> crate::query::access::FilterDesc;
    fn matches(archetype: &Archetype) -> bool;
}

impl QueryFilter for Allow {
    fn to_desc() -> crate::query::access::FilterDesc {
        crate::query::access::FilterDesc::All
    }

    fn matches(_archetype: &Archetype) -> bool {
        true
    }
}

impl<T: Component> QueryFilter for With<T> {
    fn to_desc() -> crate::query::access::FilterDesc {
        crate::query::access::FilterDesc::with::<T>()
    }

    fn matches(archetype: &Archetype) -> bool {
        archetype.has_component(std::any::TypeId::of::<T>())
    }
}

impl<T: Component> QueryFilter for Without<T> {
    fn to_desc() -> crate::query::access::FilterDesc {
        crate::query::access::FilterDesc::without::<T>()
    }

    fn matches(archetype: &Archetype) -> bool {
        !archetype.has_component(std::any::TypeId::of::<T>())
    }
}

/// Combine two filters with logical AND.
pub struct And<F1, F2>(PhantomData<(F1, F2)>);

impl<F1: QueryFilter, F2: QueryFilter> QueryFilter for And<F1, F2> {
    fn to_desc() -> crate::query::access::FilterDesc {
        crate::query::access::FilterDesc::and([F1::to_desc(), F2::to_desc()])
    }

    fn matches(archetype: &Archetype) -> bool {
        F1::matches(archetype) && F2::matches(archetype)
    }
}

/// Combine two filters with logical OR.
pub struct Or<F1, F2>(PhantomData<(F1, F2)>);

impl<F1: QueryFilter, F2: QueryFilter> QueryFilter for Or<F1, F2> {
    fn to_desc() -> crate::query::access::FilterDesc {
        crate::query::access::FilterDesc::or([F1::to_desc(), F2::to_desc()])
    }

    fn matches(archetype: &Archetype) -> bool {
        F1::matches(archetype) || F2::matches(archetype)
    }
}
