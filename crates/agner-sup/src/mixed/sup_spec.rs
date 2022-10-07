use crate::mixed::child_spec::FlatMixedChildSpec;

#[derive(Debug)]
pub struct SupSpec<ID, RS> {
    pub restart_strategy: RS,
    pub children: Vec<Box<dyn FlatMixedChildSpec<ID>>>,
}

impl<ID, RS> SupSpec<ID, RS> {
    pub fn new(restart_strategy: RS) -> Self {
        Self { restart_strategy, children: Default::default() }
    }

    pub fn with_child<CS>(mut self, child_spec: CS) -> Self
    where
        CS: Into<Box<dyn FlatMixedChildSpec<ID>>>,
    {
        self.children.push(child_spec.into());
        self
    }
}

#[tokio::test]
async fn ergonomics() {
    use std::convert::Infallible;
    use std::time::Duration;

    use agner_actors::{Context, System};

    use crate::common::InitType;
    use crate::mixed::{MixedChildSpec, OneForOne, RestartIntensity};

    async fn actor(_context: &mut Context<Infallible>, (): ()) {}

    let child_one = MixedChildSpec::mixed("first")
        .behaviour(actor)
        .args_clone(())
        .init_type(InitType::no_ack());
    let child_two = MixedChildSpec::mixed("second")
        .behaviour(actor)
        .args_clone(())
        .init_type(InitType::no_ack());

    let child_three = MixedChildSpec::mixed("third")
        .behaviour(actor)
        .args_clone(())
        .init_type(InitType::no_ack());

    let restart_intensity = RestartIntensity::new(5, Duration::from_secs(30));
    let restart_strategy = OneForOne::new(restart_intensity);
    let sup_spec = SupSpec::new(restart_strategy).with_child(child_one).with_child(child_two);

    let system = System::new(Default::default());
    let sup = system.spawn(crate::mixed::run, sup_spec, Default::default()).await.unwrap();

    crate::mixed::start_child(&system, sup, child_three).await.unwrap();
}
