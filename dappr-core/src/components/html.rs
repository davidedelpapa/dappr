//! HTML related module
use crate::communication::signals::Signal;
use crate::framework::dom;
use crate::framework::traits::Component;
#[cfg(feature = "sanitizehtml")]
use sanitize_html::rules::predefined::{BASIC, DEFAULT, RELAXED, RESTRICTED, UNTRUSTED};
#[cfg(feature = "sanitizehtml")]
use sanitize_html::sanitize_str;
use std::fmt::Display;
use web_sys::HtmlElement;

/// Sanitize level
///
/// This enum allows for various levels of sanitizing the [`RawHtml`] and
/// [`RawHtmlStatic`] components when the HTML is passed by the user.
pub enum Sanitize {
    /// Default no sanitizing
    None,
    /// Basic allows a variety of markup including formatting elements, links, and lists.
    #[cfg(any(feature = "sanitizehtml", doc))]
    Basic,
    /// Relaxed allows an even wider variety of markup, including images and tables.
    #[cfg(any(feature = "sanitizehtml", doc))]
    Relaxed,
    /// Restricted allows only very simple inline markup. No links, images, or block elements.
    #[cfg(any(feature = "sanitizehtml", doc))]
    Restricted,
    /// Untrusted removes all tags but text emphasizing and links
    #[cfg(any(feature = "sanitizehtml", doc))]
    Untrusted,
    /// RemoveAll reoves all tags, leaving just the text (*de facto* it removes all html)
    #[cfg(any(feature = "sanitizehtml", doc))]
    RemoveAll,
}

/// The RawHtml component
/// ## Description
/// The `RawHtml` component outputs raw HTML code inside the app
///
/// ## Usage
///
/// ```no_run
/// # use dappr_core::prelude::*;
/// RawHtml::new("<h1>This is an example of Raw HTML with substitution: {}</h1>", Signal::new(0usize), None);
/// ```
pub struct RawHtml {
    el: HtmlElement,
}

impl RawHtml {
    /// ## Sanitized Html
    /// Using the feature `sanitizehtml` we have access to the RawHtml::sanitized constructor which takes
    /// a raw Html *&str* and a *Sanitize* enum that specifies the level of sanification.
    /// This is useful to parse the imput from the user or untrusted sources.
    ///
    /// This component uses  the crate [sanitize_html](https://crates.io/crates/sanitize_html) under the hood.
    ///
    /// ### Usage
    ///
    /// ```no_run
    /// # use dappr_core::prelude::*;
    /// RawHtml::sanitized(
    ///     "<h1>This is an example of sanitized HTML with substituttion: {}</h1>",
    ///     Signal::new(String::new()),
    ///     Some("Intial render"),
    ///     Sanitize::None, // Does not apply Sanitize
    /// );
    /// ```
    /// Check [Sanitize] for more options.
    pub fn sanitized<T: Display + Clone + 'static>(
        template: &str,
        signal: Signal<T>,
        initial_render: Option<&str>,
        sanitize: Sanitize,
    ) -> RawHtml {
        let div_element = dom::create_element("div");

        match initial_render {
            Some(initial_content) => {
                let sanitized_html: String =
                    match sanitize {
                        Sanitize::None => initial_content.to_string(),
                        #[cfg(feature = "sanitizehtml")]
                        Sanitize::RemoveAll => sanitize_str(&DEFAULT, initial_content)
                            .expect("Failed Sanitize::RemoveAll"),
                        #[cfg(feature = "sanitizehtml")]
                        Sanitize::Basic => {
                            sanitize_str(&BASIC, initial_content).expect("Failed Sanitize::Basic")
                        }
                        #[cfg(feature = "sanitizehtml")]
                        Sanitize::Relaxed => sanitize_str(&RELAXED, initial_content)
                            .expect("Failed Sanitize::Relaxed"),
                        #[cfg(feature = "sanitizehtml")]
                        Sanitize::Restricted => sanitize_str(&RESTRICTED, initial_content)
                            .expect("Failed Sanitize::Restricted"),
                        #[cfg(feature = "sanitizehtml")]
                        Sanitize::Untrusted => sanitize_str(&UNTRUSTED, initial_content)
                            .expect("Failed Sanitize::Untrusted"),
                    };
                let div_element_cloned = div_element.clone();
                div_element_cloned.set_inner_html(&sanitized_html);
            }
            None => (),
        }

        let template_owned = template.to_string();
        let div_element_cloned = div_element.clone();
        // `.detach()`: same reasoning as in `Input::new` -- `RawHtml` has no
        // unmount concept yet, so this subscription is kept alive for the
        // element's whole lifetime, matching the previous behavior.
        signal
            .subscribe(move |new_value| {
                let content = template_owned.replace("{}", &new_value.to_string());
                let sanitized_html: String = match sanitize {
                    Sanitize::None => content,
                    #[cfg(feature = "sanitizehtml")]
                    Sanitize::RemoveAll => {
                        sanitize_str(&DEFAULT, &content).expect("Failed Sanitize::RemoveAll")
                    }
                    #[cfg(feature = "sanitizehtml")]
                    Sanitize::Basic => {
                        sanitize_str(&BASIC, &content).expect("Failed Sanitize::Basic")
                    }
                    #[cfg(feature = "sanitizehtml")]
                    Sanitize::Relaxed => {
                        sanitize_str(&RELAXED, &content).expect("Failed Sanitize::Relaxed")
                    }
                    #[cfg(feature = "sanitizehtml")]
                    Sanitize::Restricted => {
                        sanitize_str(&RESTRICTED, &content).expect("Failed Sanitize::Restricted")
                    }
                    #[cfg(feature = "sanitizehtml")]
                    Sanitize::Untrusted => {
                        sanitize_str(&UNTRUSTED, &content).expect("Failed Sanitize::Untrusted")
                    }
                };
                div_element_cloned.set_inner_html(&sanitized_html);
            })
            .detach();
        RawHtml { el: div_element }
    }
    pub fn new<T: Display + Clone + 'static>(
        template: &str,
        signal: Signal<T>,
        initial_render: Option<&str>,
    ) -> RawHtml {
        Self::sanitized(template, signal, initial_render, Sanitize::None)
    }
}

