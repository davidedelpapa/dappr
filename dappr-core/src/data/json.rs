//! Loading a [`DTable`] from JSON text.
//!
//! Three shapes are understood:
//!
//! - a list of records: `[{"name": "Ada", "age": 36}, {"name": "Alan"}]`
//! - an object of columns: `{"name": ["Ada", "Alan"], "age": [36, 41]}`
//! - a list of rows: `[[1, 2], [3, 4]]` (columns are named `column_1`, ...)
//!
//! ```
//! use dappr_core::data::table::DTable;
//!
//! let table = DTable::from_json(r#"[{"name": "Ada", "age": 36}, {"name": "Alan"}]"#).unwrap();
//! assert_eq!(table.headers(), vec!["name", "age"]);
//! assert!(table.get(1, "age").unwrap().is_null());
//! ```
// rustream's `<Json>` component (src/components/json.rs) parsed text into a
// serde_json::Value for display; turning JSON into a table is new here.
use super::table::{Cell, DColumn, DTable};
use super::DataError;
use serde_json::Value;

/// A JSON value as a cell. Nested lists and objects are kept as their JSON
/// text.
fn cell(value: &Value) -> Cell {
    match value {
        Value::Null => Cell::Null,
        Value::Bool(b) => Cell::Bool(*b),
        Value::Number(n) => n.as_f64().map_or(Cell::Null, Cell::Number),
        Value::String(s) => Cell::Text(s.clone()),
        nested => Cell::Text(nested.to_string()),
    }
}

fn from_records(records: &[Value]) -> Result<DTable, DataError> {
    // Columns: every key that appears, in order of first appearance.
    let mut titles: Vec<&str> = Vec::new();
    for record in records {
        let object = record.as_object().ok_or_else(|| {
            DataError::Shape("a list of records must contain only objects".to_string())
        })?;
        for key in object.keys() {
            if !titles.contains(&key.as_str()) {
                titles.push(key);
            }
        }
    }
    let mut table = DTable::with_columns(&titles);
    for record in records {
        let row: Vec<Cell> = titles
            .iter()
            .map(|title| record.get(title).map_or(Cell::Null, cell))
            .collect();
        table.push_row(row);
    }
    Ok(table)
}

fn from_rows(rows: &[Value]) -> Result<DTable, DataError> {
    let width = rows
        .iter()
        .map(|row| row.as_array().map_or(0, Vec::len))
        .max()
        .unwrap_or(0);
    let titles: Vec<String> = (1..=width).map(|i| format!("column_{i}")).collect();
    let titles: Vec<&str> = titles.iter().map(String::as_str).collect();
    let mut table = DTable::with_columns(&titles);
    for row in rows {
        let values = row.as_array().ok_or_else(|| {
            DataError::Shape("a list of rows must contain only lists".to_string())
        })?;
        table.push_row(values.iter().map(cell).collect::<Vec<Cell>>());
    }
    Ok(table)
}

impl DTable {
    /// A table from JSON text: a list of records, an object of columns, or
    /// a list of rows (see the [module docs](crate::data::json)).
    pub fn from_json(text: &str) -> Result<DTable, DataError> {
        let value: Value =
            serde_json::from_str(text).map_err(|e| DataError::Json(e.to_string()))?;
        match &value {
            Value::Array(items) => match items.first() {
                None => Ok(DTable::new()),
                Some(Value::Object(_)) => from_records(items),
                Some(Value::Array(_)) => from_rows(items),
                Some(_) => Err(DataError::Shape(
                    "a list must contain records (objects) or rows (lists)".to_string(),
                )),
            },
            Value::Object(columns) => {
                let mut table = DTable::new();
                for (title, values) in columns {
                    let values = values.as_array().ok_or_else(|| {
                        DataError::Shape(format!("column \"{title}\" is not a list"))
                    })?;
                    table.push_column(DColumn::from_values(title, values.iter().map(cell)));
                }
                Ok(table)
            }
            _ => Err(DataError::Shape(
                "expected a list of records, an object of columns, or a list of rows".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_take_the_union_of_keys_in_first_seen_order() {
        let table = DTable::from_json(
            r#"[{"name": "Ada", "age": 36}, {"name": "Alan", "city": "London"}]"#,
        )
        .unwrap();
        assert_eq!(table.headers(), vec!["name", "age", "city"]);
        assert_eq!(table.get(0, "age"), Some(&Cell::Number(36.0)));
        assert_eq!(table.get(0, "city"), Some(&Cell::Null));
        assert_eq!(table.get(1, "city").unwrap().as_str(), Some("London"));
    }

    #[test]
    fn object_of_columns_keeps_column_order_and_pads() {
        let table = DTable::from_json(r#"{"z": [1, 2, 3], "a": [true]}"#).unwrap();
        assert_eq!(table.headers(), vec!["z", "a"]);
        assert_eq!(table.n_rows(), 3);
        assert_eq!(table.get(0, "a"), Some(&Cell::Bool(true)));
        assert_eq!(table.get(2, "a"), Some(&Cell::Null));
    }

    #[test]
    fn list_of_rows_gets_generated_headers() {
        let table = DTable::from_json("[[1, 2], [3, 4, 5]]").unwrap();
        assert_eq!(table.headers(), vec!["column_1", "column_2", "column_3"]);
        assert_eq!(table.get(0, "column_3"), Some(&Cell::Null));
        assert_eq!(table.get(1, "column_3"), Some(&Cell::Number(5.0)));
    }

    #[test]
    fn nested_values_are_kept_as_json_text() {
        let table = DTable::from_json(r#"[{"tags": ["a", "b"], "none": null}]"#).unwrap();
        assert_eq!(table.get(0, "tags").unwrap().as_str(), Some(r#"["a","b"]"#));
        assert!(table.get(0, "none").unwrap().is_null());
    }

    #[test]
    fn errors_say_what_is_wrong() {
        assert!(matches!(
            DTable::from_json("{oops"),
            Err(DataError::Json(_))
        ));
        assert!(matches!(DTable::from_json("42"), Err(DataError::Shape(_))));
        assert!(matches!(
            DTable::from_json("[1, 2]"),
            Err(DataError::Shape(_))
        ));
        assert!(matches!(
            DTable::from_json(r#"[{"a": 1}, 2]"#),
            Err(DataError::Shape(_))
        ));
        assert!(matches!(
            DTable::from_json(r#"{"a": 1}"#),
            Err(DataError::Shape(_))
        ));
        assert_eq!(DTable::from_json("[]").unwrap().n_cols(), 0);
    }
}
