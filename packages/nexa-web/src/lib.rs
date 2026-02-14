use nexa_core::{Mutation, Runtime};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::{Document, Element, Event, Node};

#[wasm_bindgen]
pub struct WebApp {
    runtime: Rc<RefCell<Runtime>>,
    interpreter: Rc<RefCell<WebInterpreter>>,
}

struct WebInterpreter {
    document: Document,
    nodes: HashMap<u64, Node>,
    event_listeners: HashMap<u64, Vec<Closure<dyn FnMut(Event)>>>,
    root_id: Option<u64>,
}

impl WebInterpreter {
    fn new(document: Document) -> Self {
        Self {
            document,
            nodes: HashMap::new(),
            event_listeners: HashMap::new(),
            root_id: None,
        }
    }

    fn apply_mutations(&mut self, mutations: Vec<Mutation>) {
        // Efficient batching: We use a single loop to apply all mutations.
        // In a more advanced implementation, we could sort or group them.
        for mutation in mutations {
            match mutation {
                Mutation::PushRoot { id } => {
                    self.root_id = Some(id);
                }
                Mutation::CreateElement { tag, id } => {
                    let el = self.document.create_element(&tag).unwrap();
                    self.nodes.insert(id, el.into());
                }
                Mutation::CreateTextNode { text, id } => {
                    let node = self.document.create_text_node(&text);
                    self.nodes.insert(id, node.into());
                }
                Mutation::AppendChildren { id, m } => {
                    let parent = self.nodes.get(&id).unwrap();
                    for child_id in m {
                        if let Some(child) = self.nodes.get(&child_id) {
                            parent.append_child(child).unwrap();
                        }
                    }
                }
                Mutation::SetAttribute {
                    name, value, id, ..
                } => {
                    if let Some(node) = self.nodes.get(&id) {
                        if let Some(el) = node.dyn_ref::<Element>() {
                            el.set_attribute(&name, &value).unwrap();
                        }
                    }
                }
                Mutation::SetText { value, id } => {
                    if let Some(node) = self.nodes.get(&id) {
                        node.set_text_content(Some(&value));
                    }
                }
                Mutation::NewEventListener { name, id } => {
                    self.add_event_listener(id, &name);
                }
                Mutation::Remove { id } => {
                    if let Some(node) = self.nodes.remove(&id) {
                        if let Some(parent) = node.parent_node() {
                            parent.remove_child(&node).unwrap();
                        }
                    }
                    self.event_listeners.remove(&id);
                }
                _ => {
                    // Handle other mutations as needed
                }
            }
        }
    }

    fn add_event_listener(&mut self, id: u64, event_name: &str) {
        let node = self.nodes.get(&id).unwrap().clone();

        let closure = Closure::wrap(Box::new(move |event: Event| {
            if let Some(target) = event.target() {
                if let Some(input) = target.dyn_ref::<web_sys::HtmlInputElement>() {
                    if input.type_() == "file" {
                        if let Some(files) = input.files() {
                            for i in 0..files.length() {
                                if let Some(file) = files.get(i) {
                                    web_sys::console::log_1(
                                        &format!("File selected: {}", file.name()).into(),
                                    );
                                }
                            }
                        }
                    }
                }
            }
            web_sys::console::log_1(&format!("Event triggered on node {}", id).into());
        }) as Box<dyn FnMut(Event)>);

        node.add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref())
            .unwrap();

        self.event_listeners.entry(id).or_default().push(closure);
    }
}

#[wasm_bindgen]
impl WebApp {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        #[cfg(feature = "console_error_panic_hook")]
        console_error_panic_hook::set_once();

        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();

        let app = Self {
            runtime: Rc::new(RefCell::new(Runtime::new())),
            interpreter: Rc::new(RefCell::new(WebInterpreter::new(document))),
        };

        // Devtools hook injection
        app.inject_devtools();

        app
    }

    fn inject_devtools(&self) {
        let window = web_sys::window().unwrap();
        // Expose a global hook for devtools
        let _ = js_sys::Reflect::set(&window, &"__NEXA_DEVTOOLS__".into(), &JsValue::TRUE);
    }

    pub fn mount(&mut self, root_id: &str) -> Result<(), JsValue> {
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let root_el = document
            .get_element_by_id(root_id)
            .ok_or_else(|| JsValue::from_str("Root element not found"))?;

        // Initialize root mapping
        self.interpreter
            .borrow_mut()
            .nodes
            .insert(0, root_el.into());

        self.update()?;
        Ok(())
    }

    pub fn update(&mut self) -> Result<(), JsValue> {
        let window = web_sys::window().unwrap();
        let performance = window.performance().unwrap();
        let start = performance.now();

        self.runtime.borrow_mut().update();
        let mutations = self.runtime.borrow_mut().drain_mutations();

        self.interpreter.borrow_mut().apply_mutations(mutations);
        let end = performance.now();

        // Performance instrumentation
        if end - start > 16.0 {
            web_sys::console::warn_1(
                &format!("Nexa: Slow frame detected ({}ms)", end - start).into(),
            );
        }

        Ok(())
    }

    pub fn hydrate(&mut self) -> Result<(), JsValue> {
        let mut interpreter = self.interpreter.borrow_mut();
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let root = document.document_element().unwrap();

        // Use query_selector_all to find all elements with data-nexa-id
        let nodes = root.query_selector_all("[data-nexa-id]")?;
        for i in 0..nodes.length() {
            if let Some(node) = nodes.get(i) {
                if let Some(el) = node.dyn_ref::<Element>() {
                    if let Some(id_str) = el.get_attribute("data-nexa-id") {
                        if let Ok(id) = id_str.parse::<u64>() {
                            interpreter.nodes.insert(id, node);
                        }
                    }
                }
            }
        }

        web_sys::console::log_1(&"Hydration complete".into());
        Ok(())
    }

    pub fn setup_history_api(&self) {
        let window = web_sys::window().unwrap();
        let on_popstate = Closure::wrap(Box::new(|_event: web_sys::PopStateEvent| {
            web_sys::console::log_1(&"Navigation detected via History API".into());
        }) as Box<dyn FnMut(web_sys::PopStateEvent)>);

        window
            .add_event_listener_with_callback("popstate", on_popstate.as_ref().unchecked_ref())
            .unwrap();
        on_popstate.forget();
    }

    pub fn schedule_microtask(&self) {
        let promise = js_sys::Promise::resolve(&JsValue::UNDEFINED);
        let closure = Closure::wrap(Box::new(|_val: JsValue| {
            web_sys::console::log_1(&"Executing Nexa microtask batch".into());
        }) as Box<dyn FnMut(JsValue)>);

        let _ = promise.then(&closure);
        closure.forget();
    }
}
