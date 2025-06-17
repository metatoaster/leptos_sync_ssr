[![CI](https://github.com/metatoaster/leptos_sync_ssr/actions/workflows/build.yml/badge.svg?branch=0.1)](https://github.com/metatoaster/leptos_sync_ssr/actions/workflows/build.yml?query=branch:0.1)
[![crates.io](https://img.shields.io/crates/v/leptos_sync_ssr)](https://crates.io/crates/leptos_sync_ssr/)
[![docs.rs](https://docs.rs/leptos_sync_ssr/badge.svg)](https://docs.rs/leptos_sync_ssr/latest/leptos_sync_ssr/)

# `leptos_sync_ssr`

`leptos_sync_ssr` provides helpers to synchronize the access of Leptos
resources during server-side rendering (SSR) within the Leptos
frameworks.  This is achieved by providing locking primitives, which are
then built into the provided components and signals, such that when they
are used together in the prescribed manner, it would allow the affected
resources to resolve in the expected order.  This enables components
defined earlier in the view tree to wait on data that may or may not be
provided by components defined later down the view tree, ultimately
ensuring that hydration would happen correctly.

## Use case

A fairly common user interface design pattern known as [portlets](
https://en.wikipedia.org/wiki/Portlet) may be implemented in Leptos
using a struct with data fields to be rendered be available in through a
reactive signal behind a context, with the renderer being a component
that would reactively read the value from that reactive signal such that
it may be updated as required.  This is not a problem under client-side
rendering (CSR) for Leptos, but for SSR with hydration, this is a whole
other story.

If the rendering component is situated in the view tree before the
component that may write to it, as in the case of a typical "breadcrumb"
component, this creates the complication where the signal may not be set
in time by the time the breadcrumb component was streamed out to the
client with the default data.  Furthermore, the hydration script may
contain the expected data, and when used to hydrate the rendered markup
which used the default data from earlier, this mismatch wil result in
hydration error.

There are multiple solutions to this problem, and this crate provides a
number of them, when combined together, addresses the rendering and
hydration issues, without bringing in new problems that the individual
solution would bring when used in isolation.

## Example

As an example, here's a component using some resource that may be set
later through a signal:

```rust
#[component]
fn UsingSignal() -> impl IntoView {
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
`App`, as it uses the `<Provider>` component underneath to scope `Ready`
to where it's required. Refer to the [`simple`](example/simple/) example
this scoped example.  For a more practical demonstration, refer to the
[`nav_portlet_alt`](example/nav_portlet_alt/) example.

This sending of a whole resource through a signal, while feasible, is a
bit cumbersome to use and write and not as ergonomic as using a standard
signal.  This is where a second method was brought in - inspired by the
[`leptos_async_signal`](https://github.com/demiurg-dev/leptos_async_signal/)
crate, this package also provides a signal, `SsrResourceSignal`, which
is not too dissimilar to the one from that crate at first glance, as
it's the pairing of a resource that would offer the data read from the
paired signal, but there are significant difference (when compared to
`leptos_async_signal-0.6.0`) underneath.

To begin with, `SsrResourceSignal` is significantly more defined in
terms of how the wait lock is managed.  First, the wait lock is only
fully activated if a corresponding write signal is acquired, and second,
dropping of the unused lock typically also release the wait lock.  The
first part enables the `read_only` side to return the data if it's known
that nothing would write to it, due to the lack of acquisition of any
`write_only` side, or the `SsrWriteSignal`.  This part is co-ordinated
using the required `<SyncSsrSignal/>` component, such that if no
instances of `SsrWriteSignal` are around when it signals a ready, the
`read_only` resource will be able to yield the data.  The second part
simply ensures that accidental non-usage of acquired `write_only` side
will not deadlock the application, though purposefully stash and forget
that somewhere or otherwise not notifying the release will always cause
a deadlock.

Moreover, for the `SsrWriteSignal` from the `write_only` end, implements
the `Write` trait plus others, such that the full suite of [reactive
traits methods](https://docs.rs/leptos/latest/leptos/reactive/traits/)
may be used.

Naturally, this more involved implementation requires more careful use,
and this underlying signal-resource pairing is further wrapped by the
`PortletCtx` type, which provides an additional abstraction layer via
helper methods to avoid problems caused by mis-use of the underlying
`SsrResourceSignal`.  The following is a Leptos app that shows the
typical use of a `PortletCtx` context with the `<SyncSsrSignal/>`
component to create a portlet that is placed onto the header of the
`<App/>`, to serve as a rough representation of what typical use might
look like.

```rust
use leptos::prelude::*;
use leptos_router::{
    components::{Route, Router, Routes},
    path,
};
use leptos_sync_ssr::{component::SyncSsrSignal, portlet::PortletCtx};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
struct Breadcrumbs {
    // fields...
}

#[component]
fn App() -> impl IntoView {
    view! {
        <Router>
            <SyncSsrSignal setup=|| {
                <PortletCtx<Breadcrumbs>>::provide();
            }>
                <header>
                    <ShowBreadcrumbs/>
                </header>
                <article>
                    <Routes fallback=|| ()>
                        <Route path=path!("") view=HomePage/>
                        <Route path=path!("/blog/:id") view=BlogView/>
                        // plus other routes
                    </Routes>
                </article>
            </SyncSsrSignal>
        </Router>
    }
}

impl IntoRender for Breadcrumbs {
    type Output = AnyView;

    fn into_render(self) -> Self::Output {
        view! {
            todo!()
        }
        .into_any()
    }
}

#[component]
fn ShowBreadcrumbs() -> impl IntoView {
    <PortletCtx<Breadcrumbs>>::render()
}

#[component]
fn BlogView() -> impl IntoView {
    // assuming this was provided/defined elsewhere
    let blog = expect_context::<ArcResource<Result<Blog, Error>>>();
    let nav_ctx = expect_context::<PortletCtx<Breadcrumbs>>();

    view! {
        // Pass `.set_with()` with a `Future` that returns the value
        // expected, in this case it would be the `Breadcrumbs` to be
        // rendered.  This function returns a `Suspense` view which will
        // drive the update.
        //
        // If the portlet is intended to be reactive based on resources,
        // the resources should be tracked here, but only for CSR.
        // Refer to documentation for details.
        {nav_ctx.set_with(move || {
            let blog = blog.clone();
            async move {
                blog.await
                    // convert the blog into the `Breadcrumbs` type here
                    .map(|blog| todo!())
                    .ok()
            }
        })}
        <div>
            // Other components/elements.
        </div>
    }
}
```

## Supported Leptos version

This crate requires Leptos version `0.8.0` or later.

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

## Alternative crate, solutions and limitations

The approach provided by this crate is certainly not the only option for
passing values asynchronously to an earlier component.  As mentioned,
An alternative is to follow the approach taken by [`leptos_async_signal`
](https://github.com/demiurg-dev/leptos_async_signal/).  That crate
provides a mechanism for generating values asynchronously, it claims to
mimic the approach taken by `leptos_meta`, however, it does require the
`AsyncWriteSignal` be used if the paired `ArcResource` were to be read,
otherwise a deadlock will ensure, and that all clones of the write
signal must be dropped, much like the `SsrWriteSignal` offered by this
package (though that itself is not `Clone`, it is possible to generate
multiple copies through multiple invocations of `.write_only()`,
however, the difference here with `SsrWriteSignal` is that its existence
is not automatic, as explained earlier.

Hence this crate also provide a similar approach taken by
`leptos_async_signal`, but with significant improvements, such that it
is possible to provide the default value without being locked out if the
active view tree does not need to write to it, and that the full suite
of update traits may be used, rather than just `Set`, plus the option to
automatically stop waiting when the writer is dropped may be used.

That being said, the approach taken by `leptos_async_signal` is much
more rigid and reliable given its direct implementation of `Set`, which
has the bonus of being fully unaffected by the interactions between
work-stealing scheduler and how Leptos handles the `Suspend` and
resource futures, when used correctly. This was tested using
`0.8.0-beta` and with 200k requests (5 concurrent).  Whereas the
solution provided with `leptos_sync_ssr` merely extends on the existing
features so the issues of that interaction will still apply.  In 100k
requests, up to 10 requests may have an unexpected output which may or
may not affect hydration, although this may simply caused by a lack of
synchronization in Leptos itself when running inside a work-stealing
task scheduler.

That all being said, `SsrSignalResource`, which is developed with
inspirations from `leptos_async_signal`, does in fact produce the
expected output when the underlying issues affected by the work-stealing
task schedulers are solved.  Further discussions under the following
GitHub issues documents the current findings about the view/resource
structures that are used in/with this package.

- [`leptos/leptos-rs#3729`](https://github.com/leptos-rs/leptos/issues/3729)
- [`leptos/leptos-rs#4060`](https://github.com/leptos-rs/leptos/issues/4060)
- [`leptos/leptos-rs#4065`](https://github.com/leptos-rs/leptos/issues/4065)

## License

This crate is provided under the MIT license.
