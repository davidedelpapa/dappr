//! Data that arrives later: [`Loadable`], the state of something being
//! loaded, and [`parse_table`], which picks the right parser for a file.
use super::table::DTable;
use super::DataError;
use std::fmt;

/// The state of something that is loaded at runtime -- fetched from a URL,
/// or read from a file the user picked.
///
/// Loading takes time and can fail, so instead of a value you get a
/// `Signal<Loadable<T>>` that moves through these states.
#[derive(Debug, Clone, PartialEq)]
pub enum Loadable<T> {
    /// Nothing has been asked for yet (no file picked, empty URL).
    Idle,
    /// In progress.
    Loading,
    /// Done: here is the value.
    Ready(T),
    /// It failed; the text says why.
    Failed(String),
}

impl<T> Loadable<T> {
    /// The value, if it has arrived.
    pub fn ready(&self) -> Option<&T> {
        match self {
            Loadable::Ready(value) => Some(value),
            _ => None,
        }
    }

    /// Whether loading is in progress.
    pub fn is_loading(&self) -> bool {
        matches!(self, Loadable::Loading)
    }

    /// Why it failed, if it did.
    pub fn error(&self) -> Option<&str> {
        match self {
            Loadable::Failed(message) => Some(message),
            _ => None,
        }
    }

    /// Applies `f` to the value if it has arrived, keeping the other states.
    pub fn map<U>(&self, f: impl FnOnce(&T) -> U) -> Loadable<U> {
        match self {
            Loadable::Idle => Loadable::Idle,
            Loadable::Loading => Loadable::Loading,
            Loadable::Ready(value) => Loadable::Ready(f(value)),
            Loadable::Failed(message) => Loadable::Failed(message.clone()),
        }
    }

    /// Like [`map`](Loadable::map), for a step that can itself fail (such
    /// as parsing): an `Err` becomes [`Failed`](Loadable::Failed).
    pub fn and_then<U, E: fmt::Display>(&self, f: impl FnOnce(&T) -> Result<U, E>) -> Loadable<U> {
        match self.map(f) {
            Loadable::Ready(Ok(value)) => Loadable::Ready(value),
            Loadable::Ready(Err(error)) => Loadable::Failed(error.to_string()),
            Loadable::Idle => Loadable::Idle,
            Loadable::Loading => Loadable::Loading,
            Loadable::Failed(message) => Loadable::Failed(message),
        }
    }
}

impl<T: Clone + Default> Loadable<T> {
    /// The value if it has arrived, otherwise an empty one (an empty table,
    /// an empty string). Convenient to keep showing *something* while
    /// loading.
    pub fn ready_or_default(&self) -> T {
        self.ready().cloned().unwrap_or_default()
    }
}

impl<T, E: fmt::Display> From<Result<T, E>> for Loadable<T> {
    fn from(result: Result<T, E>) -> Self {
        match result {
            Ok(value) => Loadable::Ready(value),
            Err(error) => Loadable::Failed(error.to_string()),
        }
    }
}

/// Reads `text` as a table, choosing the format from the file name (or URL)
/// and, when there is one, the MIME type:
///
/// - JSON if the type mentions `json` or the name ends in `.json`;
/// - tab-separated if the name ends in `.tsv` / `.tab` or the type says so;
/// - comma-separated otherwise.
#[cfg_attr(not(any(feature = "csv", feature = "json")), allow(unused_variables))]
pub fn parse_table(name: &str, mime: &str, text: &str) -> Result<DTable, DataError> {
    // A URL's query string and fragment are not part of the file name.
    let path = name.split(['?', '#']).next().unwrap_or(name).to_lowercase();
    let mime = mime.to_lowercase();

    if mime.contains("json") || path.ends_with(".json") {
        #[cfg(feature = "json")]
        return DTable::from_json(text);
        #[cfg(not(feature = "json"))]
        return Err(unsupported("json"));
    }
    #[cfg(feature = "csv")]
    {
        if mime.contains("tab-separated") || path.ends_with(".tsv") || path.ends_with(".tab") {
            DTable::from_tsv(text)
        } else {
            DTable::from_csv(text)
        }
    }
    #[cfg(not(feature = "csv"))]
    Err(unsupported("csv"))
}

#[allow(dead_code)]
fn unsupported(feature: &str) -> DataError {
    DataError::Unsupported(format!(
        "reading this file needs dappr's \"{feature}\" feature, which is turned off"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loadable_accessors() {
        let ready: Loadable<i32> = Loadable::Ready(2);
        assert_eq!(ready.ready(), Some(&2));
        assert_eq!(ready.map(|n| n * 10), Loadable::Ready(20));
        assert_eq!(Loadable::<i32>::Loading.map(|n| n * 10), Loadable::Loading);
        assert!(Loadable::<i32>::Loading.is_loading());
        assert_eq!(Loadable::<i32>::Failed("x".into()).error(), Some("x"));
        assert_eq!(Loadable::<String>::Idle.ready_or_default(), "");
        assert_eq!(ready.ready_or_default(), 2);
    }

    #[test]
    fn and_then_turns_errors_into_failed() {
        let text: Loadable<String> = Loadable::Ready("12".into());
        assert_eq!(text.and_then(|t| t.parse::<i32>()), Loadable::Ready(12));
        let bad: Loadable<String> = Loadable::Ready("x".into());
        assert!(bad.and_then(|t| t.parse::<i32>()).error().is_some());
        assert_eq!(
            Loadable::<String>::Idle.and_then(|t| t.parse::<i32>()),
            Loadable::Idle
        );
    }

    #[cfg(all(feature = "csv", feature = "json"))]
    #[test]
    fn parse_table_picks_the_format() {
        let csv = parse_table("data.csv", "", "a,b\n1,2").unwrap();
        assert_eq!(csv.headers(), vec!["a", "b"]);
        let tsv = parse_table("DATA.TSV", "", "a\tb\n1\t2").unwrap();
        assert_eq!(tsv.headers(), vec!["a", "b"]);
        let json = parse_table(
            "https://x.test/api/rows.json?page=2#top",
            "",
            r#"[{"a": 1}]"#,
        );
        assert_eq!(json.unwrap().headers(), vec!["a"]);
        // The MIME type wins when the name says nothing.
        let api = parse_table(
            "https://x.test/api/rows",
            "application/json; charset=utf-8",
            "[]",
        );
        assert!(api.is_ok());
        // Unknown names are read as CSV.
        assert_eq!(
            parse_table("notes.txt", "text/plain", "a\n1")
                .unwrap()
                .n_rows(),
            1
        );
        assert!(parse_table("x.json", "", "a,b").is_err());
    }
}
