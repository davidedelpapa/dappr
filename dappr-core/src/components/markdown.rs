//! ## Markdown and LaTeX
//!
//! - [`Markdown`] shows Markdown text: headings, lists, tables, links, code,
//!   and (with the `latex` feature) math written as `$...$` and `$$...$$`.
//! - [`Latex`] shows one equation written in LaTeX.
//!
//! Both follow a `Signal<String>` when given one.
//!
//! Markdown is **safe by default**: raw HTML inside the text is shown as
//! text, not inserted, and links that would run script are neutralised. That
//! makes it fine for text from users, files and APIs.
//!
//! Math is rendered as MathML, which browsers display natively: no
//! JavaScript library, no stylesheet link and no fonts to load.
// Both components come from rustream (src/components/markdown.rs and
// latex.rs). Kept from its `<Markdown>`: pulldown-cmark, and the tables,
// footnotes, strikethrough and task-list extensions being on. Changed: raw
// HTML is no longer passed through by default, and `<Latex>` no longer needs
// the KaTeX script and stylesheet in index.html -- it produces MathML with
// pulldown-latex instead.
use crate::communication::signals::Signal;
use crate::framework::dom;
use crate::framework::traits::{Component, TextSource};
use web_sys::HtmlElement;

// ---- LaTeX ------------------------------------------------------------------

/// Whether an equation stands on its own line or sits within a sentence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MathDisplay {
    /// Centred on its own line, with full-size sums and integrals.
    Block,
    /// Within a line of text, sized to fit it.
    Inline,
}

/// The MathML elements an equation is made of. Anything else in the
/// renderer's output is not markup we asked for.
#[cfg(feature = "latex")]
const MATHML_ELEMENTS: &[&str] = &[
    "math",
    "mrow",
    "mi",
    "mn",
    "mo",
    "mtext",
    "mspace",
    "ms",
    "mfrac",
    "msqrt",
    "mroot",
    "mstyle",
    "merror",
    "mpadded",
    "mphantom",
    "msub",
    "msup",
    "msubsup",
    "munder",
    "mover",
    "munderover",
    "mmultiscripts",
    "mprescripts",
    "none",
    "mtable",
    "mtr",
    "mtd",
    "menclose",
    "semantics",
];
/// The attributes those elements may carry. No event handlers, no links.
#[cfg(feature = "latex")]
const MATHML_ATTRIBUTES: &[&str] = &[
    "accent",
    "accentunder",
    "class",
    "columnalign",
    "columnspacing",
    "depth",
    "display",
    "displaystyle",
    "fence",
    "form",
    "height",
    "largeop",
    "linethickness",
    "lspace",
    "mathsize",
    "mathvariant",
    "maxsize",
    "minsize",
    "movablelimits",
    "notation",
    "rowspacing",
    "rspace",
    "scriptlevel",
    "separator",
    "stretchy",
    "style",
    "symmetric",
    "voffset",
    "width",
];
/// The only CSS properties the renderer sets inline.
#[cfg(feature = "latex")]
const MATHML_STYLE_PROPERTIES: &[&str] = &[
    "color",
    "border",
    "border-color",
    "background-color",
    "margin-left",
    "height",
];

