#[cfg(feature = "ssr")]
mod app_state;
mod components;
#[cfg(feature = "ssr")]
mod key;
#[cfg(feature = "ssr")]
mod mail;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::components::app::*;
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
