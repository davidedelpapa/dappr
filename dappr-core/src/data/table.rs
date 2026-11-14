//! A small table of data, in the spirit of a DataFrame: named columns of
//! cells, every column the same length.
//!
//! ```
//! use dappr_core::data::table::DTable;
//!
//! let mut cities = DTable::with_columns(&["city", "population"]);
//! cities.push_row(("Rome", 2_750_000));
//! cities.push_row(("Milan", 1_370_000));
//!
//! let big = cities.filter(|row| row.number("population") > Some(2_000_000.0));
//! assert_eq!(big.n_rows(), 1);
//! assert_eq!(cities.column("population").unwrap().sum(), 4_120_000.0);
//! ```
// DTable and DColumn come from rustream (src/structures/table.rs and
// column.rs): the names, the column-oriented layout, and the shape of
// new_column / push_column / push_row / headers / row access and the row
// iterator. What changed: cells are a small `Cell` enum of our own instead of
// serde_json::Value (so a plain table needs no dependencies), columns are
// kept the same length, and filter / sort / column statistics are new.
use std::cmp::Ordering;
use std::fmt;

/// One value in a table.
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    /// A missing value.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// Any number.
    Number(f64),
    /// Text.
    Text(String),
}

impl Cell {
    /// Reads a cell from text, as found in a CSV file: empty is
    /// [`Null`](Cell::Null), `true`/`false` are booleans, anything that
    /// parses as a number is a number, the rest is text.
    pub fn parse(text: &str) -> Cell {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Cell::Null;
        }
        match trimmed {
            "true" | "True" | "TRUE" => return Cell::Bool(true),
            "false" | "False" | "FALSE" => return Cell::Bool(false),
            _ => {}
        }
        // `f64::from_str` also accepts "inf", "nan" and "infinity", which in
        // a data file are far more likely to be text.
        let looks_numeric = trimmed
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E'));
        match trimmed.parse::<f64>() {
            Ok(number) if looks_numeric => Cell::Number(number),
            _ => Cell::Text(text.to_string()),
        }
    }

    /// The number, if this cell is one.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Cell::Number(number) => Some(*number),
            _ => None,
        }
    }

    /// The text, if this cell is text.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Cell::Text(text) => Some(text),
            _ => None,
        }
    }

    /// The boolean, if this cell is one.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Cell::Bool(value) => Some(*value),
            _ => None,
        }
    }

    /// Whether the value is missing.
    pub fn is_null(&self) -> bool {
        matches!(self, Cell::Null)
    }

    /// The order used by [`DTable::sort_by`]: missing values first, then
    /// booleans, numbers (by value), and text (alphabetically).
    pub fn compare(&self, other: &Cell) -> Ordering {
        fn rank(cell: &Cell) -> u8 {
            match cell {
                Cell::Null => 0,
                Cell::Bool(_) => 1,
                Cell::Number(_) => 2,
                Cell::Text(_) => 3,
            }
        }
        match (self, other) {
            (Cell::Bool(a), Cell::Bool(b)) => a.cmp(b),
            (Cell::Number(a), Cell::Number(b)) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
            (Cell::Text(a), Cell::Text(b)) => a.cmp(b),
            _ => rank(self).cmp(&rank(other)),
        }
    }
}

/// How a cell is shown: a missing value is empty.
impl fmt::Display for Cell {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Cell::Null => Ok(()),
            Cell::Bool(value) => write!(f, "{value}"),
            Cell::Number(number) => write!(f, "{number}"),
            Cell::Text(text) => f.write_str(text),
        }
    }
}

macro_rules! cell_from_number {
    ($($t:ty),*) => {$(
        impl From<$t> for Cell {
            fn from(value: $t) -> Cell {
                Cell::Number(value as f64)
            }
        }
    )*};
}
cell_from_number!(f64, f32, i8, i16, i32, i64, isize, u8, u16, u32, u64, usize);

