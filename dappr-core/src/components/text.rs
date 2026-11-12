//! ## Text component
//! ### Description
//! The `Text` component creates a responsive text element (internally a `<p>`)
//!
//! ## TextStatic component
//! ### Description
//! The `TextStatic` component creates a simplified form of text element, not responsive but static.
//!
use crate::communication::signals::Signal;
use crate::framework::dom;
use crate::framework::traits::Component;
use std::fmt::Display;
use web_sys::HtmlElement;

pub struct Text {
    el: HtmlElement,
}

impl Text {
    fn new<T: Display + Clone + 'static>(
        tag: &str,
        template: &str,
        signal: Signal<T>,
        initial_render: Option<&str>,
    ) -> Self {
        let element = dom::create_element(tag);
        let text = Text { el: element };
        text.bind_signal(signal, Some(template), initial_render);
        text
    }
}
impl Component for Text {
    fn get_element(&self) -> &HtmlElement {
        &self.el
    }
}

pub struct TextStatic {
    el: HtmlElement,
}

impl TextStatic {
    fn new(tag: &str, text: &str) -> Self {
        let element = dom::create_element(tag);
        element.set_text_content(Some(&text));
        TextStatic { el: element }
    }
}

impl Component for TextStatic {
    fn get_element(&self) -> &HtmlElement {
        &self.el
    }
}

// Specialized implementations

impl Text {
    pub fn p<T: Display + Clone + 'static>(
        template: &str,
        signal: Signal<T>,
        init: Option<&str>,
    ) -> Self {
        Self::new("p", template, signal, init)
    }
    pub fn div<T: Display + Clone + 'static>(
        template: &str,
        signal: Signal<T>,
        init: Option<&str>,
    ) -> Self {
        Self::new("div", template, signal, init)
    }
    pub fn h1<T: Display + Clone + 'static>(
        template: &str,
        signal: Signal<T>,
        init: Option<&str>,
    ) -> Self {
        Self::new("h1", template, signal, init)
    }
    pub fn h2<T: Display + Clone + 'static>(
        template: &str,
        signal: Signal<T>,
        init: Option<&str>,
    ) -> Self {
        Self::new("h2", template, signal, init)
    }
    pub fn h3<T: Display + Clone + 'static>(
        template: &str,
        signal: Signal<T>,
        init: Option<&str>,
    ) -> Self {
        Self::new("h3", template, signal, init)
    }
    pub fn h4<T: Display + Clone + 'static>(
        template: &str,
        signal: Signal<T>,
        init: Option<&str>,
    ) -> Self {
        Self::new("h4", template, signal, init)
    }
    pub fn h5<T: Display + Clone + 'static>(
        template: &str,
        signal: Signal<T>,
        init: Option<&str>,
    ) -> Self {
        Self::new("h5", template, signal, init)
    }
}

impl TextStatic {
    pub fn div(text: &str) -> Self {
        Self::new("div", text)
    }

    pub fn p(text: &str) -> Self {
        Self::new("p", text)
    }

    pub fn h1(text: &str) -> Self {
        Self::new("h1", text)
    }

    pub fn h2(text: &str) -> Self {
        Self::new("h2", text)
    }

    pub fn h3(text: &str) -> Self {
        Self::new("h3", text)
    }

    pub fn h4(text: &str) -> Self {
        Self::new("h4", text)
    }

    pub fn h5(text: &str) -> Self {
        Self::new("h5", text)
    }
}
