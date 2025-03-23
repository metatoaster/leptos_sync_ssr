use leptos::prelude::*;
use leptos_meta::{MetaTags, *};
use leptos_router::{
    components::{Route, Router, Routes, A},
    path, SsrMode,
};

use leptos_sync_ssr::component::SyncSsr;
// for laziness, feature gating may be omitted, but may add to the wasm
// size as cost; likewise for all usage of Ready below.
// #[cfg(feature = "ssr")]
use leptos_sync_ssr::Ready;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone()/>
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[server]
async fn server_call() -> Result<(), ServerFnError> {
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    Ok(())
}

#[component]
pub fn App() -> impl IntoView {
    leptos::logging::log!(">>>>>>>>>>>>>>>>>>>>>>>>");
    provide_meta_context();
    let fallback = || view! { "Page not found." }.into_view();
    view! {
        <Stylesheet id="leptos" href="/pkg/simple.css"/>
        <Title text="Simple example"/>
        <Meta name="color-scheme" content="dark light"/>
        <Router>
            <header>
                <div id="notice">
                    "This WASM application has panicked, please refer to the "
                    "console log for details.  Go "
                    <a href="/" target="_self">"Home"</a>" "
                    "to restart the application."
                </div>
                <nav>
                    <a href="/" target="_self">"Home"</a>" | "
                    <A href="/non-issue">"Non-issue"</A>" | "
                    <A href="/error">"Hydration Error"</A>" | "
                    <A href="/fixed">"Fixed"</A>
                </nav>
            </header>
            <main>
                <Routes fallback>
                    <Route path=path!("") view=HomePage/>
                    <Route path=path!("non-issue") view=NonIssue ssr=SsrMode::Async/>
                    <Route path=path!("error") view=Uncorrected ssr=SsrMode::Async/>
                    <Route path=path!("fixed") view=Corrected ssr=SsrMode::Async/>
                </Routes>
            </main>
        </Router>
    }
}

#[component]
pub fn HomePage() -> impl IntoView {
    view! {
        <p>"Select one of the links above."</p>
    }
}

#[component]
pub fn UsingSignal(
    rs: ReadSignal<Option<OnceResource<String>>>,
) -> impl IntoView {
    // Ready will have _no_ effect whatsoever if this component is not
    // enclosed by the `SyncSsr` component.
    // #[cfg(feature = "ssr")]
    let ready = Ready::handle();

    // Since the signal may not actually contain a resource, it cannot
    // be awaited.  Encapsulate the usage of the signal in another
    // resource.
    let value = Resource::new_blocking(
        || (),
        move |_| {
            // #[cfg(feature = "ssr")]
            let ready = ready.clone();
            async move {
                leptos::logging::log!("preparing to subscribe and wait");
                // #[cfg(feature = "ssr")]
                ready.subscribe().wait().await;
                leptos::logging::log!("subscription finished waiting");
                let value = if let Some(Some(res)) = rs.try_get() {
                    leptos::logging::log!("readsignal has OnceResource");
                    let result = Some(res.await);
                    leptos::logging::log!("finished awaiting for OnceResource");
                    result
                } else {
                    None
                };
                leptos::logging::log!("value: {value:?}");
                value
            }
        },
    );
    view! {
        <p>
            <code>"<UsingSignal/>"</code>
            " accessing the resource for the value from signal: "
            <Suspense>{
                move || Suspend::new(async move {
                    leptos::logging::log!("Inside suspense");
                    let result = if let Some(value) = value.await {
                        leptos::logging::log!("value.await got Some");
                        Some(view! {
                            <strong>{value}</strong>
                        }
                        .into_any())
                    } else {
                        leptos::logging::log!("value.await got None");
                        None
                    };
                    leptos::logging::log!("Suspense rendered");
                    result
                })
            }</Suspense>
            "."
        </p>
    }
}

