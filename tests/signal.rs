use leptos::prelude::*;
use leptos_sync_ssr::CoReadyCoordinator;
use leptos_sync_ssr::signal::{SsrSignalResource, SsrWriteSignal};

#[cfg(feature = "ssr")]
mod ssr {
    pub use futures::StreamExt;
}
#[cfg(feature = "ssr")]
use ssr::*;

#[component]
fn Indicator() -> impl IntoView {
    let res = expect_context::<SsrSignalResource<String>>().read_only();
    view! {
        <p>
            "Indicator is: "
            <Suspense>
            {move || {
                let res = res.clone();
                Suspend::new(async move {
                    res.await
                })
            }}
            </Suspense>
        </p>
    }
}

#[component]
fn SetterUsed(ws_set: bool) -> impl IntoView {
    let sr = expect_context::<SsrSignalResource<String>>();
    let res = ArcResource::new(
        || (),
        {
            let sr = sr.clone();
            move |_| {
                let ws = sr.write_only();
                async move {
                    // a timeout here to emulate server function delay, this
                    // should be enough to delay resolution of this future
                    // such that the one on the Indicator be ready and
                    // render its output should it not have an additional
                    // wait.
                    #[cfg(feature = "ssr")]
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

                    let value = "Hello world!";
                    if ws_set {
                        ws.set(value.to_string());
                        format!("resource write signal setting value: {value}")
                    } else {
                        format!("resource write signal setting no value")
                    }
                }
            }
        },
    );

    view! {
        <Suspense>
        {move || {
            let res = res.clone();
            Suspend::new(async move {
                res.await
            })
        }}
        </Suspense>
    }
}

#[cfg(feature = "ssr")]
#[tokio::test]
async fn render_setter_set() {
    let _owner = init_renderer();

    let coord = CoReadyCoordinator::new();
    provide_context(coord.clone());
    let sr = SsrSignalResource::new(String::new());
    provide_context(sr.clone());
    let app = view! {
        <Indicator />
        <SetterUsed ws_set=true />
    };
    coord.notify();
    dbg!("let app = ...");
    // dbg!(sr.inner.ready.inner.sender.sender_count());

    let html = app.to_html_stream_in_order().collect::<String>().await;
    assert_eq!(html, "<p>Indicator is: <!>Hello world!</p>resource write signal setting value: Hello world!");
}

#[cfg(feature = "ssr")]
#[tokio::test]
async fn render_setter_unset() {
    let _owner = init_renderer();

    let coord = CoReadyCoordinator::new();
    provide_context(coord.clone());
    let sr = SsrSignalResource::new(String::new());
    provide_context(sr);
    let app = view! {
        <Indicator />
        <SetterUsed ws_set=false />
    };
    coord.notify();

    let html = app.to_html_stream_in_order().collect::<String>().await;
    assert_eq!(html, "<p>Indicator is: <!> </p>resource write signal setting no value");
}

// XXX this test deadlocks
// given the deadlock comes from the fact that there are no additional checks
// as the underlying wait_for will never get invoked, we need to provide an
// additional helper (probably via a context) to flip the check to a mode where
// check for sender count becomes active
//
// also it needs past sender count? perhaps the underlying bool is insuffient,
// but rather a subscription will bump up the past_sender_count such that when
// the mode changes it will pass
#[cfg(feature = "ssr")]
#[tokio::test]
async fn render_no_setter() {
    let _owner = init_renderer();

    let coord = CoReadyCoordinator::new();
    provide_context(coord.clone());
    let sr = SsrSignalResource::new(String::new());
    provide_context(sr);
    let app = view! {
        <Indicator />
    };
    coord.notify();

    let html = app.to_html_stream_in_order().collect::<String>().await;
    assert_eq!(html, "<p>Indicator is: <!> </p>");
}

#[cfg(feature = "ssr")]
fn init_renderer() -> Owner {
    let _ = any_spawner::Executor::init_tokio();
    let owner = Owner::new();
    owner.set();
    owner
}
