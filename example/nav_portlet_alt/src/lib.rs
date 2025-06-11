pub mod app;
pub mod portlet;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use app::*;
    use std::panic;
    panic::set_hook(Box::new(|info| {
        // this custom hook will call out to show the usual error log at
        // the console while also attempt to update the UI to indicate
        // a restart of the application is required to continue.
        console_error_panic_hook::hook(info);
        let document = leptos::prelude::document();
        let _ = document.query_selector("#notice").map(|el| {
            el.map(|el| {
                el.set_class_name("panicked");
            })
        });
    }));
    leptos::mount::hydrate_body(App);
}