impl From<bool> for Cell {
    fn from(value: bool) -> Cell {
        Cell::Bool(value)
    }
}
impl From<&str> for Cell {
    fn from(value: &str) -> Cell {
        Cell::Text(value.to_string())
    }
}
impl From<String> for Cell {
    fn from(value: String) -> Cell {
        Cell::Text(value)
    }
}
impl From<&String> for Cell {
    fn from(value: &String) -> Cell {
        Cell::Text(value.clone())
    }
}
impl<T: Into<Cell>> From<Option<T>> for Cell {
    fn from(value: Option<T>) -> Cell {
        value.map_or(Cell::Null, Into::into)
    }
}

/// Anything that can be pushed as a row: a `Vec` or array of one type, or a
/// tuple (up to 8 values) mixing types.
pub trait IntoRow {
    /// The row's cells, in column order.
    fn into_row(self) -> Vec<Cell>;
}
impl<T: Into<Cell>> IntoRow for Vec<T> {
    fn into_row(self) -> Vec<Cell> {
        self.into_iter().map(Into::into).collect()
    }
}
impl<T: Into<Cell>, const N: usize> IntoRow for [T; N] {
    fn into_row(self) -> Vec<Cell> {
        self.into_iter().map(Into::into).collect()
    }
}
macro_rules! tuple_into_row {
    ($($name:ident),+) => {
        impl<$($name: Into<Cell>),+> IntoRow for ($($name,)+) {
            #[allow(non_snake_case)]
            fn into_row(self) -> Vec<Cell> {
                let ($($name,)+) = self;
                vec![$($name.into()),+]
            }
        }
    };
}
tuple_into_row!(A);
tuple_into_row!(A, B);
tuple_into_row!(A, B, C);
tuple_into_row!(A, B, C, D);
tuple_into_row!(A, B, C, D, E);
tuple_into_row!(A, B, C, D, E, F);
tuple_into_row!(A, B, C, D, E, F, G);
tuple_into_row!(A, B, C, D, E, F, G, H);

/// A named column of cells.
#[derive(Debug, Clone, PartialEq)]
pub struct DColumn {
    title: String,
    cells: Vec<Cell>,
}

impl DColumn {
    /// An empty column.
    pub fn new(title: &str) -> DColumn {
        DColumn {
            title: title.to_string(),
            cells: Vec::new(),
        }
    }

    /// A column holding `values`.
    pub fn from_values<T: Into<Cell>>(title: &str, values: impl IntoIterator<Item = T>) -> DColumn {
        DColumn {
            title: title.to_string(),
            cells: values.into_iter().map(Into::into).collect(),
        }
    }

    /// The column's name.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Renames the column.
    pub fn set_title(&mut self, title: &str) {
        self.title = title.to_string();
    }

    /// The cells, top to bottom.
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    /// Adds a value at the bottom.
    pub fn push<T: Into<Cell>>(&mut self, value: T) {
        self.cells.push(value.into());
    }

    /// Number of cells.
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    /// Whether the column has no cells.
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// The numbers in the column, skipping anything that is not a number.
    pub fn numbers(&self) -> Vec<f64> {
        self.cells.iter().filter_map(Cell::as_f64).collect()
    }

    /// Whether the column holds numbers only (missing values aside), and at
    /// least one.
    pub fn is_numeric(&self) -> bool {
        let mut any = false;
        for cell in &self.cells {
            match cell {
                Cell::Number(_) => any = true,
                Cell::Null => {}
                _ => return false,
            }
        }
        any
    }

    /// Sum of the numbers (0 if there are none).
    pub fn sum(&self) -> f64 {
        self.numbers().iter().sum()
    }

    /// Average of the numbers, or `None` if there are none.
    pub fn mean(&self) -> Option<f64> {
        let numbers = self.numbers();
        if numbers.is_empty() {
            None
        } else {
            Some(numbers.iter().sum::<f64>() / numbers.len() as f64)
        }
    }

    /// Smallest number, or `None` if there are none.
    pub fn min(&self) -> Option<f64> {
        self.numbers().into_iter().reduce(f64::min)
    }

    /// Largest number, or `None` if there are none.
    pub fn max(&self) -> Option<f64> {
        self.numbers().into_iter().reduce(f64::max)
    }

    /// The different values in the column, in order of first appearance.
    /// Handy to fill a `select` with the choices present in the data.
    pub fn unique(&self) -> Vec<Cell> {
        let mut seen: Vec<Cell> = Vec::new();
        for cell in &self.cells {
            if !seen.contains(cell) {
                seen.push(cell.clone());
            }
        }
        seen
    }
}