impl Component for RawHtml {
    fn get_element(&self) -> &HtmlElement {
        &self.el
    }
}

/// The RawHtmlStatic component
/// ## Description
/// The `RawHtmlStatic` component outputs raw HTML code inside the app.
/// This is the non-reactive version of RawHtml.
///
/// ## Usage
///
/// ```no_run
/// # use dappr_core::prelude::*;
/// RawHtmlStatic::new("<h1>This is an example of Raw HTML</h1>");
/// ```
pub struct RawHtmlStatic {
    el: HtmlElement,
}

impl RawHtmlStatic {
    /// ## Sanitized Html
    /// Using the feature `sanitizehtml` we have access to the RawHtml::sanitized constructor which takes
    /// a raw Html *&str* and a *Sanitize* enum that specifies the level of sanification.
    /// This is useful to parse the imput from the user or untrusted sources.
    ///
    /// This component uses  the crate [sanitize_html](https://crates.io/crates/sanitize_html) under the hood.
    ///
    /// ### Usage
    ///
    /// ```no_run
    /// # use dappr_core::prelude::*;
    /// RawHtmlStatic::sanitized(
    ///     "<h1>This is an example of sanitized HTML</h1>",
    ///     Sanitize::None, // Does not apply Sanitize
    /// );
    /// ```
    /// Check [Sanitize] for more options.
    pub fn sanitized(raw_html: &str, sanitize: Sanitize) -> RawHtmlStatic {
        let sanitized_html: String = match sanitize {
            Sanitize::None => raw_html.to_string(),
            #[cfg(feature = "sanitizehtml")]
            Sanitize::RemoveAll => {
                sanitize_str(&DEFAULT, raw_html).expect("Failed Sanitize::RemoveAll")
            }
            #[cfg(feature = "sanitizehtml")]
            Sanitize::Basic => sanitize_str(&BASIC, raw_html).expect("Failed Sanitize::Basic"),
            #[cfg(feature = "sanitizehtml")]
            Sanitize::Relaxed => {
                sanitize_str(&RELAXED, raw_html).expect("Failed Sanitize::Relaxed")
            }
            #[cfg(feature = "sanitizehtml")]
            Sanitize::Restricted => {
                sanitize_str(&RESTRICTED, raw_html).expect("Failed Sanitize::Restricted")
            }
            #[cfg(feature = "sanitizehtml")]
            Sanitize::Untrusted => {
                sanitize_str(&UNTRUSTED, raw_html).expect("Failed Sanitize::Untrusted")
            }
        };
        let div_element = dom::create_element("div");
        div_element.set_inner_html(&sanitized_html);
        RawHtmlStatic { el: div_element }
    }
    pub fn new(raw_html: &str) -> RawHtmlStatic {
        Self::sanitized(raw_html, Sanitize::None)
    }
}

impl Component for RawHtmlStatic {
    fn get_element(&self) -> &HtmlElement {
        &self.el
    }
}
