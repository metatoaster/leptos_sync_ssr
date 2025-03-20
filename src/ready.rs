#[cfg(feature = "ssr")]
mod ssr {
    pub use leptos::context::{provide_context, use_context};
    pub use std::sync::{Arc, RwLock};
    pub use tokio::sync::broadcast::{channel, Receiver, Sender};
}

#[cfg(feature = "ssr")]
use ssr::*;

#[derive(Clone)]
struct Message;

#[derive(Clone)]
pub struct Ready {
    #[cfg(feature = "ssr")]
    inner: Arc<ReadyInner>,
    _phantom: Message,
}

#[cfg(feature = "ssr")]
struct ReadyInner {
    sender: Sender<Message>,
    resolved: RwLock<bool>,
}

#[derive(Clone)]
pub struct ReadyHandle {
    #[cfg(feature = "ssr")]
    inner: Option<Ready>,
    _phantom: Message,
}

pub struct ReadySubscription {
    #[cfg(feature = "ssr")]
    inner: Option<ReadySubscriptionInner>,
    _phantom: Message,
}

#[cfg(feature = "ssr")]
struct ReadySubscriptionInner {
    ready: Ready,
    receiver: Receiver<Message>,
}

impl Ready {
    pub fn handle() -> ReadyHandle {
        ReadyHandle {
            #[cfg(feature = "ssr")]
            inner: use_context::<Ready>(),
            _phantom: Message,
        }
    }
}

impl ReadyHandle {
    pub fn subscribe(&self) -> ReadySubscription {
        ReadySubscription {
            #[cfg(feature = "ssr")]
            inner: self.inner.clone().map(|ready| ReadySubscriptionInner {
                ready: ready.clone(),
                receiver: ready.inner.sender.subscribe(),
            }),
            _phantom: Message,
        }
    }
}

#[cfg(not(feature = "ssr"))]
impl ReadySubscription {
    pub fn wait(self) -> impl std::future::Future<Output = ()> {
        async {}
    }
}

#[cfg(feature = "ssr")]
impl ReadySubscription {
    pub async fn wait(mut self) {
        if let Some(mut inner) = self.inner.take() {
            if !*inner.ready.inner.resolved.read().unwrap() {
                inner
                    .receiver
                    .recv()
                    .await
                    .expect("internal error: sender not properly managed");
            } else {
                // XXX this 0 time sleep seems to be required to mitigate
                // an issue where Suspend doesn't wake up after the resource
                // runs this async method, and this path does not have an
                // await seems to cause the issue.  However, it doesn't appear
                // to be as simple as this as a simple `async {}.await` doesn't
                // work.  Without this workaroud in place, in roughly 1 in 200
                // requests it would not complete and thus the client will see
                // a timeout.  With the mitigation in place, the same tight
                // loop running in 5 different threads making 20000 requests
                // may see in total 1 to 2 timeouts triggered.  However, this
                // test also revealed that there are still other unaccounted
                // issues with SSR as there are transfer size variations seen,
                // but rate of occurrence is about 7 to 8 in 100000 from that
                // benchmark, for a total failure rate of about 0.01%.
                tokio::time::sleep(std::time::Duration::from_millis(0)).await;
            }
        }
    }
}

#[cfg(feature = "ssr")]
impl Ready {
    pub(crate) fn new() -> Ready {
        let (sender, _) = channel(1);
        let resolved = RwLock::new(false);
        let ready = Ready {
            inner: ReadyInner { sender, resolved }.into(),
            _phantom: Message,
        };
        provide_context(ready.clone());
        ready
    }

    pub(crate) fn complete(&self) {
        *self.inner.resolved.write().unwrap() = true;
        let _ = self.inner.sender.send(Message);
        // TODO if we were to provide a tracing feature...
        // if let Ok(_) = ready.inner.sender.send(Message) {
        //     leptos::logging::log!(
        //         "broadcasted complete to {} subscribers",
        //         ready.inner.sender.receiver_count(),
        //     );
        // } else {
        //     leptos::logging::log!("no subscribers available to receive completion");
        // }
    }
}
