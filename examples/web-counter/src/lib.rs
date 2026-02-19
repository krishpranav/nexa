use nexa_rsx::rsx;
use nexa_signals::{create_memo, create_signal};
use nexa_web::WebApp;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

    let mut app = WebApp::new();
    app.mount("#app", App)?;

    Ok(())
}

fn App() -> nexa_core::NodeId {
    let count = create_signal(0);

    // Derived state (double count)
    let double_count = {
        let count = count.clone();
        nexa_signals::create_memo(move || count.get() * 2)
    };

    rsx! {
        div {
            class: "container",
            style: "padding: 20px; font-family: sans-serif; text-align: center;",

            h1 { "Nexa Counter Example" },

            div {
                class: "counter-display",
                style: "font-size: 2rem; margin: 20px 0;",
                "Current Count: {count.get()}"
            },

            div {
                class: "double-display",
                style: "color: #666;",
                "Double Count: {double_count.get()}"
            },

            div {
                class: "controls",
                button {
                    onclick: {
                        let count = count.clone();
                        move |_| count.update(|c| *c -= 1)
                    },
                    style: "padding: 8px 16px; margin: 0 5px; font-size: 1.2rem;",
                    "-1"
                },
                button {
                    onclick: move |_| count.update(|c| *c += 1),
                    style: "padding: 8px 16px; margin: 0 5px; font-size: 1.2rem;",
                    "+1"
                }
            }
        }
    }
    .pop()
    .expect("App should return a single root node")
}
