#[cfg(feature = "ssr")]
mod ssr {
    pub use leptos::context::use_context;
    pub use std::sync::{Arc, Mutex, RwLock};
    pub use tokio::sync::watch::{channel, Receiver, Sender};
}

#[cfg(feature = "ssr")]
use ssr::*;

#[derive(Clone)]
struct Phantom;

/// Encapsulates the underlying ready state that may be provided as a
/// context by the [`SyncSsr`](crate::component::SyncSsr) component.
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
    pub(crate) inner: Arc<ReadyInner>,
    _phantom: Phantom,
}

/// Encapsulates the underlying ready state coordinator that must be
/// provided as a context to the current reactive owner; typically this
/// is done using the [`SyncSsrSignal`](crate::component::SyncSsrSignal)
/// component.
///
/// Under SSR, this contains a vector of [`CoReady`] that have been
/// registered to this coordinator, and that that may be notified when
/// their [`CoReadySubscription`] should continue their wait depending
/// whether if they have live outstanding ready senders.
///
/// Under CSR, this is essentially a unit newtype; all resulting methods
/// and associated functions would in essence be no-ops.
#[derive(Clone)]
pub struct CoReadyCoordinator {
    #[cfg(feature = "ssr")]
    inner: Arc<Mutex<Vec<CoReady>>>,
    _phantom: Phantom,
}

/// Encapsulates a coordinated ready state.
///
/// Under SSR, this contains a `Sender` that may be cloned, and that all
/// of them will be able to broadcast a message to all actively waiting
/// [`CoReadySubscription`] that this state has spawned to inform the
/// futures that the view tree enclosed by [`SyncSsrSignal`](
/// crate::component::SyncSsrSignal) is now ready and thus the wait is
/// over.
///
/// Under CSR, this is essentially a unit newtype; all resulting methods
/// and associated functions would in essence be no-ops, and any
/// constructors simply return a unit newtype.
#[derive(Clone)]
pub struct CoReady {
    #[cfg(feature = "ssr")]
    inner: Arc<ReadyInner>,
    _phantom: Phantom,
}

#[cfg(feature = "ssr")]
#[derive(Clone)]
pub(crate) struct ReadyInner {
    sender: Sender<Option<bool>>,
    // This determines whether the next flag may be armed
    manual_complete: bool,
    // This becomes armed if the above is set, and it will keep affected
    // `CoReadySubscriber` waiting after being notified of the first ready
    // state.
    manual_complete_armed: Arc<RwLock<bool>>,
}

#[cfg(feature = "ssr")]
#[derive(Clone)]
pub(crate) struct ReadySender {
    inner: ReadyInner,
}

/// A handle to a possibly available [`Ready`] state.
///
/// Please refer to [`Ready::handle`] for details as that's the only
/// public associated function that will return this type.
#[derive(Clone)]
pub struct ReadyHandle {
    #[cfg(feature = "ssr")]
    inner: Option<Ready>,
    _phantom: Phantom,
}

/// A subscription to the [`Ready`] state, typically held by futures
/// that require the ready signal.
pub struct ReadySubscription {
    #[cfg(feature = "ssr")]
    inner: Option<ReadySubscriptionInner>,
    _phantom: Phantom,
}

#[cfg(feature = "ssr")]
pub(crate) struct ReadySubscriptionInner {
    ready: Ready,
    receiver: Receiver<Option<bool>>,
}

/// A subscription to the [`CoReady`] state, typically held by the
/// `Resource` futures that require the signal to continue.
pub struct CoReadySubscription {
    #[cfg(feature = "ssr")]
    inner: CoReadySubscriptionInner,
    _phantom: Phantom,
}

#[cfg(feature = "ssr")]
pub(crate) struct CoReadySubscriptionInner {
    ready: CoReady,
    receiver: Receiver<Option<bool>>,
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
            _phantom: Phantom,
        }
    }
}

