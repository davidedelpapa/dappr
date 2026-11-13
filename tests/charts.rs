#![allow(dead_code)]
use dappr::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_test::*;
use web_sys::{Element, HtmlElement, KeyboardEvent, KeyboardEventInit, MouseEvent, MouseEventInit};

wasm_bindgen_test_configure!(run_in_browser);

async fn sleep(ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms)
            .unwrap();
    });
    JsFuture::from(promise).await.unwrap();
}

fn sales() -> DTable {
    DTable::from_csv("month,north,south\nJan,120,80\nFeb,150,95\nMar,90,140\nApr,200,130").unwrap()
}

fn one(root: &Element, selector: &str) -> HtmlElement {
    root.query_selector(selector)
        .unwrap()
        .unwrap_or_else(|| panic!("nothing matches {selector}"))
        .unchecked_into()
}

fn count(root: &Element, selector: &str) -> u32 {
    root.query_selector_all(selector).unwrap().length()
}

fn texts(root: &Element, selector: &str) -> Vec<String> {
    let nodes = root.query_selector_all(selector).unwrap();
    (0..nodes.length())
        .map(|i| nodes.item(i).unwrap().text_content().unwrap())
        .collect()
}

/// Moves the pointer to a spot given as fractions of the plot's size.
fn pointer_to(chart: &Chart, fx: f64, fy: f64) {
    let plot = one(chart.get_element(), ".dappr-chart-plot");
    let bounds = one(chart.get_element(), ".dappr-chart-drawing").get_bounding_client_rect();
    let init = MouseEventInit::new();
    init.set_client_x((bounds.left() + bounds.width() * fx) as i32);
    init.set_client_y((bounds.top() + bounds.height() * fy) as i32);
    let event = MouseEvent::new_with_mouse_event_init_dict("pointermove", &init).unwrap();
    plot.dispatch_event(&event).unwrap();
}

fn press(chart: &Chart, key: &str) {
    let init = KeyboardEventInit::new();
    init.set_key(key);
    let event = KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init).unwrap();
    one(chart.get_element(), ".dappr-chart-plot")
        .dispatch_event(&event)
        .unwrap();
}

fn tooltip(chart: &Chart) -> Option<Vec<String>> {
    let tip = one(chart.get_element(), ".dappr-chart-tooltip");
    (!tip.hidden()).then(|| {
        let mut lines = texts(&tip, ".dappr-chart-tooltip-title");
        let rows = tip.query_selector_all(".dappr-chart-tooltip-row").unwrap();
        for i in 0..rows.length() {
            let row: Element = rows.item(i).unwrap().unchecked_into();
            lines.push(format!(
                "{} {}",
                one(&row, "strong").text_content().unwrap(),
                one(&row, ".dappr-chart-tooltip-name")
                    .text_content()
                    .unwrap()
            ));
        }
        lines
    })
}

#[wasm_bindgen_test]
async fn test_line_chart_has_title_legend_marks_and_fits_its_place() {
    let dapp = Dapp::new();
    let chart = dapp
        .line_chart(&sales(), "month", &["north", "south"])
        .title("Monthly sales")
        .y_label("Units");
    let el = chart.get_element();
    sleep(60).await; // let the chart learn its width

    assert_eq!(el.tag_name(), "FIGURE");
    assert_eq!(texts(el, "figcaption"), vec!["Monthly sales"]);
    assert_eq!(
        texts(el, ".dappr-chart-legend-item"),
        vec!["north", "south"]
    );
    assert_eq!(count(el, "svg .dappr-chart-line"), 2);
    assert!(texts(el, "svg text").contains(&"Units".to_string()));
    assert!(texts(el, "svg text").contains(&"Jan".to_string()));

    // Drawn at the width of its place on the page, not a fixed size.
    let svg = one(el, "svg");
    let drawn: f64 = svg.get_attribute("width").unwrap().parse().unwrap();
    assert_eq!(drawn, f64::from(el.client_width()));
    assert!(drawn > 100.0);

    // The first two series take the first two palette colours, in order.
    let colour = |selector: &str| {
        web_sys::window()
            .unwrap()
            .get_computed_style(&one(el, selector))
            .unwrap()
            .unwrap()
            .get_property_value("stroke")
            .unwrap()
    };
    assert_eq!(
        colour("svg .dappr-chart-line.dappr-s0"),
        "rgb(42, 120, 214)"
    );
    assert_eq!(
        colour("svg .dappr-chart-line.dappr-s1"),
        "rgb(235, 104, 52)"
    );
}

