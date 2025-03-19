#[cfg(feature = "ssr")]
mod ssr {
    pub use leptos::{context::Provider, view};
}

#[cfg(feature = "ssr")]
use ssr::*;

use leptos::{
    children::Children,
    IntoView, component,
};

#[cfg(feature = "ssr")]
use crate::waiter::Waiter;

#[component]
pub fn SyncSsr(children: Children) -> impl IntoView {
    // leptos::logging::log!("entering SyncSsr");
    #[cfg(feature = "ssr")]
    let waiter = Waiter::new();

    #[cfg(feature = "ssr")]
    let exit = {
        let waiter = waiter.clone();
        move || {
            waiter.complete();
            // leptos::logging::log!("exiting SyncSsr");
        }
    };

    #[cfg(feature = "ssr")]
    let result = view! {
        <Provider value=waiter>
            {children()}
            {exit}
        </Provider>
    };

    #[cfg(not(feature = "ssr"))]
    let result = children();

    result
}
