#![allow(dead_code)]
use dappr::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;
use web_sys::{Event, HtmlElement, HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement};

wasm_bindgen_test_configure!(run_in_browser);

fn fire(el: &HtmlElement, event: &str) {
    el.dispatch_event(&Event::new(event).unwrap()).unwrap();
}

/// What a user typing into (or dragging) an <input> amounts to.
fn user_types(field_control: &HtmlElement, value: &str) {
    field_control
        .unchecked_ref::<HtmlInputElement>()
        .set_value(value);
    fire(field_control, "input");
}

fn as_input(el: &HtmlElement) -> &HtmlInputElement {
    el.unchecked_ref()
}

#[wasm_bindgen_test]
fn test_text_input_is_labelled_and_two_way() {
    let dapp = Dapp::new();
    let name = dapp.text_input("Name").placeholder("Ada");

    assert_eq!(name.get_element().tag_name(), "LABEL");
    assert_eq!(
        name.get_element().inner_html(),
        "<span>Name</span><input type=\"text\" placeholder=\"Ada\">"
    );
    assert_eq!(name.get(), "");

    user_types(name.control(), "Grace");
    assert_eq!(name.get(), "Grace");

    name.set("Linus".to_string());
    assert_eq!(as_input(name.control()).value(), "Linus");
}

#[wasm_bindgen_test]
fn test_textarea_is_two_way() {
    let dapp = Dapp::new();
    let notes = dapp.textarea("Notes").rows(4);
    let area: &HtmlTextAreaElement = notes.control().unchecked_ref();
    assert_eq!(area.rows(), 4);

    area.set_value("line 1\nline 2");
    fire(notes.control(), "input");
    assert_eq!(notes.get(), "line 1\nline 2");

    notes.set("reset".to_string());
    assert_eq!(area.value(), "reset");
}

#[wasm_bindgen_test]
fn test_number_gives_f64_and_ignores_non_numbers() {
    let dapp = Dapp::new();
    let price = dapp.number("Price", 9.5).min(0.0).max(100.0).step(0.5);
    assert_eq!(price.get(), 9.5);
    assert_eq!(as_input(price.control()).value(), "9.5");
    assert_eq!(price.control().get_attribute("min").unwrap(), "0");
    assert_eq!(price.control().get_attribute("step").unwrap(), "0.5");

    user_types(price.control(), "12.25");
    assert_eq!(price.get(), 12.25);

    // Emptying the box does not turn the value into NaN or zero.
    user_types(price.control(), "");
    assert_eq!(price.get(), 12.25);

    price.set(3.0);
    assert_eq!(as_input(price.control()).value(), "3");
}

/// A number field without `.step()` accepts decimals (browsers otherwise
/// treat anything but whole numbers as invalid).
#[wasm_bindgen_test]
fn test_number_accepts_decimals_by_default() {
    let dapp = Dapp::new();
    let n = dapp.number("Any", 0.0);
    user_types(n.control(), "0.123");
    assert_eq!(n.get(), 0.123);
    assert!(as_input(n.control()).check_validity());
}

#[wasm_bindgen_test]
fn test_slider_respects_range_and_shows_its_value() {
    let dapp = Dapp::new();
    // 150 is outside the browser's default 0..100 range: it must survive.
    let level = dapp.slider("Level", 100.0, 200.0, 150.0);
    assert_eq!(level.get(), 150.0);
    assert_eq!(as_input(level.control()).value(), "150");

    let shown = || {
        level
            .get_element()
            .query_selector(".dappr-field-value")
            .unwrap()
            .unwrap()
            .text_content()
            .unwrap()
    };
    assert_eq!(shown(), "150");
    // The label labels the slider, not the value shown beside it.
    let label: &web_sys::HtmlLabelElement = level.get_element().unchecked_ref();
    assert_eq!(label.control().unwrap(), *level.control());

    user_types(level.control(), "175");
    assert_eq!(level.get(), 175.0);
    assert_eq!(shown(), "175");

    level.set(120.0);
    assert_eq!(as_input(level.control()).value(), "120");
    assert_eq!(shown(), "120");
}

