//! ## Input component
//! ### Description
//! The `Input` component creates a responsive Input
//!
//! ## SimpleInput component
//! ### Description
//! The `SimpleInput` component creates a simplified form of Input suitable to use in `Dapp`
//!
use crate::communication::signals::Signal;
use crate::framework::dom;
use crate::framework::traits::Component;
use wasm_bindgen::prelude::*;
use web_sys::HtmlElement;

pub struct Input {
    el: HtmlElement,
}

impl Input {
    pub fn new(signal: Signal<String>) -> Self {
        let input_element = dom::create_element("input");
        input_element
            .set_attribute("type", "text")
            .expect("Cannot set the type text attribute on the input element");

        let input = Input {
            el: input_element.clone(),
        };
        let signal_cloned = signal.clone();

        // Update signal when user types, for as long as the input is on
        // the page.
        crate::framework::dom::bind(
            &input.el,
            input.add_event_listener("input", move |event| {
                let input_el = event
                    .target()
                    .expect("Cannot get the event target")
                    .unchecked_into::<HtmlElement>();
                let value = input_el
                    .dyn_ref::<web_sys::HtmlInputElement>()
                    .expect("Cannot get the input element")
                    .value();
                signal.set(value);
            }),
        );

        // Sync input value with signal
        input.bind_signal(signal_cloned, None, Some(""));
        input
    }

    pub fn set_placeholder(&self, placeholder: &str) {
        self.get_element()
            .set_attribute("placeholder", placeholder)
            .expect("Unable to set input placeholder");
    }
}

impl Component for Input {
    fn get_element(&self) -> &HtmlElement {
        &self.el
    }
}

pub struct SimpleInput {
    signal: Signal<String>,
    el: HtmlElement,
}

impl SimpleInput {
    pub fn new(placeholder: &str) -> Self {
        let signal = Signal::new("".to_string());
        let input = Input::new(signal.clone());
        input.set_placeholder(placeholder);
        SimpleInput {
            signal,
            el: input.get_element().clone(),
        }
    }
    pub fn text(&self) -> Signal<String> {
        self.signal.clone()
    }
}

impl Component for SimpleInput {
    fn get_element(&self) -> &HtmlElement {
        &self.el
    }
}
