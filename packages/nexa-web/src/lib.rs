use nexa_core::Runtime;
use wasm_bindgen::prelude::*;

// Platform adapter for Web (WASM/DOM)

#[wasm_bindgen]
pub struct WebApp {
    runtime: Runtime,
}

#[wasm_bindgen]
impl WebApp {
    pub fn new() -> Self {
        Self {
            runtime: Runtime::new(),
        }
    }

    pub fn launch(root_id: &str) {
        // Locate root element
        let window = web_sys::window().expect("no global `window` exists");
        let document = window.document().expect("should have a document on window");
        let _root = document
            .get_element_by_id(root_id)
            .expect("root element not found");

        // Initialize runtime
        let mut _app = WebApp::new();

        // In a real app, we'd mount the initial VNode tree here.
        // For scaffolding, we expose the entry point.
        console_log("Nexa Web App Launched");
    }
}

fn console_log(s: &str) {
    web_sys::console::log_1(&JsValue::from_str(s));
}
