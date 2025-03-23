//! Provides generic helpers to build portlets on Leptos
//!
//! Since there are some common UI design patterns that involve placing
//! elements before the main article while requiring data that's not
//! available until then, and that those patterns largely can be implemented
//! as portlets, this module provides some common helpers that will allow
//! the portlets be placed anywhere in the view tree as this implementation
//! implements the most conservative execution path that will cater to any
//! valid positions in the view tree.
//!
//! For the most simple case, a simple `RwSignal` with a simple direct
//! rendering may be all that is required.  This, much more convoluted
//! implementation is really only required if you don't know where exactly
//! where the element will ultimately placed in the view tree, but you just
//! want to write the least possible amount of code as possible to get the
//! desired results working no matter the UI element is physically located
//! in the view tree.
//!
//! In other words, this is the jack of all trades implementation, and will
//! not function as the optimized implementation for all cases.
use leptos::prelude::*;

#[cfg(feature = "ssr")]
use crate::Ready;

/// A generic portlet context.
///
/// Internally, this contains an `ArcResource` that will be provided as a
/// context throughout a typical Leptos `App`, and a refresh signal to
/// indicate when a refresh is required, e.g. after a new resource has been
/// set or have been cleared.  The usage of a resource informs Leptos that
/// additional asynchronous waiting could be done, and to allow passing of
/// raw resource definitions into here.
#[derive(Clone, Debug, Default)]
pub struct PortletCtx<T, E = ServerFnError> {
    inner: Option<ArcResource<Result<T, E>>>,
    refresh: ArcRwSignal<usize>,
}

impl<T, E> PortletCtx<T, E>
where
    T: serde::Serialize
        + serde::de::DeserializeOwned
        + Clone
        + PartialEq
        + Send
        + Sync
        + IntoRender
        + 'static,
{
    /// Clears the resource for the portlet.
    ///
    /// Coupled with the [`render_portlet`] function, upon invocation of
    /// this method, the rendering of the portlet will be `None`.
    pub fn clear(&mut self) {
        // leptos::logging::log!("PortletCtx clear");
        self.refresh.try_update(|n| *n += 1);
        self.inner = None;
    }

    /// Set the resource for this portlet.
    ///
    /// This would assign an `ArcResource` that will ultimately provide
    /// a value of type `Result<T, E>`.
    pub fn set(&mut self, value: ArcResource<Result<T, E>>) {
        // leptos::logging::log!("PortletCtx set");
        self.refresh.try_update(|n| *n += 1);
        self.inner = Some(value);
    }

    /// Provide this as a context for a Leptos `App`.
    ///
    /// The reason why there is no constructor provided and only done so
    /// via signal is to have these contexts function as a singleton.
    pub fn provide() {
        let (rs, ws) = arc_signal(PortletCtx::<T> {
            inner: None,
            refresh: ArcRwSignal::new(0),
        });
        provide_context(rs);
        provide_context(ws);
    }

    pub fn expect_write() -> ArcWriteSignal<PortletCtx<T>> {
        expect_context::<ArcWriteSignal<PortletCtx<T>>>()
    }
}

/// A generic portlet renderer using the generic portlet context.
///
/// This renderer simplifies the creation of portlet components based
/// upon the underlying `T`, for all `T` that implements `IntoRender`.
/// For usage in a Leptos `App`, it expects that the [`PortletCtx<T>`]
/// be [provided](PortletCtx::provide) as `ArcReadSignal<PortletCtx<T>>`.
/// The implementation would ensure the resource assignment will be
/// waited upon before usage, if the component invoking this has one
/// [`SyncSsr`](crate::component::SyncSsr) up its view tree.
///
/// Typical usage may look like this.
///
/// ```
/// #[component]
/// pub fn NavPortlet() -> impl IntoView {
///     render_portlet::<Nav>()
/// }
/// ```
pub fn render_portlet<T>() -> impl IntoView
where
    T: serde::Serialize
        + serde::de::DeserializeOwned
        + Clone
        + std::fmt::Debug
        + PartialEq
        + Send
        + Sync
        + IntoRender
        + 'static,
    <T as leptos::prelude::IntoRender>::Output: RenderHtml,
{
    #[cfg(feature = "ssr")]
    let ready = Ready::handle();

    let rs = expect_context::<ArcReadSignal<PortletCtx<T>>>();
    let refresh = rs.get_untracked().refresh;
    let resource = ArcResource::new_blocking(
        {
            move || {
                // leptos::logging::log!("into_render suspend resource signaled!");
                refresh.get()
            }
        },
        // move |id| {
        move |_| {
            // leptos::logging::log!("refresh id {id}");
            #[cfg(feature = "ssr")]
            let ready = ready.clone();
            let rs = rs.clone();
            async move {
                // leptos::logging::log!("PortletCtxRender Suspend resource entering");
                // leptos::logging::log!("refresh id {id}");
                #[cfg(feature = "ssr")]
                ready.subscribe().wait().await;
                let ctx = rs.get();
                // leptos::logging::log!("portlet_ctx.inner = {:?}", ctx.inner);
                let result = if let Some(resource) = ctx.inner {
                    Ok::<_, ServerFnError>(Some(resource.await?))
                } else {
                    Ok(None)
                };
                // leptos::logging::log!("PortletCtxRender Suspend resource exiting");
                result
            }
        },
    );

    let suspend = move || {
        let resource = resource.clone();
        Suspend::new(async move {
            // leptos::logging::log!("PortletCtxRender Suspend entering");
            let result = resource.await?;
            let result = if let Some(result) = result {
                // leptos::logging::log!("returning actual view");
                Ok::<_, ServerFnError>(Some(result.into_render().into_any()))
            } else {
                // leptos::logging::log!("returning empty view");
                Ok(None)
            };
            // leptos::logging::log!("PortletCtxRender Suspend exiting");
            result
        })
    };

    view! { <Transition>{move || suspend() }</Transition> }
}