#[wasm_bindgen_test]
fn test_single_series_needs_no_legend() {
    let dapp = Dapp::new();
    let chart = dapp.bar_chart(&sales(), "month", &["north"]);
    let el = chart.get_element();
    assert!(one(el, ".dappr-chart-legend").hidden());
    assert!(one(el, "figcaption").hidden());
    assert_eq!(count(el, "svg .dappr-chart-bars path"), 4);
}

#[wasm_bindgen_test]
async fn test_readout_follows_the_pointer_and_shows_every_series() {
    let dapp = Dapp::new();
    let chart = dapp.line_chart(&sales(), "month", &["north", "south"]);
    sleep(60).await;
    assert!(tooltip(&chart).is_none());

    // Far left: the first month. The pointer is nowhere near a line
    // vertically, and does not need to be.
    pointer_to(&chart, 0.12, 0.05);
    assert_eq!(
        tooltip(&chart).unwrap(),
        vec!["Jan", "120 north", "80 south"]
    );
    let el = chart.get_element();
    assert_eq!(
        one(el, ".dappr-chart-crosshair")
            .get_attribute("visibility")
            .unwrap(),
        "visible"
    );

    pointer_to(&chart, 0.9, 0.9);
    assert_eq!(
        tooltip(&chart).unwrap(),
        vec!["Apr", "200 north", "130 south"]
    );

    one(el, ".dappr-chart-plot")
        .dispatch_event(&MouseEvent::new("pointerleave").unwrap())
        .unwrap();
    assert!(tooltip(&chart).is_none());
    assert_eq!(
        one(el, ".dappr-chart-crosshair")
            .get_attribute("visibility")
            .unwrap(),
        "hidden"
    );
}

#[wasm_bindgen_test]
async fn test_readout_works_from_the_keyboard() {
    let dapp = Dapp::new();
    let chart = dapp.bar_chart(&sales(), "month", &["north"]);
    sleep(60).await;
    let plot = one(chart.get_element(), ".dappr-chart-plot");
    assert_eq!(plot.tab_index(), 0);

    plot.focus().unwrap();
    assert_eq!(tooltip(&chart).unwrap(), vec!["Jan", "120 north"]);
    press(&chart, "ArrowRight");
    assert_eq!(tooltip(&chart).unwrap(), vec!["Feb", "150 north"]);
    press(&chart, "End");
    assert_eq!(tooltip(&chart).unwrap(), vec!["Apr", "200 north"]);
    press(&chart, "ArrowRight"); // already at the end
    assert_eq!(tooltip(&chart).unwrap(), vec!["Apr", "200 north"]);
    press(&chart, "Home");
    assert_eq!(tooltip(&chart).unwrap(), vec!["Jan", "120 north"]);
    press(&chart, "Escape");
    assert!(tooltip(&chart).is_none());
}

#[wasm_bindgen_test]
async fn test_scatter_readout_needs_the_pointer_near_a_dot() {
    let dapp = Dapp::new();
    let data = DTable::from_csv("minutes,km\n10,2\n20,5\n30,9").unwrap();
    let chart = dapp.scatter_chart(&data, "minutes", &["km"]);
    sleep(60).await;
    let el = chart.get_element();
    assert_eq!(count(el, "svg .dappr-chart-dots circle"), 3);

    // Right on the middle dot.
    let drawing = one(el, ".dappr-chart-drawing").get_bounding_client_rect();
    let dot = el
        .query_selector_all("svg .dappr-chart-dots circle")
        .unwrap()
        .item(1)
        .unwrap()
        .unchecked_into::<Element>()
        .get_bounding_client_rect();
    let fx = (dot.left() + dot.width() / 2.0 - drawing.left() + 5.0) / drawing.width();
    let fy = (dot.top() + dot.height() / 2.0 - drawing.top() + 5.0) / drawing.height();
    pointer_to(&chart, fx, fy);
    assert_eq!(tooltip(&chart).unwrap(), vec!["minutes: 20", "5 km"]);

    // In empty space, away from every dot: nothing.
    pointer_to(&chart, 0.5, 0.05);
    assert!(tooltip(&chart).is_none());
}

/// Names and values from the data are shown as text, wherever they land.
#[wasm_bindgen_test]
async fn test_text_from_data_never_becomes_markup() {
    let dapp = Dapp::new();
    let mut data = DTable::with_columns(&["<b>when</b>", "<img src=x onerror=alert(1)>", "b"]);
    data.push_row(("<i>one</i>", 1, 2));
    data.push_row(("<i>two</i>", 3, 4));
    let chart = dapp.line_chart(&data, "<b>when</b>", &["<img src=x onerror=alert(1)>", "b"]);
    sleep(60).await;
    pointer_to(&chart, 0.1, 0.5);
    let el = chart.get_element();

    assert_eq!(count(el, "img, b, i"), 0);
    assert_eq!(tooltip(&chart).unwrap()[0], "<i>one</i>");
    assert_eq!(
        texts(el, ".dappr-chart-legend-item")[0],
        "<img src=x onerror=alert(1)>"
    );
}

