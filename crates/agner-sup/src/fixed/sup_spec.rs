use super::hlist::*;

#[derive(Debug)]
pub struct SupSpec<R, CS> {
    pub restart_strategy: R,
    pub children: CS,
}

impl<R> SupSpec<R, Nil> {
    pub fn new(restart_strategy: R) -> Self {
        Self { restart_strategy, children: Nil }
    }
}
impl<R, CS> SupSpec<R, CS> {
    pub fn with_child<C>(self, child: C) -> SupSpec<R, <CS as PushBack<C>>::Out>
    where
        CS: PushBack<C>,
    {
        let Self { restart_strategy, children } = self;
        SupSpec { restart_strategy, children: children.push_back(child) }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use agner_actors::Context;

    use crate::fixed;

    async fn behaviour_unit(_context: &mut Context<std::convert::Infallible>, _arg: ()) {
        std::future::pending().await
    }
    async fn behaviour_arc_unit(_context: &mut Context<std::convert::Infallible>, _arg: Arc<()>) {
        std::future::pending().await
    }

    #[test]
    fn ergonomics() {
        let restart_strategy = ();
        let _sup_spec = fixed::SupSpec::new(restart_strategy)
            .with_child(fixed::child_spec(behaviour_unit, fixed::arg_clone(())))
            .with_child(fixed::child_spec(behaviour_arc_unit, fixed::arg_arc(())))
            .with_child(fixed::child_spec(behaviour_unit, fixed::arg_call(|| ())));
    }
}
