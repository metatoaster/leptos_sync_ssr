use leptos::prelude::*;
use leptos_sync_ssr::{
    component::SyncSsrSignal,
    signal::{SsrSignalResource, SsrWriteSignal},
};

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

#[component]
fn SetterMisused() -> impl IntoView {
    let sr = expect_context::<SsrSignalResource<String>>();
    let res = ArcResource::new(
        || (),
        {
            let sr = sr.clone();
            move |_| {
                let sr = sr.clone();
                async move {
                    #[cfg(feature = "ssr")]
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    // This is the misuse as the notification wouldn't be triggered
                    // in time.
                    let ws = sr.write_only();

                    let value = "Hello world!";
                    ws.set(value.to_string());
                    format!("resource write signal setting value: {value}")
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
async fn missing_co_ready_coordinator() {
    let result = std::panic::catch_unwind(|| SsrSignalResource::new(String::new()));
    assert!(result.is_err());
}

#[cfg(feature = "ssr")]
#[tokio::test]
async fn render_setter_set() {
    let _owner = init_renderer();

    let app = view! {
        <SyncSsrSignal>{
            let sr = SsrSignalResource::new(String::new());
            provide_context(sr.clone());
            view! {
                <Indicator />
                <SetterUsed ws_set=true />
            }
        }</SyncSsrSignal>
    };
    assert_eq!(
        app.to_html_stream_in_order().collect::<String>().await,
        "<p>Indicator is: <!>Hello world!</p>resource write signal setting value: Hello world!<!>",
    );
}

#[cfg(feature = "ssr")]
#[tokio::test]
async fn render_setter_unset() {
    let _owner = init_renderer();

    let app = view! {
        <SyncSsrSignal>{
            let sr = SsrSignalResource::new(String::new());
            provide_context(sr.clone());
            view! {
                <Indicator />
                <SetterUsed ws_set=false />
            }
        }</SyncSsrSignal>
    };
    assert_eq!(
        app.to_html_stream_in_order().collect::<String>().await,
        "<p>Indicator is: <!> </p>resource write signal setting no value<!>",
    );
}

#[cfg(feature = "ssr")]
#[tokio::test]
async fn render_misused() {
    let _owner = init_renderer();

    let app = view! {
        <SyncSsrSignal>{
            let sr = SsrSignalResource::new(String::new());
            provide_context(sr.clone());
            view! {
                <Indicator />
                <SetterMisused />
            }
        }</SyncSsrSignal>
    };
    // note that the resource wrote the signal but the indicator is unable to show it.
    // also note that there is no deadlock.
    assert_eq!(
        app.to_html_stream_in_order().collect::<String>().await,
        "<p>Indicator is: <!> </p>resource write signal setting value: Hello world!<!>",
    );
}

#[cfg(feature = "ssr")]
#[tokio::test]
async fn render_no_setter() {
    let _owner = init_renderer();

    let app = view! {
        <SyncSsrSignal>{
            let sr = SsrSignalResource::new(String::new());
            provide_context(sr.clone());
            view! {
                <Indicator />
            }
        }</SyncSsrSignal>
    };
    // no setter should not cause a deadlock.
    assert_eq!(
        app.to_html_stream_in_order().collect::<String>().await,
        "<p>Indicator is: <!> </p><!>",
    );
}

#[cfg(feature = "ssr")]
fn init_renderer() -> Owner {
    let _ = any_spawner::Executor::init_tokio();
    let owner = Owner::new();
    owner.set();
    owner
}
