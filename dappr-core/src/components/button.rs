use crate::communication::dispatcher::Dispatcher;
use crate::communication::signals::Signal;
use crate::framework::dom;
use crate::framework::traits::Component;
use web_sys::HtmlElement;

/// The `Button` component creates a responsive Button
///
/// A button can be used in two ways: with a callback to execute when pressed,
/// and in addition to get a signal that can be used to trigger other components.
///
/// The Signal wraps a bool that is set to true when the button is pressed.
///
/// # Example
/// ```ignore
/// use dappr::prelude::*;
/// use wasm_bindgen::prelude::*;
///
/// #[wasm_bindgen(main)]
/// pub fn main() {
///     dapp!(dapp, {
///         dapp.title_static("Dappr buttons");
///         let button = dapp.button("Press me!");
///         button.pressed(move || {
///             text!("Button pressed"); // inside the clossure triggered by the button
///         });
///         text!(
///             "Button has been pressed",
///             button.get_signal(), // Signal to trigger the reactive text
///             "button is not yet pressed"
///         );
///     });
/// }
/// ```
pub struct Button {
    el: HtmlElement,
    dsp: Dispatcher,
    signal: Signal<bool>,
}
impl Button {
    /// Button constructor
    pub fn new(label: &str) -> Self {
        let dispatcher = Dispatcher::new();
        let signal = Signal::new(false);
        let button_element = dom::create_element("button");
        button_element.set_text_content(Some(label));

        let button = Button {
            el: button_element,
            dsp: dispatcher,
            signal,
        };

        let dispatcher_clone = button.dsp.clone();
        let signal_clone = button.signal.clone();
        // The listener lasts while the button is on the page.
        crate::framework::dom::bind(
            &button.el,
            button.add_event_listener("click", move |_event| {
                dispatcher_clone.dispatch();
                signal_clone.set(true);
            }),
        );

        button
    }
    /// Button pressed callback.
    ///
    /// The callback stays subscribed for the button's whole lifetime, and is
    /// freed with the button once it has been removed from the page. It may
    /// freely create components or call `pressed` again on the same button.
    pub fn pressed(&self, callback: impl Fn() + 'static) {
        self.dsp.subscribe(callback).detach();
    }
    /// Get the signal when the button is pressed
    pub fn get_signal(&self) -> Signal<bool> {
        self.signal.clone()
    }
}

impl Component for Button {
    fn get_element(&self) -> &HtmlElement {
        &self.el
    }
}
