//! Provides the [`SyncSsr`] component.
use leptos::{children::Children, component, view, IntoView};

#[cfg(feature = "ssr")]
mod ssr {
    pub use leptos::context::Provider;
}

#[cfg(feature = "ssr")]
use ssr::*;

#[cfg(feature = "ssr")]
use crate::ready::Ready;

/// The component that will provide a [`Ready`] coordinator to its
/// children.
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
/// that accurate SSR be done for proper hydration by the client.
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
