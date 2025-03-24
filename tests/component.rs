use leptos::prelude::*;
use leptos_sync_ssr::{component::SyncSsr, Ready};

#[cfg(feature = "ssr")]
mod ssr {
    pub use futures::StreamExt;
}
#[cfg(feature = "ssr")]
use ssr::*;

// This is roughly a smoke test to give a rough indication that the
// SyncSsr component is working, not at all an accurate representation
// of what it is supposed to enable.

#[component]
fn Indicator() -> impl IntoView {
    let rs = expect_context::<ReadSignal<Option<OnceResource<String>>>>();
    let handle = Ready::handle();
    let res = Resource::new_blocking(
        || (),
        move |_| {
            let handle = handle.clone();
            async move {
                handle.subscribe().wait().await;
                if let Some(res) = rs.get() {
                    Some(res.await)
                } else {
                    None
                }
            }
        }
    );

    view! {
        <p>
            "Indicator is: "
            <Suspense>
            {move || Suspend::new(async move {
                res.await
            })}
            </Suspense>
        </p>
    }
}

#[component]
fn Setter() -> impl IntoView {
    let ws = expect_context::<WriteSignal<Option<OnceResource<String>>>>();
    let hook = move || ws.set(Some(OnceResource::new(async move {
        "hello world".to_string()
    })));
    view! {
        {hook}
        <p>"Wrote 'hello world'"</p>
    }

}

#[component]
fn SyncedSsr() -> impl IntoView {
    let (rs, ws) = signal(None::<OnceResource<String>>);
    provide_context(rs);
    provide_context(ws);
    view! {
        <SyncSsr>
            <Indicator />
            <Setter />
        </SyncSsr>
    }
}

#[component]
fn StandardSsr() -> impl IntoView {
    let (rs, ws) = signal(None::<OnceResource<String>>);
    provide_context(rs);
    provide_context(ws);
    view! {
        <Indicator />
        <Setter />
    }
}

#[cfg(feature = "ssr")]
#[tokio::test]
async fn render_synced_ssr() {
    let _owner = init_renderer();
    let app = view! { <SyncedSsr /> };
    let html = app.to_html_stream_in_order().collect::<String>().await;
    // note the marker node
    assert!(html.contains("Indicator is: <!>hello world"));
}

#[cfg(feature = "ssr")]
#[tokio::test]
async fn render_standard_ssr() {
    let _owner = init_renderer();
    let app = view! { <StandardSsr /> };
    let _html = app.to_html_stream_in_order().collect::<String>().await;
    // Yes, the following _can_ still work, but if some kind of work
    // stealing happens the co-ordination that allow formation of the
    // expected output can fail.
    // assert!(html.contains("Indicator is: <!>hello world"));
}

#[cfg(feature = "ssr")]
fn init_renderer() -> Owner {
    let _ = any_spawner::Executor::init_tokio();
    let owner = Owner::new();
    owner.set();
    owner
}
