//! What is tied to a component ends when the component leaves the page.
#![allow(dead_code)]
use dappr::framework::dom::bound_count;
use dappr::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_test::*;
use web_sys::{Event, HtmlInputElement};

wasm_bindgen_test_configure!(run_in_browser);

/// Lets the browser deliver its "the page changed" notifications.
async fn settle() {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 20)
            .unwrap();
    });
    JsFuture::from(promise).await.unwrap();
}

fn people() -> DTable {
    DTable::from_csv("name,age\nAda,36\nAlan,41").unwrap()
}

#[wasm_bindgen_test]
fn test_unmount_ends_the_link_to_a_signal() {
    let dapp = Dapp::new();
    let name = Signal::new("Ada".to_string());
    let text = dapp.p("Hello, {}!", name.clone(), None);
    assert_eq!(name.subscriber_count(), 1);
    name.set("Alan".to_string());
    assert_eq!(text.get_element().text_content().unwrap(), "Hello, Alan!");

    text.unmount();
    assert_eq!(name.subscriber_count(), 0);
    // The removed element is left alone from now on.
    name.set("Grace".to_string());
    assert_eq!(text.get_element().text_content().unwrap(), "Hello, Alan!");
}

/// A signal derived only for a component goes with it.
#[wasm_bindgen_test]
fn test_unmount_frees_signals_derived_for_the_component() {
    let dapp = Dapp::new();
    let price = Signal::new(2.0);
    let quantity = Signal::new(3.0);
    // No handle to the derived signals is kept anywhere but the component.
    let total = dapp.p("Total: {}", price.zip(&quantity).map(|(p, q)| p * q), None);
    assert_eq!(
        (price.subscriber_count(), quantity.subscriber_count()),
        (1, 1)
    );
    quantity.set(5.0);
    assert_eq!(total.get_element().text_content().unwrap(), "Total: 10");

    total.unmount();
    assert_eq!(
        (price.subscriber_count(), quantity.subscriber_count()),
        (0, 0)
    );
}

/// Every kind of component that follows a signal lets go of it.
#[wasm_bindgen_test]
fn test_clear_releases_everything_in_a_container() {
    let dapp = Dapp::new();
    let data = Signal::new(people());
    let text = Signal::new("# Title".to_string());
    let number = Signal::new(1.0);
    let state: Signal<Loadable<DTable>> = Signal::new(Loadable::Loading);
    let before = bound_count();

    let container = dapp.container();
    container.table(&data);
    container.line_chart(&data, "name", &["age"]);
    container.histogram(&data, "age");
    container.markdown(&text);
    container.latex(text.map(|_| "x^2".to_string()));
    container.svg(text.map(|_| "<svg></svg>".to_string()));
    container
        .metric("Count", number.clone())
        .delta(&number, "%");
    container.load_status(&state);
    container.text("{}", number.clone(), None);
    container
        .canvas(10, 10)
        .draw_on(&number, |canvas, _| canvas.clear());
    // Nested, to check that clearing reaches all the way down.
    let [left, _right] = container.columns();
    left.card().table(&data);

    assert!(
        data.subscriber_count() >= 4,
        "table, two charts, nested table"
    );
    assert!(text.subscriber_count() >= 3);
    assert!(number.subscriber_count() >= 4);
    assert_eq!(state.subscriber_count(), 1);
    assert!(bound_count() > before);

    container.clear();
    assert_eq!(container.get_element().child_element_count(), 0);
    assert_eq!(data.subscriber_count(), 0);
    assert_eq!(text.subscriber_count(), 0);
    assert_eq!(number.subscriber_count(), 0);
    assert_eq!(state.subscriber_count(), 0);
    assert_eq!(bound_count(), before);

    // The container itself is still usable.
    container.table(&data);
    assert_eq!(data.subscriber_count(), 1);
    container.unmount();
    assert_eq!(data.subscriber_count(), 0);
}

