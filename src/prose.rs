//! Markdown text and LaTeX equations.
//!
//! ```no_run
//! use dappr::prelude::*;
//! # let dapp = Dapp::new();
//! dapp.markdown("## Method\n\nWe fit a **linear** model, $y = ax + b$.");
//! dapp.latex(r"\hat{a} = \frac{\sum (x_i - \bar{x})(y_i - \bar{y})}{\sum (x_i - \bar{x})^2}");
//! ```
//!
//! Both also take a `Signal<String>`, and `text!` has `_md` and `_latex`
//! specifiers for them.
use crate::app::Dapp;
#[cfg(feature = "latex")]
use dappr_core::components::markdown::Latex;
#[cfg(feature = "markdown")]
use dappr_core::components::markdown::Markdown;
use dappr_core::framework::traits::{Component, TextSource};

/// Markdown and LaTeX
impl Dapp {
    /// Shows Markdown text: headings, emphasis, lists, links, images, code,
    /// tables, task lists and footnotes. With the `latex` feature (on by
    /// default), `$...$` is inline math and `$$...$$` an equation on its own
    /// line.
    ///
    /// Pass a `Signal<String>` to update the text when it changes.
    ///
    /// The text is safe to take from anywhere -- a user, a file, an API:
    /// HTML written inside it is shown as text, and links that would run
    /// script go nowhere. See [`markdown_with_html`](Dapp::markdown_with_html)
    /// to allow HTML in text you wrote yourself.
    ///
    /// ```no_run
    /// # use dappr::prelude::*;
    /// # let dapp = Dapp::new();
    /// dapp.markdown("# Report\n\nRevenue grew **12%**, see [the data](#data).");
    ///
    /// // Live preview of what the user types.
    /// let notes = dapp.textarea("Notes (Markdown)");
    /// dapp.markdown(notes.value());
    /// ```
    #[cfg(feature = "markdown")]
    pub fn markdown(&self, source: impl TextSource) -> Markdown {
        let markdown = Markdown::new(source);
        self.append(markdown.get_element());
        markdown
    }

    /// Like [`markdown`](Dapp::markdown), but HTML written inside the
    /// Markdown is inserted into the page as HTML.
    ///
    /// Only for text that is part of your app. Given text from a user or a
    /// file, this would let its author run script in your app.
    #[cfg(feature = "markdown")]
    pub fn markdown_with_html(&self, source: impl TextSource) -> Markdown {
        let markdown = Markdown::with_html(source);
        self.append(markdown.get_element());
        markdown
    }

    /// Shows an equation written in LaTeX, centred on its own line.
    ///
    /// It is drawn as MathML, which browsers display themselves: there is
    /// no script or stylesheet to add to `index.html`. A mistake in the
    /// LaTeX is reported in place of the equation rather than failing.
    ///
    /// Pass a `Signal<String>` to update the equation when it changes.
    ///
    /// ```no_run
    /// # use dappr::prelude::*;
    /// # let dapp = Dapp::new();
    /// dapp.latex(r"e^{i\pi} + 1 = 0");
    ///
    /// let n = dapp.slider("n", 1.0, 9.0, 2.0).value();
    /// dapp.latex(n.map(|n| format!(r"\int_0^1 x^{{{n}}} \, dx = \frac{{1}}{{{}}}", n + 1.0)));
    /// ```
    #[cfg(feature = "latex")]
    pub fn latex(&self, source: impl TextSource) -> Latex {
        let latex = Latex::new(source);
        self.append(latex.get_element());
        latex
    }
}
