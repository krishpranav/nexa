use std::cell::RefCell;
use std::rc::Rc;

pub use nexa_router_macro::Routable;

pub trait Routable: Sized + std::fmt::Display + Clone + PartialEq {
    fn from_path(path: &str) -> Option<Self>;
}

#[derive(Clone)]
pub struct Navigator<R: Routable> {
    current_route: Rc<RefCell<R>>,
    history: Rc<RefCell<Vec<String>>>,
}

impl<R: Routable + Default> Navigator<R> {
    pub fn new() -> Self {
        Self {
            current_route: Rc::new(RefCell::new(R::default())),
            history: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn current(&self) -> R {
        self.current_route.borrow().clone()
    }

    pub fn push(&self, target: R) {
        let path = target.to_string();
        // Typically update browser history here via web-sys if target is wasm
        #[cfg(target_arch = "wasm32")]
        {
            let window = web_sys::window().unwrap();
            let history = window.history().unwrap();
            let _ = history.push_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&path));
        }

        *self.current_route.borrow_mut() = target;
        self.history.borrow_mut().push(path);
    }

    pub fn replace(&self, target: R) {
        let _path = target.to_string();
        #[cfg(target_arch = "wasm32")]
        {
            let window = web_sys::window().unwrap();
            let history = window.history().unwrap();
            let _ = history.replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&path));
        }

        *self.current_route.borrow_mut() = target;
        // Replace last history entry if desired or just push?
        // usually replace doesn't push to stack but replaces current.
    }
}

// Router Component Runtime Hook
// In a full implementation, we'd hook into `nexa-core`'s context to provide `use_navigator`.
// For now, we just expose the struct.
