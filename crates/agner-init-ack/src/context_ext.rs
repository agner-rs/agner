use agner_actors::{ActorID, Exit, Never};

use crate::channel::InitAckTx;

pub trait ContextInitAckExt {
    fn init_ack<E>(&mut self, result: Result<ActorID, E>)
    where
        E: Into<Exit>;

    fn init_ack_ok(&mut self, actor_id_opt: Option<ActorID>);

    fn init_ack_err<E>(&mut self, err: E)
    where
        E: Into<Exit>,
    {
        self.init_ack(Err(err))
    }
}

impl<M> ContextInitAckExt for agner_actors::Context<M> {
    fn init_ack<E>(&mut self, result: Result<ActorID, E>)
    where
        E: Into<Exit>,
    {
        if let Some(init_ack_tx) = self.take::<InitAckTx>() {
            init_ack_tx.ack(result)
        }
    }

    fn init_ack_ok(&mut self, actor_id_opt: Option<ActorID>) {
        let actor_id = actor_id_opt.unwrap_or_else(|| self.actor_id());
        self.init_ack::<Never>(Ok(actor_id))
    }
}
