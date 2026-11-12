use wasm_bindgen::prelude::*;
use web_sys::{Document, HtmlElement, Window};

/// Get the global window object
pub fn window() -> Window {
    web_sys::window().expect("No global window exists")
}

/// Get the document object from the window
pub fn document() -> Document {
    window()
        .document()
        .expect("Should have a document on the window")
}

/// Get the body element from the document
#[allow(dead_code)]
pub fn body() -> HtmlElement {
    document().body().expect("Should have a body tag")
}

/// Create a new element with the given tag
pub fn create_element(tag: &str) -> HtmlElement {
    document()
        .create_element(tag)
        .expect("Failed to create element")
        .dyn_into::<HtmlElement>()
        .expect("Failed to convert to HtmlElement")
}

/// Append the given element to the body
pub fn append_to_body(element: &HtmlElement) {
    document()
        .body()
        .expect("Document should have a body")
        .append_child(element)
        .expect("Failed to append to body");
}
