use agner_actors::{BoxError, Context};

use crate::fixed::hlist::HList;
use crate::fixed::SupSpec;

use crate::fixed::sup_spec::SupSpecStartChild;

pub enum Message {}

pub async fn fixed_sup<R, CS>(
    context: &mut Context<Message>,
    mut sup_spec: SupSpec<R, CS>,
) -> Result<(), BoxError>
where
    CS: HList,
    SupSpec<R, CS>: SupSpecStartChild<Message>,
{
    context.trap_exit(true).await;

    log::trace!("[{}] starting fixed sup with {} children", context.actor_id(), CS::LEN);

    let mut children = Vec::with_capacity(CS::LEN);

    for child_idx in 0..CS::LEN {
        log::trace!("[{}] starting child #{}...", context.actor_id(), child_idx);
        let child_id = sup_spec.start_child(context, child_idx).await?;
        children.push(child_id);
        log::trace!("[{}]   child #{}: {}", context.actor_id(), child_idx, child_id);
    }
    assert_eq!(children.len(), CS::LEN);

    std::future::pending().await
}
