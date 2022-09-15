use agner_actors::Context;

use crate::fixed::hlist::HList;
use crate::fixed::SupSpec;

pub enum Message {}

pub async fn fixed_sup<R, CS>(context: &mut Context<Message>, sup_spec: SupSpec<R, CS>)
where
    CS: HList,
{
    let children_count = CS::LEN;

    log::trace!("[{}] starting fixed sup with {} children", context.actor_id(), children_count);

    std::future::pending().await
}