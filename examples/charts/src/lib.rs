//! # Charts example
//!
//! `line_chart`, `area_chart`, `bar_chart` and `scatter_chart` each take a
//! table, the column for the x-axis, and the columns to plot; `histogram`
//! takes a table and one column. Give them a `Signal<DTable>` and they
//! redraw when the inputs change.
//!
//! Every chart comes with a legend (when it has several series), a read-out
//! that follows the pointer or the arrow keys, and a "Show data" table.
//!
//! The data is made up. `orders.csv` is the one from the `data` example.
use dappr::prelude::*;
use wasm_bindgen::prelude::*;

const MONTHLY_CSV: &str = include_str!("../monthly.csv");
const ORDERS_CSV: &str = include_str!("../../data/orders.csv");
const ALL_PRODUCTS: &str = "All products";

#[wasm_bindgen(start)]
pub fn run() {
    dapp!(dapp, {
        text!("Charts", _title);

        let (monthly, mut orders) =
            match (DTable::from_csv(MONTHLY_CSV), DTable::from_csv(ORDERS_CSV)) {
                (Ok(monthly), Ok(orders)) => (monthly, orders),
                _ => {
                    dapp.p_static("Could not read the example data.");
                    return;
                }
            };
        let revenue: Vec<Option<f64>> = orders
            .rows()
            .map(|row| Some(row.number("units")? * row.number("unit_price")?))
            .collect();
        orders.push_column(DColumn::from_values("revenue", revenue));

        // ---- A fixed line chart -------------------------------------------
        dapp.line_chart(&monthly, "month", &["north", "south", "east"])
            .title("Units sold per month, by region")
            .y_label("Units");

        // The same numbers as parts of a whole: stacked, the bar's height
        // is the total and the segments are the regions.
        dapp.bar_chart(&monthly, "month", &["north", "south", "east"])
            .stacked()
            .title("Units sold per month, all regions")
            .height(280);

        // ---- Charts that follow an input ----------------------------------
        let mut products = vec![ALL_PRODUCTS.to_string()];
        if let Some(column) = orders.column("product") {
            products.extend(column.unique().iter().map(|cell| cell.to_string()));
        }
        let product = dapp.select("Product", &products).value();

        let selected = product.map(move |product| {
            orders.filter(|row| {
                product == ALL_PRODUCTS || row.text("product") == Some(product.as_str())
            })
        });

        // Dates on the x-axis become a time axis: orders are placed by
        // when they happened, however unevenly they are spread.
        let per_day =
            selected.map(|orders| orders.sum_by("date", &["revenue"]).sort_by("date", false));
        dapp.area_chart(per_day, "date", &["revenue"])
            .title("Revenue per day")
            .height(260);

        let [left, right] = dapp.columns();

        // Bars compare totals, so first add up the orders of each region.
        let by_region = selected.map(|orders| orders.sum_by("region", &["revenue"]));
        left.bar_chart(by_region, "region", &["revenue"])
            .title("Revenue by region")
            .height(280);

        // One dot per order.
        right
            .scatter_chart(&selected, "units", &["revenue"])
            .title("Order size and revenue")
            .x_label("Units in the order")
            .height(280);

        // How are order values spread? Most are small, a few are large.
        dapp.histogram(&selected, "revenue")
            .title("Orders by revenue")
            .x_label("Revenue of the order")
            .y_label("Orders")
            .height(260);
    });
}