/// The control and the value must always agree, whatever the range and
/// step, and in whatever order they are given.
#[wasm_bindgen_test]
fn test_slider_value_and_control_agree_with_fractional_ranges() {
    let dapp = Dapp::new();
    let shown = |field: &Field<f64>| as_input(field.control()).value_as_number();

    // Not on the default step of 1: must not be moved to 1.3.
    let sigma = dapp.slider("Sigma", 0.3, 2.0, 1.0).step(0.1);
    assert_eq!(sigma.get(), 1.0);
    assert_eq!(shown(&sigma), 1.0);

    // No step given, fractional start: kept as is.
    let opacity = dapp.slider("Opacity", 0.0, 1.0, 0.8);
    assert_eq!(opacity.get(), 0.8);
    assert_eq!(shown(&opacity), 0.8);

    // A step the start does not sit on: the value follows the control.
    let coarse = dapp.slider("Coarse", 0.0, 10.0, 3.0).step(2.0);
    assert_eq!(shown(&coarse), coarse.get());
    assert_eq!(coarse.get() % 2.0, 0.0);

    // Narrowing the range clamps both.
    let capped = dapp.slider("Capped", 0.0, 100.0, 80.0).max(50.0);
    assert_eq!(capped.get(), 50.0);
    assert_eq!(shown(&capped), 50.0);
}

#[wasm_bindgen_test]
fn test_checkbox_and_switch_give_bool() {
    let dapp = Dapp::new();
    let agree = dapp.checkbox("I agree", false);
    let dark = dapp.switch("Dark mode", true);

    assert!(!agree.get());
    assert!(dark.get());
    assert!(as_input(dark.control()).checked());
    assert_eq!(dark.control().get_attribute("role").unwrap(), "switch");
    assert!(agree.control().get_attribute("role").is_none());
    // The box comes before its text.
    assert_eq!(
        agree.get_element().inner_html(),
        "<input type=\"checkbox\"><span>I agree</span>"
    );

    agree.control().click(); // a real click toggles and fires `change`
    assert!(agree.get());

    dark.set(false);
    assert!(!as_input(dark.control()).checked());
}

#[wasm_bindgen_test]
fn test_select_value_is_the_option_itself() {
    let dapp = Dapp::new();
    let year = dapp.select("Year", &[2024, 2025, 2026]);
    let select: &HtmlSelectElement = year.control().unchecked_ref();

    assert_eq!(year.get(), 2024);
    assert_eq!(select.length(), 3);

    select.set_selected_index(2);
    fire(year.control(), "change");
    assert_eq!(year.get(), 2026);

    year.set(2025);
    assert_eq!(select.selected_index(), 1);

    // Not one of the options: the dropdown stays put.
    year.set(1999);
    assert_eq!(select.selected_index(), 1);
}

#[wasm_bindgen_test]
fn test_radio_groups_are_independent() {
    let dapp = Dapp::new();
    let size = dapp.radio("Size", &["S", "M", "L"]);
    let colour = dapp.radio("Colour", &["Red", "Blue"]);
    assert_eq!(size.get(), "S");
    assert_eq!(size.get_element().tag_name(), "FIELDSET");

    let radios = |field: &Field<&'static str>| -> Vec<HtmlInputElement> {
        let list = field.get_element().query_selector_all("input").unwrap();
        (0..list.length())
            .map(|i| list.item(i).unwrap().unchecked_into())
            .collect()
    };
    let (size_radios, colour_radios) = (radios(&size), radios(&colour));

    size_radios[2].click();
    assert_eq!(size.get(), "L");
    colour_radios[1].click();
    assert_eq!(colour.get(), "Blue");
    // Choosing a colour did not clear the size.
    assert!(size_radios[2].checked());
    assert_eq!(size.get(), "L");

    size.set("M");
    assert!(size_radios[1].checked());
    assert!(!size_radios[2].checked());
}

#[wasm_bindgen_test]
fn test_with_value_and_disabled() {
    let dapp = Dapp::new();
    let name = dapp.text_input("Name").with_value("Ada".to_string());
    assert_eq!(as_input(name.control()).value(), "Ada");

    let locked = dapp.number("Locked", 1.0).disabled(true);
    assert!(as_input(locked.control()).disabled());
    locked.set_disabled(false);
    assert!(!as_input(locked.control()).disabled());
}

/// Fields work inside layout containers, and their values drive text
/// through `map` and `zip`.
#[wasm_bindgen_test]
fn test_fields_in_containers_drive_derived_text() {
    let dapp = Dapp::new();
    let card = dapp.card();
    card.id("order");
    let price = card.number("Price", 2.0);
    let quantity = card.slider("Quantity", 1.0, 10.0, 3.0);
    let total = price
        .value()
        .zip(&quantity.value())
        .map(|(p, q)| format!("{:.2}", p * q));
    let line = card.p("Total: {}", total.clone(), Some("Total: 6.00"));

    assert_eq!(total.get(), "6.00");
    user_types(quantity.control(), "5");
    assert_eq!(line.get_element().text_content().unwrap(), "Total: 10.00");
    user_types(price.control(), "1.5");
    assert_eq!(line.get_element().text_content().unwrap(), "Total: 7.50");
}
