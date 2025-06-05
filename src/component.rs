//! Provides the [`SyncSsr`] and [`SyncSsrSignal`] components.
use leptos::{children::Children, component, prelude::IntoMaybeErased, view, IntoView};

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
/// FIXME actually make it an example that uses SsrSignalResource
///
/// ```
/// use leptos::prelude::*;
/// use leptos_router::{
///     components::{Route, Router, Routes},
///     path, MatchNestedRoutes,
/// };
/// use leptos_sync_ssr::component::SyncSsrSignal;
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
///             <SyncSsrSignal>
///                 <Breadcrumbs/>
///                 <Routes fallback=|| ()>
///                     <Route path=path!("") view=HomePage/>
///                     <AuthorRoutes/>
///                     <ArticleRoutes/>
///                 </Routes>
///             </SyncSsrSignal>
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
#[component]
pub fn SyncSsrSignal(children: Children) -> impl IntoView {
    // leptos::logging::log!("entering SyncSsrSignal");
    #[cfg(feature = "ssr")]
    let coord = CoReadyCoordinator::new();

    #[cfg(feature = "ssr")]
    let exit = {
        let coord = coord.clone();
        move || {
            coord.notify();
            // leptos::logging::log!("exiting SyncSsrSignal");
        }
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
