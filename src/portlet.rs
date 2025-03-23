use leptos::prelude::*;

#[cfg(feature = "ssr")]
use crate::Ready;

#[derive(Clone, Debug, Default)]
pub struct PortletCtx<T> {
    inner: Option<ArcResource<Result<T, ServerFnError>>>,
    refresh: ArcRwSignal<usize>,
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
{
    /// Clear the resource in the portlet.  The component using this
    /// may decide to not render anything.
    pub fn clear(&mut self) {
        // leptos::logging::log!("PortletCtx clear");
        self.refresh.try_update(|n| *n += 1);
        self.inner = None;
    }

    /// Set the resource for this portlet.
    pub fn set(&mut self, value: ArcResource<Result<T, ServerFnError>>) {
        // leptos::logging::log!("PortletCtx set");
        self.refresh.try_update(|n| *n += 1);
        self.inner = Some(value);
    }

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