#[cfg(feature = "ssr")]
impl CoReadyCoordinator {
    /// Create a new `CoReadyCoordinator`.
    ///
    /// This function is provided to allow more manual notifying of the
    /// underlying `CoReady` states.  Do note that this does not in fact
    /// provide context, which [`CoReady::new`] expects, hence it's
    /// recommended to use the `<SyncSsrSignal/>` component instead as
    /// that not only provide the context but also ensures that
    /// [`CoReadyCoordinator::notify`] is also called when all its
    /// children are done processing, to ensure that those subscription
    /// without senders can stop waiting.
    pub(crate) fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Vec::new())),
            _phantom: Phantom,
        }
    }

    fn register(&self, r: CoReady) {
        self.inner.lock()
            .expect("mutex not panicked")
            .push(r);
    }

    /// Notifies all `CoReady` states that they are primed, if they are
    /// not already completed.
    ///
    /// What this means is this allow the subscribers that are actively
    /// waiting be able to check whether they should continue to wait.
    /// If there are no outstanding `ReadySender`s then they should stop
    /// waiting, otherwise they should continue to wait.
    pub(crate) fn notify(&self) {
        for ready in self.inner.lock()
            .expect("mutex not panicked")
            .iter()
        {
            if *ready.inner.sender.borrow() != Some(true) {
                let _ = ready.inner.sender.send(Some(false));
            }
        }
    }
}

// should this be exposed
// #[cfg(not(feature = "ssr"))]
// impl CoReadyCoordinator {
//     pub(crate) fn new() -> Self {
//         Self {
//             _phantom: Phantom,
//         }
//     }
// }

#[cfg(feature = "ssr")]
impl CoReady {
    /// Create and register a new instance of `CoReady` with the
    /// [`CoReadyCoordinator`] provided as a context in the reactive
    /// graph.  This context is provided by nesting inside the
    /// `<SyncSsrSignal/>` component.
    ///
    /// This standard version is configured such that senders this
    /// produced will auto-complete when dropped, such that every
    /// [`CoReadySubscription`] that are waiting will be released.
    ///
    /// ## Panics
    /// Panics if the context of `CoReadyCoordinator` is not found in
    /// the current reactive owner or its ancestors.
    #[track_caller]
    pub fn new() -> Self {
        Self::new_with_options(false)
    }

    /// Create and register a new instance of `CoReady` with the
    /// [`CoReadyCoordinator`] provided as a context in the reactive
    /// graph.  This context is provided by nesting inside the
    /// `<SyncSsrSignal/>` component.
    ///
    /// This constructor produces a `CoReady` that must be manually
    /// completed when a `ReadySender` is acquired from it.  Once
    /// acquired, every [`CoReadySubscription`] that are waiting before
    /// being notified (and released from waiting) by the underlying
    /// `CoReadyCoordinator` will continue to wait, and thus a
    /// `ReadySender::complete` must be invoked to release the wait
    /// lock, as simply dropping the `ReadySender` will no longer
    /// notify complete.
    ///
    /// ## Panics
    /// Panics if the context of `CoReadyCoordinator` is not found in
    /// the current reactive owner or its ancestors.
    #[track_caller]
    pub fn new_manually_completed() -> Self {
        Self::new_with_options(true)
    }

    /// Create and register a new instance of `CoReady` with the
    /// [`CoReadyCoordinator`] provided as a context in the reactive
    /// graph.  This context is provided by nesting inside the
    /// `<SyncSsrSignal/>` component.
    ///
    /// If `manual_complete` is `true`, the resulting `CoReady` will be
    /// configured such that when a `ReadySender` is acquired from it,
    /// every [`CoReadySubscription`] that are waiting before being
    /// notified (and released from waiting) by the underlying
    /// `CoReadyCoordinator` will continue to wait, and thus a
    /// `ReadySender::complete` must be invoked to release the wait
    /// lock, as simply dropping the `ReadySender` will no longer notify
    /// complete.
    ///
    /// ## Panics
    /// Panics if the context of `CoReadyCoordinator` is not found in
    /// the current reactive owner or its ancestors.
    #[track_caller]
    pub fn new_with_options(manual_complete: bool) -> Self {
        let location = std::panic::Location::caller();
        // FIXME a better error message
        let coordinator = use_context::<CoReadyCoordinator>().unwrap_or_else(|| {
            panic!("{location:?} expected a context of `CoReadyCoordinator` to be present")
        });
        let (sender, _) = channel(None);
        let result = Self {
            inner: Arc::new(ReadyInner::new(sender, manual_complete)),
            _phantom: Phantom,
        };
        coordinator.register(result.clone());
        result
    }

