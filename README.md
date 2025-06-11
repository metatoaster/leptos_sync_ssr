# `leptos_sync_ssr`

`leptos_sync_ssr` provides an additional component and primitive that
would aid in synchronize access of Leptos resource during server-side
rendering (SSR) within the Leptos integration frameworks.

## Introduction

A fairly common user interface design pattern, where a common reusable
element or component situated earlier (that is, placed somewhere before
the other) depending on resources provided by some later component.
Examples of such designs include breadcrumbs and portlets.

This pattern is supported well under Leptos for CSR, but under SSR +
hydration, this pattern is barely supported, if at all.  This is simply
due to how signals and resources are resolve in the order they are
defined, and once they are resolved, they may get sent down to the
client, possibly without the intended resource that would produce the
desired content set.

One approach to address this issue is to provide an additional server-
side only component that will provide a broadcast channel that resources
may listen and wait for, such that only after all the relevant
processing is completed that a signal will be sent to allow those
withheld resources to continue processing.  This additional delay will
ensure the correct values be read and the intended output be produced.

## Example

As an example, here's a component using some resource that may be set
later through a signal:

```rust
#[component]
pub fn UsingSignal() -> impl IntoView {
    let rs = expect_context::<ReadSignal<Option<OnceResource<String>>>>();
    let ready = Ready::handle();
    let value = Resource::new_blocking(
        || (),
        move |_| {
            let ready = ready.clone();
            async move {
                // This ensures the async closure will wait until the
                // ready signal is received before trying to read from
                // the signal to access the resource.  Moreover, the
                // implementation only functions under SSR, despite the
                // lack of feature gating here as a dummy no-op version
                // is provided for CSR.  Refer to examples for more
                // documentation on this.
                ready.subscribe().wait().await;
                if let Some(Some(res)) = rs.try_get() {
                    Some(res.await)
                } else {
                    None
                }
            }
        },
    );
    view! {
        <p>
            <span>"The content is: "</span>
            <Suspense>{
                move || Suspend::new(async move {
                    value.await.map(|value| {
                        view! {
                            <strong>{value}</strong>
                        }
                    })
                })
            }</Suspense>
        </p>
    }
}
```

Where the usage of the write signal may occur some time after this.  The
second part is to enclose the affected components, i.e. the reader and
all the possible writers, inside the `<SyncSsr>` component.  Typically
this may be done at inside the `<App>`, e.g.:

```rust
    view! {
        <Router>
            <header>
                <nav>
                    // link to routes...
                </nav>
            </header>
            <main>
                <SyncSsr>
                    <UsingSignal/>
                    <Routes fallback>
                        <Route path=path!("") view=HomePage/>
                        <RoutesThatMaySetSignal/>
                    </Routes>
                </SyncSsr>
            </main>
        </Router>
    }
```

The usage of `<SyncSsr>` component is not just limited to the top level
`App`, as it uses the `<Provider>` component underneath to scope
`Ready` to where it's required. Refer to the [`simple`](example/simple/)
example this scoped example, and for a more practical and complete
example, refer to the [`nav_portlet`](example/nav_portlet/) example,
which uses a different but similar `<SyncSsrSignal/>` component for a
different kind of synchronization in conjunction with `PortletCtx<T>`.

## Supported Leptos version

This package requires Leptos version `0.8.0` or later.

## Usage

To use `leptos_sync_ssr`, add it to `Cargo.toml`, and use the `ssr`
feature as per convention:

```toml
[dependencies]
leptos_sync_ssr = "0.1.0-beta"
leptos = "0.8.0"

[features]
hydrate = [
  "leptos/hydrate",
]
# the app will need the ssr feature from leptos_sync_ssr
ssr = [
  "leptos/ssr",
  "leptos_sync_ssr/ssr",
]
```

A more complete [example `Cargo.toml`](example/sample/Cargo.toml) from
the `simple` example.

## Alternative packages, solutions and limitations

The approach provided by this package is certainly not the only option
for passing values asynchronously to an earlier component.  One
alternative is to follow the approach taken by [`leptos_async_signal`](
https://github.com/demiurg-dev/leptos_async_signal/).  That package
provides a mechanism for generating values asynchronously, it claims to
mimic the approach taken by `leptos_meta`, however, it does require the
`AsyncWriteSignal` be used if the paired `ArcResource` were to be read,
otherwise a deadlock will ensure, The particular issue about unused
signals causing deadlocks may be addressed should this [pull request](
https://github.com/demiurg-dev/leptos_async_signal/pull/15) be merged.
However, there are other rules that must be followed to avoid deadlocks,
so extra care must be taken to use its `async_signal` correctly.

On the other hand, the raw signals to control waiting provided by
`leptos_sync_ssr` does not have such limitations - the waiting can
happen inside a `Suspend`, just that it may be better to have the wait
done in `Resource` simply due to how Leptos SSR will always poll
`Resource` unlike `Suspend`.  Waiting inside the `Suspend` will have
somewhat more variations and thus having somewhat lower reliability of
this working correctly under a work-stealing task scheduler.

Hence this package also provide a similar approach taken by
`leptos_async_signal`, but with significant improvements, such that it
is possible to provide the default value without being locked out if the
active view tree does not need to write to it, and that the full suite
of update traits may be used, rather than just `Set`, plus the option to
automatically stop waiting when the writer is dropped may be used.

That being said, the approach taken by `leptos_async_signal` is much
more rigid given its direct implementation of `Set`, which has the bonus
of being fully unaffected by the interactions between work-stealing
scheduler and how Leptos handles the `Suspend` and resource futures,
when used correctly. This was tested using `0.8.0-beta` and with 200k
requests (5 concurrent).  Whereas the solution provided with
`leptos_sync_ssr` merely extends on the existing features so the issues
of that interaction will still apply.  In 100k requests, up to 10
requests may have an unexpected output which may or may not affect
hydration, although this may simply caused by a lack of synchronization
in Leptos itself when running inside a work-stealing task scheduler. A
discussion of the underlying topic at [`leptos/leptos-rs#3729`](
https://github.com/leptos-rs/leptos/issues/3729) currently documents my
findings with the particular pattern I've used.

Using `SsrSignalResource`, which is developed with inspirations from
`leptos_async_signal`, approaches the expected level of correctness.
Benchmarks test results will be provided later, as the correctness
under a work-stealing task scheduler are further affected by
[`leptos/leptos-rs#4060`](https://github.com/leptos-rs/leptos/issues/4060),
and that
[`leptos/leptos-rs#4065`](https://github.com/leptos-rs/leptos/issues/4065),
may also affect this.

## License

This package is provided under the MIT license.
