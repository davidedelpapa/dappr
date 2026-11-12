#![allow(dead_code)]
use dappr::prelude::*;
use wasm_bindgen_test::*;
use web_sys::{window, Document, Element};

wasm_bindgen_test_configure!(run_in_browser);

fn document() -> Document {
    window().unwrap().document().unwrap()
}

fn computed(el: &Element, property: &str) -> String {
    window()
        .unwrap()
        .get_computed_style(el)
        .unwrap()
        .unwrap()
        .get_property_value(property)
        .unwrap()
}

#[wasm_bindgen_test]
fn test_container_holds_what_is_added_to_it() {
    let dapp = Dapp::new();
    let container = dapp.container();
    container.id("c1");
    dapp.p_static("after the container");
    container.p_static("inside the container");

    let el = document().get_element_by_id("c1").unwrap();
    assert_eq!(el.tag_name(), "DIV");
    assert_eq!(el.inner_html(), "<p>inside the container</p>");
}

#[wasm_bindgen_test]
fn test_section_card_and_expander_use_semantic_elements() {
    let dapp = Dapp::new();
    assert_eq!(dapp.section().get_element().tag_name(), "SECTION");
    assert_eq!(dapp.card().get_element().tag_name(), "ARTICLE");

    let titled = dapp.card_titled("Title");
    titled.p_static("body");
    assert_eq!(
        titled.get_element().inner_html(),
        "<header>Title</header><p>body</p>"
    );

    let closed = dapp.expander("More");
    closed.p_static("hidden");
    assert_eq!(closed.get_element().tag_name(), "DETAILS");
    assert_eq!(
        closed.get_element().inner_html(),
        "<summary>More</summary><p>hidden</p>"
    );
    assert!(closed.get_attribute("open").is_none());
    assert!(dapp.expander_open("More").get_attribute("open").is_some());
}

#[wasm_bindgen_test]
fn test_columns_count_comes_from_the_pattern() {
    let dapp = Dapp::new();
    let [a, b, c] = dapp.columns();
    a.p_static("A");
    b.p_static("B");
    c.p_static("C");

    let row = a.get_element().parent_element().unwrap();
    assert_eq!(row.child_element_count(), 3);
    assert_eq!(row.class_name(), "dappr-columns");
    assert_eq!(computed(&row, "display"), "grid");
    assert_eq!(row.text_content().unwrap(), "ABC");
}

/// The test page is wider than the 768px breakpoint, so the columns are
/// side by side and their widths follow the weights.
#[wasm_bindgen_test]
fn test_weighted_columns_have_proportional_widths() {
    let dapp = Dapp::new();
    let [wide, narrow] = dapp.columns_weighted([3, 1]);
    wide.p_static("wide");
    narrow.p_static("narrow");

    let (w, n) = (
        wide.get_element().get_bounding_client_rect(),
        narrow.get_element().get_bounding_client_rect(),
    );
    assert_eq!(w.top(), n.top(), "columns should sit on the same row");
    let ratio = w.width() / n.width();
    assert!((ratio - 3.0).abs() < 0.05, "expected 3:1, got {ratio}");
}

#[wasm_bindgen_test]
fn test_containers_nest() {
    let dapp = Dapp::new();
    let [left, _right] = dapp.columns();
    let card = left.card();
    let [inner_a, _inner_b] = card.columns();
    inner_a.button("deep");

    let button = inner_a.get_element().first_element_child().unwrap();
    assert_eq!(button.tag_name(), "BUTTON");
    assert!(left.get_element().contains(Some(&button)));
    assert!(card.get_element().contains(Some(&button)));
}

/// Inside `with`, `text!` and `dapp!` target the container; afterwards they
/// target the page again. Nested `with` blocks unwind in order.
#[wasm_bindgen_test]
fn test_with_redirects_text_macro_and_restores() {
    dapp!(dapp, {
        let outer = dapp.container();
        let inner = outer.container();
        outer.id("with-outer");
        inner.id("with-inner");

        outer.with(|_| {
            text!("outer-1");
            inner.with(|inner| {
                text!("inner-1");
                inner.p_static("inner-2");
            });
            dapp!(current, {
                current.p_static("outer-2");
            });
        });
        text!("page-level-after-with");
    });

    let inner = document().get_element_by_id("with-inner").unwrap();
    let outer = document().get_element_by_id("with-outer").unwrap();
    assert_eq!(inner.text_content().unwrap(), "inner-1inner-2");
    assert_eq!(
        outer.text_content().unwrap(),
        "inner-1inner-2outer-1outer-2"
    );
    assert!(!outer.text_content().unwrap().contains("page-level"));
    assert!(document()
        .get_element_by_id("dapp")
        .unwrap()
        .text_content()
        .unwrap()
        .contains("page-level-after-with"));
}

/// `with` returns whatever its body returns.
#[wasm_bindgen_test]
fn test_with_returns_the_body_value() {
    let dapp = Dapp::new();
    let input = dapp.card().with(|card| card.input("inside"));
    assert_eq!(
        input.get_element().parent_element().unwrap().tag_name(),
        "ARTICLE"
    );
}

#[wasm_bindgen_test]
fn test_sidebar_is_single_and_sits_before_the_app() {
    let dapp = Dapp::new();
    let first = dapp.sidebar();
    first.p_static("from the page");
    // Asking again, even from inside a container, gives the same sidebar.
    dapp.card().sidebar().p_static("from a card");

    let sidebar = document().get_element_by_id("dappr-sidebar").unwrap();
    assert_eq!(sidebar.tag_name(), "ASIDE");
    assert_eq!(sidebar.text_content().unwrap(), "from the pagefrom a card");
    let app = document().get_element_by_id("dapp").unwrap();
    assert_eq!(sidebar.next_element_sibling().unwrap(), app);
    assert!(app
        .parent_element()
        .unwrap()
        .class_list()
        .contains("dappr-has-sidebar"));
}

/// A container can be removed with everything in it.
#[wasm_bindgen_test]
fn test_container_unmount_removes_its_content() {
    let dapp = Dapp::new();
    let card = dapp.card();
    card.id("doomed");
    card.p_static("gone soon");
    card.unmount();
    assert!(document().get_element_by_id("doomed").is_none());
}
