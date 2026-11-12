#![allow(dead_code)]
use dappr::prelude::*;
use wasm_bindgen_test::*;
use web_sys::{window, Document};

wasm_bindgen_test_configure!(run_in_browser);

fn body_html() -> String {
    let document: Document = window().unwrap().document().unwrap();
    document.body().unwrap().inner_html()
}

// ---- Static heading elements --------------------------------------------
// One test per tag to guard against the h1_static/h2_static/... methods
// ever drifting apart (e.g. a copy-paste that forgets to change the tag).

#[wasm_bindgen_test]
fn test_h1_static_wraps_h1_tag() {
    let dapp = Dapp::new();
    dapp.h1_static("Heading 1");
    assert!(body_html().contains("<h1>Heading 1</h1>"));
}

#[wasm_bindgen_test]
fn test_h2_static_wraps_h2_tag() {
    let dapp = Dapp::new();
    dapp.h2_static("Heading 2");
    assert!(body_html().contains("<h2>Heading 2</h2>"));
}

#[wasm_bindgen_test]
fn test_h3_static_wraps_h3_tag() {
    let dapp = Dapp::new();
    dapp.h3_static("Heading 3");
    assert!(body_html().contains("<h3>Heading 3</h3>"));
}

#[wasm_bindgen_test]
fn test_h4_static_wraps_h4_tag() {
    let dapp = Dapp::new();
    dapp.h4_static("Heading 4");
    assert!(body_html().contains("<h4>Heading 4</h4>"));
}

#[wasm_bindgen_test]
fn test_h5_static_wraps_h5_tag() {
    let dapp = Dapp::new();
    dapp.h5_static("Heading 5");
    assert!(body_html().contains("<h5>Heading 5</h5>"));
}

#[wasm_bindgen_test]
fn test_p_static_wraps_p_tag() {
    let dapp = Dapp::new();
    dapp.p_static("Paragraph");
    assert!(body_html().contains("<p>Paragraph</p>"));
}

#[wasm_bindgen_test]
fn test_div_static_wraps_div_tag() {
    let dapp = Dapp::new();
    dapp.div_static("Div text");
    assert!(body_html().contains("<div>Div text</div>"));
}

// ---- Static semantic aliases ---------------------------------------------
// These don't have their own HTML tag; they're documented aliases for one
// of the tags above (e.g. `title_static` -> `<h1>`). The tests pin down
// that mapping so a future refactor of the macro table can't silently
// change which tag a semantic alias renders as.

#[wasm_bindgen_test]
fn test_title_static_is_semantic_alias_for_h1() {
    let dapp = Dapp::new();
    dapp.title_static("Title");
    assert!(body_html().contains("<h1>Title</h1>"));
}

#[wasm_bindgen_test]
fn test_subtitle_static_is_semantic_alias_for_div() {
    let dapp = Dapp::new();
    dapp.subtitle_static("Subtitle");
    assert!(body_html().contains("<div>Subtitle</div>"));
}

#[wasm_bindgen_test]
fn test_part_static_is_semantic_alias_for_h2() {
    let dapp = Dapp::new();
    dapp.part_static("Part");
    assert!(body_html().contains("<h2>Part</h2>"));
}

#[wasm_bindgen_test]
fn test_chapter_static_is_semantic_alias_for_h3() {
    let dapp = Dapp::new();
    dapp.chapter_static("Chapter");
    assert!(body_html().contains("<h3>Chapter</h3>"));
}

#[wasm_bindgen_test]
fn test_paragraph_static_is_semantic_alias_for_h4() {
    let dapp = Dapp::new();
    dapp.paragraph_static("Paragraph alias");
    assert!(body_html().contains("<h4>Paragraph alias</h4>"));
}

#[wasm_bindgen_test]
fn test_subparagraph_static_is_semantic_alias_for_h5() {
    let dapp = Dapp::new();
    dapp.subparagraph_static("Subparagraph");
    assert!(body_html().contains("<h5>Subparagraph</h5>"));
}

