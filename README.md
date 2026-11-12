# Dappr

Data Apps made easy (in Rust)

Dappr aims to be for Rust what [Streamlit](https://streamlit.io/) is for Python:
you describe a page top to bottom, widgets hand you reactive values, and the
framework takes care of the rest. Apps compile to WebAssembly and run entirely
in the browser.

```rust
use dappr::prelude::*;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn run() {
    dapp!(dapp, {
        text!("Hello Dappr!", _title);
        let input = dapp.input("What's your name?");
        text!("Hello, {}!", input.text());
    });
}
```

## Inputs

Inputs are typed: each returns a `Field<T>` whose `.value()` is a reactive
`Signal<T>`, so you compute with numbers and booleans, not strings.

```rust
dapp!(dapp, {
    let price = dapp.number("Price", 9.5).min(0.0).step(0.5);     // f64
    let quantity = dapp.slider("Quantity", 1.0, 20.0, 3.0);       // f64
    let gift = dapp.checkbox("Gift wrap (+2)", false);            // bool
    let unit = dapp.select("Currency", &["EUR", "USD"]);          // &str

    let total = price.value()
        .zip(&quantity.value())
        .zip(&gift.value())
        .map(|((p, q), gift)| p * q + if *gift { 2.0 } else { 0.0 });
    text!("Total: {}", total);
});
```

Available: `text_input`, `textarea`, `number`, `slider`, `checkbox`, `switch`,
`select`, `radio`. Options of `select` and `radio` can be any type that is
`Display + Clone + PartialEq`, including your own enums. `Signal::map` and
`Signal::zip` derive new values from inputs. See `just example-inputs`.

## Layout

Layout methods return a `Container`, which has every method the page has, so
layouts nest:

```rust
dapp!(dapp, {
    let sidebar = dapp.sidebar();
    let name = sidebar.input("Your name");

    let [left, right] = dapp.columns();          // or dapp.columns_weighted([2, 1])
    left.card_titled("Greeting").text("Hello, {}!", name.text(), None);

    // `with` makes text! write into a container, like Streamlit's `with col:`
    right.with(|col| {
        text!("Right column", _h3);
        col.button("Click me");
    });

    dapp.expander("Details").p_static("Hidden until opened.");
});
```

Available: `container`, `section`, `columns`, `columns_weighted`, `card`,
`card_titled`, `expander`, `expander_open`, `sidebar`. See `just example-layout`.

## Styling

Apps are styled out of the box. [Pico CSS](https://picocss.com) is vendored in
the `dappr-styles` crate and added to the page when the app starts, so your
`index.html` needs no stylesheet link and your crate needs no `build.rs`.

For your own CSS, `style!` scopes it and generates matching class names:

```rust
style!("badge", ".pill { border-radius: 999px; }");
// badge::pill == "badge-pill", badge::CSS == ".badge-pill{border-radius:999px}"

dapp.style(badge::CSS);
dapp.button("New").add_class(badge::pill);
```

## Dependencies and useful tools

Dappr's workflow employs the following tools:

- the `wasm32-unknown-unknown` Rust target (`rustup target add wasm32-unknown-unknown`)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/) ([repo](https://github.com/rustwasm/wasm-pack)) to build the library and run the browser tests
- [trunk](https://trunkrs.dev/) ([repo](https://github.com/trunk-rs/trunk)) to run examples and dapps locally
- [just](https://just.systems/) ([repo](https://github.com/casey/just) to automate builds

## Build

Dappr ships wih a .justfile, so you just run `just` as *build* is the default recipe:

```bash
just
```

## Run examples

Using `just` all examples recipes start with *example-* and use `trunk` to run.
List all recipes and select the example you want

```bash
just --list
```

For example:

```bash
just example-hello
```

## Test

```bash
just test
```

This runs the native tests and then the browser tests (headless Firefox, so
`geckodriver` must be installed).

## License

Copyright (c) 2024-2026 Davide Del Papa under the terms of the MIT License (see LICENSE file attached).
