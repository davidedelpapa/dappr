//! Showing data.
//!
//! Load a [`DTable`](crate::prelude::DTable) with `DTable::from_csv` or
//! `DTable::from_json` (or build one row by row), then show it with
//! [`Dapp::table`].
//!
//! Data can also arrive while the app runs: fetch it with
//! [`fetch_table`](crate::prelude::fetch_table), or let the user pick a file
//! with [`Dapp::file_upload`]. Both give a
//! `Signal<Loadable<DTable>>`, which [`Dapp::table`] accepts directly and
//! [`Dapp::load_status`] reports on.
use crate::app::Dapp;
use dappr_core::charts::ChartKind;
use dappr_core::communication::signals::Signal;
use dappr_core::components::chart::Chart;
use dappr_core::components::drawing::{Canvas, Svg};
use dappr_core::components::status::LoadStatus;
use dappr_core::components::table::{Table, TableSource};
use dappr_core::data::load::Loadable;
use dappr_core::framework::traits::Component;
use dappr_core::framework::traits::TextSource;

/// Data
impl Dapp {
    /// Shows a table of data.
    ///
    /// Pass a `DTable` (or `&DTable`) for a fixed table, a `Signal<DTable>`
    /// for one that redraws when the data changes, or a
    /// `Signal<Loadable<DTable>>` for data being fetched or uploaded (shown
    /// empty until it arrives).
    ///
    /// ```no_run
    /// # use dappr::prelude::*;
    /// # let dapp = Dapp::new();
    /// let mut scores = DTable::with_columns(&["player", "score"]);
    /// scores.push_row(("Ada", 97));
    /// scores.push_row(("Alan", 88));
    /// dapp.table(&scores);
    ///
    /// // Reactive: follows the checkbox.
    /// let best_first = dapp.checkbox("Best first", true);
    /// dapp.table(best_first.value().map(move |desc| scores.sort_by("score", *desc)));
    /// ```
    ///
    /// Long tables show their first 1000 rows; chain `.max_rows(n)` to change that.
    pub fn table(&self, source: impl TableSource) -> Table {
        let table = Table::new(source);
        self.append(table.get_element());
        table
    }

    /// A line that reports on data being loaded: a spinner while it loads,
    /// the reason if it failed, nothing once it is there.
    ///
    /// ```no_run
    /// # use dappr::prelude::*;
    /// # let dapp = Dapp::new();
    /// let prices = fetch_table("prices.csv");
    /// dapp.load_status(&prices);
    /// dapp.table(&prices);
    /// ```
    pub fn load_status<T: Clone + 'static>(&self, source: &Signal<Loadable<T>>) -> LoadStatus {
        let status = LoadStatus::new(source);
        self.append(status.get_element());
        status
    }

    fn chart(&self, kind: ChartKind, source: impl TableSource, x: &str, y: &[&str]) -> Chart {
        let chart = Chart::new(kind, source, x, y);
        self.append(chart.get_element());
        chart
    }

    /// A line chart: how the `y` columns change along `x`. Use it when `x`
    /// has an order -- dates, steps, a measured quantity.
    ///
    /// Like [`table`](Dapp::table), `source` is a `DTable` for a fixed chart
    /// or a `Signal<DTable>` for one that follows the data. Chain `.title()`,
    /// `.x_label()`, `.y_label()` and `.height()` to adjust it.
    ///
    /// ```no_run
    /// # use dappr::prelude::*;
    /// # let dapp = Dapp::new();
    /// let sales = DTable::from_csv("month,north,south\nJan,120,80\nFeb,150,95").unwrap();
    /// dapp.line_chart(&sales, "month", &["north", "south"])
    ///     .title("Monthly sales")
    ///     .y_label("Units");
    /// ```
    ///
    /// Up to 8 columns are drawn. Numbers on `x` are placed on a number
    /// line; text (dates included) is spaced evenly in row order.
    pub fn line_chart(&self, source: impl TableSource, x: &str, y: &[&str]) -> Chart {
        self.chart(ChartKind::Line, source, x, y)
    }

    /// A bar chart: the `y` columns compared across the categories in `x`.
    /// Bars always start at zero; several `y` columns sit side by side.
    ///
    /// ```no_run
    /// # use dappr::prelude::*;
    /// # let dapp = Dapp::new();
    /// let sales = DTable::from_csv("region,revenue\nNorth,4916\nSouth,3875").unwrap();
    /// dapp.bar_chart(&sales, "region", &["revenue"]).title("Revenue by region");
    /// ```
    pub fn bar_chart(&self, source: impl TableSource, x: &str, y: &[&str]) -> Chart {
        self.chart(ChartKind::Bar, source, x, y)
    }

    /// A scatter chart: one dot per row, to see how the `y` columns relate
    /// to `x`. Both must be numbers. Up to 3 `y` columns are drawn.
    ///
    /// ```no_run
    /// # use dappr::prelude::*;
    /// # let dapp = Dapp::new();
    /// let trips = DTable::from_csv("minutes,km\n12,3.1\n25,7.4\n8,1.9").unwrap();
    /// dapp.scatter_chart(&trips, "minutes", &["km"]);
    /// ```
    pub fn scatter_chart(&self, source: impl TableSource, x: &str, y: &[&str]) -> Chart {
        self.chart(ChartKind::Scatter, source, x, y)
    }

    /// Shows an SVG drawing from its text -- typically a chart made with a
    /// plotting crate that outputs SVG. Pass a `Signal<String>` to redraw
    /// when it changes.
    ///
    /// The text is inserted as markup: use it for SVG your own code
    /// produced, never for text that comes from a user.
    ///
    /// ```no_run
    /// # use dappr::prelude::*;
    /// # let dapp = Dapp::new();
    /// dapp.svg(r#"<svg viewBox="0 0 100 40" width="200"><circle cx="20" cy="20" r="15"/></svg>"#);
    /// ```
    pub fn svg(&self, source: impl TextSource) -> Svg {
        let svg = Svg::new(source);
        self.append(svg.get_element());
        svg
    }

    /// Adds a `<canvas>` of the given size in pixels, to draw on yourself or
    /// to hand to a library that draws on a canvas. See
    /// [`Canvas::draw`] and [`Canvas::draw_on`].
    pub fn canvas(&self, width: u32, height: u32) -> Canvas {
        let canvas = Canvas::new(width, height);
        self.append(canvas.get_element());
        canvas
    }
}