/// If `rest` (which starts with `<`) begins with a tag made only of allowed
/// elements, attributes and values, returns the tag's length in bytes.
#[cfg(feature = "latex")]
fn allowed_tag_len(rest: &str) -> Option<usize> {
    let bytes = rest.as_bytes();
    let mut i = 1;
    let closing = bytes.get(i) == Some(&b'/');
    if closing {
        i += 1;
    }
    let name_start = i;
    while bytes.get(i).is_some_and(u8::is_ascii_lowercase) {
        i += 1;
    }
    if !MATHML_ELEMENTS.contains(&&rest[name_start..i]) {
        return None;
    }
    loop {
        let before_spaces = i;
        while bytes.get(i) == Some(&b' ') {
            i += 1;
        }
        match bytes.get(i)? {
            b'>' => return Some(i + 1),
            b'/' if !closing && bytes.get(i + 1) == Some(&b'>') => return Some(i + 2),
            // An attribute: only on an opening tag, and only after a space.
            _ if closing || i == before_spaces => return None,
            _ => {}
        }
        let attr_start = i;
        while bytes.get(i).is_some_and(u8::is_ascii_lowercase) {
            i += 1;
        }
        let name = &rest[attr_start..i];
        if !MATHML_ATTRIBUTES.contains(&name) || bytes.get(i) != Some(&b'=') {
            return None;
        }
        if bytes.get(i + 1) != Some(&b'"') {
            return None;
        }
        let value_start = i + 2;
        let value_len = rest[value_start..].find('"')?;
        let value = &rest[value_start..value_start + value_len];
        let plain = |text: &str, extra: &str| {
            text.chars()
                .all(|c| c.is_ascii_alphanumeric() || extra.contains(c))
        };
        let value_ok = if name == "style" {
            value.split(';').all(|declaration| {
                let declaration = declaration.trim();
                declaration.is_empty()
                    || declaration
                        .split_once(':')
                        .is_some_and(|(property, value)| {
                            MATHML_STYLE_PROPERTIES.contains(&property.trim())
                                && plain(value, " .%#(),+-")
                                && !value.to_ascii_lowercase().contains("url")
                        })
            })
        } else {
            plain(value, " .%+-_")
        };
        if !value_ok {
            return None;
        }
        i = value_start + value_len + 1;
    }
}

/// Makes the renderer's MathML safe to insert into the page.
///
/// The renderer copies some of its input straight into its output (the
/// content of `\text{..}` and `\operatorname{..}`, a bare `<`), so LaTeX from
/// an untrusted source could otherwise smuggle in markup. This keeps the
/// tags that are plain MathML with harmless attributes, and turns every
/// other `<`, `>` and `&` into text.
#[cfg(feature = "latex")]
fn sanitize_mathml(mathml: &str) -> String {
    let mut out = String::with_capacity(mathml.len());
    let mut rest = mathml;
    while let Some(position) = rest.find(['<', '>', '&']) {
        out.push_str(&rest[..position]);
        rest = &rest[position..];
        let taken = match rest.as_bytes()[0] {
            b'<' => match allowed_tag_len(rest) {
                Some(len) => {
                    out.push_str(&rest[..len]);
                    len
                }
                None => {
                    out.push_str("&lt;");
                    1
                }
            },
            b'>' => {
                out.push_str("&gt;");
                1
            }
            _ => {
                // Keep a well-formed character reference; escape a bare `&`.
                let reference = rest[1..].find(';').filter(|end| {
                    let body = &rest[1..1 + end];
                    (1..=32).contains(&body.len())
                        && (body.chars().all(|c| c.is_ascii_alphanumeric())
                            || body.strip_prefix('#').is_some_and(|digits| {
                                !digits.is_empty()
                                    && digits.chars().all(|c| c.is_ascii_alphanumeric())
                            }))
                });
                match reference {
                    Some(end) => {
                        out.push_str(&rest[..end + 2]);
                        end + 2
                    }
                    None => {
                        out.push_str("&amp;");
                        1
                    }
                }
            }
        };
        rest = &rest[taken..];
    }
    out.push_str(rest);
    out
}

