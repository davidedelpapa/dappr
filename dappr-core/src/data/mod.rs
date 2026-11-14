//! Tabular data: the [`DTable`](table::DTable) structure, and loading it
//! from CSV and JSON.

/// Loading a `DTable` from CSV text
#[cfg(feature = "csv")]
pub mod csv;
/// Loading from a URL
pub mod fetch;
/// Loading a `DTable` from JSON text
#[cfg(feature = "json")]
pub mod json;
/// `Loadable`, the state of data that arrives later
pub mod load;
/// The `DTable`, `DColumn` and `Cell` structures
pub mod table;

use std::fmt;

/// What can go wrong when loading data.
#[derive(Debug, Clone, PartialEq)]
pub enum DataError {
    /// The text is not valid CSV (for the options in use).
    Csv(String),
    /// The text is not valid JSON.
    Json(String),
    /// The JSON is valid, but not a shape that can become a table.
    Shape(String),
    /// The format needs a Cargo feature that is turned off.
    Unsupported(String),
}

impl fmt::Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DataError::Csv(message) => write!(f, "invalid CSV: {message}"),
            DataError::Json(message) => write!(f, "invalid JSON: {message}"),
            DataError::Shape(message) => write!(f, "cannot build a table: {message}"),
            DataError::Unsupported(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for DataError {}
