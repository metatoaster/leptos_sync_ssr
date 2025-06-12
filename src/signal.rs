//! Provides the signal-resource pairing for synchronized SSR.
use std::{
    fmt::{Debug, Formatter, Result},
    panic::Location,
    sync::Arc,
};

use leptos::{
    reactive::{
        traits::{DefinedAt, Get, GetUntracked, IsDisposed, Notify, UntrackableGuard, Write},
        signal::{
            ArcReadSignal, ArcRwSignal, ArcWriteSignal,
            guards::{WriteGuard, UntrackedWriteGuard},
        },
    },
    server::ArcResource,
};
use serde::{de::DeserializeOwned, Serialize};

#[cfg(feature = "ssr")]
use crate::ready::{CoReady, ReadySender};

/// Provides a signal-resource pairing that together works to provide an
/// asynchronously waitable read signal (through the resource) under SSR.
///
/// The read-only resource will be primed to wait upon acquisition of the
/// write-only signal, as this will ensure the resource produce the intended
/// value under SSR to ensure the expected content be rendered and to allow
/// hydration to happen correctly, while also allowing the default value be
/// returned without requiring a manual unlock.  Should the write-only signal
/// be dropped, the resource may be permitted to return the value it holds
/// also.  The documentation under [`SsrSignalResource::write_only`] goes
/// in-depth on how to use it correctly to ensure the read-only resource to
/// return with the expected value that may be provided by the write-only
/// signal.
///
/// Note that this type can only be created inside components that have have
/// the [`CoReadyCoordinator`](crate::ready::CoReadyCoordinator) be provided as
/// a context, which typically involves having the
/// [`<SyncSsrSignal/>`](crate::component::SyncSsrSignal) component be one of
/// the ancestors of the component in the view tree.

#[derive(Clone)]
pub struct SsrSignalResource<T> {
    inner: Arc<SsrSignalResourceInner<T>>
}

struct SsrSignalResourceInner<T> {
    #[cfg(feature = "ssr")]
    ready: CoReady,
    resource: ArcResource<T>,
    signal_read: ArcReadSignal<T>,
    signal_write: ArcWriteSignal<T>,
}

/// The write signal created by [`SsrSignalResource::write_only`].
///
/// When created before the `CoReadyCoordinator` notified is invoked, it
/// will cause the paired resource to wait until a value is set through
/// any of trait methods for updates.  It may notify the paired resource
/// to stop waiting when dropped, refer to the documentation for
/// [`SsrSignalResource`] for details as this type is tightly coupled to
/// that type.
// Note that this type is _NOT_ Clone specifically to avoid potential
// footguns from the notify when dropped behavior.
pub struct SsrWriteSignal<T> {
    inner: Arc<SsrWriteSignalInner<T>>,
}

struct SsrWriteSignalNotifier<T> {
    inner: Arc<SsrWriteSignalInner<T>>,
}

struct SsrWriteSignalInner<T> {
    #[cfg(feature = "ssr")]
    ready_sender: ReadySender,
    signal_write: ArcWriteSignal<T>,
}

impl<T> SsrSignalResourceInner<T>
where
    T: Clone + Send + Sync + PartialEq + Serialize + DeserializeOwned + 'static,
{
    #[track_caller]
    fn new(value: T, _manual_complete: bool) -> Self {
        #[cfg(feature = "ssr")]
        let ready = CoReady::new_with_options(_manual_complete);
        let (signal_read, signal_write) = ArcRwSignal::new(value.clone()).split();

        // FIXME using `try` variants to work around issues with panics caused
        // by access of reactive value that were disposed (despite being Arc
        // variants), see:
        // - https://github.com/leptos-rs/leptos/issues/3729
        let resource = ArcResource::new(
            {
                let signal_read = signal_read.clone();
                // move || signal_read.get()
                move || signal_read.try_get().unwrap_or(value.clone())
            },
            {
                #[cfg(feature = "ssr")]
                let ready = ready.clone();
                let signal_read = signal_read.clone();
                move |original| {
                    #[cfg(feature = "ssr")]
                    let subscriber = ready.subscribe();
                    let signal_read = signal_read.clone();
                    async move {
                        #[cfg(feature = "ssr")]
                        subscriber.wait().await;
                        // given that the signal may provide a different value
                        // to what was originally passed by the time the
                        // subscriber finishes waiting, get a new value without
                        // tracking.
                        // signal_read.get_untracked()
                        signal_read.try_get_untracked().unwrap_or(original)
                    }
                }
            },
        );

        Self {
            #[cfg(feature = "ssr")]
            ready: ready,
            signal_read,
            signal_write,
            resource,
        }
    }
}

