//! # Layout example
//!
//! Every layout method returns a `Container`, and a `Container` has the same
//! methods as the page itself. So there are two ways to fill one:
//!
//! - call methods on it: `left.button("Add")`;
//! - open a `with` block, inside which `text!` writes into it.
use dappr::prelude::*;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn run() {
    dapp!(dapp, {
        // The sidebar holds the controls; the page reacts to them.
        let sidebar = dapp.sidebar();
        sidebar.h4_static("Settings");
        let name = sidebar.input("Your name");
        let city = sidebar.input("Your city");

        text!("Layout example", _title);
        text!("Sidebar, columns, cards and expanders", _subtitle);

        // Two equal columns. The count comes from the pattern on the left.
        let [left, right] = dapp.columns();

        // Style 1: methods on the container.
        let greeting = left.card_titled("Greeting");
        greeting.text(
            "Hello, {}!",
            name.text(),
            Some("Type your name in the sidebar."),
        );

        // Style 2: a `with` block, so `text!` lands in the container.
        right.card_titled("Location").with(|_| {
            text!(
                "You are in {}.",
                city.text(),
                "Type your city in the sidebar."
            );
        });

        // Weighted columns: the first is three times as wide as the second.
        let [wide, narrow] = dapp.columns_weighted([3, 1]);
        wide.with(|_| {
            text!("A wide column", _h3);
            text!("Containers nest: this column holds three more.");
            // Inside `with`, `dapp!` gives the container being filled.
            dapp!(wide, {
                let [a, b, c] = wide.columns();
                a.card().p_static("One");
                b.card().p_static("Two");
                c.card().p_static("Three");
            });
        });
        narrow.h3_static("A narrow one");
        let counter = Signal::new(0u32);
        let add = narrow.button("Add one");
        narrow.text("Clicked {} times", counter.clone(), Some("Not clicked yet"));
        add.pressed(move || counter.set(counter.get() + 1));

        let help = dapp.expander("What is going on here?");
        help.p_static(
            "Each layout method returns a Container. A Container has every \
             method the page has, so layouts nest to any depth.",
        );
    });
}
