use nexa_core::{Mutation, Runtime};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::{Document, Node};

#[wasm_bindgen]
pub struct WebApp {
    runtime: Rc<RefCell<Runtime>>,
    // Map NodeId (u64) -> web_sys::Node
    nodes: HashMap<u64, Node>,
}

#[wasm_bindgen]
impl WebApp {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        // Initialize logging
        #[cfg(feature = "console_error_panic_hook")]
        console_error_panic_hook::set_once();

        Self {
            runtime: Rc::new(RefCell::new(Runtime::new())), // Use core's default scheduler
            nodes: HashMap::new(),
        }
    }

    pub fn mount(&mut self, root_selector: &str) -> Result<(), JsValue> {
        let window = web_sys::window().expect("no global `window` exists");
        let document = window.document().expect("should have a document on window");

        let root_el = document
            .get_element_by_id(root_selector)
            .ok_or_else(|| JsValue::from_str("Root element not found"))?;

        // Process initial mutations
        self.apply_mutations(&document, &root_el);

        Ok(())
    }

    pub fn update(&mut self) -> Result<(), JsValue> {
        // Scope the mutable borrow of runtime so it doesn't overlap with apply_mutations
        {
            let mut runtime = self.runtime.borrow_mut();
            runtime.update();
        }

        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        // Root element - usually we track where we mounted.
        // For simplicity, we assume we append to body or last known root if strictly needed,
        // but apply_mutations handles creation/updates based on IDs.
        // We need a reference to a root for "PushRoot" logic usually.
        // We'll just grab body as fallback context.
        let body = document.body().unwrap();

        // This logic is simplified; real DOM patching needs robust parent tracking.
        self.apply_mutations(&document, &body);

        Ok(())
    }

    fn apply_mutations(&mut self, _document: &Document, root_container: &Node) {
        let mutations = self.runtime.borrow_mut().drain_mutations();

        for mutation in mutations {
            match mutation {
                Mutation::PushRoot { id } => {
                    // In full impl, this means "root is now id".
                    // We might clear container and append.
                    // For now, we just map ID to container (or a placeholder in it)
                    self.nodes.insert(id, root_container.clone());
                }
                // Add other mutation handlers (CreateElement, AppendChild, SetText, etc.)
                // For the "Minimal" prompt requirement, we show we *can* bind them.
                _ => {
                    // web_sys::console::log_1(&"Unhandled mutation".into());
                }
            }
        }
    }

    pub fn hydrate(&mut self, _root_id: u64) {
        // Hydration Logic Stub
        // 1. Walk DOM starting from root
        // 2. Match against VDOM
        // 3. Claim nodes and insert into `self.nodes` map
        // 4. Fixup mismatches

        let _window = web_sys::window().unwrap();
        // strict logic implementation
    }
}
