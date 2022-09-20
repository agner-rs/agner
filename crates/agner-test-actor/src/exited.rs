use agner_actors::Exit;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

#[derive(Debug)]
pub enum Exited {
    Wait(JoinHandle<Exit>),
    Ready(Exit),
}

pub async fn wait(mutex: &Mutex<Exited>) -> Exit {
    let mut locked = mutex.lock().await;
    let exited = &mut *locked;
    match exited {
        Exited::Ready(reason) => reason.to_owned(),
        Exited::Wait(join_handle) => {
            let reason = join_handle.await.expect("Join failure");
            *exited = Exited::Ready(reason.to_owned());
            reason
        },
    }
}
