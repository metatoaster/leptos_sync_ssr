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
	     ArcRwSignal, ArcWriteSignal,
            guards::{WriteGuard, UntrackedWriteGuard},
        },
    },
    server::ArcResource,
};
use serde::{de::DeserializeOwned, Serialize};

#[cfg(feature = "ssr")]
use crate::ready::{CoReady, ReadySender};

/// Provides a signal-resource pairing that together works to provide an
/// asynchronously waitable read signal (through the resource) under
/// SSR.
///
/// The read-only resource will wait upon acquisition of the write-only
/// signal, as this will ensure the resource produce the intended value
/// under SSR to ensure the expected content be rendered and to allow
/// hydration to happen correctly.  Should the write-only signal be
/// dropped, the resource will be permitted to return the value it holds
/// also.  The documentation under [`SsrSignalResource::write_only`]
/// goes in-depth on how to use it correctly to ensure the read-only
/// resource to return with the expected value that may be provided by
/// the write-only signal.
///
/// Note that this type can only be created inside components that have
/// have the [`CoReadyCoordinator`](crate::ready::CoReadyCoordinator)
/// be provided as a context, which typically involves having the
/// [`<SyncSsrSignal/>`](crate::component::SyncSsrSignal) component be
/// one of the ancestors of the component in the view tree.
#[derive(Clone)]
pub struct SsrSignalResource<T> {
    inner: Arc<SsrSignalResourceInner<T>>
}

struct SsrSignalResourceInner<T> {
    #[cfg(feature = "ssr")]
    ready: CoReady,
    resource: ArcResource<T>,
    signal_write: ArcWriteSignal<T>,
}

/// The write signal created by [`SsrSignalResource::write_only`].
///
/// When created before the `CoReadyCoordinator` notified is invoked, it
/// will cause the paired resource to wait until a value is set through
/// any of trait methods for updates or that this is dropped.
pub struct SsrWriteSignal<T> {
    inner: Arc<SsrWriteSignalInner<T>>,
}

struct SsrWriteSignalNotifier<T> {
    inner: Arc<SsrWriteSignalInner<T>>,
}

struct SsrWriteSignalInner<T> {
    #[cfg(feature = "ssr")]
    ready_sender: ReadySender,
    write_signal: ArcWriteSignal<T>,
}