impl<T> SsrSignalResource<T>
where
    T: Clone + Send + Sync + PartialEq + Serialize + DeserializeOwned + 'static,
{
    /// Creates a signal-resource pairing with the value of type `T`.
    ///
    /// Typical use case is to clone this to where they are needed so
    /// that the read-only and write-only ends may be acquired for
    /// usage.  Any [`SsrWriteSignal`] acquired from this while inside
    /// the [`<SyncSsrSignal/>`](crate::component::SyncSsrSignal)
    /// component will activate the wait lock on the underlying
    /// [`ArcResource`], and it will hold until the write signal is
    /// notified or dropped.
    ///
    /// ## Panics
    /// Panics if the context of type `CoReadyCoordinator` is not found
    /// in the current reactive owner or its ancestors.  This may be
    /// resolved by providing the context by nesting this inside the
    /// [`<SyncSsrSignal/>`](crate::component::SyncSsrSignal) component.
    #[track_caller]
    pub fn new(value: T) -> Self {
        Self {
            inner: SsrSignalResourceInner::new(value, false).into(),
        }
    }

    /// Creates a signal-resource pairing with the value of type `T`.
    ///
    /// Typical use case is to clone this to where they are needed so
    /// that the read-only and write-only ends may be acquired for
    /// usage.  Any [`SsrWriteSignal`] acquired from this while inside
    /// the [`<SyncSsrSignal/>`](crate::component::SyncSsrSignal)
    /// component will activate the wait lock on the underlying
    /// [`ArcResource`], and it will hold until the write signal is
    /// notified.
    ///
    /// In other words, use of any resulting `SsrWriteSignal` acquired
    /// is **compulsory** to avoid deadlock from reading of the
    /// underlying `ArcResource` when using this constructor.
    ///
    /// ## Panics
    /// Panics if the context of type `CoReadyCoordinator` is not found
    /// in the current reactive owner or its ancestors.  This may be
    /// resolved by providing the context by nesting this inside the
    /// [`<SyncSsrSignal/>`](crate::component::SyncSsrSignal) component.
    #[track_caller]
    pub fn new_must_notify(value: T) -> Self {
        Self {
            inner: SsrSignalResourceInner::new(value, true).into(),
        }
    }
}

impl<T> SsrSignalResource<T> {
    /// Acquire the underlying `ArcResource` side of the pair.
    ///
    /// *Under SSR*, the underlying resource will asynchronously wait
    /// until any paired [`SsrWriteSignal`] provides the value or is
    /// otherwise notifies, where the value being held by the underlying
    /// signal will be returned.
    ///
    /// The resource will also return the underlying value (typically
    /// the default value used to create the [`SsrSignalResource`])
    /// should the enclosing `SyncSsrSignal` component is done
    /// processing without a `SsrWriteSignal` being acquired from this.
    ///
    /// *Under CSR* no waiting would happen and so the underlying
    /// resource should act like an indirect [`ArcReadSignal`].
    pub fn read_only(&self) -> ArcResource<T> {
        self.inner.resource.clone()
    }

