//! Showing data.
//!
//! Load a [`DTable`](crate::prelude::DTable) with `DTable::from_csv` or
//! `DTable::from_json` (or build one row by row), then show it with
//! [`Dapp::table`].
use crate::app::Dapp;
use dappr_core::components::table::{Table, TableSource};
use dappr_core::framework::traits::Component;

/// Data
impl Dapp {
    /// Shows a table of data.
    ///
    /// Pass a `DTable` (or `&DTable`) for a fixed table, or a
    /// `Signal<DTable>` for one that redraws when the data changes.
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
}
