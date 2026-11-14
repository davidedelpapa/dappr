//! # Loading example: data that arrives while the app runs
//!
//! Two ways to get a table at runtime, both giving a
//! `Signal<Loadable<DTable>>`:
//!
//! - `fetch_table(url)` downloads it;
//! - `dapp.file_upload(..).table()` reads a file the user picks (the file
//!   never leaves the browser).
//!
//! A `Loadable` is `Idle`, `Loading`, `Ready(table)` or `Failed(reason)`.
//! `dapp.load_status` tells the user which, and `dapp.table` shows the table
//! once it is there.
//!
//! `sample.json` (made-up bike-share numbers) is copied next to the app by
//! Trunk; see `index.html`.
use dappr::prelude::*;
use wasm_bindgen::prelude::*;

const FROM_URL: &str = "Fetch from a URL";
const FROM_FILE: &str = "Upload a file";

#[wasm_bindgen(start)]
pub fn run() {
    dapp!(dapp, {
        text!("Loading data", _title);
        text!("From a URL, or from a file you pick", _subtitle);

        let sidebar = dapp.sidebar();
        sidebar.h4_static("Data source");
        let source = sidebar.radio("Where from", &[FROM_URL, FROM_FILE]).value();

        // ---- From a URL --------------------------------------------------
        // The URL to fetch is its own Signal, set when the button is
        // pressed -- not on every keystroke in the box.
        let url_box = sidebar
            .text_input("URL (CSV, TSV or JSON)")
            .with_value("sample.json".to_string());
        let url = Signal::new(url_box.get());
        let fetch_button = sidebar.button("Fetch");
        let url_for_click = url.clone();
        fetch_button.pressed(move || url_for_click.set(url_box.get()));
        let fetched = fetch_table(&url);

        // ---- From a file -------------------------------------------------
        let upload = sidebar
            .file_upload("File (CSV, TSV or JSON)")
            .accept(".csv,.tsv,.tab,.json,.txt");
        let uploaded = upload.table();

        // ---- Whichever source is selected --------------------------------
        let data = computed!(source, fetched, uploaded => {
            if source == FROM_URL { fetched } else { uploaded }
        });

        dapp.load_status(&data);

        let summary = data.map(|state| match state {
            Loadable::Idle => "Nothing loaded yet.".to_string(),
            Loadable::Loading | Loadable::Failed(_) => String::new(),
            Loadable::Ready(table) => {
                format!("{} rows, {} columns", table.n_rows(), table.n_cols())
            }
        });
        dapp.p("{}", summary.clone(), Some(&summary.get()));

        dapp.table(&data);
    });
}
