use nexa_rsx::rsx;
use nexa_signals::create_signal;
use nexa_web::WebApp;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

fn test_app() -> nexa_core::NodeId {
    let count = create_signal(0);
    rsx! {
        div {
            div { id: "counter", "Count: {count.get()}" }
            button {
                id: "inc-btn",
                onclick: move |_| count.set(count.get() + 1),
                "Inc"
            }
        }
    }
    .pop()
    .unwrap()
}

#[wasm_bindgen_test]
fn test_dom_counter() {
    let mut app = WebApp::new();

    // Setup test root
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id("test-root");
    document.body().unwrap().append_child(&root).unwrap();

    // Mount
    app.mount("#test-root", test_app).unwrap();

    // Verify initial state
    let counter_el = document.get_element_by_id("counter").unwrap();
    assert_eq!(counter_el.inner_html(), "Count: 0");

    // Simplify: manual update via signal if click simulation is flaky in some envs,
    // but click is better for integration.
    let btn = document.get_element_by_id("inc-btn").unwrap();
    if let Some(html_el) = btn.dyn_ref::<web_sys::HtmlElement>() {
        html_el.click();
    }

    // Check update
    assert_eq!(counter_el.inner_html(), "Count: 1");
}