/// Renders a LaTeX equation as MathML markup that is safe to insert into
/// the page, whatever the LaTeX contains.
///
/// Mistakes in the LaTeX do not fail: the problem is shown in place, in red,
/// where the equation would be.
#[cfg(feature = "latex")]
pub fn latex_to_mathml(latex: &str, display: MathDisplay) -> String {
    use pulldown_latex::config::{DisplayMode, RenderConfig};
    use pulldown_latex::{mathml::push_mathml, Parser, Storage};

    let storage = Storage::new();
    let parser = Parser::new(latex, &storage);
    let config = RenderConfig {
        display_mode: match display {
            MathDisplay::Block => DisplayMode::Block,
            MathDisplay::Inline => DisplayMode::Inline,
        },
        ..RenderConfig::default()
    };
    let mut mathml = String::new();
    match push_mathml(&mut mathml, parser, config) {
        Ok(()) => sanitize_mathml(&mathml),
        Err(_) => format!("<code>{}</code>", crate::charts::escape(latex)),
    }
}

/// An equation on the page, written in LaTeX and shown on its own line.
///
/// ```ignore
/// dapp.latex(r"\bar{x} = \frac{1}{n} \sum_{i=1}^{n} x_i");
/// ```
#[cfg(feature = "latex")]
pub struct Latex {
    el: HtmlElement,
    source: Signal<String>,
}

#[cfg(feature = "latex")]
impl Latex {
    /// Shows `source`: LaTeX text, or a `Signal<String>` of it.
    pub fn new(source: impl TextSource) -> Self {
        dom::inject_style(LATEX_CSS);
        let source = source.into_text_signal();
        let el = dom::create_element("div");
        el.set_class_name("dappr-latex");
        // MathML from `latex_to_mathml`, which lets only plain MathML through.
        el.set_inner_html(&latex_to_mathml(&source.get(), MathDisplay::Block));
        let el_in = el.clone();
        source
            .subscribe(move |latex| {
                el_in.set_inner_html(&latex_to_mathml(latex, MathDisplay::Block))
            })
            .detach();
        Latex { el, source }
    }

    /// The LaTeX being shown. Set it to change the equation.
    pub fn source(&self) -> Signal<String> {
        self.source.clone()
    }
}

#[cfg(feature = "latex")]
impl Component for Latex {
    fn get_element(&self) -> &HtmlElement {
        &self.el
    }
}

/// Layout rules for the MathML (see the file for where they come from).
#[cfg(feature = "latex")]
const LATEX_CSS: &str = include_str!("../assets/latex.css");

// ---- Markdown ---------------------------------------------------------------

/// Whether a link or image address is safe to put in a page: `http(s)`,
/// `mailto`, or relative. Anything else -- `javascript:` above all -- is not.
#[cfg(feature = "markdown")]
fn is_safe_url(url: &str) -> bool {
    // Browsers ignore whitespace and control characters inside the scheme,
    // so "java\tscript:" must be judged as "javascript:".
    let cleaned: String = url
        .chars()
        .filter(|c| !c.is_whitespace() && !c.is_control())
        .collect::<String>()
        .to_lowercase();
    match cleaned.split_once(':') {
        // A colon after a slash, ? or # is part of a path, not a scheme.
        Some((scheme, _)) if !scheme.contains(['/', '?', '#']) => {
            matches!(scheme, "http" | "https" | "mailto")
        }
        _ => true,
    }
}

