#![allow(dead_code)]
use crate::communication::signals::Signal;
use crate::communication::subscription::Subscription;
use std::fmt::Display;
use wasm_bindgen::JsCast;
use web_sys::{Event, HtmlElement};

/// This is the `Component` trait, which is the base for all subsequent components for this library
pub trait Component {
    /// Returns the inner `web_sys::HtmlElement`
    fn get_element(&self) -> &HtmlElement;

    /// Sets the ID of the element
    fn id(&self, id: &str) {
        self.get_element().set_id(id);
    }

    /// Adds a CSS class to the element
    fn add_class(&self, class_name: &str) {
        self.get_element()
            .class_list()
            .add_1(class_name)
            .unwrap_or_else(|_| panic!("Cannot add the class: {}", class_name));
    }

    /// Removes a CSS class from the element
    fn remove_class(&self, class_name: &str) {
        self.get_element()
            .class_list()
            .remove_1(class_name)
            .unwrap_or_else(|_| panic!("Cannot remove the class: {}", class_name));
    }

    /// Sets an attribute on the element
    fn set_attribute(&self, name: &str, value: &str) {
        self.get_element()
            .set_attribute(name, value)
            .unwrap_or_else(|_| panic!("Cannot set the attribute: {} = {}", name, value));
    }

    /// Gets the value of an attribute from the element
    fn get_attribute(&self, name: &str) -> Option<String> {
        self.get_element().get_attribute(name)
    }

    /// Adds an event listener to the element.
    ///
    /// Returns a [`Subscription`]. Drop it to remove the listener, or call
    /// `.detach()` to keep it attached for the remainder of the program.
    #[must_use = "dropping the returned Subscription immediately removes the listener -- \
                  see Subscription's docs"]
    fn add_event_listener<F>(&self, event_type: &str, callback: F) -> Subscription
    where
        F: 'static + FnMut(Event),
    {
        let closure = wasm_bindgen::closure::Closure::wrap(Box::new(callback) as Box<dyn FnMut(_)>);
        let element = self.get_element().clone();
        let event_type = event_type.to_string();

        element
            .add_event_listener_with_callback(&event_type, closure.as_ref().unchecked_ref())
            .unwrap_or_else(|_| panic!("Cannot add the event listener of type: {}", event_type));
        Subscription::new(move || {
            let _ = element
                .remove_event_listener_with_callback(&event_type, closure.as_ref().unchecked_ref());
        })
    }

    /// Removes this component's element from the DOM entirely.
    ///
    /// This is a hard removal of the underlying node. It does **not**
    /// automatically drop or detach any [`Subscription`]s (event listeners,
    /// signal bindings) that this component or its creator holds -- those
    /// are separate resources with their own lifetimes. If a component owns
    /// `Subscription`s that only make sense while it's on the page (e.g. a
    /// click listener on its own element), drop them at the same time you
    /// unmount, or store them on the component and let its own `Drop`
    /// (if it has one) take care of both together.
    ///
    /// See `examples/lifecycle` for a worked example of building and
    /// removing components, including their listeners, together.
    fn unmount(&self) {
        self.get_element().remove();
    }

    /// Binds a Signal to the component. This can also have a template and be initially rendered or not.
    ///
    /// The subscription this creates is `.detach()`'d internally: a bound
    /// component in this library currently lives for the whole app's
    /// lifetime (there's no "unmount" yet), so this preserves the exact
    /// previous behavior. If/when components gain an explicit unmount, this
    /// is the `Subscription` that unmounting would need to drop instead.
    fn bind_signal<T: Display + Clone + 'static>(
        &self,
        signal: Signal<T>,
        template: Option<&str>,
        initial_render: Option<&str>,
    ) {
        let element = self.get_element().clone();

        if let Some(initial_content) = initial_render {
            self.get_element().set_text_content(Some(initial_content));
        }

        let template_owned = template.map(|s| s.to_string());
        signal
            .subscribe(move |new_value| {
                let content = if let Some(template) = &template_owned {
                    template.replace("{}", &new_value.to_string())
                } else {
                    new_value.to_string()
                };
                element.set_text_content(Some(&content));
            })
            .detach();
    }
}
