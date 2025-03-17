use leptos::prelude::*;

#[cfg(feature = "ssr")]
use crate::waiter::{Waiter, provide_waiter};

#[component]
pub fn SyncSsr(children: Children) -> impl IntoView {
    // leptos::logging::log!("entering SyncSsr");
    #[cfg(feature = "ssr")]
    provide_waiter();

    let exit = move || {
        #[cfg(feature = "ssr")]
        Waiter::complete();
        // leptos::logging::log!("exiting SyncSsr");
    };

    view! {
        {children()}
        {exit}
    }
}
