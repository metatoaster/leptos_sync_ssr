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
    pub fn new(value: T) -> Self {
        Self {
            inner: SsrSignalResourceInner::new(value).into(),
        }
    }
}

impl<T> SsrSignalResource<T> {
    pub fn read_only(&self) -> ArcResource<T> {
        self.inner.resource.clone()
    }

    pub fn write_only(&self) -> SsrWriteSignal<T> {
        // the issue is that the desired version is a combined one, such
        // that we can pull out the write signal where that will have
        // the drop impl as that version is the complete one.
        // e.g. only when clone
        SsrWriteSignal {
            write_signal: self.inner.signal_write.clone(),
            ready_sender: self.inner.ready.to_ready_sender(),
        }
    }
}

impl<T: 'static> Write for SsrWriteSignal<T> {
    type Value = T;

    // TODO need to wrap the guard with our version that will drop the ready_sender
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
