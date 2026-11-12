use dappr_core::communication::signals::Signal;
use dappr_core::components::button::Button;
#[cfg(any(feature = "sanitizehtml", doc, test))]
use dappr_core::components::html::Sanitize;
use dappr_core::components::html::{RawHtml, RawHtmlStatic};
use dappr_core::components::input::SimpleInput;
use dappr_core::components::text::{Text, TextStatic};
use dappr_core::framework::dom::{append_to_body, create_element, document};
use dappr_core::framework::traits::Component;
//use dappr_macros::style;
use std::cell::RefCell;
use std::fmt::Display;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::HtmlElement;

/// Generates a `Dapp` method that appends a **static** text element to the page.
///
/// Every static text/heading method (`h1_static`, `title_static`, `p_static`, ...)
/// shares the exact same body: build a [`TextStatic`] of some underlying tag,
/// append it to the root, and return it. Only the method name, the underlying
/// tag, and the doc comment differ between them, so we generate them from a single
///  template.
///
/// Usage:
/// ```ignore
/// static_text_el! {
///     /// Doc comment(s) go here, same as on a normal fn.
///     fn h1_static => TextStatic::h1
/// }
/// ```
macro_rules! static_text_el {
    ($(#[$doc:meta])* fn $name:ident => TextStatic::$tag:ident) => {
        $(#[$doc])*
        pub fn $name(&self, text: &str) -> TextStatic {
            let el = TextStatic::$tag(text);
            self.append(el.get_element());
            el
        }
    };
}

/// Generates a `Dapp` method that appends a **reactive** text element to the page.
///
/// Mirrors [`static_text_el!`] for the reactive (`Signal`-backed) variants
/// (`h1`, `title`, `p`, ...), which all share the same
/// `(template, signal, init)` signature and only differ in which underlying
/// [`Text`] constructor they call.
macro_rules! reactive_text_el {
    ($(#[$doc:meta])* fn $name:ident => Text::$tag:ident) => {
        $(#[$doc])*
        pub fn $name<T: Display + Clone + 'static>(
            &self,
            template: &str,
            signal: Signal<T>,
            init: Option<&str>,
        ) -> Text {
            let el = Text::$tag(template, signal, init);
            self.append(el.get_element());
            el
        }
    };
}

/// Dapp is the main hub to create data apps in a structured way.
///
/// For a more ergonomic usage, see the following macros:
/// - [`dapp!`](crate::dapp) macro: Creates an environment for your app.
/// - [`text!`](crate::text) macro: displays text on the page.
///
/// # Example
/// ```no_run
/// use dappr::prelude::*;
/// use wasm_bindgen::prelude::*;
///
/// #[wasm_bindgen(main)]
/// pub fn main() {
///     dapp!(dapp, {
///         text!("Hello Dappr!", _title);
///         text!("Data apps in Rust made easy.", _subtitle);
///         let input = dapp.input("What's your name?");
///         text!("Hello, {}!", input.text());
///         let button = dapp.button("Press me!");
///         button.pressed(move || {
///             text!("Button pressed");
///         });
///     });
/// }
/// ```
pub struct Dapp {
    root: HtmlElement,
}

impl Dapp {
    /// [`Dapp`] main constructor.
    ///
    /// To use the Dapp without [`dapp!`](crate::dapp) macro, you need to first build your own Dapp.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dappr::prelude::*;
    /// use wasm_bindgen::prelude::*;
    ///
    /// #[wasm_bindgen(main)]
    /// fn main() {
    ///     let dapp = Dapp::new();
    ///     dapp.title_static("Hello, Dapp!");
    /// }
    /// ```
    pub fn new() -> Self {
        let root = match document().get_element_by_id("dapp") {
            Some(dapp_el) => dapp_el
                .dyn_into::<HtmlElement>()
                .expect("Failed to convert dapp root element to HtmlElement"),
            None => {
                let root = create_element("div");
                root.set_id("dapp");
                append_to_body(&root);
                root
            }
        };
        Self { root }
    }

    fn append(&self, el: &HtmlElement) {
        self.root
            .append_child(el)
            .expect("Failed to append to dapp's root");
    }

    /// Returns an input field
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dappr::prelude::*;
    /// use wasm_bindgen::prelude::*;
    ///
    /// #[wasm_bindgen(main)]
    /// fn main() {
    ///     let dapp = Dapp::new();
    ///     let input = dapp.input("What is your name?");
    ///     text!("Hello, {}!", input.text());
    /// }
    /// ```
    pub fn input(&self, placeholder: &str) -> SimpleInput {
        let el = SimpleInput::new(placeholder);
        self.append(el.get_element());
        el
    }

    /// Returns a button
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dappr::prelude::*;
    /// use wasm_bindgen::prelude::*;
    ///
    /// #[wasm_bindgen(main)]
    /// fn main() {
    ///     let dapp = Dapp::new();
    ///     let button = dapp.button("Press me!");
    ///     button.pressed(move || {
    ///         text!("Button pressed");
    ///     });
    /// }
    /// ```
    pub fn button(&self, label: &str) -> Button {
        let el = Button::new(label);
        self.append(el.get_element());
        el
    }

    // ---- Static text/heading elements ----------------------------------
    // Body: `TextStatic::<tag>(text)` -> append -> return.
    // See `static_text_el!` above for what each of these expands to.

    static_text_el! {
        /// Returns a static Text element.
        ///
        /// > Semantic Element
        ///
        /// Internally it wraps a `<p>` tag.
        ///
        /// # Example
        /// ```no_run
        /// use dappr::prelude::*;
        /// use wasm_bindgen::prelude::*;
        ///
        /// #[wasm_bindgen(main)]
        /// fn main() {
        ///     let dapp = Dapp::new();
        ///     let text = dapp.text_static("Helo, world!");
        /// }
        /// ```
        fn text_static => TextStatic::p
    }

    static_text_el! {
        /// Returns a static h1 element.
        ///
        /// Internally it wraps a `<h1>` tag.
        ///
        /// # Example
        /// ```no_run
        /// use dappr::prelude::*;
        /// use wasm_bindgen::prelude::*;
        ///
        /// #[wasm_bindgen(main)]
        /// fn main() {
        ///     let dapp = Dapp::new();
        ///     let text = dapp.h1_static("Helo, world!");
        /// }
        /// ```
        fn h1_static => TextStatic::h1
    }

    static_text_el! {
        /// Returns a static h2 element.
        ///
        /// Internally it wraps a `<h2>` tag.
        ///
        /// # Example
        /// ```no_run
        /// use dappr::prelude::*;
        /// use wasm_bindgen::prelude::*;
        ///
        /// #[wasm_bindgen(main)]
        /// fn main() {
        ///     let dapp = Dapp::new();
        ///     let text = dapp.h2_static("Helo, world!");
        /// }
        /// ```
        fn h2_static => TextStatic::h2
    }

    static_text_el! {
        /// Returns a static h3 element.
        ///
        /// Internally it wraps a `<h3>` tag.
        ///
        /// # Example
        /// ```no_run
        /// use dappr::prelude::*;
        /// use wasm_bindgen::prelude::*;
        ///
        /// #[wasm_bindgen(main)]
        /// fn main() {
        ///     let dapp = Dapp::new();
        ///     let text = dapp.h3_static("Helo, world!");
        /// }
        /// ```
        fn h3_static => TextStatic::h3
    }

    static_text_el! {
        /// Returns a static h4 element.
        ///
        /// Internally it wraps a `<h4>` tag.
        ///
        /// # Example
        /// ```no_run
        /// use dappr::prelude::*;
        /// use wasm_bindgen::prelude::*;
        ///
        /// #[wasm_bindgen(main)]
        /// fn main() {
        ///     let dapp = Dapp::new();
        ///     let text = dapp.h4_static("Helo, world!");
        /// }
        /// ```
        fn h4_static => TextStatic::h4
    }

    static_text_el! {
        /// Returns a static h5 element.
        ///
        /// Internally it wraps a `<h5>` tag.
        ///
        /// # Example
        /// ```no_run
        /// use dappr::prelude::*;
        /// use wasm_bindgen::prelude::*;
        ///
        /// #[wasm_bindgen(main)]
        /// fn main() {
        ///     let dapp = Dapp::new();
        ///     let text = dapp.h5_static("Helo, world!");
        /// }
        /// ```
        fn h5_static => TextStatic::h5
    }

    static_text_el! {
        /// Returns a static p element.
        ///
        /// Internally it wraps a `<p>` tag.
        ///
        /// # Example
        /// ```no_run
        /// use dappr::prelude::*;
        /// use wasm_bindgen::prelude::*;
        ///
        /// #[wasm_bindgen(main)]
        /// fn main() {
        ///     let dapp = Dapp::new();
        ///     let text = dapp.p_static("Helo, world!");
        /// }
        /// ```
        fn p_static => TextStatic::p
    }

    static_text_el! {
        /// Returns a static div element.
        ///
        /// Internally it wraps a `<div>` tag.
        ///
        /// # Example
        /// ```no_run
        /// use dappr::prelude::*;
        /// use wasm_bindgen::prelude::*;
        ///
        /// #[wasm_bindgen(main)]
        /// fn main() {
        ///     let dapp = Dapp::new();
        ///     let text = dapp.div_static("Helo, world!");
        /// }
        /// ```
        fn div_static => TextStatic::div
    }

    static_text_el! {
        /// Returns a static Title element.
        ///
        /// > Semantic Element
        ///
        /// Internally it wraps a `<h1>` tag.
        ///
        /// # Example
        /// ```no_run
        /// use dappr::prelude::*;
        /// use wasm_bindgen::prelude::*;
        ///
        /// #[wasm_bindgen(main)]
        /// fn main() {
        ///     let dapp = Dapp::new();
        ///     let text = dapp.title_static("Helo, world!");
        /// }
        /// ```
        fn title_static => TextStatic::h1
    }

    static_text_el! {
        /// Returns a static Subtitle element.
        ///
        /// > Semantic Element
        ///
        /// Internally it wraps a `<div>` tag.
        /// It can be applied after any header element.
        ///
        /// # Example
        /// ```no_run
        /// use dappr::prelude::*;
        /// use wasm_bindgen::prelude::*;
        ///
        /// #[wasm_bindgen(main)]
        /// fn main() {
        ///     let dapp = Dapp::new();
        ///     let text = dapp.subtitle_static("Helo, world!");
        /// }
        /// ```
        fn subtitle_static => TextStatic::div
    }

    static_text_el! {
        /// Returns a static part element.
        ///
        /// > Semantic Element
        ///
        /// Internally it wraps a `<h2>` tag.
        ///
        /// # Example
        /// ```no_run
        /// use dappr::prelude::*;
        /// use wasm_bindgen::prelude::*;
        ///
        /// #[wasm_bindgen(main)]
        /// fn main() {
        ///     let dapp = Dapp::new();
        ///     let text = dapp.part_static("Helo, world!");
        /// }
        /// ```
        fn part_static => TextStatic::h2
    }

    static_text_el! {
        /// Returns a static Chapter element.
        ///
        /// > Semantic Element
        ///
        /// Internally it wraps a `<h3>` tag.
        ///
        /// # Example
        /// ```no_run
        /// use dappr::prelude::*;
        /// use wasm_bindgen::prelude::*;
        ///
        /// #[wasm_bindgen(main)]
        /// fn main() {
        ///     let dapp = Dapp::new();
        ///     let text = dapp.chapter_static("Helo, world!");
        /// }
        /// ```
        fn chapter_static => TextStatic::h3
    }

    static_text_el! {
        /// Returns a static Paragraph element.
        ///
        /// > Semantic Element
        ///
        /// Internally it wraps a `<h4>` tag.
        ///
        /// # Example
        /// ```no_run
        /// use dappr::prelude::*;
        /// use wasm_bindgen::prelude::*;
        ///
        /// #[wasm_bindgen(main)]
        /// fn main() {
        ///     let dapp = Dapp::new();
        ///     let text = dapp.paragraph_static("Helo, world!");
        /// }
        /// ```
        fn paragraph_static => TextStatic::h4
    }

    static_text_el! {
        /// Returns a static Subparagraph element.
        ///
        /// > Semantic Element
        ///
        /// Internally it wraps a `<h5>` tag.
        ///
        /// # Example
        /// ```no_run
        /// use dappr::prelude::*;
        /// use wasm_bindgen::prelude::*;
        ///
        /// #[wasm_bindgen(main)]
        /// fn main() {
        ///     let dapp = Dapp::new();
        ///     let text = dapp.subparagraph_static("Helo, world!");
        /// }
        /// ```
        fn subparagraph_static => TextStatic::h5
    }

    static_text_el! {
        /// Returns a static element to style a date.
        ///
        /// > Semantic Element
        ///
        /// Internally it wraps a `<div>` tag.
        ///
        /// # Example
        /// ```no_run
        /// use dappr::prelude::*;
        /// use wasm_bindgen::prelude::*;
        ///
        /// #[wasm_bindgen(main)]
        /// fn main() {
        ///     let dapp = Dapp::new();
        ///     let text = dapp.date_static("Helo, world!");
        /// }
        /// ```
        fn date_static => TextStatic::div
    }

    // ---- Reactive (Signal-backed) text/heading elements -----------------
    // Body: `Text::<tag>(template, signal, init)` -> append -> return.
    // See `reactive_text_el!` above for what each of these expands to.

    reactive_text_el! {
        /// Returns a reactive Text element.
        ///
        /// > Semantic Element
        ///
        /// Internally it wraps a `<p>` tag.
        /// Optionally, it can take an initial value.
        /// If present, the text will be initialized to that initial value,
        /// otherwise the text will be rendered only when the Signal is first dispatched.
        ///
        /// # Example
        /// ```no_run
        /// use dappr::prelude::*;
        /// use wasm_bindgen::prelude::*;
        ///
        /// #[wasm_bindgen(main)]
        /// fn main() {
        ///     let dapp = Dapp::new();
        ///     let input = dapp.input("What is your name?");
        ///     let text = dapp.text("Hello, {}!", input.text(), None);
        /// }
        /// ```
        fn text => Text::p
    }

    reactive_text_el! {
        /// Returns a reactive h1 element.
        ///
        /// Internally it wraps a `<h1>` tag.
        /// Optionally, it can take an initial value.
        /// If present, the text will be initialized to that initial value,
        /// otherwise the text will be rendered only when the Signal is first dispatched.
        ///
        /// # Example
        /// ```no_run
        /// use dappr::prelude::*;
        /// use wasm_bindgen::prelude::*;
        ///
        /// #[wasm_bindgen(main)]
        /// fn main() {
        ///     let dapp = Dapp::new();
        ///     let input = dapp.input("What is your name?");
        ///     let h1 = dapp.h1("Hello, {}!", input.text(), Some("Waiting for your name..."));
        /// }
        /// ```
        fn h1 => Text::h1
    }

    reactive_text_el! {
        /// Returns a reactive h2 element.
        ///
        /// Internally it wraps a `<h2>` tag.
        /// Optionally, it can take an initial value.
        /// If present, the text will be initialized to that initial value,
        /// otherwise the text will be rendered only when the Signal is first dispatched.
        ///
        /// # Example
        /// ```no_run
        /// use dappr::prelude::*;
        /// use wasm_bindgen::prelude::*;
        ///
        /// #[wasm_bindgen(main)]
        /// fn main() {
        ///     let dapp = Dapp::new();
        ///     let input = dapp.input("What is your name?");
        ///     let h2 = dapp.h2("Hello, {}!", input.text(), Some("Waiting for your name..."));
        /// }
        /// ```
        fn h2 => Text::h2
    }

    reactive_text_el! {
        /// Returns a reactive h3 element.
        ///
        /// Internally it wraps a `<h3>` tag.
        /// Optionally, it can take an initial value.
        /// If present, the text will be initialized to that initial value,
        /// otherwise the text will be rendered only when the Signal is first dispatched.
        ///
        /// # Example
        /// ```no_run
        /// use dappr::prelude::*;
        /// use wasm_bindgen::prelude::*;
        ///
        /// #[wasm_bindgen(main)]
        /// fn main() {
        ///     let dapp = Dapp::new();
        ///     let input = dapp.input("What is your name?");
        ///     let h3 = dapp.h3("Hello, {}!", input.text(), Some("Waiting for your name..."));
        /// }
        /// ```
        fn h3 => Text::h3
    }

    reactive_text_el! {
        /// Returns a reactive h4 element.
        ///
        /// Internally it wraps a `<h4>` tag.
        /// Optionally, it can take an initial value.
        /// If present, the text will be initialized to that initial value,
        /// otherwise the text will be rendered only when the Signal is first dispatched.
        ///
        /// # Example
        /// ```no_run
        /// use dappr::prelude::*;
        /// use wasm_bindgen::prelude::*;
        ///
        /// #[wasm_bindgen(main)]
        /// fn main() {
        ///     let dapp = Dapp::new();
        ///     let input = dapp.input("What is your name?");
        ///     let h4 = dapp.h4("Hello, {}!", input.text(), Some("Waiting for your name..."));
        /// }
        /// ```
        fn h4 => Text::h4
    }

    reactive_text_el! {
        /// Returns a reactive h5 element.
        ///
        /// Internally it wraps a `<h5>` tag.
        /// Optionally, it can take an initial value.
        /// If present, the text will be initialized to that initial value,
        /// otherwise the text will be rendered only when the Signal is first dispatched.
        ///
        /// # Example
        /// ```no_run
        /// use dappr::prelude::*;
        /// use wasm_bindgen::prelude::*;
        ///
        /// #[wasm_bindgen(main)]
        /// fn main() {
        ///     let dapp = Dapp::new();
        ///     let input = dapp.input("What is your name?");
        ///     let h5 = dapp.h5("Hello, {}!", input.text(), Some("Waiting for your name..."));
        /// }
        /// ```
        fn h5 => Text::h5
    }

    reactive_text_el! {
        /// Returns a reactive p element.
        ///
        /// Internally it wraps a `<p>` tag.
        /// Optionally, it can take an initial value.
        /// If present, the text will be initialized to that initial value,
        /// otherwise the text will be rendered only when the Signal is first dispatched.
        ///
        /// # Example
        /// ```no_run
        /// use dappr::prelude::*;
        /// use wasm_bindgen::prelude::*;
        ///
        /// #[wasm_bindgen(main)]
        /// fn main() {
        ///     let dapp = Dapp::new();
        ///     let input = dapp.input("What is your name?");
        ///     let p = dapp.p("Hello, {}!", input.text(), Some("Waiting for your name..."));
        /// }
        /// ```
        fn p => Text::p
    }

    reactive_text_el! {
        /// Returns a reactive div element.
        ///
        /// Internally it wraps a `<div>` tag.
        /// Optionally, it can take an initial value.
        /// If present, the text will be initialized to that initial value,
        /// otherwise the text will be rendered only when the Signal is first dispatched.
        ///
        /// # Example
        /// ```no_run
        /// use dappr::prelude::*;
        /// use wasm_bindgen::prelude::*;
        ///
        /// #[wasm_bindgen(main)]
        /// fn main() {
        ///     let dapp = Dapp::new();
        ///     let input = dapp.input("What is your name?");
        ///     let div = dapp.div("Hello, {}!", input.text(), Some("Waiting for your name..."));
        /// }
        /// ```
        fn div => Text::div
    }

    reactive_text_el! {
        /// Returns a reactive Title element.
        ///
        /// > Semantic Element
        ///
        /// Internally it wraps a `<h1>` tag.
        /// Optionally, it can take an initial value.
        /// If present, the text will be initialized to that initial value,
        /// otherwise the text will be rendered only when the Signal is first dispatched.
        ///
        /// # Example
        /// ```no_run
        /// use dappr::prelude::*;
        /// use wasm_bindgen::prelude::*;
        ///
        /// #[wasm_bindgen(main)]
        /// fn main() {
        ///     let dapp = Dapp::new();
        ///     let input = dapp.input("What is your name?");
        ///     let title = dapp.title("Hello, {}!", input.text(), None);
        /// }
        /// ```
        fn title => Text::h1
    }

    reactive_text_el! {
        /// Returns a reactive Subtitle element.
        ///
        /// > Semantic Element
        ///
        /// Internally it wraps a `<div>` tag.
        /// Optionally, it can take an initial value.
        /// If present, the text will be initialized to that initial value,
        /// otherwise the text will be rendered only when the Signal is first dispatched.
        ///
        /// It can be placed after any semantic header element
        ///
        /// # Example
        /// ```no_run
        /// use dappr::prelude::*;
        /// use wasm_bindgen::prelude::*;
        ///
        /// #[wasm_bindgen(main)]
        /// fn main() {
        ///     let dapp = Dapp::new();
        ///     let input = dapp.input("What is your name?");
        ///     let subtitle = dapp.subtitle("Hello, {}!", input.text(), None);
        /// }
        /// ```
        fn subtitle => Text::div
    }

    reactive_text_el! {
        /// Returns a reactive element that styles a date.
        ///
        /// > Semantic Element
        ///
        /// Internally it wraps a `<div>` tag.
        /// Optionally, it can take an initial value.
        /// If present, the text will be initialized to that initial value,
        /// otherwise the text will be rendered only when the Signal is first dispatched.
        ///
        /// # Example
        /// ```no_run
        /// use dappr::prelude::*;
        /// use wasm_bindgen::prelude::*;
        ///
        /// #[wasm_bindgen(main)]
        /// fn main() {
        ///     let dapp = Dapp::new();
        ///     let date_input = dapp.input("What day is it?");
        ///     let date = dapp.title("Hello, today is {}!", date_input.text(), None);
        /// }
        /// ```
        fn date => Text::div
    }

    reactive_text_el! {
        /// Returns a reactive Part element.
        ///
        /// > Semantic Element
        ///
        /// Internally it wraps a `<h2>` tag.
        /// Optionally, it can take an initial value.
        /// If present, the text will be initialized to that initial value,
        /// otherwise the text will be rendered only when the Signal is first dispatched.
        ///
        /// # Example
        /// ```no_run
        /// use dappr::prelude::*;
        /// use wasm_bindgen::prelude::*;
        ///
        /// #[wasm_bindgen(main)]
        /// fn main() {
        ///     let dapp = Dapp::new();
        ///     let input = dapp.input("What is your name?");
        ///     let part = dapp.part("Hello, {}!", input.text(), None);
        /// }
        /// ```
        fn part => Text::h2
    }

    reactive_text_el! {
        /// Returns a reactive Chapter element.
        ///
        /// > Semantic Element
        ///
        /// Internally it wraps a `<h3>` tag.
        /// Optionally, it can take an initial value.
        /// If present, the text will be initialized to that initial value,
        /// otherwise the text will be rendered only when the Signal is first dispatched.
        ///
        /// # Example
        /// ```no_run
        /// use dappr::prelude::*;
        /// use wasm_bindgen::prelude::*;
        ///
        /// #[wasm_bindgen(main)]
        /// fn main() {
        ///     let dapp = Dapp::new();
        ///     let input = dapp.input("What is your name?");
        ///     let chapter = dapp.chapter("Hello, {}!", input.text(), None);
        /// }
        /// ```
        fn chapter => Text::h3
    }

    reactive_text_el! {
        /// Returns a reactive Paragraph element.
        ///
        /// > Semantic Element
        ///
        /// Internally it wraps a `<h4>` tag.
        /// Optionally, it can take an initial value.
        /// If present, the text will be initialized to that initial value,
        /// otherwise the text will be rendered only when the Signal is first dispatched.
        ///
        /// # Example
        /// ```no_run
        /// use dappr::prelude::*;
        /// use wasm_bindgen::prelude::*;
        ///
        /// #[wasm_bindgen(main)]
        /// fn main() {
        ///     let dapp = Dapp::new();
        ///     let input = dapp.input("What is your name?");
        ///     let paragraph = dapp.paragraph("Hello, {}!", input.text(), None);
        /// }
        /// ```
        fn paragraph => Text::h4
    }

    reactive_text_el! {
        /// Returns a reactive Subparagraph element.
        ///
        /// > Semantic Element
        ///
        /// Internally it wraps a `<h5>` tag.
        /// Optionally, it can take an initial value.
        /// If present, the text will be initialized to that initial value,
        /// otherwise the text will be rendered only when the Signal is first dispatched.
        ///
        /// # Example
        /// ```no_run
        /// use dappr::prelude::*;
        /// use wasm_bindgen::prelude::*;
        ///
        /// #[wasm_bindgen(main)]
        /// fn main() {
        ///     let dapp = Dapp::new();
        ///     let input = dapp.input("What is your name?");
        ///     let subparagraph = dapp.subparagraph("Hello, {}!", input.text(), None);
        /// }
        /// ```
        fn subparagraph => Text::h5
    }

    // ---- Raw / sanitized HTML injection ---------------------------------
    // These have different signatures from the text methods above (and the
    // sanitized variants are feature-gated), so they're left hand-written
    // rather than folded into the macros.

    /// Returns a static element that injects Raw HTML in the page.
    ///
    /// > Data Display
    /// Internally it wraps a `<div>` tag that wraps around the injected HTML.
    ///
    /// <div class="warning">This is potentially dangerous, as the HTML injected is not sanitized in any way</div>
    ///
    /// # Example
    /// ```no_run
    /// use dappr::prelude::*;
    /// use wasm_bindgen::prelude::*;
    ///
    /// #[wasm_bindgen(main)]
    /// fn main() {
    ///     let dapp = Dapp::new();
    ///     let html = dapp.html_static("<h1>Injected Title</h1>");
    /// }
    /// ```
    pub fn html_static(&self, raw_html: &str) -> RawHtmlStatic {
        let el = RawHtmlStatic::new(raw_html);
        self.append(el.get_element());
        el
    }

    /// Returns a reactive element that injects Raw HTML in the page.
    ///
    /// > Data Display
    /// Internally it wraps a `<div>` tag that wraps around the injected HTML.
    /// Optionally, it can take an initial value.
    /// If present, the text will be initialized to that initial value,
    /// otherwise the text will be rendered only when the Signal is first dispatched.
    ///
    /// <div class="warning">This is potentially dangerous, as the HTML injected is not sanitized in any way.</div>
    ///
    /// # Example
    /// ```no_run
    /// use dappr::prelude::*;
    /// use wasm_bindgen::prelude::*;
    ///
    /// #[wasm_bindgen(main)]
    /// fn main() {
    ///     let dapp = Dapp::new();
    ///     let html_input = dapp.input("Html to inject");
    ///     let html = dapp.html("{}", html_input.text(), Some("Injected code goes here!"));
    /// }
    /// ```
    pub fn html<T: Display + Clone + 'static>(
        &self,
        template: &str,
        signal: Signal<T>,
        init: Option<&str>,
    ) -> RawHtml {
        let el = RawHtml::new(template, signal, init);
        self.append(el.get_element());
        el
    }

    /// Returns a reactive static that injects sanitized HTML in the page.
    ///
    /// > Data Display
    ///
    /// Internally it wraps a `<div>` tag that wraps around the injected HTML.
    /// It takes a [`Sanitize`] struct to specify the sanification level.
    ///
    /// <div class="warning">This element requires the following feature to be activted: <b>sanitizehtml</b></div>
    ///
    /// # Example
    /// ```no_run
    /// use dappr::prelude::*;
    /// use wasm_bindgen::prelude::*;
    ///
    /// #[wasm_bindgen(main)]
    /// fn main() {
    ///     let dapp = Dapp::new();
    ///     # #[cfg(any(feature = "sanitizehtml", doc, test))]
    ///     let html = dapp.sanitized_html_static(
    ///         "Example of sanitized HTML which removes also <a href='#'>links</a>.",
    ///         Sanitize::Restricted
    ///     );
    /// }
    /// ```
    #[cfg(any(feature = "sanitizehtml", doc, test))]
    pub fn sanitized_html_static(&self, raw_html: &str, sanitize: Sanitize) -> RawHtmlStatic {
        let el = RawHtmlStatic::sanitized(raw_html, sanitize);
        self.append(el.get_element());
        el
    }

    /// Returns a reactive element that injects sanitized HTML in the page.
    ///
    /// > Data Display
    ///
    /// Internally it wraps a `<div>` tag that wraps around the injected HTML.
    /// Optionally, it can take an initial value.
    /// If present, the text will be initialized to that initial value,
    /// otherwise the text will be rendered only when the Signal is first dispatched.
    /// It takes a [`Sanitize`] struct to specify the sanification level.
    ///
    /// <div class="warning">This element requires the following feature to be activted: <b>sanitizehtml</b></div>
    ///
    /// # Example
    /// ```no_run
    /// use dappr::prelude::*;
    /// use wasm_bindgen::prelude::*;
    ///
    /// #[wasm_bindgen(main)]
    /// fn main() {
    ///     let dapp = Dapp::new();
    ///     let html_input = dapp.input("Html to inject");
    ///     # #[cfg(any(feature = "sanitizehtml", doc, test))]
    ///     let html = dapp.sanitized_html(
    ///         "{}", html_input.text(),
    ///         Some("Injected code goes here!"),
    ///         Sanitize::Basic
    ///     );
    /// }
    /// ```
    #[cfg(any(feature = "sanitizehtml", doc, test))]
    pub fn sanitized_html<T: Display + Clone + 'static>(
        &self,
        template: &str,
        signal: Signal<T>,
        init: Option<&str>,
        sanitize: Sanitize,
    ) -> RawHtml {
        let el = RawHtml::sanitized(template, signal, init, sanitize);
        self.append(el.get_element());
        el
    }
}

// Global instance of Dapp
thread_local! {
    #[doc(hidden)]
    pub static DAPP_INSTANCE: Rc<RefCell<Dapp>> = Rc::new(RefCell::new(Dapp::new()));
}

/// text! macro: prints a text to the page.
///
/// The `text!` maro iutputs a text to the page. It accepts various paraeters,
/// and it can handle both static and responsive texts.
///
/// # Examples
///
/// Text without specifiers:
/// ```no_run
/// use dappr::prelude::*;
/// use wasm_bindgen::prelude::*;
///
/// #[wasm_bindgen(main)]
/// pub fn main() {
///     dapp!(dapp, {
///         text!("Welcome to dappr!"); // Static text
///         let input = dapp.input("What's your name?");
///         text!("Hello, {}!", input.text());  // Responsive text, not initialized
///                                             // The text appears only when the input is initialized
///         // The following is initialized text
///         text!(
///             "Text in input: {}", // This appears only when the input is updated
///             input.text(),
///             "This text will be updated when you type your name" // Initial text
///         );
///     });
/// }
/// ```
///
/// Above you can see the three main kinds of `text!`:
/// - *Static text*: This is always on the page.
/// - *Responsive, un-initialized text*: This text appears when the signal it contains gets dispatched.
/// - *Responsive, initialized text*: This text has two parts,
/// one that is displayed up until the signal it contains gets dispatched, and another one that appears after.
///
/// *Notice* that the texts that appears after the Signal is dispatched doesn't need to appear inside the
///
/// ```no_run
/// use dappr::prelude::*;
/// use wasm_bindgen::prelude::*;
///
/// #[wasm_bindgen(main)]
/// pub fn main() {
///     dapp!(dapp, {
///         let input = dapp.input("What's your name?");
///         text!(
///             "Finally, you wrote something!", // The template does not carry the Signal
///             input.text(),
///             "This text will be updated when you type your name" // Initial text
///         );
///     });
/// }
/// ```
///
/// ### Specifiers
///
/// An example with specifiers, they can be static, responsive un-initialized, and responsive initialized:
/// ```no_run
/// use dappr::prelude::*;
/// use wasm_bindgen::prelude::*;
///
/// #[wasm_bindgen(main)]
/// pub fn main() {
///     dapp!(dapp, {
///         text!("<b>Welcome to dappr!<b>", _html); // Static raw HTML, injected into the page
///         let input = dapp.input("Enter some HTML code");
///         // The following two can be very dangerous, but we will wrap them in `<pre>`
///         //    to mitigate a little the effects,although `<pre>` can easily be hijacked.
///         text!("<pre>{}</pre>", input.text());  // Responsive raw HTML, not initialized
///                                             // The text appears only when the input is initialized
///         // The following is initialized text, with SANITIZED HTML
///         // For this to compile, there is need of the `sanitizehtml` feature
///         text!(
///             "Sanitized HTML: {}",
///             input.text(),
///             "The HTML code injected here will be totally stripped off." // Initial text
///         );
///     });
/// }
/// ```
/// ### Specifiers list
/// > Remeber the underscore `_` for the specifiers!
///
/// **HTML-related**
/// - `_h1`, `_h2`, `_h3`, `_h4`, `_h5`
/// - `_div`, `_p`
///
/// **Semantic**
/// - `_title`, `_part`, `_chapter`, `_paragraph`, `_subparagraph`: these roughly correspond to h1..h5.
/// - `_subtitle`: can be placed under any semantic title above.
/// - `_date`: styles a date.
///
/// **Data Display**
/// - `_html`: injects HTML on the page. If followed by a [`Sanitize`] specifier, it sanitize the HTML before injection.
#[macro_export]
macro_rules! text {
    ($text:expr) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance.borrow().text_static($text);
        });
    };
    ($text:expr, _h1) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance.borrow().h1_static($text);
        });
    };
    ($text:expr, _h2) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance.borrow().h2_static($text);
        });
    };
    ($text:expr, _h3) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance.borrow().h3_static($text);
        });
    };
    ($text:expr, _h4) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance.borrow().h4_static($text);
        });
    };
    ($text:expr, _h5) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance.borrow().h5_static($text);
        });
    };
    ($text:expr, _p) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance.borrow().p_static($text);
        });
    };
    ($text:expr, _div) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance.borrow().div_static($text);
        });
    };
    ($text:expr, _title) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance.borrow().title_static($text);
        });
    };
    ($text:expr, _subtitle) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance.borrow().subtitle_static($text);
        });
    };
    ($text:expr, _date) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance.borrow().date_static($text);
        });
    };
    ($text:expr, _part) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance.borrow().part_static($text);
        });
    };
    ($text:expr, _chapter) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance.borrow().chapter_static($text);
        });
    };
    ($text:expr, _paragraph) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance.borrow().paragraph_static($text);
        });
    };
    ($text:expr, _subparagraph) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance.borrow().subparagraph_static($text);
        });
    };
    ($text:expr, _html) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance.borrow().html_static($text);
        });
    };
    //#[cfg(any(feature = "sanitizehtml", doc, test))]
    ($text:expr, _html, $sanitize:expr) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance
                .borrow()
                .sanitized_html_static($text, $sanitize);
        });
    };
    ($template:expr, $signal:expr) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance.borrow().text($template, $signal, None);
        });
    };
    ($template:expr, $signal:expr, _h1) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance.borrow().h1($template, $signal, None);
        });
    };
    ($template:expr, $signal:expr, _h2) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance.borrow().h2($template, $signal, None);
        });
    };
    ($template:expr, $signal:expr, _h3) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance.borrow().h3($template, $signal, None);
        });
    };
    ($template:expr, $signal:expr, _h4) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance.borrow().h4($template, $signal, None);
        });
    };
    ($template:expr, $signal:expr, _h5) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance.borrow().h5($template, $signal, None);
        });
    };
    ($template:expr, $signal:expr, _p) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance.borrow().p($template, $signal, None);
        });
    };
    ($template:expr, $signal:expr, _div) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance.borrow().div($template, $signal, None);
        });
    };
    ($template:expr, $signal:expr, _title) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance.borrow().title($template, $signal, None);
        });
    };
    ($template:expr, $signal:expr, _subtitle) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance.borrow().subtitle($template, $signal, None);
        });
    };
    ($template:expr, $signal:expr, _date) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance.borrow().date($template, $signal, None);
        });
    };
    ($template:expr, $signal:expr, _part) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance.borrow().part($template, $signal, None);
        });
    };
    ($template:expr, $signal:expr, _chapter) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance.borrow().chapter($template, $signal, None);
        });
    };
    ($template:expr, $signal:expr, _paragraph) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance.borrow().paragraph($template, $signal, None);
        });
    };
    ($template:expr, $signal:expr, _subparagraph) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance
                .borrow()
                .subparagraph($template, $signal, None);
        });
    };
    ($template:expr, $signal:expr, _html) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance.borrow().html($template, $signal, None);
        });
    };
    //#[cfg(any(feature = "sanitizehtml", doc, test))]
    ($template:expr, $signal:expr, _html, $sanitize:expr) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance
                .borrow()
                .sanitized_html($template, $signal, None, $sanitize);
        });
    };
    ($template:expr, $signal:expr, $init:expr) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance.borrow().text($template, $signal, Some($init));
        });
    };
    ($template:expr, $signal:expr, $init:expr, _h1) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance.borrow().h1($template, $signal, Some($init));
        });
    };
    ($template:expr, $signal:expr, $init:expr, _h2) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance.borrow().h2($template, $signal, Some($init));
        });
    };
    ($template:expr, $signal:expr, $init:expr, _h3) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance.borrow().h3($template, $signal, Some($init));
        });
    };
    ($template:expr, $signal:expr, $init:expr, _h4) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance.borrow().h4($template, $signal, Some($init));
        });
    };
    ($template:expr, $signal:expr, $init:expr, _h5) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance.borrow().h5($template, $signal, Some($init));
        });
    };
    ($template:expr, $signal:expr, $init:expr, _p) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance.borrow().p($template, $signal, Some($init));
        });
    };
    ($template:expr, $signal:expr, $init:expr, _div) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance.borrow().div($template, $signal, Some($init));
        });
    };
    ($template:expr, $signal:expr, $init:expr, _title) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance
                .borrow()
                .title($template, $signal, Some($init));
        });
    };
    ($template:expr, $signal:expr, $init:expr, _subtitle) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance
                .borrow()
                .subtitle($template, $signal, Some($init));
        });
    };
    ($template:expr, $signal:expr, $init:expr, _date) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance.borrow().date($template, $signal, Some($init));
        });
    };
    ($template:expr, $signal:expr, $init:expr, _part) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance.borrow().part($template, $signal, Some($init));
        });
    };
    ($template:expr, $signal:expr, $init:expr, _chapter) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance
                .borrow()
                .chapter($template, $signal, Some($init));
        });
    };
    ($template:expr, $signal:expr, $init:expr, _paragraph) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance
                .borrow()
                .paragraph($template, $signal, Some($init));
        });
    };
    ($template:expr, $signal:expr, $init:expr, _subparagraph) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance
                .borrow()
                .subparagraph($template, $signal, Some($init));
        });
    };
    ($template:expr, $signal:expr, $init:expr, _html) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance.borrow().html($template, $signal, Some($init));
        });
    };
    //#[cfg(any(feature = "sanitizehtml", doc, test))]
    ($template:expr, $signal:expr, $init:expr, _html, $sanitize:expr) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            dapp_instance
                .borrow()
                .sanitized_html($template, $signal, Some($init), $sanitize);
        });
    };
}

/// The dapp! macro open a context to use the global Dapp for reactive purposes.
///
/// # Example
/// ```no_run
/// use dappr::prelude::*;
/// use wasm_bindgen::prelude::*;
///
/// #[wasm_bindgen(main)]
/// pub fn main() {
///     dapp!(dapp, {
///         let input = dapp.input("What's your name?");
///         text!("Hello, {}!", input.text());
///     });
/// }
/// ```
#[macro_export]
macro_rules! dapp {
    ($var:ident, $code:block) => {
        crate::DAPP_INSTANCE.with(|dapp_instance| {
            let $var = dapp_instance.borrow();
            $code
        });
    };
}