    /// Subscribe to this [`CoReady`] state.
    ///
    /// To make use of this subscription within a future, move a clone
    /// of this into the future and call subscribe from that.
    pub fn subscribe(&self) -> CoReadySubscription {
        CoReadySubscription {
            #[cfg(feature = "ssr")]
            inner: CoReadySubscriptionInner {
                ready: self.clone(),
                receiver: self.inner.sender.subscribe(),
            },
            _phantom: Phantom,
        }
    }

    pub(crate) fn to_ready_sender(&self) -> ReadySender {
        self.inner.to_ready_sender()
    }
}

#[cfg(not(feature = "ssr"))]
impl CoReady {
    pub fn new() -> Self {
        Self { _phantom: Phantom }
    }

    pub fn subscribe(&self) -> CoReadySubscription {
        CoReadySubscription { _phantom: Phantom }
    }
}

impl ReadyHandle {
    /// Subscribe to the [`Ready`] state.
    ///
    /// To make use of this subscription within a future, move a clone
    /// of this handle into the future and call subscribe from that.
    pub fn subscribe(&self) -> ReadySubscription {
        ReadySubscription {
            #[cfg(feature = "ssr")]
            inner: self.inner.as_ref().map(Ready::subscribe_inner),
            _phantom: Phantom,
        }
    }
}

#[cfg(not(feature = "ssr"))]
impl ReadySubscription {
    pub async fn wait(self) {}
}

#[cfg(feature = "ssr")]
impl ReadySubscription {
    /// Asynchronously wait for the ready signal under SSR.
    ///
    /// This may contain a receiver that will wait for the signal from
    /// the associated `Ready` which this subscription belongs to.  If
    /// no such receiver is in fact available (due to how the associated
    /// [`handle`](Ready::handle) providing this subscription was set
    /// up), or that a ready signal was already broadcasted, this
    /// will return immediately, otherwise it will wait for the ready
    /// message to arrive until execution will be allowed to continue.
    ///
    /// Under CSR this is essentially a no-op.
    pub async fn wait(mut self) {
        if let Some(inner) = self.inner.take() {
            inner.wait_inner().await
        }
    }
}

#[cfg(not(feature = "ssr"))]
impl CoReadySubscription {
    pub async fn wait(self) {}
}

#[cfg(feature = "ssr")]
impl CoReadySubscription {
    /// Asynchronously wait for the ready signal under SSR.
    ///
    /// This contains a receiver that will wait for the signal from
    /// the associated `CoReady` or its associated `ReadySender` bound
    /// to this subscription.
    ///
    /// This will wait until the value `Some(true)` is received, much
    /// like [`ReadySubscription`], but it will also finish waiting on a
    /// `Some(false)` value if there are no outstanding `ReadySender`
    /// and the underlying `CoReady` is not configured and armed for
    /// manual complete.  Refer to [`CoReady::new_manually_completed`]
    /// for additional details.
    ///
    /// Under CSR this is essentially a no-op.
    pub async fn wait(self) {
        self.inner.wait_inner().await
    }
}

#[cfg(feature = "ssr")]
impl ReadySubscriptionInner {
    pub(crate) async fn wait_inner(mut self) {
        self
            .receiver
            .wait_for(|v| *v == Some(true))
            .await
            .expect("internal error: sender not properly managed");
        // XXX a 0 duration sleep seems to be required to mitigate
        // an issue where Suspend doesn't wake up after the resource
        // runs this async method, and this path does not have an
        // await seems to cause the issue.
        //
        // Initial thought was to try a mitigation using a simple
        // `async {}.await`, however that does not work, and hence
        // the 0 duration sleep.
        //
        // Without this workaround in place, in roughly 1 in 200
        // requests it would not complete and thus the client will
        // see a timeout.  With the mitigation in place, the same
        // tight loop running in 5 different threads making 20000
        // requests may see in total 1 to 2 timeouts triggered.
        // However, this test also revealed that there are still
        // other unaccounted issues with SSR as there are transfer
        // size variations seen, but rate of occurrence is about 7
        // to 8 in 100000 from that benchmark, for a total failure
        // rate of about 0.01%.  The above is derived using the
        // simple example on the `http://localhost:3000/fixed`
        // endpoint under debug mode.  Under release mode, the failure
        // rate roughly doubles (in terms of transfer size variance
        // indicative of some form of hydration error/mismatch.
        //
        // Subsequent to switching the channel from broadcast to
        // watch, and upgrading to leptos-0.8.0, the sleep is still
        // required in this form as without the sleep, the following
        // panick may also happen:
        //
        //     panicked at reactive_graph-0.2.2/src/owner/arena.rs:53:17:
        //     reactive_graph-0.2.2/src/owner/arena.rs:56:21,
        //     the `sandboxed-arenas` feature is active, but no Arena is
        //     active
        //
        // Hence the underlying issue may in fact be upstream, but this
        // sleep is a sufficient mitigation.
        //
        // As for the underlying issue, they are filed at:
        //
        // - https://github.com/leptos-rs/leptos/issues/3699
        // - https://github.com/leptos-rs/leptos/issues/3729
        // - https://github.com/leptos-rs/leptos/pull/4065
        tokio::time::sleep(std::time::Duration::from_millis(0)).await;
    }
}

