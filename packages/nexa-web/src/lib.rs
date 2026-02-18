use nexa_core::{Mutation, Runtime};
use nexa_scheduler::LocalScheduler;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::{Document, Element, Event, Node};

#[wasm_bindgen]
pub struct WebApp {
    runtime: Rc<RefCell<Runtime<LocalScheduler>>>,
    interpreter: Rc<RefCell<WebInterpreter>>,
}

struct WebInterpreter {
    document: Document,
    nodes: HashMap<u64, Node>,
    event_listeners: HashMap<u64, Vec<Closure<dyn FnMut(Event)>>>,
    root_id: Option<u64>,
    runtime: Rc<RefCell<Runtime<LocalScheduler>>>,
}

impl WebInterpreter {
    fn new(document: Document, runtime: Rc<RefCell<Runtime<LocalScheduler>>>) -> Self {
        Self {
            document,
            nodes: HashMap::new(),
            event_listeners: HashMap::new(),
            root_id: None,
            runtime,
        }
    }

    fn apply_mutations(&mut self, mutations: Vec<Mutation>, handle: Rc<RefCell<WebInterpreter>>) {
        tracing::debug!("Applying {} mutations", mutations.len());
        for mutation in mutations {
            tracing::trace!("Mutation: {:?}", mutation);
            match mutation {
                Mutation::PushRoot { id } => {
                    self.root_id = Some(id);
                    tracing::debug!("Root ID set to {}", id);
                }
                Mutation::CreateElement { tag, id } => {
                    web_sys::console::log_1(
                        &format!("Created element '{}' with id {}", tag, id).into(),
                    );
                    let el = self.document.create_element(&tag).unwrap();
                    el.set_attribute("data-nexa-id", &id.to_string()).unwrap();
                    self.nodes.insert(id, el.into());
                }
                Mutation::CreateTextNode { text, id } => {
                    let node = self.document.create_text_node(&text);
                    self.nodes.insert(id, node.into());
                }
                Mutation::AppendChildren { id, m } => {
                    let parent = if id == 0 {
                        // Special case for container
                        // In mount, we inserted container as 0
                        self.nodes.get(&0).expect("Container not found (id=0)")
                    } else {
                        self.nodes.get(&id).expect("Parent node not found")
                    };

                    for child_id in m {
                        if let Some(child) = self.nodes.get(&child_id) {
                            parent.append_child(child).unwrap();
                        } else {
                            tracing::error!("Child node {} not found for append", child_id);
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
                    self.add_event_listener(id, &name, handle.clone());
                }
                Mutation::Remove { id } => {
                    if let Some(node) = self.nodes.remove(&id) {
                        if let Some(parent) = node.parent_node() {
                            parent.remove_child(&node).unwrap();
                        }
                    }
                    self.event_listeners.remove(&id);
                }
                Mutation::InsertBefore { id, m } => {
                    // id is the reference node (next sibling)
                    let ref_node = if let Some(n) = self.nodes.get(&id) {
                        n
                    } else {
                        tracing::error!("Reference node {} not found for InsertBefore", id);
                        continue;
                    };

                    if let Some(parent) = ref_node.parent_node() {
                        for child_id in m {
                            if let Some(child) = self.nodes.get(&child_id) {
                                parent.insert_before(child, Some(ref_node)).unwrap();
                            } else {
                                tracing::error!(
                                    "Child node {} not found for InsertBefore",
                                    child_id
                                );
                            }
                        }
                    } else {
                        tracing::error!("Reference node {} has no parent", id);
                    }
                }
                Mutation::RemoveAttribute { name, id } => {
                    if let Some(node) = self.nodes.get(&id) {
                        if let Some(el) = node.dyn_ref::<Element>() {
                            el.remove_attribute(&name).unwrap();
                        }
                    }
                }
                _ => {
                    // Handle other mutations as needed
                }
            }
        }
    }

    fn add_event_listener(
        &mut self,
        id: u64,
        event_name: &str,
        handle: Rc<RefCell<WebInterpreter>>,
    ) {
        web_sys::console::log_1(&format!("Adding listener '{}' to node {}", event_name, id).into());
        let node = if let Some(n) = self.nodes.get(&id) {
            n.clone()
        } else {
            tracing::error!("Cannot add listener to missing node {}", id);
            return;
        };

        // Clone runtime for the closure
        let runtime = self.runtime.clone();
        let name = event_name.to_string();
        let node_id = id;

        // Clone handle for the closure
        let interpreter_handle = handle;

        let closure = Closure::wrap(Box::new(move |event: Event| {
            // Map web_sys Event to nexa_core Event
            let nexa_event = match event.type_().as_str() {
                "click" => nexa_core::Event::Click,
                "input" => {
                    let value = event
                        .target()
                        .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                        .map(|input| input.value())
                        .unwrap_or_default();
                    nexa_core::Event::Input(value)
                }
                _ => nexa_core::Event::Unknown,
            };

            runtime
                .borrow_mut()
                .handle_event(node_id, &name, nexa_event);

            // Trigger updates
            let mutations = runtime.borrow_mut().drain_mutations();
            if !mutations.is_empty() {
                interpreter_handle
                    .borrow_mut()
                    .apply_mutations(mutations, interpreter_handle.clone());
            }
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

        let scheduler = LocalScheduler::new();
        let runtime = Rc::new(RefCell::new(Runtime::new(scheduler)));
        let interpreter = Rc::new(RefCell::new(WebInterpreter::new(document, runtime.clone())));

        let app = Self {
            runtime,
            interpreter,
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

    pub fn update(&mut self) -> Result<(), JsValue> {
        let window = web_sys::window().unwrap();
        let performance = window.performance().unwrap();
        let start = performance.now();

        self.runtime.borrow_mut().update();
        let mutations = self.runtime.borrow_mut().drain_mutations();

        let interpreter_clone = self.interpreter.clone();
        self.interpreter
            .borrow_mut()
            .apply_mutations(mutations, interpreter_clone);
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

impl WebApp {
    pub fn mount(
        &mut self,
        root_id: &str,
        root_fn: fn() -> nexa_core::NodeId,
    ) -> Result<(), JsValue> {
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let id = root_id.strip_prefix('#').unwrap_or(root_id);
        let root_el = document
            .get_element_by_id(id)
            .ok_or_else(|| JsValue::from_str(&format!("Root element not found: {}", id)))?;

        // Initialize root mapping
        self.interpreter
            .borrow_mut()
            .nodes
            .insert(0, root_el.into());

        self.runtime.borrow_mut().mount("Root", root_fn);

        self.update()?;
        Ok(())
    }
}
