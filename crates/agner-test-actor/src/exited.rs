use std::fmt;
use std::future::Future;
use std::pin::Pin;

use agner_actors::Exit;
use agner_utils::std_error_pp::StdErrorPP;
use tokio::sync::Mutex;

pub enum Exited {
    Waiting(Pin<Box<dyn Future<Output = Exit> + Send + Sync + 'static>>),
    Ready(Exit),
}

pub async fn wait(mutex: &Mutex<Exited>) -> Exit {
    let mut locked = mutex.lock().await;
    let exited = &mut *locked;
    match exited {
        Exited::Ready(reason) => reason.to_owned(),
        Exited::Waiting(join_handle) => {
            let reason = join_handle.await;
            *exited = Exited::Ready(reason.to_owned());
            reason
        },
    }
}

impl fmt::Debug for Exited {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ready(reason) => write!(f, "Exited: {}", reason.pp()),
            Self::Waiting { .. } => write!(f, "Waiting"),
        }
    }
}