#[wasm_bindgen_test]
fn test_date_static_is_semantic_alias_for_div() {
    let dapp = Dapp::new();
    dapp.date_static("2026-08-22");
    assert!(body_html().contains("<div>2026-08-22</div>"));
}

// ---- Reactive heading elements -------------------------------------------
// Mirrors the static tests above, but for the Signal-backed methods, using
// an initial value so the assertion doesn't depend on a signal dispatch.

#[wasm_bindgen_test]
fn test_h1_reactive_wraps_h1_tag() {
    let dapp = Dapp::new();
    let signal = Signal::new(1);
    dapp.h1("count: {}", signal, Some("Heading 1 init"));
    assert!(body_html().contains("<h1>Heading 1 init</h1>"));
}

#[wasm_bindgen_test]
fn test_div_reactive_wraps_div_tag() {
    let dapp = Dapp::new();
    let signal = Signal::new(1);
    dapp.div("count: {}", signal, Some("Div init"));
    assert!(body_html().contains("<div>Div init</div>"));
}

// ---- Reactive semantic aliases --------------------------------------------

#[wasm_bindgen_test]
fn test_title_reactive_is_semantic_alias_for_h1() {
    let dapp = Dapp::new();
    let signal = Signal::new(1);
    dapp.title("count: {}", signal, Some("Reactive title"));
    assert!(body_html().contains("<h1>Reactive title</h1>"));
}

#[wasm_bindgen_test]
fn test_subtitle_reactive_is_semantic_alias_for_div() {
    let dapp = Dapp::new();
    let signal = Signal::new(1);
    dapp.subtitle("count: {}", signal, Some("Reactive subtitle"));
    assert!(body_html().contains("<div>Reactive subtitle</div>"));
}

#[wasm_bindgen_test]
fn test_part_reactive_is_semantic_alias_for_h2() {
    let dapp = Dapp::new();
    let signal = Signal::new(1);
    dapp.part("count: {}", signal, Some("Reactive part"));
    assert!(body_html().contains("<h2>Reactive part</h2>"));
}

#[wasm_bindgen_test]
fn test_chapter_reactive_is_semantic_alias_for_h3() {
    let dapp = Dapp::new();
    let signal = Signal::new(1);
    dapp.chapter("count: {}", signal, Some("Reactive chapter"));
    assert!(body_html().contains("<h3>Reactive chapter</h3>"));
}

#[wasm_bindgen_test]
fn test_paragraph_reactive_is_semantic_alias_for_h4() {
    let dapp = Dapp::new();
    let signal = Signal::new(1);
    dapp.paragraph("count: {}", signal, Some("Reactive paragraph"));
    assert!(body_html().contains("<h4>Reactive paragraph</h4>"));
}

#[wasm_bindgen_test]
fn test_subparagraph_reactive_is_semantic_alias_for_h5() {
    let dapp = Dapp::new();
    let signal = Signal::new(1);
    dapp.subparagraph("count: {}", signal, Some("Reactive subparagraph"));
    assert!(body_html().contains("<h5>Reactive subparagraph</h5>"));
}

#[wasm_bindgen_test]
fn test_date_reactive_is_semantic_alias_for_div() {
    let dapp = Dapp::new();
    let signal = Signal::new(1);
    dapp.date("count: {}", signal, Some("2026-08-22"));
    assert!(body_html().contains("<div>2026-08-22</div>"));
}

// ---- append() plumbing: order + button/input still work alongside text --

#[wasm_bindgen_test]
fn test_elements_append_in_call_order() {
    let dapp = Dapp::new();
    dapp.h1_static("First");
    dapp.p_static("Second");

    let html = body_html();
    let first_pos = html.find("First").expect("First should be present");
    let second_pos = html.find("Second").expect("Second should be present");
    assert!(
        first_pos < second_pos,
        "elements should appear in the order Dapp methods were called"
    );
}