/// The scenario that used to leak: a callback rebuilding part of the page
/// from long-lived signals, again and again.
#[wasm_bindgen_test]
fn test_rebuilding_a_region_does_not_accumulate() {
    let dapp = Dapp::new();
    let data = Signal::new(people());
    let threshold = Signal::new(0.0);
    let region = dapp.container();
    let before = bound_count();

    let (after_first, mut after_last) = (std::cell::Cell::new(None), (0, 0, 0));
    for round in 0..40 {
        region.clear();
        let shown = computed!(data, threshold => {
            data.filter(|row| row.number("age") >= Some(threshold))
        });
        region.metric("Rows", shown.map(|t| t.n_rows().to_string()));
        region.table(&shown);
        region.bar_chart(&shown, "name", &["age"]);
        let slider = region.slider("Minimum age", 0.0, 100.0, 0.0);
        region.text("At least {}", slider.value(), None);

        threshold.set(f64::from(round));
        after_last = (
            data.subscriber_count(),
            threshold.subscriber_count(),
            bound_count() - before,
        );
        if after_first.get().is_none() {
            after_first.set(Some(after_last));
        }
    }
    // Round 40 holds exactly what round 1 did.
    assert_eq!(Some(after_last), after_first.get());
    assert_eq!(after_last.0, 1, "one computed signal listens to the data");

    region.clear();
    assert_eq!(
        (data.subscriber_count(), threshold.subscriber_count()),
        (0, 0)
    );
    assert_eq!(bound_count(), before);
}

/// Bindings follow the page even when other code removes the element.
#[wasm_bindgen_test]
async fn test_removal_by_plain_dom_code_is_noticed() {
    let dapp = Dapp::new();
    let value = Signal::new(1);
    let card = dapp.card();
    card.text("{}", value.clone(), None);
    card.h3("{}", value.map(|v| v * 2), None);
    settle().await;
    assert_eq!(value.subscriber_count(), 2);

    // Not `unmount`: the element is simply taken out of the document.
    card.get_element().remove();
    settle().await;
    assert_eq!(value.subscriber_count(), 0);
}

#[wasm_bindgen_test]
async fn test_moving_an_element_keeps_its_bindings() {
    let dapp = Dapp::new();
    let value = Signal::new(1);
    let [left, right] = dapp.columns();
    let text = left.text("{}", value.clone(), None);
    settle().await;

    // Taken out and put back elsewhere in the same turn: a move.
    right
        .get_element()
        .append_child(text.get_element())
        .unwrap();
    settle().await;
    assert_eq!(value.subscriber_count(), 1);
    value.set(7);
    assert_eq!(text.get_element().text_content().unwrap(), "7");
    assert!(right.get_element().contains(Some(text.get_element())));
}

/// A component built before it is added to the page is not mistaken for a
/// removed one.
#[wasm_bindgen_test]
async fn test_a_component_not_yet_on_the_page_keeps_its_bindings() {
    let dapp = Dapp::new();
    let data = Signal::new(people());
    let table = Table::new(&data); // built, not added
    dapp.p_static("something else changes the page");
    settle().await;
    assert_eq!(data.subscriber_count(), 1);

    dapp.container()
        .get_element()
        .append_child(table.get_element())
        .unwrap();
    settle().await;
    data.set(people().head(1));
    assert_eq!(
        table
            .get_element()
            .query_selector_all("tbody tr")
            .unwrap()
            .length(),
        1
    );
    table.unmount();
    assert_eq!(data.subscriber_count(), 0);
}

