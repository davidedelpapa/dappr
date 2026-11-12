//! # Data example: an orders explorer
//!
//! The whole shape of a data app in one file:
//!
//! 1. load a table (`DTable::from_csv`),
//! 2. put inputs in the sidebar,
//! 3. compute the table to show from the inputs (`computed!`),
//! 4. show it (`dapp.table`) along with a few numbers derived from it.
//!
//! The data is made up, and compiled into the app with `include_str!`.
use dappr::prelude::*;
use wasm_bindgen::prelude::*;

const ORDERS_CSV: &str = include_str!("../orders.csv");
const ALL_REGIONS: &str = "All regions";

/// Loads the orders and adds a `revenue` column computed from two others.
fn load_orders() -> Result<DTable, DataError> {
    let mut orders = DTable::from_csv(ORDERS_CSV)?;
    let revenue: Vec<Option<f64>> = orders
        .rows()
        .map(|row| Some(row.number("units")? * row.number("unit_price")?))
        .collect();
    orders.push_column(DColumn::from_values("revenue", revenue));
    Ok(orders)
}

#[wasm_bindgen(start)]
pub fn run() {
    dapp!(dapp, {
        text!("Orders explorer", _title);

        let orders = match load_orders() {
            Ok(orders) => orders,
            Err(error) => {
                dapp.p_static(&format!("Could not load the orders: {error}"));
                return;
            }
        };

        // ---- Inputs, in the sidebar --------------------------------------
        let sidebar = dapp.sidebar();
        sidebar.h4_static("Filters");

        // The choices come from the data itself.
        let mut regions = vec![ALL_REGIONS.to_string()];
        if let Some(column) = orders.column("region") {
            regions.extend(column.unique().iter().map(|cell| cell.to_string()));
        }
        let region = sidebar.select("Region", &regions).value();
        let search = sidebar
            .text_input("Product contains")
            .placeholder("e.g. coffee")
            .value();
        let min_units = sidebar
            .slider("At least this many units", 1.0, 12.0, 1.0)
            .value();
        let hide_returned = sidebar.checkbox("Hide returned orders", false).value();

        sidebar.h4_static("Order");
        let sort_by = sidebar.select("Sort by", &orders.headers()).value();
        let descending = sidebar.switch("Largest first", false).value();

        // ---- The table to show, recomputed when any input changes -------
        let shown = computed!(region, search, min_units, hide_returned, sort_by, descending => {
            let search = search.trim().to_lowercase();
            orders
                .filter(|row| {
                    (region == ALL_REGIONS || row.text("region") == Some(region.as_str()))
                        && row
                            .text("product")
                            .is_some_and(|product| product.to_lowercase().contains(&search))
                        && row.number("units") >= Some(min_units)
                        && !(hide_returned && row.bool("returned") == Some(true))
                })
                .sort_by(&sort_by, descending)
        });

        // ---- Numbers derived from the table being shown ------------------
        let metric = |card: Container, value: Signal<String>| {
            card.h3("{}", value.clone(), Some(&value.get()));
        };
        let [orders_card, units_card, revenue_card] = dapp.columns();
        metric(
            orders_card.card_titled("Orders"),
            shown.map(|table| table.n_rows().to_string()),
        );
        metric(
            units_card.card_titled("Units"),
            shown.map(|table| table.column("units").map_or(0.0, DColumn::sum).to_string()),
        );
        metric(
            revenue_card.card_titled("Revenue"),
            shown.map(|table| format!("{:.2}", table.column("revenue").map_or(0.0, DColumn::sum))),
        );

        dapp.table(&shown);
    });
}
