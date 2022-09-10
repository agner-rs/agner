use std::sync::Arc;

pub trait Arg: Sized + Send + Sync + 'static {}
impl<T> Arg for T where T: Sized + Send + Sync + 'static {}

pub trait Seed: Send + Sync + 'static {
	type Value;
	fn value_opt(&self) -> Option<&Self::Value>;

	fn take(&mut self) -> Self;
}

pub trait NonEmptySeed: Seed {
	fn value(&self) -> &Self::Value;
}
pub trait ClonableSeed: Seed + Clone {}
impl<S> ClonableSeed for S where S: Seed + Clone {}

pub trait SeedMut: Seed {
	fn value_mut(&mut self) -> Option<&mut Self::Value>;
	fn take_value(&mut self) -> Option<Self::Value>;
}

#[derive(Debug)]
pub struct UniqueSeed<V>(Option<V>);

#[derive(Debug)]
pub struct SharedSeed<V>(Arc<V>);

impl<V> Clone for SharedSeed<V> {
	fn clone(&self) -> Self {
		Self(Arc::clone(&self.0))
	}
}

impl<V> From<V> for UniqueSeed<V> {
	fn from(v: V) -> Self {
		Self(Some(v))
	}
}

impl<V> From<V> for SharedSeed<V> {
	fn from(v: V) -> Self {
		Self(Arc::new(v))
	}
}

impl<V> Seed for UniqueSeed<V>
where
	V: Send + Sync + 'static,
{
	type Value = V;
	fn value_opt(&self) -> Option<&Self::Value> {
		self.0.as_ref()
	}

	fn take(&mut self) -> Self {
		Self(self.0.take())
	}
}

impl<V> SeedMut for UniqueSeed<V>
where
	V: Send + Sync + 'static,
{
	fn value_mut(&mut self) -> Option<&mut Self::Value> {
		self.0.as_mut()
	}
	fn take_value(&mut self) -> Option<Self::Value> {
		self.0.take()
	}
}

impl<V> Seed for SharedSeed<V>
where
	V: Send + Sync + 'static,
{
	type Value = V;

	fn value_opt(&self) -> Option<&Self::Value> {
		Some(self.0.as_ref())
	}

	fn take(&mut self) -> Self {
		Self(Arc::clone(&self.0))
	}
}
impl<V> NonEmptySeed for SharedSeed<V>
where
	V: Send + Sync + 'static,
{
	fn value(&self) -> &Self::Value {
		self.0.as_ref()
	}
}
