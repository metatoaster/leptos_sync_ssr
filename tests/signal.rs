use std::time::Duration;

use leptos::prelude::*;
use leptos_sync_ssr::{component::SyncSsrSignal, signal::SsrSignalResource};
use tokio::time::timeout;

#[cfg(feature = "ssr")]
mod ssr {
    pub use futures::StreamExt;
}
#[cfg(feature = "ssr")]
use ssr::*;

#[derive(Clone, Copy)]
enum Mode {
    Set,
    Update,
    UpdateUntracked,
}

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
fn SetterUsed(mode: Option<Mode>) -> impl IntoView {
    let sr = expect_context::<SsrSignalResource<String>>();
    let res = ArcResource::new(|| (), {
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
                match mode {
                    None => format!("resource write signal setting no value"),
                    Some(Mode::Set) => {
                        ws.set(value.to_string());
                        format!("resource write signal setting value: {value}")
                    }
                    Some(Mode::Update) => {
                        ws.update(|s| s.push_str(value));
                        format!("resource write signal pushed value: {value}")
                    }
                    Some(Mode::UpdateUntracked) => {
                        ws.update_untracked(|s| s.push_str(value));
                        format!("resource write signal pushed value (untracked): {value}")
                    }
                }
            }
        }
    });

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
fn SetterInSuspense() -> impl IntoView {
    // While the value is set through the signal, it will not be read, due
    // to how `Suspense` works.
    let sr = expect_context::<SsrSignalResource<String>>();
    view! {
        <Suspense>
        {move || {
            let sr = sr.clone();
            // This does not in fact render successfully.
            Suspend::new(async move {
                // First pass - this ensures the lock...
                let ws = sr.write_only();
                // However, upon this await point, the whole `Suspend` is
                // dropped, which also drops the write_only signal, which
                // signals to the read_only end that it can read and return
                // the value... which is the dfault.
                #[cfg(feature = "ssr")]
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                let value = "Hello world!";
                // So even when the second pass got to this point, it's
                // never read.
                ws.set(value.to_string());
                format!("resource write signal setting value: {value}")
                // What if the drop is not signaled and a flag is held to
                // keep it armed?  The `Suspense` that is ultimately waiting
                // on the signal (`<Indicator/>`) will never return, which
                // means this `Suspense` will also never get to run, which
                // results in a deadlock as the resolver is locked out from
                // being executed.
                //
                // Hence it's typically better to rely on the default case
                // which avoids the deadlock situation.
            })
        }}
        </Suspense>
    }
}

/*
#[component]
fn SetterMisusedWriteOnlyCloned() -> impl IntoView {
    let sr = expect_context::<SsrSignalResource<String>>();
    let ws = sr.write_only();
    let res = ArcResource::new(
        || (),
        {
            let ws = ws.clone();
            move |_| {
                let ws = ws.clone();
                async move {
                    #[cfg(feature = "ssr")]
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    let value = "Hello world!";
                    // not using this will deadlock using the original naive
                    // implementation of `CoReadyCoordinator::notify`.
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
*/

