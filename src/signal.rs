//! Provides the signal-resource pairing for synchronized SSR.
use std::{
    ops::{Deref, DerefMut},
    panic::Location,
    sync::Arc,
};

use leptos::{
    reactive::{
        traits::{DefinedAt, Get, GetUntracked, IntoInner, IsDisposed, Notify, UntrackableGuard, Write},
        signal::{ArcRwSignal, ArcWriteSignal, guards::UntrackedWriteGuard},
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
/// also.
///
/// Note that this type can only be created inside components that have
/// have the [`CoReadyCoordinator`](crate::ready::CoReadyCoordinator)
/// be provided as a context, which typically involves having the
/// [`SyncSsrSignal`](crate::component::SyncSsrSignal) component be one
/// of the ancestors of the component in the view tree.
#[derive(Clone)]
pub struct SsrSignalResource<T>
where
    T: 'static,
{
    inner: Arc<SsrSignalResourceInner<T>>
}

struct SsrSignalResourceInner<T>
where
    T: 'static,
{
    #[cfg(feature = "ssr")]
    ready: CoReady,
    signal_write: ArcWriteSignal<T>,
    resource: ArcResource<T>,
}

/// The write signal created by [`SsrSignalResource::write_only`].
///
/// When created before the `CoReadyCoordinator` notified is invoked,
/// it will cause the paired resource to wait until a value is set
/// through any of trait methods for updates or that this is dropped.
pub struct SsrWriteSignal<T>
where
    T: 'static,
{
    #[cfg(feature = "ssr")]
    ready_sender: ReadySender,
    write_signal: ArcWriteSignal<T>,
}

impl<T> SsrSignalResourceInner<T>
where
    T: Clone + Send + Sync + PartialEq + Serialize + DeserializeOwned,
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
                move |_| {
                    #[cfg(feature = "ssr")]
                    let subscriber = ready.subscribe();
                    let signal_read = signal_read.clone();
                    async move {
                        // TODO need to insert debug to check number of broadcast/waiters
                        #[cfg(feature = "ssr")]
                        subscriber.wait().await;
                        signal_read.get_untracked()
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
    T: Clone + Send + Sync + PartialEq + Serialize + DeserializeOwned,
{
    /// Creates a signal-resource pairing with the value of type `T`.
    ///
    /// Typical use case is to clone this to where they are needed so
    /// that the read-only and write-only ends may be acquired for
    /// usage.
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

    /// Acquire a wrapper containing the underlying `ArcWriteSignal`
    /// side of the pairing.
    ///
    /// Under SSR, holding copies of this while without dropping any of
    /// them will ensure the paired `ArcResource` wait forever.
    ///
    /// Setting a value through the standard update methods (e.g.
    /// `set()`, `update()`) will ensure the resource be notified that
    /// it should continue.
    ///
    /// Upon dropping of this, which typically happens when the setter
    /// is dropped out of scope, will also notify the resource that it
    /// may return whatever value it holds.
    ///
    /// Under CSR this behaves exactly like an `ArcWriteSignal`.
    pub fn write_only(&self) -> SsrWriteSignal<T> {
        SsrWriteSignal {
            write_signal: self.inner.signal_write.clone(),
            #[cfg(feature = "ssr")]
            ready_sender: self.inner.ready.to_ready_sender(),
        }
    }
}

// it was thought that a customized guard need to be done, but it turns out
// eventually having the `SsrWriteSignal` dropping eventually is enough.
impl<T: 'static> Write for SsrWriteSignal<T> {
    type Value = T;

    fn try_write(&self) -> Option<impl UntrackableGuard<Target = Self::Value>> {
        self.write_signal.try_write()
    }

    #[allow(refining_impl_trait)]
    fn try_write_untracked(&self) -> Option<UntrackedWriteGuard<Self::Value>> {
        self.write_signal.try_write_untracked()
    }
}

impl<T> DefinedAt for SsrWriteSignal<T> {
    fn defined_at(&self) -> Option<&'static Location<'static>> {
        // TODO just simply leverage the underlying implementation;
        // TODO figure out if we want to actually implement this
        self.write_signal.defined_at()
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
        self.write_signal.notify();
        // assume when this is marked dirty, a change has happened and so it
        // is now safe for the reader to continue execution
        #[cfg(feature = "ssr")]
        self.ready_sender.complete();
    }
}
