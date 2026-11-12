//! ## Table: a view of a [`DTable`]
//!
//! Shows a table of data on the page. Give it a `DTable` for a fixed table,
//! or a `Signal<DTable>` for one that redraws whenever the data changes --
//! for instance a table filtered by the app's inputs.
// The idea of a view component over a DTable comes from rustream's
// `<ViewTable>` / `<ViewCsv>` (src/components/dataview.rs). The rendering is
// new: it builds DOM nodes instead of an HTML string, so cell text is always
// escaped, and it no longer cuts text cells to 7 characters.
use crate::communication::signals::Signal;
use crate::data::load::Loadable;
use crate::data::table::{Cell, DTable};
use crate::framework::dom;
use crate::framework::traits::Component;
use std::cell::Cell as StdCell;
use std::rc::Rc;
use web_sys::HtmlElement;

/// How many rows a [`Table`] shows unless told otherwise.
pub const DEFAULT_MAX_ROWS: usize = 1000;

/// What a [`Table`] can show: a `DTable` (by value or reference) for a
/// fixed table, a `Signal<DTable>` for a reactive one, or a
/// `Signal<Loadable<DTable>>` for data being fetched or uploaded.
pub trait TableSource {
    /// The data as a Signal.
    fn into_signal(self) -> Signal<DTable>;
}
impl TableSource for DTable {
    fn into_signal(self) -> Signal<DTable> {
        Signal::new(self)
    }
}
impl TableSource for &DTable {
    fn into_signal(self) -> Signal<DTable> {
        Signal::new(self.clone())
    }
}
impl TableSource for Signal<DTable> {
    fn into_signal(self) -> Signal<DTable> {
        self
    }
}
impl TableSource for &Signal<DTable> {
    fn into_signal(self) -> Signal<DTable> {
        self.clone()
    }
}

/// Data that is still loading (or failed to) is shown as an empty table;
/// pair it with a load status to tell the user what is going on.
impl TableSource for Signal<Loadable<DTable>> {
    fn into_signal(self) -> Signal<DTable> {
        self.map(Loadable::ready_or_default)
    }
}
impl TableSource for &Signal<Loadable<DTable>> {
    fn into_signal(self) -> Signal<DTable> {
        self.map(Loadable::ready_or_default)
    }
}

/// A table of data on the page.
///
/// # Example
/// ```ignore
/// use dappr::prelude::*;
/// use wasm_bindgen::prelude::*;
///
/// #[wasm_bindgen(main)]
/// pub fn main() {
///     dapp!(dapp, {
///         let people = DTable::from_csv("name,age\nAda,36\nAlan,41").unwrap();
///         let min_age = dapp.slider("Minimum age", 0.0, 100.0, 0.0);
///
///         // Redrawn every time the slider moves.
///         let shown = min_age.value().map(move |min| {
///             people.filter(|row| row.number("age") >= Some(*min))
///         });
///         dapp.table(shown);
///     });
/// }
/// ```
pub struct Table {
    el: HtmlElement,
    data: Signal<DTable>,
    max_rows: Rc<StdCell<usize>>,
}

impl Table {
    /// A table showing `source`.
    pub fn new(source: impl TableSource) -> Self {
        let data = source.into_signal();
        // The wrapper scrolls sideways when the table is wider than the
        // page, instead of stretching the page.
        let el = dom::create_element("div");
        el.set_class_name("overflow-auto dappr-table");
        let max_rows = Rc::new(StdCell::new(DEFAULT_MAX_ROWS));

        render(&el, &data.get(), max_rows.get());
        let (el_in, max_rows_in) = (el.clone(), Rc::clone(&max_rows));
        data.subscribe(move |table| render(&el_in, table, max_rows_in.get()))
            .detach();

        Table { el, data, max_rows }
    }

    /// The data being shown. Set it to change what the table shows.
    pub fn data(&self) -> Signal<DTable> {
        self.data.clone()
    }

    /// Replaces the data being shown.
    pub fn set(&self, table: DTable) {
        self.data.set(table);
    }

    /// Shows at most this many rows (default [`DEFAULT_MAX_ROWS`]), with a
    /// note below the table when rows are left out. A page with tens of
    /// thousands of table rows is slow to draw and to scroll. Chainable.
    pub fn max_rows(self, max_rows: usize) -> Self {
        self.max_rows.set(max_rows);
        render(&self.el, &self.data.get(), max_rows);
        self
    }
}

impl Component for Table {
    fn get_element(&self) -> &HtmlElement {
        &self.el
    }
}

fn child(parent: &HtmlElement, tag: &str) -> HtmlElement {
    let el = dom::create_element(tag);
    parent.append_child(&el).expect("Failed to build the table");
    el
}

/// A cell's text in a table. Numbers lose the binary floating-point noise
/// that arithmetic leaves behind: `12.0 * 34.9` is shown as `418.8`, not
/// `418.79999999999995`. The data itself is untouched.
fn display(cell: &Cell) -> String {
    match cell {
        Cell::Number(number) if number.is_finite() && number.abs() < 1e15 => {
            const SCALE: f64 = 1e10;
            ((number * SCALE).round() / SCALE).to_string()
        }
        other => other.to_string(),
    }
}

/// Replaces the content of `wrapper` with a `<table>` for `data`.
fn render(wrapper: &HtmlElement, data: &DTable, max_rows: usize) {
    wrapper.set_text_content(None);
    let table = child(wrapper, "table");
    table.set_class_name("striped");

    if !data.title().is_empty() {
        child(&table, "caption").set_text_content(Some(data.title()));
    }

    // Numbers are right-aligned so their digits line up.
    let numeric: Vec<bool> = data.columns().iter().map(|c| c.is_numeric()).collect();

    let head_row = child(&child(&table, "thead"), "tr");
    for (column, is_numeric) in data.columns().iter().zip(&numeric) {
        let th = child(&head_row, "th");
        th.set_attribute("scope", "col")
            .expect("Failed to build the table");
        if *is_numeric {
            th.set_class_name("dappr-number");
        }
        th.set_text_content(Some(column.title()));
    }

    let body = child(&table, "tbody");
    for row in data.rows().take(max_rows) {
        let tr = child(&body, "tr");
        for cell in row.cells() {
            let td = child(&tr, "td");
            if matches!(cell, Cell::Number(_)) {
                td.set_class_name("dappr-number");
            }
            td.set_text_content(Some(&display(cell)));
        }
    }

    let shown = data.n_rows().min(max_rows);
    if shown < data.n_rows() {
        let note = child(wrapper, "small");
        note.set_class_name("dappr-table-note");
        note.set_text_content(Some(&format!(
            "Showing the first {shown} of {} rows",
            data.n_rows()
        )));
    }
}
