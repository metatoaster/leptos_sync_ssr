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
//! together to implement the portlet UI pattern in a largely managed manner.
//! Given that it makes use of [`SsrSignalResource`] internally, the resulting
//! component responsible for the rendering may be placed anywhere on the view
//! tree, as the resource providing the data will wait for the signal be
//! written to first, and only if necessary to not lock the rendering up when
//! under SSR.  Naturally, a [`SyncSsrSignal`](crate::component::SyncSsrSignal)
//! must be placed in a higher level of the view tree before `PortletCtx` may
//! be [provided](PortletCtx::provide) as a context.

use std::future::Future;

use leptos::{
    prelude::{
        expect_context, provide_context, AnyView, IntoAny, IntoRender, Render, RenderHtml, Suspend,
    },
    reactive::{signal::ArcWriteSignal, traits::Set},
    server::ArcResource,
    suspense::Transition,
    view, IntoView,
};

use crate::signal::SsrSignalResource;

/// A generic portlet context.
///
/// Internally this contains an [`SsrSignalResource<Option<T>>`].  While no
/// direct access to that underlying is provided, its write signal may be
/// indirectly used through [`PortletCtx::set_with`], where the guidelines
/// around the use of `SsrSignalResource` are completely followed to ensure
/// the expected usage and end-user experience.  The only way this may be
/// constructed is through the [`PortletCtx::provide`] method to encourage
/// a consistent usage pattern.
///
/// Code examples below are modified code snippets from the [`nav_portlet`](
/// https://github.com/metatoaster/leptos_sync_ssr/tree/main/example/nav_portlet)
/// example.
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
        + 'static,
{
    /// Provide this as a context for a Leptos `App`.
    ///
    /// The reason why there is no constructor provided and only done so
    /// via signal is to have the ability for these contexts to be
    /// provided as singletons, as portlets are typically unique for
    /// ease of management.
    ///
    /// Typical usage may look something like this:
    ///
    /// ```
    /// use leptos::prelude::*;
    /// use leptos_router::{
    ///     components::{Route, Router, Routes},
    ///     path,
    /// };
    /// use leptos_sync_ssr::{component::SyncSsrSignal, portlet::PortletCtx};
    ///
    /// # #[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
    /// # struct Nav;
    /// #
    /// #[component]
    /// pub fn App() -> impl IntoView {
    ///     let fallback = || view! { "Page not found." }.into_view();
    ///     // This would panic here
    ///     // <PortletCtx<Nav>>::provide();
    ///     view! {
    ///         <Router>
    ///             <SyncSsrSignal setup=|| {
    ///                 // This is fine as this is inside `<SyncSsrSignal/>`.
    ///                 <PortletCtx<Nav>>::provide();
    ///                 // Other provides and other setup code may go here.
    ///             }>
    ///                 <header>
    ///                     // The portlet component, refer to documentation on
    ///                     // `.render()` for how this component may be defined.
    ///                     <NavPortlet/>
    ///                 </header>
    ///                 <article>
    ///                     <Routes fallback>
    ///                         <Route path=path!("") view=HomePage/>
    ///                         <Route path=path!("authors") view=AuthorListing/>
    ///                         // plus other routes
    ///                     </Routes>
    ///                 </article>
    ///             </SyncSsrSignal>
    ///         </Router>
    ///     }
    /// }
    /// #
    /// # #[component]
    /// # pub fn AuthorListing() -> impl IntoView {
    /// #     ()
    /// # }
    /// #
    /// # #[component]
    /// # pub fn HomePage() -> impl IntoView {
    /// #     ()
    /// # }
    /// #
    /// # #[component]
    /// # pub fn NavPortlet() -> impl IntoView {
    /// #     ()
    /// # }
    /// #
    /// # #[cfg(feature = "ssr")]
    /// # tokio_test::block_on(async {
    /// #     use leptos_router::location::RequestUrl;
    /// #     let _ = any_spawner::Executor::init_tokio();
    /// #     let owner = Owner::new();
    /// #     owner.set();
    /// #     provide_context(RequestUrl::new(""));
    /// #     let _ = view! { <App/> }.to_html();
    /// # });
    /// ```
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

    /// Alias for [`expect_context::<PortletCtx<T>>()`](expect_context).
    ///
    /// ## Panics
    /// Panics if `PortletCtx<T>` is not found in the current reactive
    /// owner or its ancestors.
    pub fn expect() -> PortletCtx<T> {
        expect_context::<PortletCtx<T>>()
    }

    /// Set the portlet with the provided data fetcher.
    ///
    /// This helper function returns a view that should be added to the
    /// view tree such that the desired set function can be effected.
    /// See [`SsrSignalResource::set_with`] for full documentation.
    ///
    /// Typical usage may look like this.
    ///
    /// ```
    /// # use leptos::{
    /// #     prelude::{ServerFnError, expect_context},
    /// #     server::ArcResource,
    /// #     component, view, IntoView,
    /// # };
    /// # use leptos_sync_ssr::portlet::PortletCtx;
    /// #
    /// # #[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
    /// # struct Author;
    /// # #[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
    /// # struct Nav;
    /// #
    /// #[component]
    /// pub fn AuthorListing() -> impl IntoView {
    ///     let authors = expect_context::<ArcResource<Result<Vec<(String, Author)>, ServerFnError>>>();
    ///     let nav_ctx = expect_context::<PortletCtx<Nav>>();
    ///
    ///     #[cfg(not(feature="ssr"))]
    ///     on_cleanup({
    ///         let nav_ctx = nav_ctx.clone();
    ///         move || nav_ctx.clear()
    ///     });
    ///
    ///     view! {
    ///         // This ensures `PortletCtx<Nav>` is updated with data provided by
    ///         // `authors`.
    ///         {nav_ctx.set_with(move || {
    ///             let authors = authors.clone();
    ///             // Optionally ensure updates to the authors resource are tracked;
    ///             // this particular usage is a current workaround.
    ///             // See: https://github.com/leptos-rs/leptos/pull/4061
    ///             // #[cfg(not(feature = "ssr"))]
    ///             // authors.track();
    ///             async move {
    ///                 authors.await
    ///                     // TODO conversion of list of authors to `Nav` type
    ///                     .map(|authors| todo!())
    ///                     .ok()
    ///             }
    ///         })}
    ///         <div>
    ///             // Other components/elements.
    ///         </div>
    ///     }
    /// }
    /// ```
    ///
    /// Note that this method returns a `Suspense`, which should be
    /// included into the view tree to be returned by the component like
    /// in the above example, as that would ensure the update happen as
    /// the component renders.
    pub fn set_with<Fut>(&self, fetcher: impl Fn() -> Fut + Send + Sync + 'static) -> impl IntoView
    where
        Fut: Future<Output = Option<T>> + Send + 'static,
    {
        self.inner.set_with(fetcher)
    }

    /// Update the portlet with the provided data fetcher and the
    /// updater function.
    ///
    /// This helper function returns a view that should be added to the
    /// view tree such that the desired set function can be effected.
    /// See [`SsrSignalResource::update_with`] for full documentation.
    ///
    /// Typical usage may look like this.
    ///
    /// ```
    /// # use leptos::{
    /// #     prelude::{ServerFnError, expect_context},
    /// #     server::ArcResource,
    /// #     component, view, IntoView,
    /// # };
    /// # use leptos_sync_ssr::portlet::PortletCtx;
    /// #
    /// # #[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
    /// # struct Article;
    /// # #[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
    /// # struct History;
    /// #
    /// # impl History {
    /// #     fn push(&mut self, article: Article) {
    /// #     }
    /// # }
    /// #
    /// #[component]
    /// pub fn ArticleListing() -> impl IntoView {
    ///     let article = expect_context::<ArcResource<Result<Article, ServerFnError>>>();
    ///     let history = expect_context::<PortletCtx<History>>();
    ///
    ///     view! {
    ///         {history.update_with(
    ///             move || {
    ///                 let article = article.clone();
    ///                 async move {
    ///                     article.await.ok()
    ///                 }
    ///             },
    ///             |history, article| if let Some(article) = article {
    ///                 // if the history portlet is visible, include this article
    ///                 if let Some(history) = history {
    ///                     history.push(article)
    ///                 }
    ///             },
    ///         )}
    ///         <div>
    ///             // Other components/elements.
    ///         </div>
    ///     }
    /// }
    /// ```
    ///
    /// Note that this method returns a `<Suspense/>`, which should be
    /// included into the view tree to be returned by the component like
    /// in the above example to ensure the update happen as the component
    /// renders.
    pub fn update_with<Fut, U>(
        &self,
        fetcher: impl Fn() -> Fut + Send + Sync + 'static,
        updater: impl Fn(&mut Option<T>, U) + Send + Sync + 'static,
    ) -> impl IntoView
    where
        Fut: Future<Output = U> + Send + 'static,
    {
        self.inner.update_with(fetcher, updater)
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
    /// impl IntoRender for Nav {
    ///     type Output = AnyView;
    ///
    ///     fn into_render(self) -> Self::Output {
    ///         view! {
    ///             todo!()
    ///         }
    ///         .into_any()
    ///     }
    /// }
    ///
    /// #[component]
    /// pub fn NavPortlet() -> impl IntoView {
    ///     <PortletCtx<Nav>>::render()
    /// }
    /// ```
    ///
    /// ## Panics
    /// Panics if `PortletCtx<T>` is not found in the current reactive
    /// owner or its ancestors.
    pub fn render() -> impl IntoView
    where
        T: IntoRender,
        <T as leptos::prelude::IntoRender>::Output: RenderHtml + Send + 'static,
        Suspend<Option<AnyView>>: RenderHtml + Render,
    {
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
                Some(resource.await?.into_render().into_any())
            })
        };
        view! { <Transition>{move || suspend() }</Transition> }
    }

    /// Clears the portlet.
    ///
    /// Upon invocation of this method, a `None` will be written to the
    /// underlying write signal, which should trigger the re-rendering
    /// through the associated function [`render`](PortletCtx::render).
    /// Given the `None` value, this typically results in nothing being
    /// rendered, achieving the goal of clearing the portlet.
    ///
    /// Note that this is typically expected to be used in conjunction
    /// with [`on_cleanup`](leptos::reactive::owner::on_cleanup) under
    /// CSR.  Usage under SSR may lead to unexpected behavior.
    pub fn clear(&self) {
        self.inner.inner_write_only().set(None);
    }

    /// Acquire the inner `ArcWriteSignal`.
    ///
    /// This calls the inner's [`SsrWriteSignal::inner_write_only`] to
    /// return the raw write signal as per that method.  This is the
    /// same signal used for the `clear` method, except rather than
    /// setting to `None` directly a more careful cleanup approach may
    /// be applied.
    ///
    /// Note that this is typically expected to be used in conjunction
    /// with [`on_cleanup`](leptos::reactive::owner::on_cleanup) under
    /// CSR.  Usage under SSR may lead to unexpected behavior.
    pub fn inner_write_signal(&self) -> ArcWriteSignal<Option<T>> {
        self.inner.inner_write_only()
    }

    /// Acquire the inner `ArcResource`.
    ///
    /// This calls the inner's [`SsrWriteSignal::read_only`] to acquire
    /// a clone of the resource as per that method.  This is provided to
    /// facilitate more complex rendering requirements, such as the need
    /// to `await` for other resources beyond this one.
    pub fn inner_resource(&self) -> ArcResource<Option<T>> {
        self.inner.read_only()
    }
}
