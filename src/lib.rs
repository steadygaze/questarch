mod components;
#[cfg(feature = "ssr")]
mod ssr;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::components::app::*;
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
