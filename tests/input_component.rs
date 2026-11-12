#![allow(dead_code)]
use dappr::prelude::*;
use wasm_bindgen_test::*;
use web_sys::{window, Document};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_input_insertion() {
    let dapp = Dapp::new();
    dapp.input("Test Input");

    let document: Document = window().unwrap().document().unwrap();
    let body = document.body().unwrap();
    let content = body.inner_html();

    assert!(content.contains("<input type=\"text\" placeholder=\"Test Input\">"));
}
