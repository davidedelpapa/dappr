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
        // `new_owning`: this action owns `closure`, which must stay alive for
        // as long as the listener is attached -- including after `.detach()`.
        // (To tie a listener to the component's time on the page instead,
        // pass this Subscription to `dom::bind`.)
        Subscription::new_owning(move || {
            let _ = element
                .remove_event_listener_with_callback(&event_type, closure.as_ref().unchecked_ref());
        })
    }

    /// Removes this component from the page, and ends what was tied to it.
    ///
    /// The element is taken out of the DOM, and everything bound to it or
    /// to elements inside it stops at once: links to Signals, event
    /// listeners, and Signals derived only for them. For a container, that
    /// covers everything in it.
    ///
    /// A [`Subscription`] you are holding yourself -- one returned by
    /// [`add_event_listener`](Component::add_event_listener), say -- is
    /// yours: it ends when you drop it.
    ///
    /// See `examples/lifecycle` for components being created and removed
    /// while the app runs.
    fn unmount(&self) {
        let element = self.get_element();
        element.remove();
        crate::framework::dom::release(element);
    }

    /// Binds a Signal to the component. This can also have a template and be initially rendered or not.
    ///
    /// The link lasts while the component's element is on the page: once
    /// it is removed, the component stops listening, and a Signal derived
    /// just for it is freed.
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
        let subscription = signal.subscribe(move |new_value| {
            let content = if let Some(template) = &template_owned {
                template.replace("{}", &new_value.to_string())
            } else {
                new_value.to_string()
            };
            element.set_text_content(Some(&content));
        });
        crate::framework::dom::bind(self.get_element(), subscription);
    }
}

/// Text for a component that can follow a Signal: plain text (`&str`,
/// `String`) for something fixed, or a `Signal<String>` to update whenever
/// it changes.
pub trait TextSource {
    /// The text as a Signal.
    fn into_text_signal(self) -> Signal<String>;
}
impl TextSource for &str {
    fn into_text_signal(self) -> Signal<String> {
        Signal::new(self.to_string())
    }
}
impl TextSource for String {
    fn into_text_signal(self) -> Signal<String> {
        Signal::new(self)
    }
}
impl TextSource for &String {
    fn into_text_signal(self) -> Signal<String> {
        Signal::new(self.clone())
    }
}
impl TextSource for Signal<String> {
    fn into_text_signal(self) -> Signal<String> {
        self
    }
}
impl TextSource for &Signal<String> {
    fn into_text_signal(self) -> Signal<String> {
        self.clone()
    }
}

/// A number for a component that can follow a Signal: a plain number for
/// something fixed, or a `Signal<f64>` to update whenever it changes.
pub trait NumberSource {
    /// The number as a Signal.
    fn into_number_signal(self) -> Signal<f64>;
}
impl NumberSource for f64 {
    fn into_number_signal(self) -> Signal<f64> {
        Signal::new(self)
    }
}
impl NumberSource for i32 {
    fn into_number_signal(self) -> Signal<f64> {
        Signal::new(f64::from(self))
    }
}
impl NumberSource for Signal<f64> {
    fn into_number_signal(self) -> Signal<f64> {
        self
    }
}
impl NumberSource for &Signal<f64> {
    fn into_number_signal(self) -> Signal<f64> {
        self.clone()
    }
}
