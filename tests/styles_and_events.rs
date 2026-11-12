#![allow(dead_code)]
use dappr::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;
use web_sys::{window, Document, HtmlElement};

wasm_bindgen_test_configure!(run_in_browser);

style!("testbadge", ".pill { border-radius: 999px; }");

fn document() -> Document {
    window().unwrap().document().unwrap()
}

fn style_elements_containing(needle: &str) -> usize {
    let head = document().head().unwrap();
    let mut count = 0;
    let mut child = head.first_element_child();
    while let Some(el) = child {
        if el.tag_name() == "STYLE" && el.text_content().unwrap_or_default().contains(needle) {
            count += 1;
        }
        child = el.next_element_sibling();
    }
    count
}

/// `Dapp::new()` adds the vendored baseline stylesheet (Pico) exactly once,
/// as the first thing in `<head>`, no matter how many Dapps are created.
#[wasm_bindgen_test]
fn test_base_style_is_injected_once_and_first() {
    let _a = Dapp::new();
    let _b = Dapp::new();

    let first = document().head().unwrap().first_element_child().unwrap();
    assert_eq!(first.id(), "dappr-base-style");
    assert!(first.text_content().unwrap().contains("--pico-"));
    assert_eq!(style_elements_containing("--pico-font-family"), 1);
}

/// The class constant `style!` generates and the selector in the CSS it
/// generates are the same scoped name, so the rule really applies.
#[wasm_bindgen_test]
fn test_style_macro_constant_matches_injected_css() {
    assert_eq!(testbadge::pill, "testbadge-pill");
    assert!(testbadge::CSS.contains(".testbadge-pill{"));

    let dapp = Dapp::new();
    dapp.style(testbadge::CSS);
    dapp.style(testbadge::CSS); // no-op the second time
    assert_eq!(style_elements_containing(".testbadge-pill{"), 1);

    let button = dapp.button("Scoped");
    button.add_class(testbadge::pill);
    let radius = window()
        .unwrap()
        .get_computed_style(button.get_element())
        .unwrap()
        .unwrap()
        .get_property_value("border-radius")
        .unwrap();
    assert_eq!(radius, "999px");
}

/// A button keeps working after its internal listener is `.detach()`ed:
/// every click runs the `pressed` callback and sets the signal.
#[wasm_bindgen_test]
fn test_button_click_runs_callback_every_time() {
    let dapp = Dapp::new();
    let button = dapp.button("Click me");
    let clicks = Signal::new(0usize);
    let clicks_in_callback = clicks.clone();
    button.pressed(move || clicks_in_callback.set(clicks_in_callback.get() + 1));

    button.get_element().click();
    button.get_element().click();

    assert_eq!(clicks.get(), 2);
    assert!(button.get_signal().get());
}

/// Dropping the `Subscription` from `add_event_listener` removes the
/// listener; `.detach()` keeps it.
#[wasm_bindgen_test]
fn test_event_listener_subscription_drop_and_detach() {
    let dapp = Dapp::new();
    let dropped = dapp.button("dropped");
    let kept = dapp.button("kept");
    let (dropped_hits, kept_hits) = (Signal::new(0), Signal::new(0));

    let hits = dropped_hits.clone();
    let sub = dropped.add_event_listener("click", move |_| hits.set(hits.get() + 1));
    let hits = kept_hits.clone();
    kept.add_event_listener("click", move |_| hits.set(hits.get() + 1))
        .detach();

    dropped.get_element().click();
    drop(sub);
    dropped.get_element().click();
    kept.get_element().click();
    kept.get_element().click();

    assert_eq!(dropped_hits.get(), 1);
    assert_eq!(kept_hits.get(), 2);
}

/// A `pressed` callback may add components (which borrows the global Dapp
/// again) and subscribe further callbacks to the same button.
#[wasm_bindgen_test]
fn test_pressed_callback_can_use_dapp_and_resubscribe() {
    dapp!(dapp, {
        let button = Rc::new(dapp.button("Outer"));
        let inner_hits = Signal::new(0);

        let (button_in, hits) = (Rc::clone(&button), inner_hits.clone());
        button.pressed(move || {
            text!("added from a callback");
            let hits = hits.clone();
            button_in.pressed(move || hits.set(hits.get() + 1));
        });

        let el: HtmlElement = button.get_element().clone().unchecked_into();
        el.click(); // subscribes inner #1
        el.click(); // runs inner #1, subscribes inner #2
        assert_eq!(inner_hits.get(), 1);
    });
    let body = document().body().unwrap().inner_html();
    assert!(body.contains("added from a callback"));
}
