use std::time::Duration;

use leptos::prelude::*;
use leptos_sync_ssr::{component::SyncSsrSignal, portlet::PortletCtx};

#[cfg(feature = "ssr")]
mod ssr {
    pub use futures::StreamExt;
    use leptos::prelude::Owner;

    pub fn init_renderer() -> Owner {
        let _ = any_spawner::Executor::init_tokio();
        let owner = Owner::new();
        owner.set();
        owner
    }
}
#[cfg(feature = "ssr")]
use ssr::*;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct Item(String);

pub type Ctx = PortletCtx<Item>;

impl IntoRender for Item {
    type Output = AnyView;

    fn into_render(self) -> Self::Output {
        self.0.into_any()
    }
}

#[component]
pub fn Portlet() -> impl IntoView {
    Ctx::render()
}

#[component]
pub fn Setter() -> impl IntoView {
    let ctx = expect_context::<Ctx>();

    view! {
        {ctx.update_with(move || {
            async move {
                #[cfg(feature = "ssr")]
                tokio::time::sleep(Duration::from_millis(100)).await;
                Some(Item("Hello world!".to_string()))
            }
        })}
    }
}

#[cfg(feature = "ssr")]
#[tokio::test]
async fn portlet_setter() {
    let _owner = init_renderer();

    let app = view! {
        <SyncSsrSignal setup=|| Ctx::provide()>
            <Portlet />
            <Setter />
        </SyncSsrSignal>
    };
    assert_eq!(
        app.to_html_stream_in_order().collect::<String>().await,
        "<!>Hello world!<!><!>",
    );
}
