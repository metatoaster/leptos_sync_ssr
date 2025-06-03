use std::{
    ops::{Deref, DerefMut},
    panic::Location,
    sync::Arc,
};

use leptos::prelude::*;
use leptos::reactive::{
    traits::{DefinedAt, IntoInner, IsDisposed, Notify, UntrackableGuard, Write},
    signal:: guards::UntrackedWriteGuard,
};
use serde::{de::DeserializeOwned, Serialize};

use crate::ready::{CoReady, ReadySender};

/// Provides a signal-resource pairing that together works to provide an
/// asynchronously waitable read signal (through the resource) under
/// SSR, where upon acquisition of the write-only side will ensure the
/// resource wait for the value be written (or the write signal be
/// disposed of) before continuing execution.
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
/// This can cause the paired resource be stuck in waiting until this or
/// its copies are dropped.
#[derive(Clone)]
pub struct SsrWriteSignal<T>
where
    T: 'static,
{
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
        let (signal_read, signal_write) = arc_signal(value);

        let resource = ArcResource::new(
            {
                let signal_read = signal_read.clone();
                move || signal_read.get()
            },
            {
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
    /// Under SSR, the provided resource will asynchronously wait until
    /// any paired [`SsrWriteSignal`] provides the value or is dropped.
    /// This would also finish waiting of the enclosing `SyncSsrSignal`
    /// component is done processing without a `SsrWriteSignal` being
    /// paired, where the underlying value (typically the default value
    /// used to create the [`SsrSignalResource`]) is returned.
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
        self.ready_sender.complete();
    }
}
