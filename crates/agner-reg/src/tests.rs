use agner_actors::ActorID;

#[tokio::test]
async fn happy_case() {
    let id_1: ActorID = "1.0.0".parse().unwrap();
    let id_2: ActorID = "1.1.1".parse().unwrap();

    let (tx, mut rx) = super::new();

    assert!(rx.resolve().is_none());
    {
        let _registered = tx.register(id_1);
        assert_eq!(rx.resolve(), Some(id_1));
        assert_eq!(rx.wait().await, Some(id_1));
    }
    assert!(rx.resolve().is_none());
    {
        let _registered = tx.register(id_2);
        assert_eq!(rx.resolve(), Some(id_2));
        assert_eq!(rx.wait().await, Some(id_2));
    }
    assert!(rx.resolve().is_none());

    std::mem::drop(tx);
    assert!(rx.wait().await.is_none());
}
