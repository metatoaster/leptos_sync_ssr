//! This crate provides helpers to synchronize the access of Leptos resources
//! during server-side rendering (SSR) within the Leptos frameworks.  This is
//! achieved by providing locking primitives, which are then built into the
//! provided components and signals, such that when they are used together in
//! the prescribed manner, it would allow the affected resources to resolve in
//! the expected order.  This enables components defined earlier in the view
//! tree to wait on data that may or may not be provided by components defined
//! later down the view tree, ultimately ensuring that hydration would happen
//! correctly.
//!
//! ## Use case
//!
//! A fairly common user interface design pattern known as [portlets](
//! https://en.wikipedia.org/wiki/Portlet) may be implemented in Leptos
//! using a struct with data fields to be rendered be available in through a
//! reactive signal behind a context, with the renderer being a component
//! that would reactively read the value from that reactive signal such that
//! it may be updated as required.  This is not a problem under client-side
//! rendering (CSR) for Leptos, but for SSR with hydration, this is a whole
//! other story.
//!
//! If the rendering component is situated in the view tree before the
//! component that may write to it, as in the case of a typical "breadcrumb"
//! component, this creates the complication where the signal may not be set
//! in time by the time the breadcrumb component was streamed out to the
//! client with the default data.  Furthermore, the hydration script may
//! contain the expected data, and when used to hydrate the rendered markup
//! which used the default data from earlier, this mismatch wil result in
//! hydration error.
//!
//! There are multiple solutions to this problem, and this crate provides a
//! number of them, when combined together, addresses the rendering and
//! hydration issues, without bringing in new problems that the individual
//! solution would bring when used in isolation.
//!
//! # Example
//!
//! This package does in fact implement [`portlets`](crate::portlet) as a
//! module, where a minimum amount of user code is required to set up the
//! scenario as described above.  The following is an example on how that
//! module may be used:
//!
//! ```
//! use leptos::prelude::*;
//! use leptos_router::{
//!     components::{Route, Router, Routes},
//!     path,
//! };
//! use leptos_sync_ssr::{component::SyncSsrSignal, portlet::PortletCtx};
//! #
//! # #[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
//! # struct Blog;
//!
//! #[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
//! struct Breadcrumbs {
//!     // fields...
//! }
//!
//! // The main `App` - note how the `SyncSsrSignal` is placed.
//! #[component]
//! fn App() -> impl IntoView {
//!     view! {
//!         <Router>
//!             <SyncSsrSignal setup=|| {
//!                 <PortletCtx<Breadcrumbs>>::provide();
//!             }>
//!                 <header>
//!                     // Note how the breadcrumb comes before anything that
//!                     // might set the signal for it.
//!                     <ShowBreadcrumbs/>
//!                 </header>
//!                 <article>
//!                     <Routes fallback=|| ()>
//!                         <Route path=path!("") view=HomePage/>
//!                         <Route path=path!("/blog/:id") view=BlogView/>
//!                         // plus other routes
//!                     </Routes>
//!                 </article>
//!             </SyncSsrSignal>
//!         </Router>
//!     }
//! }
//!
//! // This implements the rendering for `BreadCrumbs`
//! impl IntoRender for Breadcrumbs {
//!     type Output = AnyView;
//!
//!     fn into_render(self) -> Self::Output {
//!         view! {
//!             todo!()
//!         }
//!         .into_any()
//!     }
//! }
//!
//! #[component]
//! fn ShowBreadcrumbs() -> impl IntoView {
//!     <PortletCtx<Breadcrumbs>>::render()
//! }
//!
//! // This renders the blog, but also sets the breadcrumbs as appropriate.
//! #[component]
//! fn BlogView() -> impl IntoView {
//!     // assuming this was provided/defined elsewhere
//!     let blog = expect_context::<ArcResource<Result<Blog, ServerFnError>>>();
//!     let nav_ctx = expect_context::<PortletCtx<Breadcrumbs>>();
//!
//!     view! {
//!         // This ensures `PortletCtx<Breadcrumbs>` is updated with data provided by
//!         // `authors`.
//!         {nav_ctx.update_with(move || {
//!             let blog = blog.clone();
//!             async move {
//!                 blog.await
//!                     // TODO conversion of the blog info to `Breadcrumbs` type
//!                     .map(|blog| todo!())
//!                     .ok()
//!                 // Once this returns, the breadcrumb will render on SSR.
//!             }
//!         })}
//!         <div>
//!             // Other components/elements.
//!         </div>
//!     }
//! }
//!
//! #[component]
//! fn HomePage() -> impl IntoView {
//!     // Note how the homepage doesn't set breadcrumbs - it then shouldn't
//!     // show the breadcrumbs, and that lack of write signal creation/use
//!     // wouldn't cause a deadlock.
//!     view! {
//!         todo!()
//!     }
//! }
//! ```
//!
//! For a more complete example on something similar to above, [`nav_portlet`](
//! https://github.com/metatoaster/leptos_sync_ssr/tree/main/example/nav_portlet)
//! provides a great demonstration on how this might work in action, and
//! that there is also an alternative implementation called [`nav_portlet_alt`](
//! https://github.com/metatoaster/leptos_sync_ssr/tree/main/example/nav_portlet_alt)
//! that use the other set of primitives to achieve a similar effect.
//! The documentation under individual modules and types also include further
//! explanation, usage examples, and does go more into the implementation details,
//! please check them out.
//!
//! # Feature Flags
#![cfg_attr(
    feature = "document-features",
    cfg_attr(doc, doc = ::document_features::document_features!())
)]

pub mod component;
#[cfg(feature = "portlet")]
pub mod portlet;
mod ready;
pub mod signal;

#[cfg(test)]
mod tests;

pub use ready::{
    CoReady, CoReadyCoordinator, CoReadySubscription, Ready, ReadyHandle, ReadySubscription,
};
