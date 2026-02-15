use nexa_core::*;
use nexa_desktop::DesktopApp;
use nexa_rsx::rsx;
use nexa_signals::*;

fn main() {
    env_logger::init();

    // DesktopApp should handle window creation and event loop
    let app = DesktopApp::new();
    app.run(App);
}

#[allow(non_snake_case)]
fn App() -> NodeId {
    let count = signal(0);

    let increment = {
        let count = count.clone();
        move |_: ()| count.set(count.get() + 1)
    };

    let decrement = {
        let count = count.clone();
        move |_: ()| count.set(count.get() - 1)
    };

    let mut nodes = rsx! {
        div {
            // Style properties are simpler in desktop renderer for now (just layout hints or ignored)
            h1 { "Nexa Desktop Counter" },
            div {
                button {
                    // onclick: "decrement",
                    "-"
                },
                span {
                    "{count.get()}"
                },
                button {
                    // onclick: "increment",
                    "+"
                }
            }
        }
    };
    nodes.remove(0)
}
