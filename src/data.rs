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
use dappr_core::communication::signals::Signal;
use dappr_core::components::status::LoadStatus;
use dappr_core::components::table::{Table, TableSource};
use dappr_core::data::load::Loadable;
use dappr_core::framework::traits::Component;

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
}