#[component]
pub fn SettingSignal(
    ws: WriteSignal<Option<OnceResource<String>>>,
) -> impl IntoView {
    on_cleanup(move || {
        leptos::logging::log!("Running on_cleanup");
        ws.try_set(None);
    });

    let server_call = Resource::new_blocking(
        || (),
        |_| async move {
            server_call().await
        },
    );

    let static_value = "Hello World!";
    // while the resource can be set via the signal directly here, having
    // a hook that gets called when the view is initiated emulates a kind
    // of reactivity that would determine whether or not it is set.
    let hook = move || ws.set(Some(OnceResource::new(async move {
        let _ = server_call.await;
        static_value.to_string()
    })));
    view! {
        {hook}
        // If the following block is inside a Suspense that awaits some
        // resource, it can sort of function as a workaround as it would
        // provide a delay.  However this method is not reliable as it
        // depends on a race condition, where if the suspense is
        // triggered it would also delay the stream from being sent out
        // with the resource not already set.  It probably will work,
        // but this is not reliable especially when running inside a
        // work-stealing async runtime, and will just ultimately be a
        // source of Heisenbugs.
        <p>
            <code>"<SettingSignal/>"</code>
            " set a signal with a resource that will provide the value: "
            {static_value.to_string()}"."
        </p>
    }
}

#[component]
pub fn NonIssue() -> impl IntoView {
    // Rather than simply passing the string via the signal, pass a
    // resource as it's one way to set a value asynchronous, and that
    // dealing with Tokio channels on the server requires async.
    //
    // Moreover, using a resource is a requirement for passing values
    // backwards for isomorphic SSR/CSR rendering as it will allow it
    // be awaited inside `Suspense` under modes like `SsrMode::Async`.
    let (rs, ws) = signal(None::<OnceResource<String>>);

    view! {
        <h1>"Probably a non-issue"</h1>
        <p>
            "Reload this page under SSR to see if the hydration issue may be "
            "triggered.  This version shouldn't have issue as the setter is "
            "before the user.  Do note that when running under the default "
            "Tokio multithread runtime (for Axum), there may be variations "
            "with the size of the document from the effects of uncontrolled "
            "race conditions from the interactions between the signals and "
            "resources used even in this small example."
        </p>
        <dl>
            <dt>
                <code>"<SettingSignal/>"</code>
            </dt>
            <dd>
                <SettingSignal ws/>
            </dd>
            <dt>
                <code>"<UsingSignal/>"</code>
            </dt>
            <dd>
                <UsingSignal rs/>
            </dd>
        </dl>
    }
}

#[component]
pub fn HydrationIssue(
    #[prop(default = "Below can result in hydration issue")]
    title: &'static str,
    children: Children,
) -> impl IntoView {
    let (rs, ws) = signal(None::<OnceResource<String>>);

    view! {
        <h1>{title}</h1>
        <p>{children()}</p>
        <dl>
            <dt>
                <code>"<UsingSignal/>"</code>
            </dt>
            <dd>
                <UsingSignal rs/>
            </dd>
            <dt>
                <code>"<SettingSignal/>"</code>
            </dt>
            <dd>
                <SettingSignal ws/>
            </dd>
        </dl>
    }
}

#[component]
pub fn Uncorrected() -> impl IntoView {
    view! {
        <HydrationIssue>
            "Reload this page under SSR to see if the hydration "
            "issue may be triggered.  This version may have issues during "
            "hydration given that the signal setter is after the user, and "
            "this uncertainty is partly due to the work-stealing task "
            "runner that Axum uses - it may or may not result in the "
            "correct race condition to happen to trigger the desired "
            "outcome."
        </HydrationIssue>
    }
}

#[component]
pub fn Corrected() -> impl IntoView {
    view! {
        <SyncSsr>
            <HydrationIssue title="Below shouldn't have hydration issues.">
                "Reload this page under SSR to see if the hydration issue "
                "may be triggered.  This version should not trigger any "
                "hydration issues even though it is the exact same "
                "component, just that it's been wrapped with the "<code>
                "<SyncSsr>"</code>" component."
            </HydrationIssue>
        </SyncSsr>
    }
}