impl<T> SsrSignalResourceInner<T>
where
    T: Clone + Send + Sync + PartialEq + Serialize + DeserializeOwned + 'static,
{
    #[track_caller]
    fn new(value: T) -> Self {
        #[cfg(feature = "ssr")]
        let ready = CoReady::new();
        let (signal_read, signal_write) = ArcRwSignal::new(value).split();

        let resource = ArcResource::new(
            {
                let signal_read = signal_read.clone();
                move || signal_read.get()
            },
            {
                #[cfg(feature = "ssr")]
                let ready = ready.clone();
                move |original| {
                    #[cfg(feature = "ssr")]
                    let subscriber = ready.subscribe();
                    let signal_read = signal_read.clone();
                    async move {
                        #[cfg(feature = "ssr")]
                        subscriber.wait().await;
                        // given that the signal may provide a different value
                        // to what was originally passed by the time the
                        // subscriber finishes waiting, try to get a new value.
                        // using `try_get_untracked` to work around potential
                        // disposal issues.
                        signal_read.try_get_untracked().unwrap_or(original)
                    }
                }
            },
        );

        Self {
            #[cfg(feature = "ssr")]
            ready: ready,
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
    /// usage.
    ///
    /// ## Panics
    /// Panics if the context of type `CoReadyCoordinator` is not found
    /// in the current reactive owner or its ancestors.  This may be
    /// resolved by providing the context by nesting this inside the
    /// [`<SyncSsrSignal/>`](crate::component::SyncSsrSignal) component.
    #[track_caller]
    pub fn new(value: T) -> Self {
        Self {
            inner: SsrSignalResourceInner::new(value).into(),
        }
    }
}

impl<T> SsrSignalResource<T> {
    /// Acquire the underlying `ArcResource` side of the pair.
    ///
    /// Under SSR, the underlying resource will asynchronously wait
    /// until any paired [`SsrWriteSignal`] provides the value or is
    /// dropped, where the provided value will be returned.
    ///
    /// The resource will also return the underlying value (typically
    /// the default value used to create the [`SsrSignalResource`])
    /// should the enclosing `SyncSsrSignal` component is done
    /// processing without a `SsrWriteSignal` being paired.
    ///
    /// Under CSR no waiting would happen and so the underlying resource
    /// should act like an indirect ArcReadSignal.
    pub fn read_only(&self) -> ArcResource<T> {
        self.inner.resource.clone()
    }

    /// Acquire a `SsrWriteSignal`, which is a wrapper containing the
    /// underlying `ArcWriteSignal` side of the pairing, along with a
    /// `ReadySender` when under SSR.
    ///
    /// *Under SSR*, holding types of this while without dropping any
    /// of them will ensure the paired `ArcResource` wait forever.
    ///
    /// Upon creation of the wrapper, a `ReadySender` is acquire, which
    /// will prevent the paired resource from continuing upon receiving
    /// the notification from the `CoReadyCoordinator` that it may be
    /// safe to continue as it is no longer the case (due to this being
    /// the live write signal.  Upon drop of the wrapper, it will also
    /// notify the resource that it should return the value.  These two
    /// implicit features alone is how the signaling mechanism works,
    /// and misplacing the invocation of this function call will have
    /// the consequence of the resource returning the value it holds too
    /// early.  See usage below for details.
    ///
    /// Setting a value through the standard update methods (e.g.
    /// `set()`, `update()`) will ensure the resource be notified that
    /// it should continue.
    ///
    /// Upon dropping of this, which typically happens when the setter
    /// is dropped out of scope, will also notify the resource that it
    /// may return whatever value it holds.
    ///
    /// *Under CSR* this behaves exactly like an `ArcWriteSignal`.
    ///
    /// # Usage
    /// This should only be invoked inside a resource closure (or the
    /// `Future`), but before any `.await` points - the reason for this
    /// because the reactive system will immediately poll the `Future`
    /// once under SSR while keeping it around for further polling.
    /// This simply allow the allows the paired resource to stay in a
    /// waiting state rather than simply returning when the coordinator
    /// notifies, and this being kept alive means it also won't
    /// prematurely allow the resource to stop waiting prematurely.
    /// The following usage examples shows how this might look:
    ///
    /// ```
    /// # use std::sync::{Arc, Mutex};
    /// # use futures::StreamExt;
    /// # use leptos::prelude::*;
    /// # use leptos_sync_ssr::{component::SyncSsrSignal, signal::SsrSignalResource};
    /// # use leptos_sync_ssr::CoReadyCoordinator;
    /// # tokio_test::block_on(async {
    /// #     let _ = any_spawner::Executor::init_tokio();
    /// #     let mut tasks = Arc::new(Mutex::new(vec![]));
    /// #     let owner = Owner::new();
    /// #     owner.set();
    /// #     let some_other_resource = ArcResource::new(
    /// #         || (),
    /// #         move |_| { async move {
    /// #             tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    /// #         } }
    /// #     );
    /// #     let new_tasks = tasks.clone();
    /// #     view! {
    /// #         <SyncSsrSignal>{
    /// #             let mut new_tasks = new_tasks.lock().unwrap();
    /// #             let ssrsigres = SsrSignalResource::new(String::new());
    /// #             let read_only = ssrsigres.read_only();
    /// #             // this is done to simply drive the future
    /// #             new_tasks.push(tokio::spawn(async move {
    /// #                 assert_eq!(read_only.await, "Hello world!");
    /// #             }));
    /// let res = ArcResource::new(
    ///     || (),
    ///     {
    ///         let ssrsigres = ssrsigres.clone();
    ///         let some_other_resource = some_other_resource.clone();
    ///         move |_| {
    ///             let ws = ssrsigres.write_only();
    ///             let some_other_resource = some_other_resource.clone();
    ///             async move {
    ///                 // Some future
    ///                 let value = some_other_resource.await;
    ///                 // update/set the value
    ///                 ws.set("Hello world!".to_string());
    ///             }
    ///         }
    ///     }
    /// );
    /// #             let read_only = ssrsigres.read_only();
    /// #             new_tasks.push(tokio::spawn(async move {
    /// #                 let foo = res.await;
    /// #             }));
    /// #         }</SyncSsrSignal>
    /// #     };
    /// #     for task in Arc::into_inner(tasks).unwrap().into_inner().unwrap() {
    /// #         task.await.unwrap();
    /// #     }
    /// # });
    /// ```
    pub fn write_only(&self) -> SsrWriteSignal<T> {
        SsrWriteSignal {
            inner: Arc::new(SsrWriteSignalInner {
                write_signal: self.inner.signal_write.clone(),
                #[cfg(feature = "ssr")]
                ready_sender: self.inner.ready.to_ready_sender(),
            })
        }
    }

    pub fn write_only_untracked(&self) -> ArcWriteSignal<T> {
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
            .write_signal
            .try_write_untracked()
            .map(|guard| WriteGuard::new(notifier, guard))
    }

    #[allow(refining_impl_trait)]
    fn try_write_untracked(&self) -> Option<UntrackedWriteGuard<Self::Value>> {
        self.inner.write_signal.try_write_untracked()
    }
}

impl<T> DefinedAt for SsrWriteSignal<T> {
    fn defined_at(&self) -> Option<&'static Location<'static>> {
        // TODO just simply leverage the underlying implementation;
        // TODO figure out if we want to actually implement this
        self.inner.write_signal.defined_at()
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
        self.inner.write_signal.notify();
        // assume when this is marked dirty, a change has happened and so it
        // is now safe for the reader to continue execution
        #[cfg(feature = "ssr")]
        self.inner.ready_sender.complete();
    }
}

impl<T> Notify for SsrWriteSignalNotifier<T> {
    fn notify(&self) {
        leptos::logging::log!("[!] SsrWriteSignalNotifier::notify");
        self.inner.write_signal.notify();
        // assume when this is marked dirty, a change has happened and so it
        // is now safe for the reader to continue execution
        #[cfg(feature = "ssr")]
        self.inner.ready_sender.complete();
    }
}
