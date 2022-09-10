pub trait Message: Send + Sync + 'static {}

impl<M> Message for M where M: Send + Sync + 'static {}