/// A table: named columns, all the same length.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DTable {
    title: String,
    columns: Vec<DColumn>,
}

impl DTable {
    /// An empty table, with no columns.
    pub fn new() -> DTable {
        DTable::default()
    }

    /// An empty table with these columns, ready for [`push_row`](DTable::push_row).
    pub fn with_columns(titles: &[&str]) -> DTable {
        DTable {
            title: String::new(),
            columns: titles.iter().map(|title| DColumn::new(title)).collect(),
        }
    }

    /// A table from an `ndarray` 2-D array: one table row per array row.
    /// Columns are named `0`, `1`, ...; rename them with
    /// [`set_headers`](DTable::set_headers).
    #[cfg(feature = "ndarray")]
    pub fn from_ndarray<T>(array: &ndarray::Array2<T>) -> DTable
    where
        T: Clone + Into<Cell>,
    {
        let mut table = DTable::new();
        for index in 0..array.ncols() {
            table.new_column(&index.to_string());
        }
        for row in array.rows() {
            table.push_row(row.to_vec());
        }
        table
    }

    /// The table's title (empty if it has none). A table view shows it as
    /// the caption.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Sets the title.
    pub fn set_title(&mut self, title: &str) {
        self.title = title.to_string();
    }

    /// Chainable form of [`set_title`](DTable::set_title).
    pub fn titled(mut self, title: &str) -> DTable {
        self.set_title(title);
        self
    }

    /// Number of rows.
    pub fn n_rows(&self) -> usize {
        self.columns.first().map_or(0, DColumn::len)
    }

    /// Number of columns.
    pub fn n_cols(&self) -> usize {
        self.columns.len()
    }

    /// Whether the table has no rows.
    pub fn is_empty(&self) -> bool {
        self.n_rows() == 0
    }

    /// The column names.
    pub fn headers(&self) -> Vec<String> {
        self.columns.iter().map(|col| col.title.clone()).collect()
    }

    /// Renames the columns, left to right. Extra names are ignored.
    pub fn set_headers(&mut self, titles: &[&str]) {
        for (column, title) in self.columns.iter_mut().zip(titles) {
            column.set_title(title);
        }
    }

    /// All columns.
    pub fn columns(&self) -> &[DColumn] {
        &self.columns
    }

    /// The column with this name.
    pub fn column(&self, title: &str) -> Option<&DColumn> {
        self.columns.iter().find(|column| column.title == title)
    }

    /// Adds an empty column (filled with missing values if the table
    /// already has rows).
    pub fn new_column(&mut self, title: &str) {
        self.push_column(DColumn::new(title));
    }

    /// Adds a column. If its length differs from the table's, the shorter
    /// side is padded with missing values.
    pub fn push_column(&mut self, mut column: DColumn) {
        let rows = self.n_rows().max(column.len());
        column.cells.resize(rows, Cell::Null);
        for existing in &mut self.columns {
            existing.cells.resize(rows, Cell::Null);
        }
        self.columns.push(column);
    }

    /// Adds a row: a tuple, `Vec` or array with one value per column.
    /// A row that is too short is padded with missing values; one that is
    /// too long is cut.
    ///
    /// ```
    /// # use dappr_core::data::table::DTable;
    /// let mut table = DTable::with_columns(&["name", "age", "member"]);
    /// table.push_row(("Ada", 36, true));
    /// table.push_row(("Alan", None::<f64>, false));
    /// ```
    pub fn push_row(&mut self, row: impl IntoRow) {
        let mut cells = row.into_row().into_iter();
        for column in &mut self.columns {
            column.cells.push(cells.next().unwrap_or(Cell::Null));
        }
    }

    /// The cell at this row and column name.
    pub fn get(&self, row: usize, column: &str) -> Option<&Cell> {
        self.column(column)?.cells.get(row)
    }