    /// Acquire a `SsrWriteSignal`, which is a wrapper containing the
    /// underlying [`ArcWriteSignal`] side of the pairing, along with a
    /// `ReadySender` when under SSR.  While it is possible to use this
    /// signal to set a value inside a `Suspend`, it does not typically
    /// lead to the expected outcome.  Typically this signal may be used
    /// inside a resource to ensure the expected behavior, it does
    /// however lead to other unexpected behaviors.  All this will be
    /// better explained in the example usages and will be further
    /// elaborated on after.
    ///
    /// *Under SSR*, holding types of this around forever without
    /// notifying any of them will ensure the paired `ArcResource` wait
    /// forever.  Dropping this may result in all waiting subscriptions
    /// be notified of their release, if the underlying was created with
    /// the standard [`SsrSignalResource::new()`] constructor - this
    /// typically happens if this falls out of scope after not being
    /// used to notify - allowing the underlying [`ArcResource`] to stop
    /// waiting to return whatever value the underlying signal holds.
    ///
    /// If this was created by [`SsrSignalResource::new_must_notify()`],
    /// dropping of `SsrWriteSignal` will not notify, moreover, it arms
    /// the subscriber to wait until it's notified.  This means the
    /// underlying `ArcResource` will wait until any instances of
    /// related `SsrWriteSignal` to notify before the lock holding the
    /// `ArcResource` in wait be released.  Further explanations below
    /// typically assume the auto-notify from drop is in place.
    ///
    /// Upon creation of the wrapper, a `ReadySender` is acquire, which
    /// prevents the paired [`ArcResource`] from continuing, even upon
    /// being notified by the `CoReadyCoordinator` should this be held
    /// past then.  Typically, this happens if this is called in a
    /// resource fetcher closure, which doesn't drop until its `Future`
    /// terminates.
    ///
    /// Setting a value through the standard update methods (e.g.
    /// `set()`, `update()`) will ensure the wait lock within the
    /// underlying `ArcResource` be notified to continue, such that
    /// it will be allowed to return the value held by the underlying
    /// `ArcReadSignal`.
    ///
    /// The combination of the implicit actions described above is how
    /// how the signaling mechanism works, and misplacing the invocation
    /// of this function call will have the consequence of the resource
    /// either wait forever, or return the value it holds too early.
    /// See usage below for details.
    ///
    /// *Under CSR* this behaves exactly like an `ArcWriteSignal`, or
    /// the one returned by [`SsrSignalResource::inner_write_only`].
    ///
    /// # Usage
    /// This should only be invoked inside a resource closure, or in its
    /// `async` block but before any `.await` points - the reason for
    /// this is because the reactive system will immediately poll the
    /// `Future` once under SSR while keeping it around for further
    /// polling, which allow the `ReadySender` to get set up and remain
    /// alive, which prevents the `CoReadyCoordinator`'s notify from
    /// allowing the wait to resolve due to the active sender.  This
    /// keeps `ArcResource` on the other end waiting until a value is
    /// set with this, or this is otherwise dropped (unless the parent
    /// was created with [`SsrSignalResource::new_must_notify()`] as
    /// documented).
    ///
    /// The following usage examples shows how typical usage might look:
    ///
    /// ```
    /// # use std::sync::{Arc, Mutex};
    /// # use futures::StreamExt;
    /// # use leptos::prelude::*;
    /// # use leptos_sync_ssr::{component::SyncSsrSignal, signal::SsrSignalResource};
    /// # use leptos_sync_ssr::CoReadyCoordinator;
    /// #
    /// # tokio_test::block_on(async {
    /// #     let _ = any_spawner::Executor::init_tokio();
    /// #     let mut tasks = Arc::new(Mutex::new(vec![]));
    /// #     let owner = Owner::new();
    /// #     owner.set();
    /// #
    /// #     async fn some_future() {
    /// #         tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    /// #     }
    /// #
    /// #     let new_tasks = tasks.clone();
    /// #     let app = view! {
    /// #         <SyncSsrSignal setup=|| ()>{
    /// #             let mut new_tasks = new_tasks.lock().unwrap();
    /// let ssrsigres = SsrSignalResource::new(String::new());
    /// // it's assumed that its `read_only` resource passed elsewhere for use
    /// #
    /// #             let read_only = ssrsigres.read_only();
    /// #             // this is done to simply drive the future
    /// #             new_tasks.push(tokio::spawn(async move {
    /// #                 assert_eq!(read_only.await, "Hello world!");
    /// #             }));
    /// #
    /// let res = ArcResource::new(
    ///     || (),
    ///     {
    ///         let ssrsigres = ssrsigres.clone();
    ///         move |_| {
    ///             // This ensures a lock is acquired while still within the context
    ///             // of `SyncSsrSignal`
    ///             let ws = ssrsigres.write_only();
    ///             async move {
    ///                 // Invoking some other future
    ///                 let value = some_future().await;
    ///                 // Update/set the value, releasing the wait lock to allow
    ///                 // `SsrSignalResource::read_only()` to return with the this
    ///                 // value.
    ///                 ws.set("Hello world!".to_string());
    ///             }
    ///         }
    ///     }
    /// );
    /// #
    /// #             let read_only = ssrsigres.read_only();
    /// #             new_tasks.push(tokio::spawn(async move {
    /// #                 let foo = res.await;
    /// #             }));
    /// #         }</SyncSsrSignal>
    /// #     };
    /// #
    /// #     let _ = app.to_html();
    /// #     for task in Arc::into_inner(tasks).unwrap().into_inner().unwrap() {
    /// #         task.await.unwrap();
    /// #     }
    /// # });
    /// ```
    ///
    /// Acquiring the `SsrWriteSignal` after an await point will result
    /// in the underlying `ArcResource` not being able to wait for the
    /// value.  In the following failing example, the value provided by
    /// `ws.set()` will not be read under SSR by the underlying resource
    /// as the lock was not signaled as required in time.  Essentially,
    /// no `ReadySender` were available and this allow the wait to
    /// resolve when the `CoReadyCoordinator`'s notifies, allowing the
    /// underlying `ArcResource` to return the value before the expected
    /// one was assigned.
    ///
    /// ```should_panic
    /// # use std::sync::{Arc, Mutex};
    /// # use futures::StreamExt;
    /// # use leptos::prelude::*;
    /// # use leptos_sync_ssr::{component::SyncSsrSignal, signal::SsrSignalResource};
    /// # use leptos_sync_ssr::CoReadyCoordinator;
    /// #
    /// # tokio_test::block_on(async {
    /// #     let _ = any_spawner::Executor::init_tokio();
    /// #     let mut tasks = Arc::new(Mutex::new(vec![]));
    /// #     let owner = Owner::new();
    /// #     owner.set();
    /// #
    /// #     async fn some_future() {
    /// #         tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    /// #     }
    /// #
    /// #     let new_tasks = tasks.clone();
    /// #     let app = view! {
    /// #         <SyncSsrSignal setup=|| ()>{
    /// #             let mut new_tasks = new_tasks.lock().unwrap();
    /// let ssrsigres = SsrSignalResource::new(String::new());
    /// // it's assumed that its `read_only` resource passed elsewhere for use
    /// #
    /// #             let read_only = ssrsigres.read_only();
    /// #             // this is done to simply drive the future
    /// #             new_tasks.push(tokio::spawn(async move {
    /// #                 assert_eq!(read_only.await, "Hello world!");
    /// #             }));
    /// #
    /// let res = ArcResource::new(
    ///     || (),
    ///     {
    ///         let ssrsigres = ssrsigres.clone();
    ///         move |_| {
    ///             let ssrsigres = ssrsigres.clone();
    ///             async move {
    ///                 // Invoking some other future
    ///                 let value = some_future().await;
    ///                 // FIXME this should have came before the `.await` point in
    ///                 // order to ensure the subscriber held by the `ArcResource`
    ///                 // provided by the `SsrSignalResource` will wait.
    ///                 let ws = ssrsigres.write_only();
    ///                 // While this would update/set the value, it wouldn't be read
    ///                 // in time as the wait was released before the lock was
    ///                 // declared under SSR.
    ///                 ws.set("Hello world!".to_string());
    ///             }
    ///         }
    ///     }
    /// );
    /// #
    /// #             let read_only = ssrsigres.read_only();
    /// #             new_tasks.push(tokio::spawn(async move {
    /// #                 let foo = res.await;
    /// #             }));
    /// #         }</SyncSsrSignal>
    /// #     };
    /// #
    /// #     let _ = app.to_html();
    /// #     for task in Arc::into_inner(tasks).unwrap().into_inner().unwrap() {
    /// #         task.await.unwrap();
    /// #     }
    /// # });
    /// ```
    ///
    /// Notice how in the examples, the signals are used inside resource
    /// rather than a more natural (and expected) `Suspend`.  Reason for
    /// this is twofold - resources are futures that run concurrently
    /// while under SSR, but suspense are futures that run in sequence,
    /// one after another.  Given the point of this signal is to allow
    /// something later down the view tree to send a value used by
    /// earlier up the view tree, if an earlier part is locked waiting
    /// for the value, any suspense later will simply not be able to run
    /// by definition - this is a classic deadlock situation, where the
    /// earlier lock prevents the later unlock from happening.
    ///
    /// However, in practice, with the standard `SsrSignalResource`,
    /// calling this under SSR will not result in a deadlock, as the
    /// `Suspend` will be dropped during setup upon encounter of an
    /// `.await` point, this would typically release the lock and the
    /// result is that the underlying `ArcResource` may simply return
    /// the default value, not the expected one that gets set, much like
    /// a standard `RwSignal`.
    ///
    /// Setting a signal in a resource, however, introduces a different
    /// problem when hydration is involved.  When a given signal is set
    /// inside a resource, the result of that is hydrated and none of
    /// the code within the resource runs on the client - this includes
    /// the code that sets the signal.  The effect is that unexpected
    /// application behavior post-hydration vs CSR, as the underlying
    /// [`inner_read_only`](SsrSignalResource::inner_read_only) signal
    /// (backed by a `ArcRwSignal`) won't be written to upon hydration.
    /// This typically requires an additional work around to ensure this
    /// signal is written to with the expected value, and is something
    /// that [`PortletCtx`](crate::portlet::PortletCtx) does to ensure
    /// expected client-side behavior.  All of this is by design, refer
    /// to discussion under the GitHub issue [leptos-rs/leptos#4044](
    /// https://github.com/leptos-rs/leptos/issues/4044) for additional
    /// details.
    pub fn write_only(&self) -> SsrWriteSignal<T> {
        SsrWriteSignal {
            inner: Arc::new(SsrWriteSignalInner {
                signal_write: self.inner.signal_write.clone(),
                #[cfg(feature = "ssr")]
                ready_sender: self.inner.ready.to_ready_sender(),
            })
        }
    }

