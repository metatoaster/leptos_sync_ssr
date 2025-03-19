use leptos::prelude::*;
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast::{channel, Receiver, Sender};

#[derive(Clone)]
struct Message;

#[derive(Clone)]
pub struct Waiter {
    inner: Arc<WaiterInner>,
}

struct WaiterInner {
    sender: Sender<Message>,
    resolved: RwLock<bool>,
}

#[derive(Clone)]
pub struct WaiterHandle {
    inner: Option<Waiter>,
}

pub struct WaiterSubscription {
    inner: Option<WaiterSubscriptionInner>,
}

struct WaiterSubscriptionInner {
    waiter: Waiter,
    receiver: Receiver<Message>,
}

impl Waiter {
    pub fn handle() -> WaiterHandle {
        WaiterHandle {
            inner: use_context::<Waiter>()
        }
    }
}

impl WaiterHandle {
    pub fn subscribe(&self) -> WaiterSubscription {
        WaiterSubscription {
            inner: self.inner.clone().map(|waiter| WaiterSubscriptionInner {
                waiter: waiter.clone(),
                receiver: waiter.inner.sender.subscribe(),
            })
        }
    }
}

impl WaiterSubscription {
    pub async fn wait(mut self) {
        if let Some(mut inner) = self.inner.take() {
            leptos::logging::log!("waiter has handle... checking resolved status");
            if !*inner.waiter.inner.resolved.read().unwrap() {
                leptos::logging::log!("handle's waiter not resolved, waiting...");
                inner
                    .receiver
                    .recv()
                    .await
                    .expect("internal error: sender not properly managed");
                leptos::logging::log!("handle is now resolved");
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
                leptos::logging::log!("handle was resolved");
            }
        } else {
            leptos::logging::log!("there's no waiter!");
        }
    }
}

impl Waiter {
    pub(crate) fn complete() {
        let waiter = expect_context::<Waiter>();
        *waiter.inner.resolved.write().unwrap() = true;
        if let Ok(_) = waiter.inner.sender.send(Message) {
            leptos::logging::log!(
                "broadcasted complete to {} subscribers",
                waiter.inner.sender.receiver_count(),
            );
        } else {
            leptos::logging::log!("no subscribers available to receive completion");
        }
    }
}

pub(crate) fn provide_waiter() -> Waiter {
    let (sender, _) = channel(1);
    let resolved = RwLock::new(false);
    let waiter = Waiter {
        inner: WaiterInner { sender, resolved }.into(),
    };
    provide_context(waiter.clone());
    waiter
}