    /// The row at `index`.
    pub fn row(&self, index: usize) -> Option<Row<'_>> {
        (index < self.n_rows()).then_some(Row { table: self, index })
    }

    /// The rows, top to bottom.
    pub fn rows(&self) -> impl Iterator<Item = Row<'_>> {
        (0..self.n_rows()).map(move |index| Row { table: self, index })
    }

    fn with_rows(&self, indices: &[usize]) -> DTable {
        DTable {
            title: self.title.clone(),
            columns: self
                .columns
                .iter()
                .map(|column| DColumn {
                    title: column.title.clone(),
                    cells: indices.iter().map(|&i| column.cells[i].clone()).collect(),
                })
                .collect(),
        }
    }

    /// A new table with only the rows for which `keep` returns true.
    ///
    /// ```
    /// # use dappr_core::data::table::DTable;
    /// # let mut cities = DTable::with_columns(&["city", "country"]);
    /// # cities.push_row(("Rome", "Italy"));
    /// let italian = cities.filter(|row| row.text("country") == Some("Italy"));
    /// # assert_eq!(italian.n_rows(), 1);
    /// ```
    pub fn filter(&self, keep: impl Fn(&Row) -> bool) -> DTable {
        let indices: Vec<usize> = self
            .rows()
            .filter(|row| keep(row))
            .map(|row| row.index)
            .collect();
        self.with_rows(&indices)
    }

    /// A new table sorted by a column: missing values first, then by value.
    /// With `descending`, the whole order is reversed. Rows that compare
    /// equal keep their relative order. An unknown column leaves the order
    /// unchanged.
    pub fn sort_by(&self, column: &str, descending: bool) -> DTable {
        let Some(column) = self.column(column) else {
            return self.clone();
        };
        let mut indices: Vec<usize> = (0..self.n_rows()).collect();
        indices.sort_by(|&a, &b| {
            let order = column.cells[a].compare(&column.cells[b]);
            if descending {
                order.reverse()
            } else {
                order
            }
        });
        self.with_rows(&indices)
    }

    /// A new table with one row per distinct value of `key`, holding the
    /// sum of each of `columns` over the rows that share it. Groups come in
    /// order of first appearance; values that are not numbers count as zero.
    /// This is the usual step before a bar chart.
    ///
    /// ```
    /// # use dappr_core::data::table::DTable;
    /// let mut orders = DTable::with_columns(&["region", "units"]);
    /// orders.push_row(("North", 2));
    /// orders.push_row(("South", 5));
    /// orders.push_row(("North", 3));
    /// let totals = orders.sum_by("region", &["units"]);
    /// assert_eq!(totals.n_rows(), 2);
    /// assert_eq!(totals.row(0).unwrap().number("units"), Some(5.0));
    /// ```
    pub fn sum_by(&self, key: &str, columns: &[&str]) -> DTable {
        let mut titles = vec![key];
        titles.extend(columns.iter().filter(|name| self.column(name).is_some()));
        let mut out = DTable::with_columns(&titles);
        out.title = self.title.clone();
        let Some(key_column) = self.column(key) else {
            return out;
        };
        let groups = key_column.unique();
        for group in groups {
            let mut row = vec![group.clone()];
            for title in &titles[1..] {
                let cells = self.column(title).map(DColumn::cells).unwrap_or(&[]);
                let sum: f64 = key_column
                    .cells
                    .iter()
                    .zip(cells)
                    .filter(|(k, _)| **k == group)
                    .filter_map(|(_, cell)| cell.as_f64())
                    .sum();
                row.push(Cell::Number(sum));
            }
            out.push_row(row);
        }
        out
    }

    /// A new table with the first `n` rows.
    pub fn head(&self, n: usize) -> DTable {
        let indices: Vec<usize> = (0..self.n_rows().min(n)).collect();
        self.with_rows(&indices)
    }

    /// A new table with only these columns, in this order. Unknown names
    /// are skipped.
    pub fn select(&self, columns: &[&str]) -> DTable {
        DTable {
            title: self.title.clone(),
            columns: columns
                .iter()
                .filter_map(|title| self.column(title).cloned())
                .collect(),
        }
    }
}

/// One row of a [`DTable`], with access to its cells by column name.
#[derive(Clone, Copy)]
pub struct Row<'a> {
    table: &'a DTable,
    index: usize,
}

impl<'a> Row<'a> {
    /// The row's position in the table.
    pub fn index(&self) -> usize {
        self.index
    }

