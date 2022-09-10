use agner_actor::{Message, Seed};

pub trait SeedFactory<In, Out>: Send + Sync + 'static
where
	In: Message,
	Out: Seed,
{
	fn seed(&self, request: In) -> Out;
}

pub trait SeedFactoryMut<In, Out>: Send + Sync + 'static
where
	In: Message,
	Out: Seed,
{
	fn seed(&mut self, request: In) -> Out;
}

impl<F, In, Out> SeedFactory<In, Out> for F
where
	F: Fn(In) -> Out,
	F: Send + Sync + 'static,
	In: Message,
	Out: Seed,
{
	fn seed(&self, request: In) -> Out {
		self(request)
	}
}

impl<F, In, Out> SeedFactoryMut<In, Out> for F
where
	F: FnMut(In) -> Out,
	F: Send + Sync + 'static,
	In: Message,
	Out: Seed,
{
	fn seed(&mut self, request: In) -> Out {
		self(request)
	}
}
