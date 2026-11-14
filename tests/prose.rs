#![allow(dead_code)]
use dappr::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;
use web_sys::{Element, HtmlElement};

wasm_bindgen_test_configure!(run_in_browser);

const MATHML: &str = "http://www.w3.org/1998/Math/MathML";

fn one(root: &Element, selector: &str) -> HtmlElement {
    root.query_selector(selector)
        .unwrap()
        .unwrap_or_else(|| panic!("nothing matches {selector}"))
        .unchecked_into()
}

fn count(root: &Element, selector: &str) -> u32 {
    root.query_selector_all(selector).unwrap().length()
}

#[wasm_bindgen_test]
fn test_markdown_becomes_real_elements() {
    let dapp = Dapp::new();
    let md = dapp.markdown(
        "## Title\n\nSome **bold**, a [link](https://example.com).\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\n- [x] done",
    );
    let el = md.get_element();
    assert_eq!(one(el, "h2").text_content().unwrap(), "Title");
    assert_eq!(one(el, "strong").text_content().unwrap(), "bold");
    assert_eq!(
        one(el, "a").get_attribute("href").unwrap(),
        "https://example.com"
    );
    assert_eq!(count(el, "table td"), 2);
    assert_eq!(count(el, "li input[type=checkbox]"), 1);
}

/// Text from anywhere can be given to `markdown`: nothing in it runs.
#[wasm_bindgen_test]
fn test_markdown_from_untrusted_text_is_inert() {
    let dapp = Dapp::new();
    let hostile = "Hi <img src=x onerror=\"window.pwned=1\"> <b>there</b>\n\n\
                   <script>window.pwned = 1</script>\n\n\
                   [click](javascript:window.pwned=1) ![pic](javascript:window.pwned=1)\n\n\
                   $\\operatorname{<img src=x onerror=window.pwned=1>}$";
    let md = dapp.markdown(hostile);
    let el = md.get_element();

    assert_eq!(count(el, "script, b, [onerror], [onclick]"), 0);
    // The only image is the Markdown one, pointing nowhere.
    assert_eq!(count(el, "img"), 1);
    assert_eq!(one(el, "img").get_attribute("src").unwrap(), "#");
    assert_eq!(one(el, "a").get_attribute("href").unwrap(), "#");
    // The HTML is visible as text instead.
    assert!(el.text_content().unwrap().contains("<b>there</b>"));
    let pwned = js_sys::Reflect::get(&web_sys::window().unwrap(), &"pwned".into()).unwrap();
    assert!(pwned.is_undefined());
}

#[wasm_bindgen_test]
fn test_markdown_with_html_inserts_trusted_html() {
    let dapp = Dapp::new();
    let md = dapp.markdown_with_html("Water is H<sub>2</sub>O, <mark>really</mark>.");
    assert_eq!(one(md.get_element(), "sub").text_content().unwrap(), "2");
    assert_eq!(count(md.get_element(), "mark"), 1);
}

#[wasm_bindgen_test]
fn test_markdown_follows_a_signal() {
    let dapp = Dapp::new();
    let notes = dapp.textarea("Notes");
    let preview = dapp.markdown(notes.value());
    assert_eq!(preview.get_element().child_element_count(), 0);

    notes.set("# One".to_string());
    assert_eq!(
        one(preview.get_element(), "h1").text_content().unwrap(),
        "One"
    );
    notes.set("*two*".to_string());
    assert_eq!(count(preview.get_element(), "h1"), 0);
    assert_eq!(
        one(preview.get_element(), "em").text_content().unwrap(),
        "two"
    );
}

/// Math is real MathML that the browser lays out, inline and as a block.
#[wasm_bindgen_test]
fn test_math_in_markdown_is_native_mathml() {
    let dapp = Dapp::new();
    let md = dapp.markdown("Inline $x^2$ here.\n\n$$\\frac{a}{b}$$\n\nPrice: $5 today.");
    let el = md.get_element();
    assert_eq!(count(el, "math"), 2);

    let fraction = one(el, "mfrac");
    assert_eq!(fraction.namespace_uri().unwrap(), MATHML);
    // Laid out as a fraction: numerator above denominator.
    let numerator = fraction
        .first_element_child()
        .unwrap()
        .get_bounding_client_rect();
    let denominator = fraction
        .last_element_child()
        .unwrap()
        .get_bounding_client_rect();
    assert!(numerator.bottom() <= denominator.top() + 1.0);

    assert_eq!(
        one(el, "math[display=block]").tag_name().to_lowercase(),
        "math"
    );
    assert!(el.text_content().unwrap().contains("$5 today"));
}

#[wasm_bindgen_test]
fn test_latex_renders_follows_a_signal_and_reports_mistakes() {
    let dapp = Dapp::new();
    let power = Signal::new(2);
    let equation = dapp.latex(power.map(|n| format!("x^{{{n}}}")));
    let el = equation.get_element();

    let math = one(el, "math");
    assert_eq!(math.namespace_uri().unwrap(), MATHML);
    assert_eq!(math.get_attribute("display").unwrap(), "block");
    assert_eq!(one(el, "msup").text_content().unwrap(), "x2");
    power.set(7);
    assert_eq!(one(el, "msup").text_content().unwrap(), "x7");

    let broken = dapp.latex(r"\frac{1}{");
    assert_eq!(count(broken.get_element(), "merror"), 1);
}

#[wasm_bindgen_test]
fn test_latex_from_untrusted_text_is_inert() {
    let dapp = Dapp::new();
    let equation = dapp.latex(
        r#"\text{<img src=x onerror="window.pwned2=1">} \operatorname{<svg onload=window.pwned2=1>}"#,
    );
    let el = equation.get_element();
    assert_eq!(count(el, "img, svg, [onerror], [onload]"), 0);
    assert!(el.text_content().unwrap().contains("<img src=x"));
    let pwned = js_sys::Reflect::get(&web_sys::window().unwrap(), &"pwned2".into()).unwrap();
    assert!(pwned.is_undefined());
}

#[wasm_bindgen_test]
fn test_text_macro_md_and_latex_specifiers() {
    let total = Signal::new(3);
    dapp!(dapp, {
        let card = dapp.card();
        card.id("prose-macro");
        card.with(|_| {
            text!("Some *emphasis*", _md);
            text!("Total: **{}**", total.clone(), _md);
            text!(r"\sqrt{2}", _latex);
            text!(r"n = {}", total.clone(), _latex);
        });
    });
    let card: Element = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .get_element_by_id("prose-macro")
        .unwrap();
    assert_eq!(one(&card, "em").text_content().unwrap(), "emphasis");
    assert_eq!(one(&card, "strong").text_content().unwrap(), "3");
    assert_eq!(count(&card, "msqrt"), 1);
    assert_eq!(count(&card, "math"), 2);

    total.set(8);
    assert_eq!(one(&card, "strong").text_content().unwrap(), "8");
    assert!(card
        .query_selector_all("math")
        .unwrap()
        .item(1)
        .unwrap()
        .text_content()
        .unwrap()
        .contains('8'));
}
