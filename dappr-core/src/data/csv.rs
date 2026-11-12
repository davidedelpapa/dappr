//! Loading a [`DTable`] from CSV text.
//!
//! ```
//! use dappr_core::data::table::DTable;
//!
//! let table = DTable::from_csv("name,age\nDavid,37\nStephanie,31").unwrap();
//! assert_eq!(table.headers(), vec!["name", "age"]);
//! assert_eq!(table.column("age").unwrap().sum(), 68.0);
//! ```
// The four options (delimiter, headers, flexible, trim), their defaults and
// their meaning come from rustream's `<Csv>` component
// (src/components/csv.rs), as does building the reader with the `csv`
// crate's ReaderBuilder. What changed: it returns a Result instead of
// silently dropping bad records, cells are typed instead of all strings, and
// a file without headers no longer loses its first row.
use super::table::{Cell, DTable};
use super::DataError;
use ::csv::{ReaderBuilder, Trim};

/// How to read a CSV file. Start from [`CsvOptions::new`], change what you
/// need, then [`parse`](CsvOptions::parse).
///
/// ```
/// use dappr_core::data::csv::CsvOptions;
///
/// let table = CsvOptions::new()
///     .delimiter(b';')
///     .headers(false)
///     .parse("1;2\n3;4")
///     .unwrap();
/// assert_eq!(table.headers(), vec!["column_1", "column_2"]);
/// ```
#[derive(Debug, Clone)]
pub struct CsvOptions {
    delimiter: u8,
    headers: bool,
    flexible: bool,
    trim: bool,
    infer_types: bool,
}

impl Default for CsvOptions {
    fn default() -> Self {
        CsvOptions {
            delimiter: b',',
            headers: true,
            flexible: true,
            trim: true,
            infer_types: true,
        }
    }
}

impl CsvOptions {
    /// The defaults: comma-separated, first line is the headers, rows may
    /// have missing fields, whitespace around fields is trimmed, and cell
    /// types are inferred.
    pub fn new() -> Self {
        Self::default()
    }

    /// The character between fields: `b','` (default), `b'\t'`, `b';'`, ...
    /// It must be a single ASCII character.
    pub fn delimiter(mut self, delimiter: u8) -> Self {
        self.delimiter = delimiter;
        self
    }

    /// Whether the first line holds the column names (default `true`).
    /// Without headers, columns are named `column_1`, `column_2`, ...
    pub fn headers(mut self, headers: bool) -> Self {
        self.headers = headers;
        self
    }

    /// Whether rows may have a different number of fields than the first
    /// line (default `true`). Short rows are padded with missing values and
    /// long rows are cut. With `false`, such a row is an error, which catches
    /// a damaged file early.
    pub fn flexible(mut self, flexible: bool) -> Self {
        self.flexible = flexible;
        self
    }

    /// Whether to remove spaces around headers and fields (default `true`).
    pub fn trim(mut self, trim: bool) -> Self {
        self.trim = trim;
        self
    }

    /// Whether to turn fields into numbers and booleans where they look
    /// like one (default `true`); see [`Cell::parse`]. With `false`, every
    /// non-empty field stays text.
    pub fn infer_types(mut self, infer_types: bool) -> Self {
        self.infer_types = infer_types;
        self
    }

    /// Reads `text` with these options.
    pub fn parse(&self, text: &str) -> Result<DTable, DataError> {
        let mut reader = ReaderBuilder::new()
            .delimiter(self.delimiter)
            .has_headers(self.headers)
            .flexible(self.flexible)
            .trim(if self.trim { Trim::All } else { Trim::None })
            .from_reader(text.as_bytes());
        let csv_error = |e: ::csv::Error| DataError::Csv(e.to_string());

        let mut table = DTable::new();
        if self.headers {
            for title in reader.headers().map_err(csv_error)? {
                table.new_column(title);
            }
        }

        for record in reader.records() {
            let record = record.map_err(csv_error)?;
            if !self.headers && table.n_cols() == 0 {
                // No header line: the first record tells how many columns
                // there are (and is itself data).
                for index in 1..=record.len() {
                    table.new_column(&format!("column_{index}"));
                }
            }
            let row: Vec<Cell> = record
                .iter()
                .map(|field| {
                    if self.infer_types {
                        Cell::parse(field)
                    } else if field.is_empty() {
                        Cell::Null
                    } else {
                        Cell::Text(field.to_string())
                    }
                })
                .collect();
            table.push_row(row);
        }
        Ok(table)
    }
}

impl DTable {
    /// A table from comma-separated text whose first line is the headers.
    /// For anything else, see [`CsvOptions`].
    pub fn from_csv(text: &str) -> Result<DTable, DataError> {
        CsvOptions::new().parse(text)
    }

    /// A table from tab-separated text whose first line is the headers.
    pub fn from_tsv(text: &str) -> Result<DTable, DataError> {
        CsvOptions::new().delimiter(b'\t').parse(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_headers_and_infers_types() {
        let table =
            DTable::from_csv("name, age, member\nDavid, 37, true\nStephanie, 31, false").unwrap();
        assert_eq!(table.headers(), vec!["name", "age", "member"]);
        assert_eq!(table.n_rows(), 2);
        assert_eq!(table.get(0, "name"), Some(&Cell::Text("David".into())));
        assert_eq!(table.get(1, "age"), Some(&Cell::Number(31.0)));
        assert_eq!(table.get(0, "member"), Some(&Cell::Bool(true)));
    }

    #[test]
    fn quoted_fields_keep_delimiters_and_newlines() {
        let table = DTable::from_csv("city,note\n\"Rome, Italy\",\"two\nlines\"").unwrap();
        assert_eq!(table.get(0, "city").unwrap().as_str(), Some("Rome, Italy"));
        assert_eq!(table.get(0, "note").unwrap().as_str(), Some("two\nlines"));
    }

    #[test]
    fn without_headers_the_first_row_is_data() {
        let table = CsvOptions::new().headers(false).parse("1,2\n3,4").unwrap();
        assert_eq!(table.headers(), vec!["column_1", "column_2"]);
        assert_eq!(table.n_rows(), 2);
        assert_eq!(table.get(0, "column_1"), Some(&Cell::Number(1.0)));
    }

    #[test]
    fn flexible_pads_short_rows_and_strict_rejects_them() {
        let text = "a,b,c\n1,2\n3,4,5";
        let table = DTable::from_csv(text).unwrap();
        assert_eq!(table.get(0, "c"), Some(&Cell::Null));
        assert_eq!(table.get(1, "c"), Some(&Cell::Number(5.0)));

        let strict = CsvOptions::new().flexible(false).parse(text);
        assert!(matches!(strict, Err(DataError::Csv(_))));
    }

    #[test]
    fn delimiter_trim_and_types_can_be_changed() {
        let table = DTable::from_tsv("a\tb\n1\tx").unwrap();
        assert_eq!(table.get(0, "b").unwrap().as_str(), Some("x"));

        let raw = CsvOptions::new()
            .delimiter(b';')
            .trim(false)
            .infer_types(false)
            .parse("a;b\n 1;")
            .unwrap();
        assert_eq!(raw.get(0, "a"), Some(&Cell::Text(" 1".into())));
        assert_eq!(raw.get(0, "b"), Some(&Cell::Null));
    }

    #[test]
    fn empty_text_gives_an_empty_table() {
        let table = DTable::from_csv("").unwrap();
        assert_eq!((table.n_rows(), table.n_cols()), (0, 0));
        let only_headers = DTable::from_csv("a,b").unwrap();
        assert_eq!((only_headers.n_rows(), only_headers.n_cols()), (0, 2));
    }
}
