//! This crate provides helpers to support Leptos SSR apps that need to
//! correctly hydrate components situated earlier in the view tree but
//! require data source that may be asynchronously defined by components
//! that sits later down the view tree.
//!
//! Any valid solution to this problem must ensure any components that may
//! set the data source would successfully do so before processing is allow
//! to begin prematurely in the earlier component, otherwise either SSR
//! fails to include the desired content with or without hydration error
//! when rendered by the client, or they are included but with hydration
//! error.
//!
//! There are multiple ways to approach this problem.  The solution offered
//! by this crate is by providing a synchronization primitive that would
//! asynchronously wait for the signal to continue before allowing the
//! resource to continue to process forward to access the data source that
//! is set by some later component, with an additional component that
//! encloses all components that may assign the data source to that affected
//! resource.
//!
//! # Example
//!
//! An example on where the above scenario might manifest may be found in
//! [portlets](https://en.wikipedia.org/wiki/Portlet).  For instance, a
//! common navigation site element may be situated earlier in the view tree,
//! and it would provide a common structure which any data source may
//! convert into.
//!
//! A practical example of a portlet would be a navigational portlet. Some
//! blog may provide a listing of most recent articles in the navigation
//! portlet when a user is viewing some articles, but that same portlet can
//! instead show a listing of other authors upon the user access a link to
//! view an author of that article.
//!
//! If the portlet is situated earlier in the view tree but the routes and
//! components for those articles and users are provided later in the view
//! tree, it would require a way to ensure the data source is assigned to
//! the portlet so it may render both SSR and the hydration script correctly
//! as expected by the user, rather than letting the rendering stream out to
//! the client without the portlet and result in the dreaded hydration
//! error.
//!
//! Another example is website breadcrumbs, which is similar to a portlet
//! where it typically situated above all site content, and if the site
//! content may or may not in fact be available to the user, the value
//! representing the breadcrumbs would clearly need to reflect that.
//! Likewise, that earlier view tree component clearly depends on the
//! readiness of some component later in the view tree.
//!
//! As the portlet pattern is a useful abstraction and is easily
//! encapsulated, this package also provides a portlet module that allow the
//! removal of much of the boilerplate for providing and rendering portlets
//! within a Leptos application.  This feature is gated behind the `portlet`
//! flag.
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

pub use ready::{Ready, ReadyHandle, ReadySubscription};
