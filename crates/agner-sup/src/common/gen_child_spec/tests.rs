use agner_actors::{ActorID, Context, Exit, Never, System};
use agner_init_ack::ContextInitAckExt;

#[cfg(feature = "reg")]
use agner_reg::Service;

use crate::common::gen_child_spec::traits::CreateChild;
use crate::common::gen_child_spec::GenChildSpec;
use crate::common::init_type::WithAck;

#[tokio::test]
async fn t01() {
    async fn sup(context: &mut Context<Never>, (): ()) {
        context.init_ack_ok(Default::default());
        std::future::pending().await
    }

    async fn actor(context: &mut Context<Never>, _args: String) {
        context.init_ack_ok(Default::default());
        std::future::pending().await
    }

    let system: System = System::new(Default::default());
    let sup_id: ActorID = system.spawn(sup, (), Default::default()).await.unwrap();

    #[cfg(feature = "reg")]
    let service = Service::new();

    let gen_child_spec = GenChildSpec::new()
        .behaviour(actor)
        .args_clone("hello".to_owned())
        .init_type(WithAck::default());

    #[cfg(feature = "reg")]
    let gen_child_spec = gen_child_spec.register(service);

    let mut gen_child_spec = gen_child_spec.to_owned();

    let child_id = gen_child_spec.create_child(&system, sup_id, ()).await.unwrap();
    eprintln!("started: {}", child_id);

    let child_id = gen_child_spec.create_child(&system, sup_id, ()).await.unwrap();
    eprintln!("started: {}", child_id);
}

#[tokio::test]
async fn t02() {
    async fn sup(context: &mut Context<Never>, (): ()) {
        context.init_ack_ok(Default::default());
        std::future::pending().await
    }

    async fn actor(context: &mut Context<Never>, unique_opt: Option<String>) {
        context.init_ack_ok(Default::default());

        let exit_reason =
            if unique_opt.is_some() { Exit::normal() } else { Exit::from_message("arg gone") };

        context.exit(exit_reason).await;
    }

    let system: System = System::new(Default::default());
    let sup_id: ActorID = system.spawn(sup, (), Default::default()).await.unwrap();

    let mut gen_child_spec = GenChildSpec::new()
        .behaviour(actor)
        .args_unique("hello".to_owned())
        .init_type(WithAck::default());

    let child_id = gen_child_spec.create_child(&system, sup_id, ()).await.unwrap();
    assert!(system.wait(child_id).await.is_normal());
    let child_id = gen_child_spec.create_child(&system, sup_id, ()).await.unwrap();
    assert!(system.wait(child_id).await.is_custom());
}

#[tokio::test]
async fn t03() {
    async fn sup(context: &mut Context<Never>, (): ()) {
        context.init_ack_ok(Default::default());
        std::future::pending().await
    }

    async fn actor(context: &mut Context<Never>, incarnation: usize) {
        context.init_ack_ok(Default::default());
        eprintln!("[{}] incarnation: {}", context.actor_id(), incarnation);
    }

    let system: System = System::new(Default::default());
    let sup_id: ActorID = system.spawn(sup, (), Default::default()).await.unwrap();

    let mut incarnation = 0;
    let mut gen_child_spec = GenChildSpec::new()
        .behaviour(actor)
        .args_call0(move || {
            incarnation += 1;
            incarnation
        })
        .init_type(WithAck::default());

    let child_id = gen_child_spec.create_child(&system, sup_id, ()).await.unwrap();
    assert!(system.wait(child_id).await.is_normal());
    let child_id = gen_child_spec.create_child(&system, sup_id, ()).await.unwrap();
    assert!(system.wait(child_id).await.is_normal());
    let child_id = gen_child_spec.create_child(&system, sup_id, ()).await.unwrap();
    assert!(system.wait(child_id).await.is_normal());
    let child_id = gen_child_spec.create_child(&system, sup_id, ()).await.unwrap();
    assert!(system.wait(child_id).await.is_normal());
}

#[tokio::test]
async fn t04() {
    async fn sup(context: &mut Context<Never>, (): ()) {
        context.init_ack_ok(Default::default());
        std::future::pending().await
    }

    async fn actor(context: &mut Context<Never>, worker_id: usize) {
        context.init_ack_ok(Default::default());
        eprintln!("[{}] worker_id: {}", context.actor_id(), worker_id);
    }

    let system: System = System::new(Default::default());
    let sup_id: ActorID = system.spawn(sup, (), Default::default()).await.unwrap();

    let mut gen_child_spec = GenChildSpec::new()
        .behaviour(actor)
        .args_call1(move |worker_id| worker_id)
        .init_type(WithAck::default());

    let child_id = gen_child_spec.create_child(&system, sup_id, 1).await.unwrap();
    assert!(system.wait(child_id).await.is_normal());
    let child_id = gen_child_spec.create_child(&system, sup_id, 42).await.unwrap();
    assert!(system.wait(child_id).await.is_normal());
    let child_id = gen_child_spec.create_child(&system, sup_id, 777).await.unwrap();
    assert!(system.wait(child_id).await.is_normal());
    let child_id = gen_child_spec.create_child(&system, sup_id, 11).await.unwrap();
    assert!(system.wait(child_id).await.is_normal());
}
