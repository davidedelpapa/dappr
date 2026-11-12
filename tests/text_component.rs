#![allow(dead_code)]
use dappr::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;
use web_sys::{window, Document};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_static_text_insertion() {
    let dapp = Dapp::new();
    dapp.text_static("Test Static Text");

    let document: Document = window().unwrap().document().unwrap();
    let body = document.body().unwrap();
    let content = body.inner_html();

    assert!(content.contains("<p>Test Static Text</p>"));
}

#[wasm_bindgen_test]
fn test_text_template_initial() {
    let signal = Signal::new(42);
    let dapp = Dapp::new();
    dapp.text(
        "Test Template Text: {}",
        signal.clone(),
        Some("Initial Text"),
    );

    let document: Document = window().unwrap().document().unwrap();
    let body = document.body().unwrap();
    let content = body.inner_html();

    assert!(content.contains("<p>Initial Text</p>"));
}

#[wasm_bindgen_test]
fn test_text_template_reactivity() {
    let signal = Signal::new(42);
    let dapp = Dapp::new();
    dapp.text("Test Template Text: {}", signal.clone(), Some(""));
    signal.update_after_delay(100, 1500);

    // Check the updates to the signal after a delay
    wasm_bindgen_futures::spawn_local(async move {
        let closure = Closure::wrap(Box::new(move || {
            let document: Document = window().unwrap().document().unwrap();
            let body = document.body().unwrap();
            let content = body.inner_html();

            assert!(content.contains("<p>Test Template Text: 100</p>"));
        }) as Box<dyn FnMut()>);

        // Set the timeout to invoke the closure
        window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                closure.as_ref().unchecked_ref(),
                2000, // 2000 milliseconds = 2 seconds
            )
            .expect("Failed to set timeout");

        // Prevent the Closure from being dropped prematurely
        closure.forget();
    });
}
