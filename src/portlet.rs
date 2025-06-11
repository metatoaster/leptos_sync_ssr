//! Provides generic helpers to build portlets on Leptos
//!
//! A common UI design patterns that involve having common, optionally rendered
//! comopnents rendered on a page; in the context of web portals, this concept
//! is known as portlets.  Portlets may display additional information from the
//! main article, and its placement may come earlier in the view tree.  This
//! has the consequence where a standard signal like `RwSignal` may fail to
//! return the expected value under SSR as it would be rendered before the data
//! provided by the article is available.
//!
//! This module provides [`PortletCtx`] which contains a few methods that works
//! together to implement the portlet UI pattern in a fully managed manner.
//! Given that it makes use of [`SsrSignalResource`] internally, the resulting
//! component responsible for the rendering may be placed anywhere on the view
//! tree, as the resource providing the data will wait for the signal be
//! written to first, and only if necessary to not lock the rendering up when
//! under SSR.  Naturally, a [`SyncSsrSignal`](crate::component::SyncSsrSignal)
//! must be placed in a higher level of the view tree before `PortletCtx` may
//! be [provided](PortletCtx::provide) as a context.

use std::{future::Future, sync::Arc};

use leptos::prelude::*;
use leptos::server_fn::error::FromServerFnError;

use crate::signal::{SsrSignalResource, SsrWriteSignal};

/// A generic portlet context.
///
/// Internally this contains an [`SsrSignalResource<Option<T>>`].  While no
/// direct access to that underlying is provided, its write signal may be
/// accessed through [`PortletCtx::expect_write`], subjected to the usual
/// guidelines around the use of `SsrSignalResource`.  The only way this may be
/// constructed is through the [`PortletCtx::provide`] method to encourage
/// consistent usage pattern.
#[derive(Clone, Debug)]
pub struct PortletCtx<T> {
    inner: SsrSignalResource<Option<T>>,
}

