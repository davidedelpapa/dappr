#![allow(dead_code)]
use dappr::prelude::*;
use wasm_bindgen_test::*;
use web_sys::Element;

wasm_bindgen_test_configure!(run_in_browser);

fn people() -> DTable {
    let mut table = DTable::with_columns(&["name", "age", "member"]);
    table.push_row(("Ada", 36, true));
    table.push_row(("Alan", 41, false));
    table.push_row(("Grace", None::<f64>, true));
    table
}

fn texts(root: &Element, selector: &str) -> Vec<String> {
    let nodes = root.query_selector_all(selector).unwrap();
    (0..nodes.length())
        .map(|i| nodes.item(i).unwrap().text_content().unwrap())
        .collect()
}

#[wasm_bindgen_test]
fn test_table_renders_headers_and_rows() {
    let dapp = Dapp::new();
    let table = dapp.table(&people());
    let el = table.get_element();

    assert_eq!(texts(el, "thead th"), vec!["name", "age", "member"]);
    assert_eq!(texts(el, "tbody tr").len(), 3);
    assert_eq!(
        texts(el, "tbody tr:first-child td"),
        vec!["Ada", "36", "true"]
    );
    // A missing value is an empty cell.
    assert_eq!(
        texts(el, "tbody tr:last-child td"),
        vec!["Grace", "", "true"]
    );
    assert!(el.query_selector("caption").unwrap().is_none());
    assert!(el.query_selector(".dappr-table-note").unwrap().is_none());
}

#[wasm_bindgen_test]
fn test_numbers_are_marked_for_right_alignment() {
    let dapp = Dapp::new();
    let table = dapp.table(people());
    let el = table.get_element();

    // The numeric column's header, and its number cells (not the empty one).
    assert_eq!(texts(el, "th.dappr-number"), vec!["age"]);
    assert_eq!(texts(el, "td.dappr-number"), vec!["36", "41"]);
    let cell = el.query_selector("td.dappr-number").unwrap().unwrap();
    let align = web_sys::window()
        .unwrap()
        .get_computed_style(&cell)
        .unwrap()
        .unwrap()
        .get_property_value("text-align")
        .unwrap();
    assert_eq!(align, "right");
}

/// Cell text is data, never markup.
#[wasm_bindgen_test]
fn test_cell_text_is_escaped() {
    let dapp = Dapp::new();
    let mut data = DTable::with_columns(&["<i>header</i>"]);
    data.push_row(["<img src=x onerror=alert(1)><b>bold</b>"]);
    let table = dapp.table(data);
    let el = table.get_element();

    assert!(el.query_selector("img, b, i").unwrap().is_none());
    assert_eq!(
        texts(el, "td"),
        vec!["<img src=x onerror=alert(1)><b>bold</b>"]
    );
    assert_eq!(texts(el, "th"), vec!["<i>header</i>"]);
}

#[wasm_bindgen_test]
fn test_title_becomes_the_caption() {
    let dapp = Dapp::new();
    let table = dapp.table(people().titled("People"));
    assert_eq!(texts(table.get_element(), "caption"), vec!["People"]);
}

#[wasm_bindgen_test]
fn test_floating_point_noise_is_not_shown() {
    let dapp = Dapp::new();
    let mut data = DTable::with_columns(&["value"]);
    data.push_row([12.0 * 34.9]); // 418.79999999999995
    data.push_row([0.1 + 0.2]);
    data.push_row([1.0 / 3.0]);
    let table = dapp.table(&data);
    assert_eq!(
        texts(table.get_element(), "td"),
        vec!["418.8", "0.3", "0.3333333333"]
    );
    // The data keeps its exact value.
    assert_eq!(data.get(0, "value"), Some(&Cell::Number(12.0 * 34.9)));
}

#[wasm_bindgen_test]
fn test_max_rows_caps_the_view_and_says_so() {
    let dapp = Dapp::new();
    let mut data = DTable::with_columns(&["n"]);
    for n in 0..25 {
        data.push_row([n]);
    }
    let table = dapp.table(&data).max_rows(10);
    let el = table.get_element();
    assert_eq!(texts(el, "tbody tr").len(), 10);
    assert_eq!(
        texts(el, ".dappr-table-note"),
        vec!["Showing the first 10 of 25 rows"]
    );

    // The cap still applies when the data changes.
    table.set(data.head(12));
    assert_eq!(texts(el, "tbody tr").len(), 10);
    assert_eq!(
        texts(el, ".dappr-table-note"),
        vec!["Showing the first 10 of 12 rows"]
    );
    table.set(data.head(3));
    assert_eq!(texts(el, "tbody tr").len(), 3);
    assert!(el.query_selector(".dappr-table-note").unwrap().is_none());
}

/// A table given a Signal redraws when inputs change the data.
#[wasm_bindgen_test]
fn test_reactive_table_follows_inputs() {
    let dapp = Dapp::new();
    let data = people();
    let members_only = dapp.checkbox("Members only", false);
    let sort_desc = dapp.switch("Oldest first", false);

    let (members, desc) = (members_only.value(), sort_desc.value());
    let shown = computed!(members, desc => {
        data.filter(|row| !members || row.bool("member") == Some(true))
            .sort_by("age", desc)
    });
    let table = dapp.table(&shown);
    let names = || texts(table.get_element(), "tbody td:first-child");

    assert_eq!(names(), vec!["Grace", "Ada", "Alan"]);
    sort_desc.set(true);
    assert_eq!(names(), vec!["Alan", "Ada", "Grace"]);
    members_only.set(true);
    assert_eq!(names(), vec!["Ada", "Grace"]);
    assert_eq!(shown.get().n_rows(), 2);
}

/// CSV and JSON loading work in the browser too, and feed a table.
#[wasm_bindgen_test]
fn test_loaded_data_can_be_shown_in_a_container() {
    let dapp = Dapp::new();
    let card = dapp.card();
    let from_csv = DTable::from_csv("city,pop\nRome,2.75\nMilan,1.37").unwrap();
    let from_json = DTable::from_json(r#"[{"city": "Paris", "pop": 2.1}]"#).unwrap();
    let csv_table = card.table(from_csv);
    let json_table = card.table(from_json);

    assert_eq!(
        csv_table.get_element().parent_element().unwrap().tag_name(),
        "ARTICLE"
    );
    assert_eq!(
        texts(csv_table.get_element(), "td"),
        vec!["Rome", "2.75", "Milan", "1.37"]
    );
    assert_eq!(texts(json_table.get_element(), "td"), vec!["Paris", "2.1"]);
}