    /// Returns the inner `ArcReadSignal`.  This bypasses the
    /// asynchronous waiting mechanism ensured by the `ArcResource`.
    /// Typically this is used for diagnostic purposes.
    pub fn inner_read_only(&self) -> ArcReadSignal<T> {
        self.inner.signal_read.clone()
    }

    /// Returns the inner `ArcWriteSignal`.  Under SSR this bypasses the
    /// `ReadySender` mechanism, but otherwise is functionally the same
    /// as the [`SsrWriteSignal`].  Typically this is used to ensure the
    /// underlying signal is synchronized when hydrated , such as in the
    /// implementation of [`PortletCtx`](crate::portlet::PortletCtx).
    pub fn inner_write_only(&self) -> ArcWriteSignal<T> {
        self.inner.signal_write.clone()
    }
}

impl<T> Debug for SsrSignalResource<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_struct("SsrSignalResource")
            .field("read_only", &self.inner.resource)
            .field("write_only", &self.inner.signal_write)
            .finish()
    }
}

// it was thought that a customized guard need to be done, but it turns out
// eventually having the `SsrWriteSignal` dropping eventually is enough.
impl<T: 'static> Write for SsrWriteSignal<T> {
    type Value = T;

    fn try_write(&self) -> Option<impl UntrackableGuard<Target = Self::Value>> {
        // Include the use of our notifier and create our own guard with it
        let notifier = SsrWriteSignalNotifier {
            inner: self.inner.clone(),
        };
        self.inner
            .signal_write
            .try_write_untracked()
            .map(|guard| WriteGuard::new(notifier, guard))
    }

    #[allow(refining_impl_trait)]
    fn try_write_untracked(&self) -> Option<UntrackedWriteGuard<Self::Value>> {
        self.inner.signal_write.try_write_untracked()
    }
}

impl<T> DefinedAt for SsrWriteSignal<T> {
    fn defined_at(&self) -> Option<&'static Location<'static>> {
        // TODO just simply leverage the underlying implementation;
        // TODO figure out if we want to actually implement this
        self.inner.signal_write.defined_at()
    }
}

impl<T> IsDisposed for SsrWriteSignal<T> {
    #[inline(always)]
    fn is_disposed(&self) -> bool {
        false
    }
}

impl<T> Notify for SsrWriteSignal<T> {
    fn notify(&self) {
        self.inner.signal_write.notify();
        // assume when this is marked dirty, a change has happened and so it
        // is now safe for the reader to continue execution
        #[cfg(feature = "ssr")]
        self.inner.ready_sender.complete();
    }
}

impl<T> Notify for SsrWriteSignalNotifier<T> {
    fn notify(&self) {
        self.inner.signal_write.notify();
        // assume when this is marked dirty, a change has happened and so it
        // is now safe for the reader to continue execution
        #[cfg(feature = "ssr")]
        self.inner.ready_sender.complete();
    }
}