#[wasm_bindgen_test]
async fn test_chart_and_its_table_follow_the_data() {
    let dapp = Dapp::new();
    let big_only = dapp.checkbox("Only months above 100", false);
    let data = sales();
    let shown = big_only
        .value()
        .map(move |big| data.filter(|row| !big || row.number("north") > Some(100.0)));
    let chart = dapp.bar_chart(&shown, "month", &["north"]);
    let el = chart.get_element();
    sleep(60).await;

    // The table view holds exactly the plotted columns.
    assert_eq!(texts(el, "details thead th"), vec!["month", "north"]);
    assert_eq!(count(el, "details tbody tr"), 4);
    assert_eq!(count(el, "svg .dappr-chart-bars path"), 4);

    big_only.set(true);
    assert_eq!(count(el, "svg .dappr-chart-bars path"), 3);
    assert_eq!(
        texts(el, "details tbody td:first-child"),
        vec!["Jan", "Feb", "Apr"]
    );

    // Nothing left to draw: say so instead of an empty frame.
    chart.data().set(DTable::with_columns(&["month", "north"]));
    assert_eq!(texts(el, "svg text"), vec!["No data to plot"]);
}

#[wasm_bindgen_test]
fn test_too_many_series_is_reported() {
    let dapp = Dapp::new();
    let data = DTable::from_csv("x,a,b,c,d\n1,1,2,3,4\n2,2,3,4,5").unwrap();
    let chart = dapp.scatter_chart(&data, "x", &["a", "b", "c", "d"]);
    let el = chart.get_element();
    assert_eq!(texts(el, ".dappr-chart-legend-item"), vec!["a", "b", "c"]);
    let note = one(el, ".dappr-chart-note");
    assert!(!note.hidden());
    assert!(note
        .text_content()
        .unwrap()
        .starts_with("Showing the first 3 of 4 series"));
}

// ---- More kinds of chart -----------------------------------------------------

#[wasm_bindgen_test]
async fn test_area_chart_has_a_fill_and_a_line_readout() {
    let dapp = Dapp::new();
    let chart = dapp.area_chart(&sales(), "month", &["north"]);
    sleep(60).await;
    let el = chart.get_element();
    assert_eq!(count(el, "svg .dappr-chart-area"), 1);
    assert_eq!(count(el, "svg .dappr-chart-line"), 1);
    // The fill is a light wash of the series colour.
    let style = web_sys::window()
        .unwrap()
        .get_computed_style(&one(el, "svg .dappr-chart-area"))
        .unwrap()
        .unwrap();
    assert_eq!(
        style.get_property_value("fill").unwrap(),
        "rgb(42, 120, 214)"
    );
    assert!(
        style
            .get_property_value("opacity")
            .unwrap()
            .parse::<f64>()
            .unwrap()
            < 0.2
    );

    pointer_to(&chart, 0.9, 0.5);
    assert_eq!(tooltip(&chart).unwrap(), vec!["Apr", "200 north"]);
    assert_eq!(
        one(el, ".dappr-chart-crosshair")
            .get_attribute("visibility")
            .unwrap(),
        "visible"
    );
}

#[wasm_bindgen_test]
async fn test_dates_get_a_time_axis() {
    let dapp = Dapp::new();
    let data = DTable::from_csv("day,v\n2026-01-01,1\n2026-01-02,2\n2026-01-11,3").unwrap();
    let chart = dapp.line_chart(&data, "day", &["v"]);
    sleep(60).await;
    let el = chart.get_element();
    // Ticks are round dates, not the three rows' own labels.
    let ticks = texts(el, "svg .dappr-chart-ticks text");
    assert!(ticks.iter().any(|t| t.starts_with("Jan ")), "{ticks:?}");
    assert!(!ticks.contains(&"2026-01-11".to_string()));
    // The read-out still shows the date as written.
    pointer_to(&chart, 0.95, 0.5);
    assert_eq!(tooltip(&chart).unwrap(), vec!["2026-01-11", "3 v"]);
}