/// Renders Markdown as HTML markup.
///
/// With `allow_html` false (what [`Markdown::new`] uses), raw HTML in the
/// text comes out as visible text and unsafe link addresses are replaced
/// with `#`, so the result is safe to insert whatever the text contains.
#[cfg(feature = "markdown")]
pub fn markdown_to_html(markdown: &str, allow_html: bool) -> String {
    use pulldown_cmark::{html, CowStr, Event, Options, Parser, Tag};

    let mut options = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_GFM;
    // Without the `latex` feature, dollar signs stay ordinary characters.
    if cfg!(feature = "latex") {
        options |= Options::ENABLE_MATH;
    }

    let safe = |url: CowStr<'_>| -> CowStr<'static> {
        if allow_html || is_safe_url(&url) {
            url.into_static()
        } else {
            CowStr::Borrowed("#")
        }
    };
    let events = Parser::new_ext(markdown, options).map(|event| match event {
        // Raw HTML becomes text, which the HTML writer escapes.
        Event::Html(html) | Event::InlineHtml(html) if !allow_html => Event::Text(html),
        Event::Start(Tag::Link {
            link_type,
            dest_url,
            title,
            id,
        }) => Event::Start(Tag::Link {
            link_type,
            dest_url: safe(dest_url),
            title,
            id,
        }),
        Event::Start(Tag::Image {
            link_type,
            dest_url,
            title,
            id,
        }) => Event::Start(Tag::Image {
            link_type,
            dest_url: safe(dest_url),
            title,
            id,
        }),
        #[cfg(feature = "latex")]
        Event::InlineMath(latex) => {
            Event::InlineHtml(latex_to_mathml(&latex, MathDisplay::Inline).into())
        }
        #[cfg(feature = "latex")]
        Event::DisplayMath(latex) => {
            Event::InlineHtml(latex_to_mathml(&latex, MathDisplay::Block).into())
        }
        other => other,
    });

    let mut out = String::new();
    html::push_html(&mut out, events);
    out
}

/// Markdown text on the page.
///
/// ```ignore
/// dapp.markdown("## Results\n\nThe mean is **4.2** (see the [method](#method)).");
/// ```
#[cfg(feature = "markdown")]
pub struct Markdown {
    el: HtmlElement,
    source: Signal<String>,
}

#[cfg(feature = "markdown")]
impl Markdown {
    fn build(source: impl TextSource, allow_html: bool) -> Self {
        #[cfg(feature = "latex")]
        dom::inject_style(LATEX_CSS);
        let source = source.into_text_signal();
        let el = dom::create_element("div");
        el.set_class_name("dappr-markdown");
        el.set_inner_html(&markdown_to_html(&source.get(), allow_html));
        let el_in = el.clone();
        source
            .subscribe(move |markdown| {
                el_in.set_inner_html(&markdown_to_html(markdown, allow_html))
            })
            .detach();
        Markdown { el, source }
    }

    /// Shows `source`: Markdown text, or a `Signal<String>` of it. Raw HTML
    /// in the text is shown as text, so this is safe for text from anywhere.
    pub fn new(source: impl TextSource) -> Self {
        Self::build(source, false)
    }

    /// Like [`new`](Markdown::new), but HTML written inside the Markdown is
    /// inserted into the page as HTML. Only for text you wrote yourself:
    /// with text from a user or a file, this lets them run script in your app.
    pub fn with_html(source: impl TextSource) -> Self {
        Self::build(source, true)
    }

    /// The Markdown being shown. Set it to change the text.
    pub fn source(&self) -> Signal<String> {
        self.source.clone()
    }
}

#[cfg(feature = "markdown")]
impl Component for Markdown {
    fn get_element(&self) -> &HtmlElement {
        &self.el
    }
}

#[cfg(all(test, feature = "markdown"))]
mod tests {
    use super::*;

    #[test]
    fn renders_the_usual_markdown() {
        let html = markdown_to_html(
            "# Title\n\nSome **bold** and `code`.\n\n- one\n- two",
            false,
        );
        assert!(html.contains("<h1>Title</h1>"));
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<code>code</code>"));
        assert!(html.contains("<li>one</li>"));
    }

    #[test]
    fn renders_tables_strikethrough_and_task_lists() {
        let html = markdown_to_html(
            "| a | b |\n|---|---|\n| 1 | 2 |\n\n~~gone~~\n\n- [x] done",
            false,
        );
        assert!(html.contains("<table>") && html.contains("<td>1</td>"));
        assert!(html.contains("<del>gone</del>"));
        assert!(html.contains("type=\"checkbox\""));
    }