impl<T> PortletCtx<T>
where
    T: serde::Serialize
        + serde::de::DeserializeOwned
        + Clone
        + PartialEq
        + Send
        + Sync
        + IntoRender
        + 'static,
    <T as leptos::prelude::IntoRender>::Output: RenderHtml + Send + 'static,
    // ArcResource<Result<Option<T>, E>>: IntoFuture<Output = Result<Option<T>, E>>,
    // <ArcResource<Result<Option<T>, E>> as IntoFuture>::IntoFuture: Send,
    Suspend<Option<AnyView>>: RenderHtml + Render,
{
    /// Clears the value for the portlet.
    ///
    /// Upon invocation of this method, the rendering of the portlet
    /// will be `None` through the associated function [`render`](
    /// PortletCtx::render), which functions to render nothing, hence
    /// implements the optionally rendered part.
    pub fn clear(&self) {
        self.inner.inner_write_only().set(None);
    }

    /// Provide this as a context for a Leptos `App`.
    ///
    /// The reason why there is no constructor provided and only done so
    /// via signal is to have the ability for these contexts to be
    /// provided as singletons, as portlets are typically unique for
    /// ease of management.
    ///
    /// ## Panics
    /// Given the use of `SsrSignalResource`, this panics if the context
    /// type `CoReadyCoordinator` is not found in the current reactive
    /// owner or its ancestors.  This may be resolved by providing the
    /// context by nesting this function call inside the
    /// [`<SyncSsrSignal/>`](crate::component::SyncSsrSignal) component.
    pub fn provide() {
        // TODO ensure the singleton aspect.
        provide_context(PortletCtx::<T> {
            inner: SsrSignalResource::new(None),
        });
    }

    /// Update the portlet with the provided data fetcher.
    ///
    /// Similar to the fetcher for typical `Resource`s, this use it to
    /// generates a new `Future` to get the latest data.  When created
    /// within a reactive context, any invocation of [`.track()`](
    /// leptos::reactive::traits::Track::track) on /any reactive data
    /// will result in the expected reactivity.
    ///
    /// Internally, the full functionality of [`SsrSignalResource`] is
    /// only used under SSR, as the usage of the underlying locks must
    /// be used with `ArcResource`, but given the idea is that this
    /// wraps a signal, under CSR (well, after await on the applicable
    /// resources) the signals are written to directly.
    ///
    /// This implementation is even more complex than just using the
    /// `SsrSignalResource` directly, however, the end result is that
    /// under CSR only the standard `ArcRwSignal` is what's effectively
    /// used.
    pub fn update_with<Fut>(&self, fetcher: impl Fn() -> Fut + Send + Sync + 'static) -> impl IntoView
    where
        Fut: Future<Output = Option<T>> + Send + 'static,
    {
        let ctx = self.clone();
        // This fetcher will need to be called inside a resource first as it
        // reconfigures the underlying `SsrSignalResource` to manual release
        // mode upon acquisition of the `SsrWriteSignal` - this ensures the
        // `ArcResource` on the other end will only unlock when signaled, which
        // the following resource will as it directly leads to `.set()` being
        // called to signal the unlock.
        let fetcher = Arc::new(fetcher);
        // Note this resource only used on the server - the fetcher is invoked
        // again directly to write to underlying `ArcWriteSignal` directly, and
        // this second invocation will not be problematic as the same data is
        // being written to for the second time under SSR, and for the first
        // (and only) time under hydrate/CSR which would set the underlying
        // signal with the real expected value without the other end waiting.
        let res = ArcResource::new(
            || (),
            {
                let ctx = ctx.clone();
                let fetcher = fetcher.clone();
                move |_| {
                    let ws = ctx.inner.write_only_manual();
                    let fut = fetcher();
                    async move {
                        ws.set(fut.await);
                    }
                }
            }
        );
        view! {
            <Suspense>{
                let ctx = ctx.clone();
                let fetcher = fetcher.clone();
                move || {
                    #[cfg(feature = "ssr")]
                    let res = res.clone();
                    let ctx = ctx.clone();
                    let fut = fetcher();
                    Suspend::new(async move {
                        // Again, only under SSR we need the unlock signal.
                        // Under CSR, this signal is absent so use the fetcher
                        // directly to acquire the future to acquire the value
                        // to update
                        #[cfg(feature = "ssr")]
                        res.await;
                        // This must be done normally anyway to ensure the
                        // read signal is updated on the other end.
                        ctx.inner.inner_write_only().set(fut.await);
                    })
                }
            }</Suspense>
        }
    }

    /// A generic portlet renderer via this generic portlet context.
    ///
    /// This renderer simplifies the creation of portlet components based
    /// upon the underlying `T`, for all `T` that implements `IntoRender`.
    /// For usage in a Leptos `App`, it expects that the [`PortletCtx<T>`]
    /// be [provided](PortletCtx::provide).  The underlying
    /// [`SsrSignalResource`] will ensure any correctly set value be rendered,
    /// provided that the component invoking this has the required
    /// [`SyncSsrSignal`](crate::component::SyncSsrSignal) up its view tree.
    ///
    /// Typical usage may look like this.
    ///
    /// ```
    /// # use leptos::{
    /// #     prelude::{AnyView, IntoAny, IntoRender, ServerFnError},
    /// #     component, view, IntoView,
    /// # };
    /// # use leptos_sync_ssr::portlet::PortletCtx;
    /// #
    /// # #[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
    /// # struct Nav;
    /// #
    /// # impl IntoRender for Nav {
    /// #     type Output = AnyView;
    /// #
    /// #     fn into_render(self) -> Self::Output {
    /// #         view! {
    /// #             todo!()
    /// #         }
    /// #         .into_any()
    /// #     }
    /// # }
    /// #
    /// #[component]
    /// pub fn NavPortlet() -> impl IntoView {
    ///     <PortletCtx<Nav>>::render()
    /// }
    /// ```
    ///
    /// ## Panics
    /// Panics if the underlying `PortletCtx<T>` is not found.
    pub fn render() -> impl IntoView {
        let ctx = expect_context::<PortletCtx<T>>();
        // The resource must be used and not the underlying `ArcReadSignal`,
        // hydration error results otherwise.
        let resource = ctx.inner.read_only();
        let suspend = move || {
            let resource = resource.clone();
            Suspend::new(async move {
                // While it is be possible to use the inner `ArcReadSignal`
                // under CSR, with hydration this can be problematic given
                // that the value may be set later in the view tree but with
                // the hydration done early this causes a mismatch.  This
                // following match will only await for the resource when
                // necessary, but given this doubles the use of `None` for
                // "missing" and "not ready", this isn't probably the best.
                // Leaving this in place for some future consideration.
                //
                // match ctx.inner.inner_read_only().get() {
                //     Some(v) => Some(v),
                //     None => ctx.inner.read_only().await,
                // };
                Some(resource.await?
                    .into_render()
                    .into_any())
            })
        };
        view! { <Transition>{move || suspend() }</Transition> }
    }
}
