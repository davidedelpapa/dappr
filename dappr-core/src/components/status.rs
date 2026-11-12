//! ## LoadStatus: what is happening with data being loaded
use crate::communication::signals::Signal;
use crate::data::load::Loadable;
use crate::framework::dom;
use crate::framework::traits::Component;
use web_sys::HtmlElement;

/// A line that tells the user about a [`Loadable`]: a spinner and
/// "Loading..." while it loads, the reason if it failed, and nothing at all
/// (it is hidden) otherwise.
pub struct LoadStatus {
    el: HtmlElement,
}

impl LoadStatus {
    /// A status line following `source`.
    pub fn new<T: Clone + 'static>(source: &Signal<Loadable<T>>) -> Self {
        let el = dom::create_element("p");
        el.set_class_name("dappr-load-status");
        // Screen readers announce changes to this line.
        el.set_attribute("role", "status")
            .expect("Failed to build the load status");

        show(&el, &source.get());
        let el_in = el.clone();
        source.subscribe(move |state| show(&el_in, state)).detach();
        LoadStatus { el }
    }
}

impl Component for LoadStatus {
    fn get_element(&self) -> &HtmlElement {
        &self.el
    }
}

fn show<T>(el: &HtmlElement, state: &Loadable<T>) {
    let (text, busy, failed) = match state {
        Loadable::Loading => (Some("Loading\u{2026}".to_string()), true, false),
        Loadable::Failed(message) => (
            Some(format!("Could not load the data: {message}")),
            false,
            true,
        ),
        Loadable::Idle | Loadable::Ready(_) => (None, false, false),
    };
    el.set_hidden(text.is_none());
    el.set_text_content(text.as_deref());
    // Pico draws a spinner on anything with aria-busy="true".
    let _ = el.set_attribute("aria-busy", if busy { "true" } else { "false" });
    let _ = el
        .class_list()
        .toggle_with_force("dappr-load-failed", failed);
}