    #[test]
    fn raw_html_is_shown_as_text_by_default() {
        let text = "Hi <script>alert(1)</script> <b>there</b>\n\n<img src=x onerror=alert(2)>";
        let safe = markdown_to_html(text, false);
        assert!(!safe.contains("<script>") && !safe.contains("<img") && !safe.contains("<b>"));
        assert!(safe.contains("&lt;script&gt;"));
        assert!(safe.contains("&lt;img src=x onerror=alert(2)&gt;"));

        let trusted = markdown_to_html(text, true);
        assert!(trusted.contains("<b>there</b>"));
    }

    #[test]
    fn script_links_are_neutralised() {
        for bad in [
            "[x](javascript:alert(1))",
            "[x](JavaScript:alert(1))",
            "[x](<java\tscript:alert(1)>)",
            "[x](data:text/html,<script>alert(1)</script>)",
            "[x](vbscript:msgbox(1))",
            "![x](javascript:alert(1))",
        ] {
            let html = markdown_to_html(bad, false);
            assert!(!html.to_lowercase().contains("script:"), "{bad} -> {html}");
            assert!(!html.contains("data:"), "{bad} -> {html}");
        }
        for good in [
            "[x](https://example.com/a?b=c:d)",
            "[x](/docs/page#part)",
            "[x](page.html)",
            "[x](mailto:a@example.com)",
            "[x](#top)",
        ] {
            let html = markdown_to_html(good, false);
            assert!(!html.contains("href=\"#\""), "{good} -> {html}");
        }
        assert!(is_safe_url("docs/a:b"));
        assert!(!is_safe_url(" javascript:alert(1)"));
    }

    #[cfg(feature = "latex")]
    #[test]
    fn math_in_markdown_becomes_mathml() {
        let html = markdown_to_html("Inline $x^2$ and\n\n$$\\frac{a}{b}$$", false);
        assert_eq!(html.matches("<math").count(), 2);
        assert!(html.contains("display=\"block\""));
        assert!(html.contains("<mfrac>"));
        // A lone dollar amount is not math.
        let money = markdown_to_html("It costs $5 today.", false);
        assert!(!money.contains("<math") && money.contains("$5"));
    }

    #[cfg(feature = "latex")]
    #[test]
    fn latex_renders_and_survives_mistakes() {
        let block = latex_to_mathml(r"\sum_{i=1}^{n} x_i", MathDisplay::Block);
        assert!(block.starts_with("<math") && block.contains("display=\"block\""));
        assert!(block.contains("<munderover") || block.contains("<msubsup"));
        let inline = latex_to_mathml("a+b", MathDisplay::Inline);
        assert!(inline.contains("display=\"inline\""));
        // Colours and environments keep their styling through the sanitiser.
        let coloured = latex_to_mathml(r"\textcolor{red}{x}", MathDisplay::Inline);
        assert!(coloured.contains("style=\"color: rgb("), "{coloured}");
        let aligned = latex_to_mathml(
            r"\begin{aligned} a &= b \\ c &= d \end{aligned}",
            MathDisplay::Block,
        );
        assert!(
            aligned.contains("<mtable") && aligned.contains("class=\""),
            "{aligned}"
        );

        // Unknown command, unbalanced braces: still markup, with the error shown.
        for bad in [r"\notacommand{x}", r"\frac{a}{", "x^"] {
            let out = latex_to_mathml(bad, MathDisplay::Block);
            assert!(
                out.contains("<merror") || out.contains("<code>"),
                "{bad} -> {out}"
            );
        }
    }

