use leptos::prelude::*;
use leptos_meta::{MetaTags, *};
use leptos_router::{
    components::{Route, Router, Routes, A},
    path, SsrMode,
};

use leptos_sync_ssr::component::SyncSsr;
#[cfg(feature = "ssr")]
use leptos_sync_ssr::waiter::Waiter;

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
                    "console log for details.  Go back "
                    <a href="/" target="_self">"Home"</a>
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
                    <Route path=path!("error") view=HydrationIssue ssr=SsrMode::Async/>
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
pub fn UsingSignal() -> impl IntoView {
    // Get the signal for the resource in here at the component level.
    // Rather than getting some string, the result is encapsulated in a
    // resource to simulate how this string may in fact be generated
    // using data from a server function, but that is set elsewhere.
    let rs = expect_context::<ReadSignal<Option<OnceResource<String>>>>();

    // This waiter will have _no_ effect whatsoever if this component
    // is not enclosed by the `SyncSsr` component.
    #[cfg(feature = "ssr")]
    let waiter = Waiter::handle();

    // Since the signal may not actually contain a resource, it cannot
    // be awaited.  Encapsulate the usage of the signal in another
    // resource.
    let value = Resource::new_blocking(
        || (),
        move |_| {
            #[cfg(feature = "ssr")]
            let waiter = waiter.clone();
            async move {
                leptos::logging::log!("preparing to subscribe and wait");
                #[cfg(feature = "ssr")]
                waiter.subscribe().wait().await;
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
pub fn SettingSignal() -> impl IntoView {
    let ws = expect_context::<WriteSignal<Option<OnceResource<String>>>>();
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
    // The idea is to provide a signal of a resource, and using a
    // resource is required for this pattern simply because they will
    // be waited under modes like `SsrMode::Async.
    let (rs, ws) = signal(None::<OnceResource<String>>);
    provide_context(rs);
    provide_context(ws);

    view! {
        <h1>"Probably a non-issue"</h1>
        <p>
            "Reload this page under SSR to see if the hydration issue may be "
            "triggered.  This version shouldn't have issue as the setter is "
            "before the user.  Do note that when running under the default "
            "tokio multithread runtime (for axum), there may be variations "
            "with the size of the document from the effects of uncontrolled "
            "race conditions from the interactions between the signals and "
            "resources used even in this small example."
        </p>
        <dl>
            <dt>
                <code>"<SettingSignal/>"</code>
            </dt>
            <dd>
                <SettingSignal/>
            </dd>
            <dt>
                <code>"<UsingSignal/>"</code>
            </dt>
            <dd>
                <UsingSignal/>
            </dd>
        </dl>
    }
}

#[component]
pub fn HydrationIssue() -> impl IntoView {
    // The idea is to provide a signal of a resource, and using a
    // resource is required for this pattern simply because they will
    // be waited under modes like `SsrMode::Async.
    let (rs, ws) = signal(None::<OnceResource<String>>);
    provide_context(rs);
    provide_context(ws);

    view! {
        <h1>"Below can result in hydration issue"</h1>
        <p>
            "Reload this page under SSR to see if the hydration issue may be "
            "triggered.  This version may have issues during hydration given "
            "that the signal setter is after the user."
        </p>
        <dl>
            <dt>
                <code>"<UsingSignal/>"</code>
            </dt>
            <dd>
                <UsingSignal/>
            </dd>
            <dt>
                <code>"<SettingSignal/>"</code>
            </dt>
            <dd>
                <SettingSignal/>
            </dd>
        </dl>
    }
}

#[component]
pub fn Corrected() -> impl IntoView {
    view! {
        <SyncSsr>
            <HydrationIssue/>
        </SyncSsr>
    }
}
