use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub use nexa_router_macro::Routable;

pub trait Routable: Sized + std::fmt::Display + Clone + PartialEq {
    fn from_path(path: &str) -> Option<Self>;
}

pub struct Navigator<R: Routable> {
    current_route: Rc<RefCell<R>>,
    history: Rc<RefCell<Vec<String>>>,
    scroll_positions: Rc<RefCell<HashMap<String, (f64, f64)>>>,
}

impl<R: Routable + Default> Navigator<R> {
    pub fn new() -> Self {
        Self {
            current_route: Rc::new(RefCell::new(R::default())),
            history: Rc::new(RefCell::new(Vec::new())),
            scroll_positions: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    pub fn current(&self) -> R {
        self.current_route.borrow().clone()
    }

    pub fn push(&self, target: R) {
        let path = target.to_string();

        // Browser history integration
        #[cfg(target_arch = "wasm32")]
        {
            let window = web_sys::window().expect("Window not found");
            let history = window.history().expect("History not found");

            // Save current scroll before navigating
            let scroll_x = window.scroll_x().unwrap_or(0.0);
            let scroll_y = window.scroll_y().unwrap_or(0.0);
            self.scroll_positions
                .borrow_mut()
                .insert(self.current().to_string(), (scroll_x, scroll_y));

            let _ = history.push_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&path));

            // Auto-scroll to top or restore
            window.scroll_to_with_x_and_y(0.0, 0.0);
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
    }

    pub fn restore_scroll(&self, path: &str) {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some((x, y)) = self.scroll_positions.borrow().get(path) {
                let window = web_sys::window().unwrap();
                window.scroll_to_with_x_and_y(*x, *y);
            }
        }
    }

    pub fn resolve_from_path(&self, path: &str) -> Option<R> {
        R::from_path(path)
    }

    pub fn extract_query_params(path: &str) -> HashMap<String, String> {
        let mut params = HashMap::new();
        if let Some(query_start) = path.find('?') {
            let query = &path[query_start + 1..];
            for pair in query.split('&') {
                let mut it = pair.split('=');
                if let (Some(k), Some(v)) = (it.next(), it.next()) {
                    params.insert(k.to_string(), v.to_string());
                }
            }
        }
        params
    }
}

pub struct Redirect<R: Routable> {
    pub to: R,
}
