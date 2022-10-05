use crate::mixed::child_spec::ChildSpec;

#[derive(Debug)]
pub struct SupSpec<ID, RS> {
    pub restart_strategy: RS,
    pub children: Vec<ChildSpec<ID>>,
}

impl<ID, RS> SupSpec<ID, RS> {
    pub fn new(restart_strategy: RS) -> Self {
        Self { restart_strategy, children: Default::default() }
    }

    pub fn with_child(mut self, child_spec: ChildSpec<ID>) -> Self {
        self.children.push(child_spec);
        self
    }
}

#[test]
fn ergonomics() {
    use crate::common::{args_factory, produce_child, InitType};
    use agner_actors::Context;
    use std::convert::Infallible;

    async fn actor(_context: &mut Context<Infallible>, (): ()) {}

    let child_one = ChildSpec::new(
        "first",
        produce_child::new(actor, args_factory::clone(()), InitType::NoAck),
    );
    let child_two = ChildSpec::new(
        "second",
        produce_child::new(actor, args_factory::clone(()), InitType::NoAck),
    );

    let _sup_spec = SupSpec::new(()).with_child(child_one).with_child(child_two);
}