#[wasm_bindgen_test]
async fn test_stacked_bars_stack_and_list_every_series() {
    let dapp = Dapp::new();
    let grouped = dapp.bar_chart(&sales(), "month", &["north", "south"]);
    let stacked = dapp
        .bar_chart(&sales(), "month", &["north", "south"])
        .stacked();
    sleep(60).await;

    let first_two = |chart: &Chart| {
        let bars = chart
            .get_element()
            .query_selector_all("svg .dappr-chart-bars path")
            .unwrap();
        // The first bar of each series: both belong to January.
        let rect = |i: u32| {
            bars.item(i)
                .unwrap()
                .unchecked_into::<Element>()
                .get_bounding_client_rect()
        };
        (rect(0), rect(4))
    };
    let (north, south) = first_two(&grouped);
    assert!(
        south.left() > north.right(),
        "grouped bars sit side by side"
    );
    let (north, south) = first_two(&stacked);
    assert!(
        (south.left() - north.left()).abs() < 0.5,
        "stacked bars share a column"
    );
    assert!(
        south.bottom() <= north.top(),
        "the second series sits on the first"
    );

    assert_eq!(
        texts(stacked.get_element(), ".dappr-chart-legend-item"),
        vec!["north", "south"]
    );
    pointer_to(&stacked, 0.2, 0.5);
    assert_eq!(
        tooltip(&stacked).unwrap(),
        vec!["Jan", "120 north", "80 south"]
    );
}

#[wasm_bindgen_test]
async fn test_histogram_counts_and_follows_the_data() {
    let dapp = Dapp::new();
    let data = DTable::from_csv("minutes\n1\n2\n2\n3\n11\n12\n13\n14\n15\n28\n30").unwrap();
    let source = Signal::new(data.clone());
    let chart = dapp.histogram(&source, "minutes").bins(3);
    sleep(60).await;
    let el = chart.get_element();

    assert_eq!(count(el, "svg .dappr-chart-bars path"), 3);
    assert!(one(el, ".dappr-chart-legend").hidden());
    // The table view lists the ranges and their counts.
    assert_eq!(texts(el, "details thead th"), vec!["from", "to", "rows"]);
    assert_eq!(
        texts(el, "details tbody td"),
        vec!["0", "10", "4", "10", "20", "5", "20", "30", "2"]
    );
    pointer_to(&chart, 0.5, 0.5);
    assert_eq!(tooltip(&chart).unwrap(), vec!["10 \u{2013} 20", "5 rows"]);

    // Fewer rows: the bars and the table follow.
    source.set(data.filter(|row| row.number("minutes") < Some(10.0)));
    assert_eq!(
        texts(el, "details tbody td:last-child")
            .iter()
            .map(|t| t.parse::<f64>().unwrap())
            .sum::<f64>(),
        4.0
    );

    // Asking for more bars redraws with more.
    let finer = dapp.histogram(&data, "minutes").bins(6);
    assert!(count(finer.get_element(), "details tbody tr") > 3);
}

// ---- Bring your own drawing -------------------------------------------------

#[wasm_bindgen_test]
fn test_svg_shows_markup_and_follows_a_signal() {
    let dapp = Dapp::new();
    let radius = Signal::new(5);
    let drawing = dapp.svg(radius.map(|r| {
        format!(r#"<svg viewBox="0 0 40 40" width="80"><circle cx="20" cy="20" r="{r}"/></svg>"#)
    }));
    let el = drawing.get_element();
    assert_eq!(one(el, "circle").get_attribute("r").unwrap(), "5");
    radius.set(12);
    assert_eq!(one(el, "circle").get_attribute("r").unwrap(), "12");
    assert_eq!(count(el, "svg"), 1);

    let fixed = dapp.svg(r#"<svg width="10" height="10"><rect width="10" height="10"/></svg>"#);
    assert_eq!(count(fixed.get_element(), "rect"), 1);
}

#[wasm_bindgen_test]
fn test_canvas_draws_and_redraws() {
    let dapp = Dapp::new();
    let first = dapp.canvas(40, 20);
    let second = dapp.canvas(40, 20);
    assert_eq!((first.canvas().width(), first.canvas().height()), (40, 20));
    assert_ne!(first.id(), second.id());

    let pixel = |canvas: &Canvas, x: f64| -> Vec<u8> {
        canvas
            .context_2d()
            .get_image_data(x, 5.0, 1.0, 1.0)
            .unwrap()
            .data()
            .to_vec()
    };
    // Nothing drawn yet: transparent.
    assert_eq!(pixel(&first, 5.0), vec![0, 0, 0, 0]);

    let width = Signal::new(10.0);
    first.draw_on(&width, |canvas, width| {
        let context = canvas.context_2d();
        context.set_fill_style_str("rgb(255, 0, 0)");
        context.fill_rect(0.0, 0.0, *width, 20.0);
    });
    assert_eq!(pixel(&first, 5.0), vec![255, 0, 0, 255]);
    assert_eq!(pixel(&first, 30.0), vec![0, 0, 0, 0]);

    // Wider, then narrower: the canvas is erased between draws.
    width.set(35.0);
    assert_eq!(pixel(&first, 30.0), vec![255, 0, 0, 255]);
    width.set(10.0);
    assert_eq!(pixel(&first, 30.0), vec![0, 0, 0, 0]);
}
