//! Tabular data: the [`DTable`](table::DTable) structure, and loading it
//! from CSV and JSON.

/// Loading a `DTable` from CSV text
#[cfg(feature = "csv")]
pub mod csv;
/// Loading a `DTable` from JSON text
#[cfg(feature = "json")]
pub mod json;
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
}

impl fmt::Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DataError::Csv(message) => write!(f, "invalid CSV: {message}"),
            DataError::Json(message) => write!(f, "invalid JSON: {message}"),
            DataError::Shape(message) => write!(f, "cannot build a table: {message}"),
        }
    }
}

impl std::error::Error for DataError {}
