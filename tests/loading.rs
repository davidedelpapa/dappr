#![allow(dead_code)]
use dappr::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_test::*;
use web_sys::{DataTransfer, Event, File, HtmlInputElement};

wasm_bindgen_test_configure!(run_in_browser);

async fn sleep(ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms)
            .unwrap();
    });
    JsFuture::from(promise).await.unwrap();
}

/// Waits (up to ~3s) until `signal` is no longer loading, and returns it.
async fn settled<T: Clone + 'static>(signal: &Signal<Loadable<T>>) -> Loadable<T> {
    for _ in 0..150 {
        if !signal.get().is_loading() {
            break;
        }
        sleep(20).await;
    }
    signal.get()
}

/// What the browser does when a user picks a file in a file input.
fn user_picks(field: &Field<Option<UploadedFile>>, name: &str, content: &str) {
    let parts = js_sys::Array::of1(&content.into());
    let file = File::new_with_str_sequence(&parts, name).unwrap();
    let transfer = DataTransfer::new().unwrap();
    transfer.items().add_with_file(&file).unwrap();
    let input: &HtmlInputElement = field.control().unchecked_ref();
    input.set_files(Some(&transfer.files().unwrap()));
    input
        .dispatch_event(&Event::new("change").unwrap())
        .unwrap();
}

async fn until_file(field: &Field<Option<UploadedFile>>, name: &str) {
    for _ in 0..150 {
        if field.get().is_some_and(|file| file.name() == name) {
            return;
        }
        sleep(20).await;
    }
    panic!("the file {name} never arrived");
}

fn csv_url(csv: &str) -> String {
    format!(
        "data:text/csv,{}",
        String::from(js_sys::encode_uri_component(csv))
    )
}

// ---- File upload ------------------------------------------------------------

#[wasm_bindgen_test]
async fn test_upload_reads_the_picked_file() {
    let dapp = Dapp::new();
    let upload = dapp.file_upload("Your data").accept(".csv,.json");
    assert_eq!(
        upload.get_element().inner_html(),
        "<span>Your data</span><input type=\"file\" accept=\".csv,.json\">"
    );
    assert!(upload.get().is_none());

    user_picks(&upload, "people.csv", "name,age\nAda,36\nAlan,41");
    until_file(&upload, "people.csv").await;

    let file = upload.get().unwrap();
    assert_eq!(file.name(), "people.csv");
    assert_eq!(file.size(), 23);
    assert_eq!(file.text(), "name,age\nAda,36\nAlan,41");
    assert_eq!(file.table().unwrap().column("age").unwrap().sum(), 77.0);
}

#[wasm_bindgen_test]
async fn test_upload_table_picks_the_parser_and_reports_bad_files() {
    let dapp = Dapp::new();
    let upload = dapp.file_upload("Data");
    let data = upload.table();
    let status = dapp.load_status(&data);
    let view = dapp.table(&data);
    let rows = || {
        view.get_element()
            .query_selector_all("tbody tr")
            .unwrap()
            .length()
    };

    assert_eq!(data.get(), Loadable::Idle);
    assert!(status.get_element().hidden());
    assert_eq!(rows(), 0);

    user_picks(
        &upload,
        "cities.json",
        r#"[{"city": "Rome"}, {"city": "Milan"}]"#,
    );
    until_file(&upload, "cities.json").await;
    assert_eq!(data.get().ready().unwrap().headers(), vec!["city"]);
    assert_eq!(rows(), 2);
    assert!(status.get_element().hidden());

    user_picks(&upload, "broken.json", "{ not json");
    until_file(&upload, "broken.json").await;
    assert!(data.get().error().unwrap().contains("invalid JSON"));
    assert_eq!(rows(), 0);
    assert!(!status.get_element().hidden());
    assert!(status
        .get_element()
        .text_content()
        .unwrap()
        .starts_with("Could not load the data: invalid JSON"));

    // Clearing from code empties the picker and goes back to idle.
    upload.set(None);
    assert_eq!(data.get(), Loadable::Idle);
    assert_eq!(
        upload.control().unchecked_ref::<HtmlInputElement>().value(),
        ""
    );
    assert!(status.get_element().hidden());
}

// ---- Fetch ------------------------------------------------------------------

#[wasm_bindgen_test]
async fn test_fetch_text_goes_from_loading_to_ready() {
    let text = fetch_text("data:text/plain,hello%20dappr");
    assert_eq!(text.get(), Loadable::Loading);
    assert_eq!(
        settled(&text).await,
        Loadable::Ready("hello dappr".to_string())
    );
}

#[wasm_bindgen_test]
async fn test_fetch_table_reads_csv_and_json() {
    let csv = fetch_table(csv_url("city,pop\nRome,2.75\nMilan,1.37"));
    let table = settled(&csv).await;
    let table = table.ready().expect("csv should load");
    assert_eq!(table.headers(), vec!["city", "pop"]);
    assert_eq!(table.column("pop").unwrap().max(), Some(2.75));

    // No file name to go by: the Content-Type says it is JSON.
    let json = fetch_table("data:application/json,%5B%7B%22a%22%3A1%7D%2C%7B%22a%22%3A2%7D%5D");
    let table = settled(&json).await;
    assert_eq!(table.ready().expect("json should load").n_rows(), 2);
}

#[wasm_bindgen_test]
async fn test_fetch_reports_missing_files_and_bad_content() {
    let missing = fetch_table("/definitely-not-here-dappr.csv");
    let state = settled(&missing).await;
    let message = state.error().expect("a 404 should fail").to_string();
    assert!(message.contains("404"), "unexpected message: {message}");

    let bad = fetch_table("data:application/json,%7Boops");
    assert!(settled(&bad)
        .await
        .error()
        .expect("bad JSON should fail")
        .contains("invalid JSON"));
}

#[wasm_bindgen_test]
async fn test_fetch_follows_a_url_signal_and_drives_the_page() {
    let dapp = Dapp::new();
    let url = Signal::new(String::new());
    let data = fetch_table(&url);
    let status = dapp.load_status(&data);
    let view = dapp.table(&data);
    let cells = || -> Vec<String> {
        let nodes = view.get_element().query_selector_all("td").unwrap();
        (0..nodes.length())
            .map(|i| nodes.item(i).unwrap().text_content().unwrap())
            .collect()
    };

    // An empty URL asks for nothing.
    assert_eq!(data.get(), Loadable::Idle);
    assert!(status.get_element().hidden());

    url.set(csv_url("n\n1\n2"));
    assert!(data.get().is_loading());
    assert!(!status.get_element().hidden());
    assert_eq!(
        status.get_element().get_attribute("aria-busy").unwrap(),
        "true"
    );
    settled(&data).await;
    assert_eq!(cells(), vec!["1", "2"]);
    assert!(status.get_element().hidden());
    assert_eq!(
        status.get_element().get_attribute("aria-busy").unwrap(),
        "false"
    );

    // Two changes in a row: only the last one's answer is kept.
    url.set(csv_url("n\n10"));
    url.set(csv_url("n\n99"));
    settled(&data).await;
    sleep(100).await;
    assert_eq!(cells(), vec!["99"]);

    url.set("  ".to_string());
    assert_eq!(data.get(), Loadable::Idle);
    assert_eq!(cells(), Vec::<String>::new());
}
