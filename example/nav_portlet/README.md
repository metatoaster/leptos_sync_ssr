# Portlet Example in Leptos

An example to demostrate reusable portlets via effects and signals in
Leptos.  In this particular example, there is a navigation portlet
mounted as a component inside the `App` that would render data from a
specially prepared `ArcResource` created from a `SsrSignalResource`,
Naturally, another component will typically assign the data through the
corresponding `ArcWriteSignal`.  The usual flow is that the act of
assignment of a new resource will trigger the corresponding updates to
the rendering.

The caveat in this example is that the portlet component is mounted
before the `Routes`.  It means when under SSR, the signal for the
resource should be read before it's assigned, and it's very likely the
rendered output was already prepared and sent down the wire before it's
assigned with some resource.  By that time, it's already too late and
the only way it would ever render would be via client-side rendering.
At least this is how the story typically goes.

If there is some way to pause the resource that will read from the
signal under SSR, the situation will be different.  The specially
prepared resource just simply read from a `ArcReadSignal`, but under SSR
that is first guarded by a wait lock.  On the other end, the
`SsrSignalResource` also provides a `SsrWriteSignal` that will signal
the clearing of that lock should a value be written to it, allowing the
component later down the view tree to send a value as if back in time to
one that sits earlier up the view tree.

The underlying wait lock is provided by the `leptos_sync_ssr` package is
also dependent on the `SyncSsrSignal` component which encloses the
components that should be synchronized so that the underlying `CoReady`
and `CoReadySubscription` instances will be signaled to continue at the
appropriate times.

In this example, the use of `PortletCtx` is used instead, which
abstracts the intricacies of using the `SsrSignalResource` as there are
guidelines to ensure it doesn't produce unexpected results or worse,
deadlock the application.  This also serves as a reference on how to
implement this particular pattern under Leptos in a managed manner.

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
