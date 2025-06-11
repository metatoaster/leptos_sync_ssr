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
//! For the most simple case, a simple [`RwSignal`] with a simple direct
//! rendering may be all that is required.  This, much more convoluted
//! implementation is really only required if you don't know where exactly
//! where the element will ultimately placed in the view tree, but you just
//! want to write the least possible amount of code as possible to get the
//! desired results working no matter the UI element is physically located
//! in the view tree.
//!
//! In other words, this is the jack of all trades implementation, using the
//! most basic locking and control and will not function as the optimized
//! implementation for all cases.
use std::future::IntoFuture;

use leptos::prelude::*;
use leptos::server_fn::error::FromServerFnError;

#[cfg(feature = "ssr")]
use leptos_sync_ssr::Ready;

/// A generic portlet context.
///
/// Internally, this contains an [`ArcResource`] that will be provided as a
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
    <T as leptos::prelude::IntoRender>::Output: RenderHtml + Send + 'static,
    E: serde::Serialize
        + serde::de::DeserializeOwned
        + Clone
        + Send
        + Sync
        + FromServerFnError
        + From<ServerFnError>,
    ArcResource<Result<Option<T>, E>>: IntoFuture<Output = Result<Option<T>, E>>,
    <ArcResource<Result<Option<T>, E>> as IntoFuture>::IntoFuture: Send,
    Suspend<Result<Option<AnyView>, E>>: RenderHtml + Render,
{
    /// Clears the resource for the portlet.
    ///
    /// Upon invocation of this method, the rendering of the portlet
    /// will be `None` through the associated function [`render`](
    /// PortletCtx::render).
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
        let (rs, ws) = arc_signal(PortletCtx::<T, E> {
            inner: None,
            refresh: ArcRwSignal::new(0),
        });
        provide_context(rs);
        provide_context(ws);
    }

    /// Acquire via [`expect_context`] the write signal for this.
    ///
    /// Using this associated function will ensure the correct write
    /// signal will be returned.
    pub fn expect_write() -> ArcWriteSignal<PortletCtx<T, E>> {
        expect_context::<ArcWriteSignal<PortletCtx<T, E>>>()
    }

    /// A generic portlet renderer via this generic portlet context.
    ///
    /// This renderer simplifies the creation of portlet components based
    /// upon the underlying `T`, for all `T` that implements `IntoRender`.
    /// For usage in a Leptos `App`, it expects that the [`PortletCtx<T, E>`]
    /// be [provided](PortletCtx::provide) as `ArcReadSignal<PortletCtx<T, E>>`.
    /// The implementation would ensure the resource assignment will be
    /// waited upon before usage, provided that the component invoking this
    /// has a [`SyncSsr`](crate::component::SyncSsr) up its view tree.
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
    pub fn render() -> impl IntoView {
        #[cfg(feature = "ssr")]
        let ready = Ready::handle();

        let rs = expect_context::<ArcReadSignal<PortletCtx<T, E>>>();
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
                    let ctx = rs.get_untracked();
                    // leptos::logging::log!("portlet_ctx.inner = {:?}", ctx.inner);
                    // let result = if let Some(resource) = ctx.inner {
                    if let Some(resource) = ctx.inner {
                        Ok(Some(resource.await?))
                    } else {
                        Ok(None)
                    }
                    // };
                    // leptos::logging::log!("PortletCtxRender Suspend resource exiting");
                    // result
                }
            },
        );

        let suspend = move || {
            let resource = resource.clone();
            Suspend::new(async move {
                // leptos::logging::log!("PortletCtxRender Suspend entering");
                let result = resource.await?;
                // let result = if let Some(result) = result {
                if let Some(result) = result {
                    // leptos::logging::log!("returning actual view");
                    Ok::<_, E>(Some(result.into_render().into_any()))
                } else {
                    // leptos::logging::log!("returning empty view");
                    Ok(None)
                }
                // };
                // leptos::logging::log!("PortletCtxRender Suspend exiting");
                // result
            })
        };

        view! { <Transition>{move || suspend() }</Transition> }
    }
}
