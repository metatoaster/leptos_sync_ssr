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
use crate::ready::Ready;

#[component]
pub fn SyncSsr(children: Children) -> impl IntoView {
    // leptos::logging::log!("entering SyncSsr");
    #[cfg(feature = "ssr")]
    let ready = Ready::new();

    #[cfg(feature = "ssr")]
    let exit = {
        let ready = ready.clone();
        move || {
            ready.complete();
            // leptos::logging::log!("exiting SyncSsr");
        }
    };

    #[cfg(feature = "ssr")]
    let result = view! {
        <Provider value=ready>
            {children()}
            {exit}
        </Provider>
    };

    #[cfg(not(feature = "ssr"))]
    let result = children();

    result
}