#[component]
fn SetterMisusedWriteOnlyCreatedLate() -> impl IntoView {
    let sr = expect_context::<SsrSignalResource<String>>();
    let res = ArcResource::new(|| (), {
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
    });

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
fn SetterMisusedWriteOnlyKeptAlive() -> impl IntoView {
    let sr = expect_context::<SsrSignalResource<String>>();
    let ws = sr.write_only();
    // DO NOT DO THIS: it will cause the paired resource deadlock!
    provide_context(ws);
    "Stuffed the write_only into the reactive graph to force a deadlock"
}

#[cfg(feature = "ssr")]
#[tokio::test]
async fn missing_co_ready_coordinator() {
    let result = std::panic::catch_unwind(|| SsrSignalResource::new(String::new()));
    assert!(result.is_err());
}

#[cfg(feature = "ssr")]
#[tokio::test]
async fn render_setter_set_in_suspense() {
    let _owner = init_renderer();

    let app = view! {
        <SyncSsrSignal setup=|| {
            let sr = SsrSignalResource::new(String::new());
            provide_context(sr.clone());
        }>
            <Indicator />
            <SetterInSuspense />
        </SyncSsrSignal>
    };
    assert_eq!(
        app.to_html_stream_in_order().collect::<String>().await,
        "<!><p>Indicator is: <!> </p>resource write signal setting value: Hello world!<!>",
    );
}

#[cfg(feature = "ssr")]
#[tokio::test]
async fn render_setter_set() {
    let _owner = init_renderer();

    let app = view! {
        <SyncSsrSignal setup=|| {
            let sr = SsrSignalResource::new(String::new());
            provide_context(sr.clone());
        }>
            <Indicator />
            <SetterUsed mode=Some(Mode::Set) />
        </SyncSsrSignal>
    };
    assert_eq!(
        app.to_html_stream_in_order().collect::<String>().await,
        "<!><p>Indicator is: <!>Hello world!</p>resource write signal setting value: Hello world!<!>",
    );
}

#[cfg(feature = "ssr")]
#[tokio::test]
async fn render_setter_update() {
    let _owner = init_renderer();

    let app = view! {
        <SyncSsrSignal setup=|| {
            let sr = SsrSignalResource::new(String::new());
            provide_context(sr.clone());
        }>
            <Indicator />
            <SetterUsed mode=Some(Mode::Update) />
        </SyncSsrSignal>
    };
    assert_eq!(
        app.to_html_stream_in_order().collect::<String>().await,
        "<!><p>Indicator is: <!>Hello world!</p>resource write signal pushed value: Hello world!<!>",
    );
}

#[cfg(feature = "ssr")]
#[tokio::test]
async fn render_setter_update_untracked() {
    let _owner = init_renderer();

    let app = view! {
        <SyncSsrSignal setup=|| {
            let sr = SsrSignalResource::new(String::new());
            provide_context(sr.clone());
        }>
            <Indicator />
            <SetterUsed mode=Some(Mode::UpdateUntracked) />
        </SyncSsrSignal>
    };
    assert_eq!(
        app.to_html_stream_in_order().collect::<String>().await,
        "<!><p>Indicator is: <!>Hello world!</p>\
        resource write signal pushed value (untracked): Hello world!<!>",
    );
}

#[cfg(feature = "ssr")]
#[tokio::test]
async fn render_setter_not_set() {
    let _owner = init_renderer();

    let app = view! {
        <SyncSsrSignal setup=|| {
            let sr = SsrSignalResource::new(String::new());
            provide_context(sr.clone());
        }>
            <Indicator />
            <SetterUsed mode=None />
        </SyncSsrSignal>
    };
    assert_eq!(
        app.to_html_stream_in_order().collect::<String>().await,
        "<!><p>Indicator is: <!> </p>resource write signal setting no value<!>",
    );
}

#[cfg(feature = "ssr")]
#[tokio::test]
async fn setter_set_render() {
    let _owner = init_renderer();

    let app = view! {
        <SyncSsrSignal setup=|| {
            let sr = SsrSignalResource::new(String::new());
            provide_context(sr.clone());
        }>
            <SetterUsed mode=Some(Mode::Set) />
            <Indicator />
        </SyncSsrSignal>
    };
    assert_eq!(
        app.to_html_stream_in_order().collect::<String>().await,
        "<!>resource write signal setting value: Hello world!<p>Indicator is: <!>Hello world!</p><!>",
    );
}

#[cfg(feature = "ssr")]
#[tokio::test]
async fn setter_update_render() {
    let _owner = init_renderer();

    let app = view! {
        <SyncSsrSignal setup=|| {
            let sr = SsrSignalResource::new(String::new());
            provide_context(sr.clone());
        }>
            <SetterUsed mode=Some(Mode::Update) />
            <Indicator />
        </SyncSsrSignal>
    };
    assert_eq!(
        app.to_html_stream_in_order().collect::<String>().await,
        "<!>resource write signal pushed value: Hello world!<p>Indicator is: <!>Hello world!</p><!>",
    );
}

#[cfg(feature = "ssr")]
#[tokio::test]
async fn setter_update_untracked_render() {
    let _owner = init_renderer();

    let app = view! {
        <SyncSsrSignal setup=|| {
            let sr = SsrSignalResource::new(String::new());
            provide_context(sr.clone());
        }>
            <SetterUsed mode=Some(Mode::UpdateUntracked) />
            <Indicator />
        </SyncSsrSignal>
    };
    assert_eq!(
        app.to_html_stream_in_order().collect::<String>().await,
        "<!>resource write signal pushed value (untracked): Hello world!\
        <p>Indicator is: <!>Hello world!</p><!>",
    );
}

#[cfg(feature = "ssr")]
#[tokio::test]
async fn setter_not_set_render() {
    let _owner = init_renderer();

    let app = view! {
        <SyncSsrSignal setup=|| {
            let sr = SsrSignalResource::new(String::new());
            provide_context(sr.clone());
        }>
            <SetterUsed mode=None />
            <Indicator />
        </SyncSsrSignal>
    };
    assert_eq!(
        app.to_html_stream_in_order().collect::<String>().await,
        "<!>resource write signal setting no value<p>Indicator is: <!> </p><!>",
    );
}

/*
#[cfg(feature = "ssr")]
#[tokio::test]
async fn misused_write_only_cloned() {
    let _owner = init_renderer();

    let app = view! {
        <SyncSsrSignal setup=|| {
            let sr = SsrSignalResource::new(String::new());
            provide_context(sr.clone());
        }>
            <Indicator />
            <SetterMisusedWriteOnlyCloned />
        </SyncSsrSignal>
    };
    // Note that should the write signal be clonable, the naive implementation
    // of `CoReadyCoordinator::notify` would reset the value from Some(true)
    // back to a Some(false) which will result in a deadlock if a value is not
    // set as the drop condition gets reverted.
    //
    // In the updated implementation, it will simply cause the wait to not
    // happen due to the early drop and will not result in the correct value.
    assert_eq!(
        app.to_html_stream_in_order().collect::<String>().await,
        "<!><p>Indicator is: <!>Hello world!</p>resource write signal setting value: Hello world!<!>",
    );
}
*/

#[cfg(feature = "ssr")]
#[tokio::test]
async fn misused_write_only_created_late() {
    let _owner = init_renderer();

    let app = view! {
        <SyncSsrSignal setup=|| {
            let sr = SsrSignalResource::new(String::new());
            provide_context(sr.clone());
        }>
            <Indicator />
            <SetterMisusedWriteOnlyCreatedLate />
        </SyncSsrSignal>
    };
    // note that the resource wrote the signal but the indicator is unable to show it.
    // also note that there is no deadlock.
    assert_eq!(
        app.to_html_stream_in_order().collect::<String>().await,
        "<!><p>Indicator is: <!> </p>resource write signal setting value: Hello world!<!>",
    );
}

#[cfg(feature = "ssr")]
#[tokio::test]
async fn misused_write_only_kept_alive_deadlocks() {
    let _owner = init_renderer();

    let app = view! {
        <SyncSsrSignal setup=|| {
            let sr = SsrSignalResource::new(String::new());
            provide_context(sr.clone());
        }>
            <Indicator />
            <SetterMisusedWriteOnlyKeptAlive />
        </SyncSsrSignal>
    };

    // This deadlock happens because the setter was kept alive in the reactive
    // graph without being dropped (or otherwise written to).
    assert!(timeout(
        Duration::from_millis(500),
        app.to_html_stream_in_order().collect::<String>()
    )
    .await
    .is_err())
}

#[cfg(feature = "ssr")]
#[tokio::test]
async fn render_indicator_only() {
    let _owner = init_renderer();

    let app = view! {
        <SyncSsrSignal setup=|| {
            let sr = SsrSignalResource::new(String::new());
            provide_context(sr.clone());
        }>
            <Indicator />
        </SyncSsrSignal>
    };
    // no setter should not cause a deadlock.
    assert_eq!(
        app.to_html_stream_in_order().collect::<String>().await,
        "<!><p>Indicator is: <!> </p><!>",
    );
}

#[cfg(feature = "ssr")]
fn init_renderer() -> Owner {
    let _ = any_spawner::Executor::init_tokio();
    let owner = Owner::new();
    owner.set();
    owner
}