#[wasm_bindgen_test]
fn test_unmounted_inputs_and_buttons_stop_listening() {
    let dapp = Dapp::new();

    let name = dapp.text_input("Name");
    let value = name.value();
    let input: HtmlInputElement = name.control().clone().unchecked_into();
    let type_text = |text: &str| {
        input.set_value(text);
        input.dispatch_event(&Event::new("input").unwrap()).unwrap();
    };
    type_text("Ada");
    assert_eq!(value.get(), "Ada");
    name.unmount();
    assert_eq!(value.subscriber_count(), 0);
    type_text("ignored");
    assert_eq!(value.get(), "Ada");

    let clicks = Signal::new(0);
    let button = dapp.button("Click");
    let clicks_in = clicks.clone();
    button.pressed(move || clicks_in.set(clicks_in.get() + 1));
    button.get_element().click();
    assert_eq!(clicks.get(), 1);
    button.unmount();
    button.get_element().click();
    assert_eq!(clicks.get(), 1);

    // A radio group: one listener per option, all released together.
    let size = dapp.radio("Size", &["S", "M"]);
    let size_value = size.value();
    let before = bound_count();
    size.unmount();
    assert_eq!(size_value.subscriber_count(), 0);
    assert_eq!(before - bound_count(), 3, "the group and its two options");
}

/// Data being fetched stops when nobody is waiting for it any more.
#[wasm_bindgen_test]
async fn test_fetch_stops_following_the_url_when_unused() {
    let dapp = Dapp::new();
    let url = Signal::new(String::new());
    let data = fetch_table(&url);
    assert_eq!(url.subscriber_count(), 1);
    let view = dapp.table(&data);
    drop(data); // the table is now the only thing using the fetch
    assert_eq!(url.subscriber_count(), 1);

    url.set("data:text/csv,n%0A1%0A2".to_string());
    for _ in 0..100 {
        if view
            .get_element()
            .query_selector("tbody tr")
            .unwrap()
            .is_some()
        {
            break;
        }
        settle().await;
    }
    assert_eq!(
        view.get_element()
            .query_selector_all("tbody tr")
            .unwrap()
            .length(),
        2
    );

    // Off the page, the table no longer listens -- but `view` is still a
    // handle to its data, so the fetch behind it is kept until that goes too.
    view.unmount();
    assert_eq!(url.subscriber_count(), 1);
    drop(view);
    assert_eq!(url.subscriber_count(), 0);
}

/// A subscription you hold yourself is yours: unmounting does not end it.
#[wasm_bindgen_test]
fn test_subscriptions_you_hold_are_left_alone() {
    let dapp = Dapp::new();
    let value = Signal::new(0);
    let seen = Signal::new(0);
    let seen_in = seen.clone();
    let mine = value.subscribe(move |v| seen_in.set(*v));
    let text = dapp.text("{}", value.clone(), None);
    assert_eq!(value.subscriber_count(), 2);

    text.unmount();
    assert_eq!(value.subscriber_count(), 1);
    value.set(9);
    assert_eq!(seen.get(), 9);
    drop(mine);
    assert_eq!(value.subscriber_count(), 0);
}

/// A component may remove itself from inside its own click handler.
#[wasm_bindgen_test]
fn test_a_component_can_unmount_itself_while_handling_its_own_click() {
    let dapp = Dapp::new();
    let count = Signal::new(3);
    let after = Signal::new(false);
    let before = bound_count();

    let chip = dapp.container();
    chip.id("self-removing");
    let button = chip.button("dismiss");
    button.bind_signal(count.map(|c| format!("{c} left")), None, None);
    assert_eq!(count.subscriber_count(), 1);

    let (count_in, after_in) = (count.clone(), after.clone());
    button.pressed(move || {
        chip.unmount();
        // Still running, with everything it captured intact.
        count_in.set(count_in.get() - 1);
        after_in.set(true);
    });
    let element = button.get_element().clone();
    drop(button);

    element.click();
    assert!(after.get());
    assert_eq!(count.get(), 2);
    assert_eq!(count.subscriber_count(), 0);
    assert_eq!(bound_count(), before);
    assert!(web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .get_element_by_id("self-removing")
        .is_none());
    // The listener went with it.
    element.click();
    assert_eq!(count.get(), 2);
}
