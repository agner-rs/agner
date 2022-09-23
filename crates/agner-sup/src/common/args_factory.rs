use std::marker::PhantomData;

#[cfg(test)]
mod tests;

pub trait ArgsFactory: Send + Sync + 'static {
    type Input: Send + Sync + 'static;
    type Output: Send + Sync + 'static;

    fn make_args(&mut self, args: Self::Input) -> Self::Output;
}

pub fn clone<T>(value: T) -> Box<dyn ArgsFactory<Input = (), Output = T>>
where
    CloneValue<T>: ArgsFactory<Input = (), Output = T>,
{
    let af = CloneValue(value);
    Box::new(af)
}

pub fn call<F, Out>(func: F) -> Box<dyn ArgsFactory<Input = (), Output = Out>>
where
    Call<F, Out>: ArgsFactory<Input = (), Output = Out>,
{
    let af = Call(func, Default::default());
    Box::new(af)
}

pub fn map<F, In, Out>(func: F) -> Box<dyn ArgsFactory<Input = In, Output = Out>>
where
    Map<F, In, Out>: ArgsFactory<Input = In, Output = Out>,
{
    let af = Map(func, Default::default());
    Box::new(af)
}

#[derive(Debug)]
pub struct CloneValue<T>(T);

#[derive(Debug)]
pub struct Map<F, In, Out>(F, PhantomData<(In, Out)>);

#[derive(Debug)]
pub struct Call<F, Out>(F, PhantomData<Out>);

impl<In, Out> ArgsFactory for Box<dyn ArgsFactory<Input = In, Output = Out>>
where
    In: Send + Sync + 'static,
    Out: Send + Sync + 'static,
{
    type Input = In;
    type Output = Out;

    fn make_args(&mut self, args: Self::Input) -> Self::Output {
        self.as_mut().make_args(args)
    }
}

impl<T> ArgsFactory for CloneValue<T>
where
    T: Clone + Send + Sync + 'static,
{
    type Input = ();
    type Output = T;

    fn make_args(&mut self, (): Self::Input) -> Self::Output {
        self.0.to_owned()
    }
}

impl<F, Out> ArgsFactory for Call<F, Out>
where
    F: FnMut() -> Out,
    F: Send + Sync + 'static,
    Out: Send + Sync + 'static,
{
    type Input = ();
    type Output = Out;

    fn make_args(&mut self, (): Self::Input) -> Self::Output {
        (self.0)()
    }
}

impl<F, In, Out> ArgsFactory for Map<F, In, Out>
where
    F: FnMut(In) -> Out,
    F: Send + Sync + 'static,
    In: Send + Sync + 'static,
    Out: Send + Sync + 'static,
{
    type Input = In;
    type Output = Out;

    fn make_args(&mut self, input: Self::Input) -> Self::Output {
        (self.0)(input)
    }
}
