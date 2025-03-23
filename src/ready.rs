#[cfg(feature = "ssr")]
mod ssr {
    pub use leptos::context::{provide_context, use_context};
    pub use std::sync::{Arc, RwLock};
    pub use tokio::sync::broadcast::{channel, Receiver, Sender};
}

#[cfg(feature = "ssr")]
use ssr::*;

#[derive(Clone)]
struct Message;  // also functions as a private phantom

/// The coordinator provided as a context by the [`SyncSsr`](
/// crate::component::SyncSsr) component.
///
/// Under SSR, this contains a `Sender` that will be able to broadcast
/// a message to all instances of actively waiting [`ReadySubscription`]
/// to inform the futures that the view tree enclosed by `SyncSsr` is
/// now ready and thus the wait is over.
///
/// Under CSR, this is essentially a unit newtype; all resulting methods
/// and associated functions would in essence be no-ops.
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

/// A handle to a possibly available [`Ready`] coordinator.
///
/// Please refer to [`Ready::handle`] for details as that's the only
/// public associated function that will return this type.
#[derive(Clone)]
pub struct ReadyHandle {
    #[cfg(feature = "ssr")]
    inner: Option<Ready>,
    _phantom: Message,
}

/// A subscription to the [`Ready`] coordinator, typically held by
/// futures that require the ready signal.
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
    /// Acquire a handle to a possibly available instance of `Ready`.
    ///
    /// This make use of [`use_context`] underneath the hood, so this
    /// should be called at the component's top level.  In any case, a
    /// handle will be returned, but the waiting will only happen if a
    /// `Ready` is in fact provided as a context.
    ///
    /// Moreover, given the use of `use_context`, this handle may or may
    /// not in fact point to the actual `Ready` underneath.  As the only
    /// function of this type is to ultimately listen for a message, the
    /// lack of such would only mean no waiting will happen when the
    /// [`ReadySubscription`] provided by that resulting handle tries to
    /// [`wait`](ReadySubscription::wait).
    ///
    /// This is purposefully designed as such to permit flexible usage
    /// in any context without the resulting resource and or components
    /// being tightly coupled to the `SyncSsr` component - the lack of
    /// such in the parent view tree would simply mean nothing will
    /// happen.
    pub fn handle() -> ReadyHandle {
        ReadyHandle {
            #[cfg(feature = "ssr")]
            inner: use_context::<Ready>(),
            _phantom: Message,
        }
    }
}

impl ReadyHandle {
    /// Subscribe to the [`Ready`] coordinator.
    ///
    /// To make use of a subscription within a future, move a clone of
    /// the handle into the future and call subscribe from there.
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
    /// Asynchronously wait for the ready signal.
    ///
    /// This may contain a receiver that will wait for the signal from
    /// the associated `Ready` which this subscription belongs to.  If
    /// no such receiver is in fact available (due to how the associated
    /// [`handle`](Ready::handle) providing this subscription was set
    /// up), or that a ready signal was already broadcasted, this
    /// will return immediately, otherwise it will wait for the ready
    /// message to arrive until execution will be allowed to continue.
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
