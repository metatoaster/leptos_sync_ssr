//! Provides the [`SyncSsr`] and [`SyncSsrSignal`] components.
use leptos::{children::Children, component, view, IntoView};

#[cfg(feature = "ssr")]
mod ssr {
    pub use leptos::context::Provider;
    pub use crate::ready::{CoReadyCoordinator, Ready};
}

#[cfg(feature = "ssr")]
use ssr::*;

/// This component provides the [`Ready`] context to its children.
///
/// Typical usage of this component will simply enclose the components
/// that desire to signal to an earlier component some value that should
/// be used, with the component that would allow a later component to
/// set a value it would then use.  Once this component is rendered
/// under SSR, the signal will be sent to all actively waiting
/// [`ReadySubscription::wait`](crate::ReadySubscription::wait), so that
/// all futures waiting on that be allowed to continue, which hopefully
/// will see the expected value being set while they are waiting for
/// later.
///
/// ```
/// use leptos::prelude::*;
/// use leptos_router::{
///     components::{Route, Router, Routes},
///     path, MatchNestedRoutes,
/// };
/// use leptos_sync_ssr::component::SyncSsr;
///
/// #[component]
/// fn MyApp() -> impl IntoView {
///     view! {
///         <Router>
///             <nav>
///                 <a href="/">"Home"</a>
///                 <a href="/author/">"Authors"</a>
///                 <a href="/article/">"Articles"</a>
///             </nav>
///             <SyncSsr>
///                 <Breadcrumbs/>
///                 <Routes fallback=|| ()>
///                     <Route path=path!("") view=HomePage/>
///                     <AuthorRoutes/>
///                     <ArticleRoutes/>
///                 </Routes>
///             </SyncSsr>
///         </Router>
///     }
/// }
/// #
/// # #[component]
/// # fn HomePage() -> impl IntoView {
/// #     ()
/// # }
/// #
/// # #[component]
/// # fn Breadcrumbs() -> impl IntoView {
/// #     ()
/// # }
/// #
/// # #[component]
/// # pub fn ArticleRoutes() -> impl MatchNestedRoutes + Clone {
/// #     view! {
/// #         <Route path=path!("") view=HomePage/>
/// #     }
/// #     .into_inner()
/// # }
/// #
/// # #[component]
/// # pub fn AuthorRoutes() -> impl MatchNestedRoutes + Clone {
/// #     view! {
/// #         <Route path=path!("") view=HomePage/>
/// #     }
/// #     .into_inner()
/// # }
/// ```
///
/// In the above example, both `<Routes>` and `<Breadcrumbs>` are
/// enclosed.  This would enable the resources inside `<Breadcrumbs>` to
/// wait for the ready signal before reading of signals of values that
/// may be set by other components enclosed inside the `<Routes>` so
/// that SSR be done in the expected order to ensure proper hydration by
/// the client.
#[component]
pub fn SyncSsr(children: Children) -> impl IntoView {
    // leptos::logging::log!("entering SyncSsr");
    #[cfg(feature = "ssr")]
    let ready = Ready::new();

    #[cfg(feature = "ssr")]
    let exit = {
        let ready = ready.clone();
        move || {
            ready.complete();
            // leptos::logging::log!("exiting SyncSsr");
        }
    };

    #[cfg(feature = "ssr")]
    let result = view! {
        <Provider value=ready>
            {children()}
            {exit}
        </Provider>
    };

    #[cfg(not(feature = "ssr"))]
    let result = view! {
        {children()}
        {}
    };

    result
}

/// This component provides the [`CoReadyCoordinator`] context to its
/// children.
///
/// Given how [`SsrSignalResource`](crate::signal::SsrSignalResource)
/// requires the `CoReadyCoordinator` be available as a context, usage
/// of this component to enclose the components making use of that type
/// is the recommended way to setup and teardown the context.
///
/// This enables the correct processing order to ensure that the values
/// to be provided by the resource is provided after waiting correctly.
///
/// The following represents typical usage.
///
/// ```
/// use leptos::prelude::*;
/// use leptos_router::{
///     components::{Route, Router, Routes},
///     path, MatchNestedRoutes,
/// };
/// use leptos_sync_ssr::component::SyncSsrSignal;
/// use leptos_sync_ssr::signal::SsrSignalResource;
///
/// #[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
/// enum BreadCrumbs {
///     Home,
///     Author(String),
///     Article(u64),
/// };
///
/// #[component]
/// pub fn App() -> impl IntoView {
///     let fallback = || view! { "Page not found." }.into_view();
///     // This would panic here
///     // let breadcrumbs = SsrSignalResource::new(BreadCrumbs::Home);
///     view! {
///         <Router>
///             <SyncSsrSignal>{
///                 // Provide the SsrSignalResource here
///                 let breadcrumbs = SsrSignalResource::new(BreadCrumbs::Home);
///                 provide_context(breadcrumbs);
///
///                 view! {
///                     <header>
///                         <Breadcrumbs/>
///                     </header>
///                     <article>
///                         <Routes fallback=|| ()>
///                             <Route path=path!("") view=HomePage/>
///                             <AuthorRoutes/>
///                             <ArticleRoutes/>
///                         </Routes>
///                     </article>
///                 }
///             }</SyncSsrSignal>
///         </Router>
///     }
/// }
/// #
/// # #[component]
/// # fn HomePage() -> impl IntoView {
/// #     ()
/// # }
/// #
/// # #[component]
/// # fn Breadcrumbs() -> impl IntoView {
/// #     ()
/// # }
/// #
/// # #[component]
/// # pub fn ArticleRoutes() -> impl MatchNestedRoutes + Clone {
/// #     view! {
/// #         <Route path=path!("") view=HomePage/>
/// #     }
/// #     .into_inner()
/// # }
/// #
/// # #[component]
/// # pub fn AuthorRoutes() -> impl MatchNestedRoutes + Clone {
/// #     view! {
/// #         <Route path=path!("") view=HomePage/>
/// #     }
/// #     .into_inner()
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

#[component]
pub fn SyncSsrSignal(children: Children) -> impl IntoView {
    #[cfg(feature = "ssr")]
    let coord = CoReadyCoordinator::new();

    #[cfg(feature = "ssr")]
    let exit = {
        let coord = coord.clone();
        move || coord.notify()
    };

    #[cfg(feature = "ssr")]
    let result = view! {
        <Provider value=coord>
            {children()}
            {exit}
        </Provider>
    };

    #[cfg(not(feature = "ssr"))]
    let result = view! {
        {children()}
        {}
    };

    result
}
