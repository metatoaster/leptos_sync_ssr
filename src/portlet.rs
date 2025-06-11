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

use std::future::Future;

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
        self.inner.write_only_untracked().set(None);
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
    /// Similar to resources, the fetcher generates a new `Future` to
    /// get the latest data, and when created within a reactive context,
    /// calling `.track()`[leptos::reactive::traits::Track::track] on
    /// any reactive data being used will ensure they be tracked for
    /// updates to ensure reactivity.  The result is that whenever some
    /// data is returned, that will be used to render the portlet.  As
    /// this is intended to work with the reactive system, this returns
    /// a view that should be plugged into the view tree to be returned
    /// by the component that intends to activate the portlet.
    pub fn update_with<Fut>(&self, fetcher: impl Fn() -> Fut + Send + Sync + 'static) -> impl IntoView
    where
        Fut: Future<Output = Option<T>> + Send + 'static,
    {
        let ctx = self.clone();
        let res = ArcResource::new(
            || (),
            {
                let ctx = ctx.clone();
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
                move || {
                    let res = res.clone();
                    let ctx = ctx.clone();
                    Suspend::new(async move {
                        res.await;
                        // This additional round-tripping seems redundant,
                        // but is absolutely vital to ensure the original
                        // value in the signal is also reflected under
                        // hydration - while the resource will reflect the
                        // later value because it can wait, the original
                        // signal would have the default value as the
                        // resources aren't run again during hydration,
                        // and this descrepancy will not be resolved until
                        // the signal is finally written to by chance of
                        // user's interaction, which by that point it may
                        // already result in difference in observed app
                        // behavior between SSR+hydrate and CSR.
                        //
                        // TODO maybe under CSR, we can skip the resource?
                        if let Some(value) = ctx.inner.read_only().get_untracked() {
                            ctx.inner.write_only_untracked().set(value);
                        }
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
        let resource = ctx.inner.read_only();
        let suspend = move || {
            let resource = resource.clone();
            Suspend::new(async move {
                let result = resource.await;
                Some(result?
                    .into_render()
                    .into_any())
            })
        };

        view! { <Transition>{move || suspend() }</Transition> }
    }
}