    /// The cell in this column.
    pub fn get(&self, column: &str) -> Option<&'a Cell> {
        self.table.get(self.index, column)
    }

    /// The number in this column, if it is one.
    pub fn number(&self, column: &str) -> Option<f64> {
        self.get(column)?.as_f64()
    }

    /// The text in this column, if it is text.
    pub fn text(&self, column: &str) -> Option<&'a str> {
        self.get(column)?.as_str()
    }

    /// The boolean in this column, if it is one.
    pub fn bool(&self, column: &str) -> Option<bool> {
        self.get(column)?.as_bool()
    }

    /// All the row's cells, in column order.
    pub fn cells(&self) -> Vec<&'a Cell> {
        self.table
            .columns
            .iter()
            .map(|column| &column.cells[self.index])
            .collect()
    }
}

/// Plain-text rendering: tab-separated, headers first. Meant for logging
/// and quick checks; use a table view to show data on the page.
// Adapted from rustream's `impl Display for DTable`, without cutting text
// cells to 7 characters.
impl fmt::Display for DTable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if !self.title.is_empty() {
            writeln!(f, "{}:", self.title)?;
        }
        writeln!(f, "{}", self.headers().join("\t"))?;
        for row in self.rows() {
            let cells: Vec<String> = row.cells().iter().map(|cell| cell.to_string()).collect();
            writeln!(f, "{}", cells.join("\t"))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cities() -> DTable {
        let mut table = DTable::with_columns(&["city", "country", "population"]);
        table.push_row(("Rome", "Italy", 2.75));
        table.push_row(("Paris", "France", 2.1));
        table.push_row(("Milan", "Italy", 1.37));
        table.push_row(("Lyon", "France", None::<f64>));
        table
    }

    #[test]
    fn cell_parse_infers_types() {
        assert_eq!(Cell::parse(""), Cell::Null);
        assert_eq!(Cell::parse("  "), Cell::Null);
        assert_eq!(Cell::parse("true"), Cell::Bool(true));
        assert_eq!(Cell::parse("FALSE"), Cell::Bool(false));
        assert_eq!(Cell::parse("42"), Cell::Number(42.0));
        assert_eq!(Cell::parse(" -1.5e3 "), Cell::Number(-1500.0));
        assert_eq!(Cell::parse("Rome"), Cell::Text("Rome".into()));
        // Words that f64 would accept stay text.
        assert_eq!(Cell::parse("nan"), Cell::Text("nan".into()));
        assert_eq!(Cell::parse("Infinity"), Cell::Text("Infinity".into()));
        assert_eq!(Cell::parse("1-2"), Cell::Text("1-2".into()));
    }

    #[test]
    fn cell_display_and_conversions() {
        assert_eq!(Cell::from(3).to_string(), "3");
        assert_eq!(Cell::from(2.5).to_string(), "2.5");
        assert_eq!(Cell::from(true).to_string(), "true");
        assert_eq!(Cell::from("hi").to_string(), "hi");
        assert_eq!(Cell::from(None::<i32>), Cell::Null);
        assert_eq!(Cell::Null.to_string(), "");
    }

    #[test]
    fn push_row_pads_and_cuts() {
        let mut table = DTable::with_columns(&["a", "b"]);
        table.push_row([1]);
        table.push_row(vec![1, 2, 3]);
        assert_eq!(table.n_rows(), 2);
        assert_eq!(table.get(0, "b"), Some(&Cell::Null));
        assert_eq!(table.get(1, "b"), Some(&Cell::Number(2.0)));
    }

    #[test]
    fn push_column_keeps_columns_the_same_length() {
        let mut table = cities();
        table.push_column(DColumn::from_values("capital", [true, true]));
        assert_eq!(table.n_cols(), 4);
        assert_eq!(table.column("capital").unwrap().len(), 4);
        assert_eq!(table.get(3, "capital"), Some(&Cell::Null));

        table.push_column(DColumn::from_values("rank", [1, 2, 3, 4, 5]));
        assert_eq!(table.n_rows(), 5);
        assert_eq!(table.get(4, "city"), Some(&Cell::Null));
    }

    #[test]
    fn rows_give_access_by_column_name() {
        let table = cities();
        let row = table.row(1).unwrap();
        assert_eq!(row.text("city"), Some("Paris"));
        assert_eq!(row.number("population"), Some(2.1));
        assert_eq!(row.number("city"), None);
        assert_eq!(row.get("nope"), None);
        assert!(table.row(4).is_none());
        assert_eq!(table.rows().count(), 4);
    }

    #[test]
    fn filter_keeps_matching_rows_and_headers() {
        let italy = cities().filter(|row| row.text("country") == Some("Italy"));
        assert_eq!(italy.headers(), vec!["city", "country", "population"]);
        assert_eq!(italy.n_rows(), 2);
        assert_eq!(italy.get(1, "city"), Some(&Cell::Text("Milan".into())));

        let none = cities().filter(|_| false);
        assert_eq!(none.n_rows(), 0);
        assert_eq!(none.n_cols(), 3);
    }

    #[test]
    fn sort_by_orders_values_with_missing_first() {
        let names = |table: &DTable| -> Vec<String> {
            table
                .column("city")
                .unwrap()
                .cells()
                .iter()
                .map(|c| c.to_string())
                .collect()
        };
        let ascending = cities().sort_by("population", false);
        assert_eq!(names(&ascending), vec!["Lyon", "Milan", "Paris", "Rome"]);
        let descending = cities().sort_by("population", true);
        assert_eq!(names(&descending), vec!["Rome", "Paris", "Milan", "Lyon"]);
        let by_text = cities().sort_by("city", false);
        assert_eq!(names(&by_text), vec!["Lyon", "Milan", "Paris", "Rome"]);
        assert_eq!(cities().sort_by("nope", false), cities());
    }

    #[test]
    fn sum_by_groups_in_first_seen_order() {
        let totals = cities().sum_by("country", &["population", "nope"]);
        assert_eq!(totals.headers(), vec!["country", "population"]);
        assert_eq!(totals.get(0, "country"), Some(&Cell::from("Italy")));
        assert!((totals.row(0).unwrap().number("population").unwrap() - 4.12).abs() < 1e-9);
        // Lyon's missing population counts as zero.
        assert_eq!(totals.row(1).unwrap().number("population"), Some(2.1));
        assert_eq!(cities().sum_by("nope", &["population"]).n_rows(), 0);
    }

    #[test]
    fn head_and_select() {
        let table = cities();
        assert_eq!(table.head(2).n_rows(), 2);
        assert_eq!(table.head(99).n_rows(), 4);
        let narrow = table.select(&["population", "city", "nope"]);
        assert_eq!(narrow.headers(), vec!["population", "city"]);
        assert_eq!(narrow.n_rows(), 4);
    }

    #[test]
    fn column_statistics_skip_non_numbers() {
        let table = cities();
        let population = table.column("population").unwrap();
        assert!(population.is_numeric());
        assert!(!table.column("city").unwrap().is_numeric());
        assert!((population.sum() - 6.22).abs() < 1e-9);
        assert!((population.mean().unwrap() - 6.22 / 3.0).abs() < 1e-9);
        assert_eq!(population.min(), Some(1.37));
        assert_eq!(population.max(), Some(2.75));
        assert_eq!(DColumn::new("empty").mean(), None);
        assert_eq!(
            table.column("country").unwrap().unique(),
            vec![Cell::from("Italy"), Cell::from("France")]
        );
    }

    #[test]
    fn display_is_tab_separated() {
        let table = cities().head(2).titled("Cities");
        assert_eq!(
            table.to_string(),
            "Cities:\ncity\tcountry\tpopulation\nRome\tItaly\t2.75\nParis\tFrance\t2.1\n"
        );
    }

    #[cfg(feature = "ndarray")]
    #[test]
    fn from_ndarray_makes_one_row_per_array_row() {
        let array = ndarray::array![[1., 2., 3.], [4., 5., 6.]];
        let table = DTable::from_ndarray(&array);
        assert_eq!(table.headers(), vec!["0", "1", "2"]);
        assert_eq!(table.n_rows(), 2);
        assert_eq!(table.get(1, "2"), Some(&Cell::Number(6.0)));
    }
}
