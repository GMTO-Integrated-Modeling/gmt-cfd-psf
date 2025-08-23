// pub mod app;
pub mod claude_app;
pub mod components;
pub mod server;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use claude_app::*;
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
