use std::cell::RefCell;
use std::collections::HashSet;
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

/// The `id` of the `<style>` element holding Dappr's baseline stylesheet.
pub const BASE_STYLE_ID: &str = "dappr-base-style";

thread_local! {
    /// Stylesheets already added through [`inject_style`], so that calling it
    /// again with the same CSS (e.g. from a function that builds a component
    /// many times) adds nothing.
    static INJECTED_STYLES: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

fn head() -> web_sys::HtmlHeadElement {
    document().head().expect("Document should have a head")
}

/// Puts the baseline stylesheet in a `<style>` element at the very start of
/// `<head>`, once per document.
///
/// Being first means anything else on the page -- a stylesheet linked from
/// `index.html`, or CSS added with [`inject_style`] -- comes later in the
/// cascade and can override it.
pub fn inject_base_style(css: &str) {
    if document().get_element_by_id(BASE_STYLE_ID).is_some() {
        return;
    }
    let style = create_element("style");
    style.set_id(BASE_STYLE_ID);
    style.set_text_content(Some(css));
    let head = head();
    head.insert_before(&style, head.first_child().as_ref())
        .expect("Failed to add the base style to head");
}

/// Appends `css` to `<head>` in its own `<style>` element. Adding the same
/// CSS a second time does nothing.
pub fn inject_style(css: &str) {
    let is_new = INJECTED_STYLES.with(|seen| seen.borrow_mut().insert(css.to_string()));
    if !is_new {
        return;
    }
    let style = create_element("style");
    style.set_text_content(Some(css));
    head()
        .append_child(&style)
        .expect("Failed to add a style to head");
}
