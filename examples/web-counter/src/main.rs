use nexa_core::*;
use nexa_rsx::rsx;
use nexa_signals::*;
use nexa_web::WebApp;

fn main() {
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default();

    let mut app = WebApp::new();
    app.mount("#app", App).unwrap();
    // Leak the app to keep it alive
    Box::leak(Box::new(app));
}

use std::cell::RefCell;

thread_local! {
    static COUNT: RefCell<Option<Signal<i32>>> = RefCell::new(None);
}

#[allow(non_snake_case)]
fn App() -> NodeId {
    // Initialize signal if not exists
    let count = COUNT.with(|c| {
        let mut cell = c.borrow_mut();
        if cell.is_none() {
            *cell = Some(signal(0));
        }
        cell.as_ref().unwrap().clone()
    });

    let _increment = {
        let count = count.clone();
        move |_| count.set(count.get() + 1)
    };

    let _decrement = {
        let count = count.clone();
        move |_| count.set(count.get() - 1)
    };

    let mut nodes = rsx! {
        div {
            class: "container",
            h1 { "Nexa Web Counter" },
            div {
                class: "counter-box",
                button {
                    onclick: _decrement,
                    "-"
                },
                span {
                    class: "count",
                    {count.get()}
                },
                button {
                    onclick: _increment,
                    "+"
                }
            }
        }
    };
    nodes.remove(0)
}
