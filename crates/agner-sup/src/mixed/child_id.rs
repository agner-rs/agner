use std::fmt;
use std::hash::Hash;

/// A trait marking every type that can be used to identify a child in a
/// [`SupSpec`](crate::mixed::sup_spec::SupSpec)
pub trait ChildID:
    fmt::Debug + Unpin + Clone + Copy + PartialEq + Eq + Hash + Send + Sync + 'static
{
}

impl<T> ChildID for T where
    T: fmt::Debug + Unpin + Clone + Copy + PartialEq + Eq + Hash + Send + Sync + 'static
{
}