#[cfg(feature = "ssr")]
impl CoReadySubscriptionInner {
    pub(crate) async fn wait_inner(mut self) {
        let sender = &self.ready.inner.sender;
        let manual_complete = self.ready.inner.manual_complete;
        self
            .receiver
            .wait_for(|v| {
                let v = *v;
                v == Some(true) ||
                    (!manual_complete && v == Some(false) && sender.sender_count() == 1)
            })
            .await
            .expect("internal error: sender not properly managed");
    }
}

#[cfg(feature = "ssr")]
impl ReadyInner {
    pub(crate) fn new(
        sender: Sender<Option<bool>>,
        manual_complete: bool,
    ) -> Self {
        Self {
            sender,
            manual_complete,
            manual_complete_armed: Arc::new(RwLock::new(false)),
        }
    }

    pub(crate) fn complete(&self) {
        let _ = self.sender.send(Some(true));
        // TODO if we were to provide a tracing feature...
        // if let Ok(_) = self.sender.send(Some(true)) {
        //     leptos::logging::log!(
        //         "broadcasted complete to {} subscribers",
        //         self.inner.sender.receiver_count(),
        //     );
        // } else {
        //     leptos::logging::log!("no subscribers available to receive completion");
        // }
    }

    // this creates a new sender
    pub(crate) fn to_ready_sender(&self) -> ReadySender {
        if self.manual_complete && !*self.manual_complete_armed.read().expect("not poisoned") {
            let mut armed = self.manual_complete_armed.write().expect("not poisoned");
            *armed = true;
        }
        ReadySender {
            inner: self.clone(),
        }
    }
}

#[cfg(feature = "ssr")]
impl Ready {
    pub(crate) fn new() -> Ready {
        let (sender, _) = channel(Some(false));
        Ready {
            inner: ReadyInner::new(sender, false).into(),
            _phantom: Phantom,
        }
    }

    pub(crate) fn complete(&self) {
        self.inner.complete();
    }

    pub(crate) fn subscribe_inner(&self) -> ReadySubscriptionInner {
        ReadySubscriptionInner {
            ready: self.clone(),
            receiver: self.inner.sender.subscribe(),
        }
    }
}

#[cfg(feature = "ssr")]
impl Drop for ReadySender {
    fn drop(&mut self) {
        if !*self.inner.manual_complete_armed.read().expect("not poisoned") {
            self.complete();
        }
    }
}

#[cfg(feature = "ssr")]
impl ReadySender {
    pub(crate) fn complete(&self) {
        self.inner.complete();
    }
}

#[cfg(feature = "ssr")]
mod debug {
    use super::*;
    use std::fmt;

    impl fmt::Debug for Ready {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("Ready")
                .field("resolved", &*self.inner.sender.borrow())
                .field("senders", &self.inner.sender.sender_count())
                .field("subscribers", &self.inner.sender.receiver_count())
                .finish()
        }
    }

    impl fmt::Debug for CoReady {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("CoReady")
                .field("resolved", &*self.inner.sender.borrow())
                .field("senders", &self.inner.sender.sender_count())
                .field("subscribers", &self.inner.sender.receiver_count())
                .field("manual_complete", &self.inner.manual_complete)
                .finish()
        }
    }

    impl fmt::Debug for ReadyHandle {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("ReadyHandle")
                .field("inner", &self.inner)
                .finish()
        }
    }

    impl fmt::Debug for ReadySubscription {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("ReadySubscription")
                .field("ready", &self.inner.as_ref().map(|v| v.ready.clone()))
                .finish()
        }
    }
}