    /// LaTeX from an untrusted source must not be able to add markup.
    #[cfg(feature = "latex")]
    #[test]
    fn latex_cannot_inject_markup() {
        // Every tag left in the output must be plain MathML.
        fn tags(markup: &str) -> Vec<String> {
            markup
                .split('<')
                .skip(1)
                .map(|piece| {
                    piece
                        .trim_start_matches('/')
                        .chars()
                        .take_while(|c| c.is_ascii_alphanumeric())
                        .collect()
                })
                .collect()
        }
        for hostile in [
            r#"\text{</math><script>alert(1)</script>}"#,
            r#"\operatorname{<img src=x onerror=alert(1)>}"#,
            r#"\operatorname{<b>}"#,
            r#"\text{<svg onload=alert(1)>}"#,
            r#"\text{<a href="javascript:alert(1)">x</a>}"#,
            r#"\text{<mi onclick="alert(1)">x</mi>}"#,
            r#"\text{<mi style="background: url(https://evil.example/x)">x</mi>}"#,
            r#"\text{<mi style="position: fixed; top: 0">x</mi>}"#,
            r#"\text{<math href="javascript:alert(1)">x</math>}"#,
            r#"\text{<annotation-xml encoding="text/html"><img src=x></annotation-xml>}"#,
            r#"\text{<mi class='a' onmouseover=alert(1)>}"#,
            r#"x < y > z"#,
            r#"\text{a & b &lt; c &#60;script&#62;}"#,
        ] {
            for display in [MathDisplay::Block, MathDisplay::Inline] {
                let out = latex_to_mathml(hostile, display);
                for tag in tags(&out) {
                    assert!(
                        MATHML_ELEMENTS.contains(&tag.as_str()),
                        "<{tag}> from {hostile}: {out}"
                    );
                }
                let lower = out.to_lowercase();
                for needle in [
                    "onerror=\"",
                    "onclick=\"",
                    "onload=\"",
                    "href=\"",
                    "url(",
                    "position:",
                ] {
                    // These may appear as visible text, never inside a tag.
                    for tag in lower.split('<').skip(1).filter_map(|p| p.split('>').next()) {
                        assert!(
                            !tag.contains(needle),
                            "{needle} in a tag from {hostile}: {out}"
                        );
                    }
                }
            }
        }
        // Through Markdown as well.
        let html = markdown_to_html(r#"$\operatorname{<img src=x onerror=alert(1)>}$"#, false);
        assert!(!html.contains("<img"), "{html}");
    }

    #[cfg(feature = "latex")]
    #[test]
    fn sanitizer_keeps_plain_mathml_and_escapes_the_rest() {
        let ok = r#"<math display="block"><mrow><mi mathvariant="normal">x</mi><mspace width="1em"/><mo stretchy="false">(</mo></mrow></math>"#;
        assert_eq!(sanitize_mathml(ok), ok);
        assert_eq!(sanitize_mathml("<mo><</mo>"), "<mo>&lt;</mo>");
        assert_eq!(sanitize_mathml("<mo>></mo>"), "<mo>&gt;</mo>");
        assert_eq!(
            sanitize_mathml("a & b &amp; &#x3C; &notanentity"),
            "a &amp; b &amp; &#x3C; &amp;notanentity"
        );
        assert_eq!(sanitize_mathml("<mi><b></mi>"), "<mi>&lt;b&gt;</mi>");
        assert_eq!(
            sanitize_mathml(r#"<mi onclick="x">"#),
            r#"&lt;mi onclick="x"&gt;"#
        );
        assert_eq!(
            sanitize_mathml(r#"<mi style="color: rgb(1 2 3); border: 0.06em solid rgb(1 2 3)">"#),
            r#"<mi style="color: rgb(1 2 3); border: 0.06em solid rgb(1 2 3)">"#
        );
        assert!(sanitize_mathml(r#"<mi style="background: red">"#).starts_with("&lt;"));
        assert!(sanitize_mathml(r#"<mi style="color: url(x)">"#).starts_with("&lt;"));
        assert!(sanitize_mathml("<mi class=a>").starts_with("&lt;"));
        assert!(sanitize_mathml("<mi").starts_with("&lt;"));
        assert!(sanitize_mathml("</mi onclick=\"x\">").starts_with("&lt;"));
        assert!(sanitize_mathml("<mix>").starts_with("&lt;"));
    }
}
