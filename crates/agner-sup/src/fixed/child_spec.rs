use std::marker::PhantomData;
use std::sync::Arc;

use agner_actors::Actor;

use crate::Registered;

pub fn child_spec<B, AF, A, M>(behaviour: B, arg_factory: AF) -> impl ChildSpec
where
    AF: ArgFactory<A>,
    B: for<'a> Actor<'a, A, M>,
    AF: Send + Sync + 'static,
    M: Send + Sync + Unpin + 'static,
    B: Clone,
    A: Send + Sync + 'static,
{
    ChildSpecImpl {
        name: None,
        regs: Default::default(),
        behaviour,
        arg_factory,
        _pd: Default::default(),
    }
}
pub fn arg_call<F, A>(f: F) -> impl ArgFactory<A>
where
    F: FnMut() -> A,
{
    ArgFactoryCall(f)
}
pub fn arg_clone<A>(a: A) -> impl ArgFactory<A>
where
    A: Clone,
{
    ArgFactoryClone(a)
}
pub fn arg_arc<A>(a: impl Into<Arc<A>>) -> impl ArgFactory<Arc<A>> {
    ArgFactoryArc(a.into())
}

pub trait ChildSpec {
    type Message: Send + Sync + Unpin + 'static;
    type Behaviour: for<'a> Actor<'a, Self::Arg, Self::Message> + Send + Sync + 'static;
    type Arg: Send + Sync + 'static;

    fn create(&mut self) -> (Self::Behaviour, Self::Arg);
    fn with_name(self, name: impl Into<String>) -> Self;
    fn register(self, registered: Registered) -> Self;
    fn regs(&self) -> &[Registered];
}

pub trait ArgFactory<Arg> {
    fn create(&mut self) -> Arg;
}

impl<F, A> ArgFactory<A> for ArgFactoryCall<F>
where
    F: FnMut() -> A,
{
    fn create(&mut self) -> A {
        (self.0)()
    }
}
impl<A> ArgFactory<A> for ArgFactoryClone<A>
where
    A: Clone,
{
    fn create(&mut self) -> A {
        self.0.clone()
    }
}
impl<A> ArgFactory<Arc<A>> for ArgFactoryArc<A> {
    fn create(&mut self) -> Arc<A> {
        Arc::clone(&self.0)
    }
}

#[derive(Debug, Clone)]
struct ArgFactoryCall<F>(F);

#[derive(Debug, Clone)]
struct ArgFactoryClone<A>(A);

#[derive(Debug, Clone)]
struct ArgFactoryArc<A>(Arc<A>);

#[derive(Debug, Clone)]
struct ChildSpecImpl<B, AF, A, M> {
    behaviour: B,
    name: Option<String>,
    regs: Vec<Registered>,
    arg_factory: AF,
    _pd: PhantomData<(A, M)>,
}

impl<B, AF, A, M> ChildSpec for ChildSpecImpl<B, AF, A, M>
where
    AF: ArgFactory<A>,
    B: for<'a> Actor<'a, A, M>,
    AF: Send + Sync + 'static,
    M: Send + Sync + Unpin + 'static,
    B: Clone + Send + Sync + 'static,
    A: Send + Sync + 'static,
{
    type Message = M;
    type Behaviour = B;
    type Arg = A;

    fn create(&mut self) -> (Self::Behaviour, Self::Arg) {
        let arg = self.arg_factory.create();
        (self.behaviour.to_owned(), arg)
    }

    fn with_name(self, name: impl Into<String>) -> Self {
        Self { name: Some(name.into()), ..self }
    }

    fn register(mut self, reg: Registered) -> Self {
        self.regs.push(reg);
        self
    }

    fn regs(&self) -> &[Registered] {
        &self.regs
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use agner_actors::{Context, System};

    use crate::fixed;
    use crate::fixed::ChildSpec;

    #[tokio::test]
    async fn ergonomics() {
        async fn behaviour_unit(_context: &mut Context<std::convert::Infallible>, _arg: ()) {
            std::future::pending().await
        }
        async fn behaviour_arc_unit(
            _context: &mut Context<std::convert::Infallible>,
            _arg: Arc<()>,
        ) {
            std::future::pending().await
        }

        let system = System::new(Default::default());

        let mut child_spec_1 = fixed::child_spec(behaviour_unit, fixed::arg_call(|| ()));
        let mut child_spec_2 = fixed::child_spec(behaviour_unit, fixed::arg_clone(()));
        let mut child_spec_3 = fixed::child_spec(behaviour_arc_unit, fixed::arg_arc(()));
        let mut child_spec_4 = fixed::child_spec(behaviour_arc_unit, fixed::arg_arc(Box::new(())));
        let mut child_spec_5 = fixed::child_spec(behaviour_arc_unit, fixed::arg_arc(Arc::new(())));

        let (b_1, a_1) = child_spec_1.create();
        let (b_2, a_2) = child_spec_2.create();
        let (b_3, a_3) = child_spec_3.create();
        let (b_4, a_4) = child_spec_4.create();
        let (b_5, a_5) = child_spec_5.create();

        let a1 = system.spawn(b_1, a_1, Default::default()).await.unwrap();
        let a2 = system.spawn(b_2, a_2, Default::default()).await.unwrap();
        let a3 = system.spawn(b_3, a_3, Default::default()).await.unwrap();
        let a4 = system.spawn(b_4, a_4, Default::default()).await.unwrap();
        let a5 = system.spawn(b_5, a_5, Default::default()).await.unwrap();

        for a in [a1, a2, a3, a4, a5] {
            eprintln!("spawned: {}", a);
        }
    }
}
