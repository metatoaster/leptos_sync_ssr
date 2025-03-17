# Portlet Example in Leptos

An example to demostrate reusable portlets via effects and signals in
Leptos.  In this particular example, there is a navigation portlet
mounted as a component inside the `App` that would render data from a
resource provided from a `ReadSignal`.  Naturally, another component
will typically assign the desired resource through the corresponding
`WriteSignal`.  The usual flow is that the act of assignment of a new
resource will trigger the corresponding updates to the rendering.

The caveat in this example is that the portlet component is mounted
before the `Routes`.  It means when under SSR, the signal for the
resource is read before it's assigned, and very likely the rendered
output was already prepared and sent down the wire before it's assigned
with some resource.  By that time, it's already too late as the client
will try to hydrate based on the fact that a resource is available from
the signal, but the SSR was not rendered as such, resulting in the much
dreaded hydration error.

If there is some way to pause the resource that will read from the
signal under SSR, the situation will be different.  This `Waiter`
provided by the `leptos_sync_ssr` package is one part of the solution
that will achieve that, as it will wait until a signal is provided
before it will continue.  The other part is the `SyncSsr` component
which encloses the components that should be synchronized so that the
`Waiter` will be signaled to continue on from this synchronized state.

In this example, the `ReadSignal` will not be read until the `Waiter`
resolves, and by enclosing the portlet component with the routes will
ensure that whatever routes setting the resource through the signal will
do so in a way what ensures the component will read it correctly during
SSR so that the hydration error be averted.

## Quick Start

This demo is implemented in a way that integrates both axum and actix
as options that may be toggled using ``--bin-features`` flag.

Run:

- `cargo leptos watch --bin-features axum` to serve using axum.
- `cargo leptos watch --bin-features actix` to serve using actix.

Reason for providing both these SSR runtimes is to show how the task
stealing scheduler used by Axum does not impact on this feature, or at
the very least, Actix should show no such issue.  Nonetheless this is a
tool to help identify bugs.
