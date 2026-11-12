//! Typed inputs.
//!
//! Each method adds a labelled control and returns a [`Field<T>`] whose
//! [`value()`](Field::value) is a [`Signal<T>`](crate::prelude::Signal) of the
//! right type: no parsing strings yourself.
//!
//! | Method | Control | Value |
//! |---|---|---|
//! | [`text_input`](Dapp::text_input) | single-line text | `String` |
//! | [`textarea`](Dapp::textarea) | multi-line text | `String` |
//! | [`number`](Dapp::number) | number box | `f64` |
//! | [`slider`](Dapp::slider) | slider | `f64` |
//! | [`checkbox`](Dapp::checkbox) | checkbox | `bool` |
//! | [`switch`](Dapp::switch) | on/off toggle | `bool` |
//! | [`select`](Dapp::select) | dropdown | the chosen option |
//! | [`radio`](Dapp::radio) | radio buttons | the chosen option |
//!
//! ```no_run
//! use dappr::prelude::*;
//! use wasm_bindgen::prelude::*;
//!
//! #[wasm_bindgen(main)]
//! pub fn main() {
//!     dapp!(dapp, {
//!         let price = dapp.number("Price", 9.5).min(0.0).step(0.5);
//!         let quantity = dapp.slider("Quantity", 1.0, 20.0, 3.0);
//!         let total = price.value().zip(&quantity.value()).map(|(p, q)| p * q);
//!         text!("Total: {}", total);
//!     });
//! }
//! ```
use crate::app::Dapp;
use dappr_core::components::field::Field;
use dappr_core::framework::traits::Component;
use std::fmt::Display;

/// Typed inputs
impl Dapp {
    fn add_field<T>(&self, field: Field<T>) -> Field<T> {
        self.append(field.get_element());
        field
    }

    /// A single-line text input. Chain `.placeholder("...")` for a hint.
    ///
    /// ```no_run
    /// # use dappr::prelude::*;
    /// # let dapp = Dapp::new();
    /// let name = dapp.text_input("Name").placeholder("Ada Lovelace");
    /// dapp.text("Hello, {}!", name.value(), None);
    /// ```
    pub fn text_input(&self, label: &str) -> Field<String> {
        self.add_field(Field::text(label))
    }

    /// A multi-line text input. Chain `.rows(n)` to set its height.
    pub fn textarea(&self, label: &str) -> Field<String> {
        self.add_field(Field::textarea(label))
    }

    /// A number input starting at `initial`. Chain `.min()`, `.max()` and
    /// `.step()` to constrain it.
    ///
    /// ```no_run
    /// # use dappr::prelude::*;
    /// # let dapp = Dapp::new();
    /// let age = dapp.number("Age", 30.0).min(0.0).max(120.0).step(1.0);
    /// dapp.text("Age: {}", age.value(), None);
    /// ```
    pub fn number(&self, label: &str, initial: f64) -> Field<f64> {
        self.add_field(Field::number(label, initial))
    }

    /// A slider from `min` to `max`, starting at `initial`, with the current
    /// value shown beside the label. Chain `.step()` for steps other than 1.
    ///
    /// ```no_run
    /// # use dappr::prelude::*;
    /// # let dapp = Dapp::new();
    /// let opacity = dapp.slider("Opacity", 0.0, 1.0, 0.8).step(0.05);
    /// ```
    pub fn slider(&self, label: &str, min: f64, max: f64, initial: f64) -> Field<f64> {
        self.add_field(Field::slider(label, min, max, initial))
    }

    /// A checkbox.
    pub fn checkbox(&self, label: &str, initial: bool) -> Field<bool> {
        self.add_field(Field::checkbox(label, initial))
    }

    /// An on/off switch.
    pub fn switch(&self, label: &str, initial: bool) -> Field<bool> {
        self.add_field(Field::switch(label, initial))
    }

    /// A dropdown to pick one of `options`, starting on the first.
    ///
    /// The value is the option itself, so the options can be strings,
    /// numbers, or your own type.
    ///
    /// ```no_run
    /// # use dappr::prelude::*;
    /// # let dapp = Dapp::new();
    /// let unit = dapp.select("Unit", &["km", "miles"]);
    /// let year = dapp.select("Year", &[2024, 2025, 2026]); // a Field<i32>
    /// ```
    ///
    /// # Panics
    /// If `options` is empty.
    pub fn select<T>(&self, label: &str, options: &[T]) -> Field<T>
    where
        T: Display + Clone + PartialEq + 'static,
    {
        self.add_field(Field::select(label, options))
    }

    /// Radio buttons to pick one of `options`, starting on the first.
    /// Best for a handful of options; use [`select`](Dapp::select) for more.
    ///
    /// # Panics
    /// If `options` is empty.
    pub fn radio<T>(&self, label: &str, options: &[T]) -> Field<T>
    where
        T: Display + Clone + PartialEq + 'static,
    {
        self.add_field(Field::radio(label, options))
    }
}
