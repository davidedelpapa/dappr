use crate::communication::subscription::Subscription;
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

/// Calls `callback` every time `event` fires on `element`, until the
/// returned [`Subscription`] is dropped. Usually you hand that straight to
/// [`bind`], so the listener lasts while the element is on the page.
#[must_use = "dropping the returned Subscription immediately removes the listener"]
pub fn on(
    element: &HtmlElement,
    event: &str,
    callback: impl FnMut(web_sys::Event) + 'static,
) -> Subscription {
    let closure = Closure::<dyn FnMut(web_sys::Event)>::new(callback);
    element
        .add_event_listener_with_callback(event, closure.as_ref().unchecked_ref())
        .unwrap_or_else(|_| panic!("Cannot add the event listener of type: {event}"));
    let (element, event) = (element.clone(), event.to_string());
    // Owns `closure`, which must stay alive for as long as the listener is
    // attached, and is freed with it.
    Subscription::new_owning(move || {
        let _ =
            element.remove_event_listener_with_callback(&event, closure.as_ref().unchecked_ref());
    })
}

// ---- Lifetimes: things that last while an element is on the page ------------

/// One element and what is tied to it.
struct Bound {
    element: web_sys::Element,
    subscriptions: Vec<Subscription>,
    /// Whether the element has been seen on the page. Until it has, not
    /// being on the page only means "not added yet".
    seen_on_page: bool,
}

thread_local! {
    static BOUND: RefCell<Vec<Bound>> = const { RefCell::new(Vec::new()) };
    static WATCHING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Makes `subscription` last for as long as `element` is on the page.
///
/// This is how components stay reactive without leaking. A component's
/// link to a Signal, and its event listeners, are bound to its element:
/// they work while the element is shown and end when it is removed --
/// through [`Component::unmount`](crate::framework::traits::Component::unmount),
/// by a container being cleared, or by any other code taking the element
/// out of the page. Ending them frees what they held, including Signals
/// derived just for that component.
///
/// An element that is taken out and put back within the same turn of the
/// event loop (moved, in other words) keeps its bindings.
pub fn bind(element: &HtmlElement, subscription: Subscription) {
    watch_for_removals();
    BOUND.with(|bound| {
        let mut bound = bound.borrow_mut();
        let element: &web_sys::Element = element;
        match bound.iter_mut().find(|entry| entry.element == *element) {
            Some(entry) => entry.subscriptions.push(subscription),
            None => bound.push(Bound {
                element: element.clone(),
                subscriptions: vec![subscription],
                seen_on_page: element.is_connected(),
            }),
        }
    });
}

/// Ends everything bound to `element` and to the elements inside it, now.
/// Called when a component is unmounted or a container cleared.
pub fn release(element: &web_sys::Element) {
    let released =
        take_bound(|entry| *element == entry.element || element.contains(Some(&entry.element)));
    drop(released);
}

/// How many elements currently have something bound to them. For tests.
pub fn bound_count() -> usize {
    BOUND.with(|bound| bound.borrow().len())
}

/// Removes and returns the entries matching `gone`. They are handed back
/// rather than dropped here, so that ending them -- which runs code -- never
/// happens while the list is borrowed.
fn take_bound(mut gone: impl FnMut(&mut Bound) -> bool) -> Vec<Bound> {
    BOUND.with(|bound| {
        let mut bound = bound.borrow_mut();
        let mut released = Vec::new();
        let mut index = 0;
        while index < bound.len() {
            if gone(&mut bound[index]) {
                released.push(bound.swap_remove(index));
            } else {
                index += 1;
            }
        }
        released
    })
}

/// Ends the bindings of every element that was on the page and no longer is.
fn sweep() {
    let released = take_bound(|entry| {
        if entry.element.is_connected() {
            entry.seen_on_page = true;
            false
        } else {
            entry.seen_on_page
        }
    });
    drop(released);
}

/// Starts (once) watching the document for elements being added or
/// removed, so bindings follow what is actually on the page whoever
/// changes it.
fn watch_for_removals() {
    if WATCHING.with(|watching| watching.replace(true)) {
        return;
    }
    // The browser calls this after the change, once the code that made it
    // has finished -- so an element merely being moved is back in place.
    let on_change = Closure::<dyn FnMut()>::new(sweep);
    let Ok(observer) = web_sys::MutationObserver::new(on_change.as_ref().unchecked_ref()) else {
        return;
    };
    let options = web_sys::MutationObserverInit::new();
    options.set_child_list(true);
    options.set_subtree(true);
    if let Some(root) = document().document_element() {
        let _ = observer.observe_with_options(&root, &options);
    }
    // One observer for the whole life of the page.
    on_change.forget();
    std::mem::forget(observer);
}
