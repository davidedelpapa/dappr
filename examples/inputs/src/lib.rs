//! # Typed inputs example: a pizza order
//!
//! Every input returns a `Field<T>` whose `.value()` is a `Signal` of the
//! right type -- `f64`, `bool`, `String`, or the option you picked -- so the
//! app computes with values, not with strings.
//!
//! `Signal::map` derives one value from another, and `Signal::zip` pairs two
//! signals up so `map` can use both.
use dappr::prelude::*;
use std::fmt;
use wasm_bindgen::prelude::*;

/// Options of a select or radio can be your own type: it needs `Display`
/// (what the user sees), `Clone` and `PartialEq`.
#[derive(Clone, Copy, PartialEq)]
enum Size {
    Small,
    Medium,
    Large,
}

impl Size {
    fn price(self) -> f64 {
        match self {
            Size::Small => 6.0,
            Size::Medium => 8.5,
            Size::Large => 11.0,
        }
    }
}

impl fmt::Display for Size {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match self {
            Size::Small => "Small",
            Size::Medium => "Medium",
            Size::Large => "Large",
        })
    }
}

#[wasm_bindgen(start)]
pub fn run() {
    dapp!(dapp, {
        text!("Pizza order", _title);
        text!("Typed inputs driving computed values", _subtitle);

        let [form, summary] = dapp.columns_weighted([3, 2]);

        // ---- The inputs -------------------------------------------------
        let name = form.text_input("Name").placeholder("Who is it for?");
        let size = form.radio("Size", &[Size::Small, Size::Medium, Size::Large]);
        let topping = form.select("Topping", &["Margherita", "Funghi", "Diavola"]);
        let quantity = form.slider("How many", 1.0, 10.0, 2.0);
        let extra_cheese = form.checkbox("Extra cheese (+1.50 each)", false);
        let tip = form.number("Tip", 0.0).min(0.0).step(0.5);
        let delivery = form.switch("Deliver it (+3.00)", true);
        let notes = form.textarea("Notes").rows(2);

        // ---- Computed values --------------------------------------------
        // Each pizza: the size's price, plus cheese if ticked.
        let unit_price = size
            .value()
            .zip(&extra_cheese.value())
            .map(|(size, cheese)| size.price() + if *cheese { 1.5 } else { 0.0 });
        let subtotal = unit_price
            .zip(&quantity.value())
            .map(|(unit, quantity)| unit * quantity);
        let total = subtotal
            .zip(&tip.value())
            .zip(&delivery.value())
            .map(|((subtotal, tip), delivery)| subtotal + tip + if *delivery { 3.0 } else { 0.0 });

        // ---- The summary ------------------------------------------------
        let card = summary.card_titled("Your order");
        card.with(|card| {
            let who = name.value().map(|name| {
                if name.trim().is_empty() {
                    "someone hungry".to_string()
                } else {
                    name.trim().to_string()
                }
            });
            card.p("For {}", who.clone(), Some(&format!("For {}", who.get())));

            let what = quantity
                .value()
                .zip(&size.value())
                .zip(&topping.value())
                .map(|((quantity, size), topping)| format!("{quantity} x {size} {topping}"));
            card.p("{}", what.clone(), Some(&what.get()));

            let money = |amount: &f64| format!("{amount:.2}");
            let subtotal = subtotal.map(money);
            card.p(
                "Pizzas: {}",
                subtotal.clone(),
                Some(&format!("Pizzas: {}", subtotal.get())),
            );
            let total = total.map(money);
            card.h3(
                "Total: {}",
                total.clone(),
                Some(&format!("Total: {}", total.get())),
            );

            card.div("{}", notes.value(), Some(""));
        });

        // Inputs can be driven from code too: the controls follow.
        let reset = summary.button("Start over");
        reset.pressed(move || {
            name.set(String::new());
            size.set(Size::Small);
            topping.set("Margherita");
            quantity.set(2.0);
            extra_cheese.set(false);
            tip.set(0.0);
            delivery.set(true);
            notes.set(String::new());
        });
    });
}
