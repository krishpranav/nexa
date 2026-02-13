# Nexa Tutorial

This guide introduces the basics of building an app with Nexa.

## 1. Installation

```bash
cargo install nexa-cli
nexa new hello-nexa
cd hello-nexa
```

## 2. Basic Component

Define a component using `rsx!`:

```rust
use nexa::prelude::*;

fn app(cx: Scope) -> Element {
    let count = use_signal(cx, || 0);

    cx.render(rsx! {
        div {
            h1 { "Hello Nexa" }
            button {
                onclick: move |_| count += 1,
                "Count: {count}"
            }
        }
    })
}
```

## 3. Running

Start the dev server:

```bash
nexa dev
```

Open `http://localhost:8080` to see your app.
