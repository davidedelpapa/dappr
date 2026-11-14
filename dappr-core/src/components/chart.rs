//! ## Chart: a line, area, bar or scatter chart, or a histogram, of a [`DTable`]
//!
//! Like [`Table`](crate::components::table::Table), a chart takes a
//! `DTable` for a fixed chart, or a `Signal<DTable>` for one that redraws
//! when the data changes.
//!
//! Around the drawing itself, a chart carries:
//!
//! - a **legend**, when there is more than one series;
//! - a **read-out** that follows the pointer, and the arrow keys when the
//!   chart has keyboard focus;
//! - a **"Show data" table** with the plotted values, so nothing can only
//!   be read by hovering or by telling colours apart.
use crate::charts::{format_value, histogram_table, render, ChartKind, ChartSpec, Layout};
use crate::communication::signals::Signal;
use crate::components::table::{Table, TableSource};
use crate::data::table::DTable;
use crate::framework::dom;
use crate::framework::traits::Component;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Element, HtmlElement, KeyboardEvent, MouseEvent, ResizeObserver};

/// Width used until the chart knows how wide its place on the page is.
const FALLBACK_WIDTH: f64 = 640.0;
/// How close (in pixels) the pointer must be to a scatter dot to select it.
const SCATTER_REACH: f64 = 24.0;

struct State {
    spec: ChartSpec,
    title: String,
    data: DTable,
    width: f64,
    layout: Option<Layout>,
    /// The selected position: a row for line and bar charts, an index into
    /// [`scatter_points`] for a scatter chart.
    selected: Option<usize>,
}

/// The pieces of the chart that get updated.
#[derive(Clone)]
struct Parts {
    title: HtmlElement,
    legend: HtmlElement,
    plot: HtmlElement,
    drawing: HtmlElement,
    tooltip: HtmlElement,
    note: HtmlElement,
    /// What the "Show data" table shows.
    table: Signal<DTable>,
}

/// A chart on the page.
///
/// # Example
/// ```ignore
/// use dappr::prelude::*;
/// use wasm_bindgen::prelude::*;
///
/// #[wasm_bindgen(main)]
/// pub fn main() {
///     dapp!(dapp, {
///         let sales = DTable::from_csv("month,north,south\nJan,120,80\nFeb,150,95").unwrap();
///         dapp.line_chart(&sales, "month", &["north", "south"])
///             .title("Monthly sales")
///             .y_label("Units");
///     });
/// }
/// ```
pub struct Chart {
    el: HtmlElement,
    parts: Parts,
    state: Rc<RefCell<State>>,
    data: Signal<DTable>,
}

fn child(parent: &HtmlElement, tag: &str, class: &str) -> HtmlElement {
    let el = dom::create_element(tag);
    el.set_class_name(class);
    parent.append_child(&el).expect("Failed to build the chart");
    el
}

impl Chart {
    /// A chart of `source`: column `x` along the x-axis, one series per
    /// column in `y`.
    pub fn new(kind: ChartKind, source: impl TableSource, x: &str, y: &[&str]) -> Self {
        let data = source.into_signal();
        let spec = ChartSpec::new(kind, x, y);

        let el = dom::create_element("figure");
        el.set_class_name("dappr-chart");
        let title = child(&el, "figcaption", "dappr-chart-title");
        title.set_hidden(true);
        let legend = child(&el, "div", "dappr-chart-legend");
        let plot = child(&el, "div", "dappr-chart-plot");
        // Focusable, so the read-out can be driven with the arrow keys.
        plot.set_tab_index(0);
        let drawing = child(&plot, "div", "dappr-chart-drawing");
        let tooltip = child(&plot, "div", "dappr-chart-tooltip");
        tooltip.set_hidden(true);
        let note = child(&el, "small", "dappr-chart-note");
        note.set_hidden(true);

        // The table view: the same columns, as numbers.
        let details = child(&el, "details", "dappr-chart-data");
        child(&details, "summary", "").set_text_content(Some("Show data"));
        // Filled in by `redraw`, which knows what was plotted.
        let table_data = Signal::new(DTable::new());
        let table = Table::new(&table_data);
        details
            .append_child(table.get_element())
            .expect("Failed to build the chart");

        let parts = Parts {
            title,
            legend,
            plot,
            drawing,
            tooltip,
            note,
            table: table_data,
        };
        let state = Rc::new(RefCell::new(State {
            spec,
            title: String::new(),
            data: data.get(),
            width: FALLBACK_WIDTH,
            layout: None,
            selected: None,
        }));

        redraw(&state, &parts);

        // Redraw when the data changes.
        let (state_in, parts_in) = (Rc::clone(&state), parts.clone());
        data.subscribe(move |table| {
            state_in.borrow_mut().data = table.clone();
            redraw(&state_in, &parts_in);
        })
        .detach();

        // Redraw at the right size when the chart's place on the page
        // changes width (including the first time it gets one).
        let (state_in, parts_in, el_in) = (Rc::clone(&state), parts.clone(), el.clone());
        let on_resize = Closure::<dyn FnMut()>::new(move || {
            let width = f64::from(el_in.client_width());
            if width > 0.0 && (width - state_in.borrow().width).abs() >= 1.0 {
                state_in.borrow_mut().width = width;
                redraw(&state_in, &parts_in);
            }
        });
        if let Ok(observer) = ResizeObserver::new(on_resize.as_ref().unchecked_ref()) {
            observer.observe(&el);
        }
        on_resize.forget(); // lives as long as the page

        // The read-out follows the pointer...
        let (state_in, parts_in) = (Rc::clone(&state), parts.clone());
        dom::on(&parts.plot, "pointermove", move |event| {
            let Some(event) = event.dyn_ref::<MouseEvent>() else {
                return;
            };
            let bounds = parts_in.drawing.get_bounding_client_rect();
            let x = f64::from(event.client_x()) - bounds.left();
            let y = f64::from(event.client_y()) - bounds.top();
            let target = state_in
                .borrow()
                .layout
                .as_ref()
                .and_then(|layout| nearest(layout, x, y));
            select(&state_in, &parts_in, target);
        });
        let (state_in, parts_in) = (Rc::clone(&state), parts.clone());
        dom::on(&parts.plot, "pointerleave", move |_| {
            select(&state_in, &parts_in, None);
        });

        // ...and the arrow keys, when the chart has keyboard focus.
        let (state_in, parts_in) = (Rc::clone(&state), parts.clone());
        dom::on(&parts.plot, "focus", move |_| {
            let current = state_in.borrow().selected;
            select(&state_in, &parts_in, Some(current.unwrap_or(0)));
        });
        let (state_in, parts_in) = (Rc::clone(&state), parts.clone());
        dom::on(&parts.plot, "blur", move |_| {
            select(&state_in, &parts_in, None);
        });
        let (state_in, parts_in) = (Rc::clone(&state), parts.clone());
        dom::on(&parts.plot, "keydown", move |event| {
            let Some(key_event) = event.dyn_ref::<KeyboardEvent>() else {
                return;
            };
            let (count, current) = {
                let state = state_in.borrow();
                let count = state.layout.as_ref().map_or(0, positions);
                (count, state.selected)
            };
            if count == 0 {
                return;
            }
            let last = count - 1;
            let target = match key_event.key().as_str() {
                "ArrowRight" => Some(current.map_or(0, |i| (i + 1).min(last))),
                "ArrowLeft" => Some(current.map_or(last, |i| i.saturating_sub(1))),
                "Home" => Some(0),
                "End" => Some(last),
                "Escape" => None,
                _ => return,
            };
            event.prevent_default();
            select(&state_in, &parts_in, target);
        });

        Chart {
            el,
            parts,
            state,
            data,
        }
    }

    fn update(self, change: impl FnOnce(&mut State)) -> Self {
        change(&mut self.state.borrow_mut());
        redraw(&self.state, &self.parts);
        self
    }

    /// A title above the chart. Chainable.
    pub fn title(self, title: &str) -> Self {
        self.update(|state| state.title = title.to_string())
    }

    /// Text under the x-axis. Chainable.
    pub fn x_label(self, label: &str) -> Self {
        self.update(|state| state.spec.x_label = label.to_string())
    }

    /// Text above the y-axis, typically the unit. Chainable.
    pub fn y_label(self, label: &str) -> Self {
        self.update(|state| state.spec.y_label = label.to_string())
    }

    /// Height of the drawing in pixels (default 320). The width always
    /// follows the chart's place on the page. Chainable.
    pub fn height(self, height: u32) -> Self {
        self.update(|state| state.spec.height = f64::from(height))
    }

    /// Bar charts: stacks the series on top of each other instead of
    /// placing them side by side. Use it when the series are parts of a
    /// whole and the total matters. Chainable.
    pub fn stacked(self) -> Self {
        self.update(|state| state.spec.stacked = true)
    }

    /// Bar charts: draws the bars sideways, with the categories listed
    /// down the left. Use it when the category names are long or there are
    /// many of them; the chart grows taller to give each one a row.
    /// Combines with [`stacked`](Chart::stacked). Chainable.
    pub fn horizontal(self) -> Self {
        self.update(|state| state.spec.horizontal = true)
    }

    /// Says how the dates on the x-axis are written, when they are in a
    /// form that could be read two ways or is not recognised on its own.
    /// In the format, `%d` is the day, `%m` the month as a number, `%b` its
    /// name, `%Y` the year (`%y` for two digits), and `%H` `%M` `%S` the
    /// time; everything else must match as written. Chainable.
    ///
    /// ```ignore
    /// // 03/05/2026 is the 5th of March here, not the 3rd of May.
    /// dapp.line_chart(&data, "date", &["visits"]).date_format("%m/%d/%Y");
    /// ```
    pub fn date_format(self, format: &str) -> Self {
        self.update(|state| state.spec.date_format = Some(format.to_string()))
    }

    /// Histograms: roughly how many bars to draw. The ranges are rounded to
    /// tidy numbers, so the exact count can differ. Chainable.
    pub fn bins(self, bins: usize) -> Self {
        self.update(|state| state.spec.bins = bins)
    }

    /// The data being charted. Set it to change what the chart shows.
    pub fn data(&self) -> Signal<DTable> {
        self.data.clone()
    }
}

impl Component for Chart {
    fn get_element(&self) -> &HtmlElement {
        &self.el
    }
}

/// How many positions the read-out can be on.
fn positions(layout: &Layout) -> usize {
    match layout.kind {
        ChartKind::Scatter => scatter_points(layout).len(),
        _ => layout.rows.len(),
    }
}

/// A scatter chart's dots as (row, series), left to right.
fn scatter_points(layout: &Layout) -> Vec<(usize, usize)> {
    let mut points: Vec<(usize, usize)> = Vec::new();
    for (series_index, series) in layout.series.iter().enumerate() {
        for (row, point) in series.points.iter().enumerate() {
            if point.is_some() {
                points.push((row, series_index));
            }
        }
    }
    points.sort_by(|a, b| {
        layout.rows[a.0]
            .x
            .partial_cmp(&layout.rows[b.0].x)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    points
}

/// The position the pointer at (`x`, `y`) refers to. On line and bar charts
/// that is the nearest x position, wherever the pointer is vertically, so
/// nobody has to aim at a thin line. On a scatter chart it is the nearest
/// dot within reach.
fn nearest(layout: &Layout, x: f64, y: f64) -> Option<usize> {
    let closest = |candidates: Vec<(usize, f64)>| {
        candidates
            .into_iter()
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
    };
    match layout.kind {
        ChartKind::Scatter => {
            let distances = scatter_points(layout)
                .into_iter()
                .enumerate()
                .filter_map(|(index, (row, series))| {
                    let point = layout.series[series].points[row]?;
                    let distance =
                        ((layout.rows[row].x - x).powi(2) + (point.y - y).powi(2)).sqrt();
                    Some((index, distance))
                })
                .collect();
            closest(distances)
                .filter(|(_, distance)| *distance <= SCATTER_REACH)
                .map(|(index, _)| index)
        }
        _ => {
            // Sideways bars: the rows run down the page, so it is the
            // pointer's height that picks one.
            let along = if layout.horizontal {
                if y < layout.top || y > layout.bottom {
                    return None;
                }
                y
            } else {
                if x < layout.left - layout.band || x > layout.right + layout.band {
                    return None;
                }
                x
            };
            let distances = layout
                .rows
                .iter()
                .enumerate()
                .map(|(index, row)| (index, (row.x - along).abs()))
                .collect();
            closest(distances).map(|(index, _)| index)
        }
    }
}

fn set(el: &Element, name: &str, value: &str) {
    let _ = el.set_attribute(name, value);
}

/// One line of the read-out: a colour key, the value, and what it is.
fn tooltip_row(tooltip: &HtmlElement, series_index: usize, value: &str, name: &str) {
    let row = child(tooltip, "div", "dappr-chart-tooltip-row");
    child(
        &row,
        "span",
        &format!("dappr-chart-key dappr-s{series_index}"),
    );
    child(&row, "strong", "").set_text_content(Some(value));
    child(&row, "span", "dappr-chart-tooltip-name").set_text_content(Some(name));
}

/// Moves the read-out to `target` (or hides it), and remembers the choice.
fn select(state: &Rc<RefCell<State>>, parts: &Parts, target: Option<usize>) {
    let mut state = state.borrow_mut();
    let Some(layout) = state.layout.clone() else {
        return;
    };
    let target = target.filter(|index| *index < positions(&layout));
    // Leaving with the pointer keeps the last position for the keyboard.
    if target.is_some() {
        state.selected = target;
    }
    drop(state);

    let find = |selector: &str| parts.drawing.query_selector(selector).ok().flatten();
    let hover_dots = parts
        .drawing
        .query_selector_all(".dappr-chart-hover")
        .expect("valid selector");
    let hide_dots = || {
        for i in 0..hover_dots.length() {
            if let Some(dot) = hover_dots
                .item(i)
                .and_then(|n| n.dyn_into::<Element>().ok())
            {
                set(&dot, "visibility", "hidden");
            }
        }
    };

    let Some(index) = target else {
        parts.tooltip.set_hidden(true);
        hide_dots();
        for selector in [".dappr-chart-crosshair", ".dappr-chart-band"] {
            if let Some(el) = find(selector) {
                set(&el, "visibility", "hidden");
            }
        }
        return;
    };

    // Everything put in the read-out comes from the data, so it goes in as
    // text, never as markup.
    parts.tooltip.set_text_content(None);
    let anchor_x;
    let mut anchor_y = layout.top;
    match layout.kind {
        ChartKind::Scatter => {
            let (row, series_index) = scatter_points(&layout)[index];
            let Some(point) = layout.series[series_index].points[row] else {
                return;
            };
            hide_dots();
            if let Some(dot) = hover_dots
                .item(series_index as u32)
                .and_then(|n| n.dyn_into::<Element>().ok())
            {
                set(&dot, "cx", &format!("{:.1}", layout.rows[row].x));
                set(&dot, "cy", &format!("{:.1}", point.y));
                set(&dot, "visibility", "visible");
            }
            // A dot has two values; say which one this is.
            child(&parts.tooltip, "div", "dappr-chart-tooltip-title").set_text_content(Some(
                &format!("{}: {}", layout.x_name, layout.rows[row].label),
            ));
            tooltip_row(
                &parts.tooltip,
                series_index,
                &format_value(point.value),
                &layout.series[series_index].name,
            );
            anchor_x = layout.rows[row].x;
            anchor_y = (point.y - 12.0).max(layout.top);
        }
        kind => {
            let row = &layout.rows[index];
            if matches!(kind, ChartKind::Line | ChartKind::Area) {
                if let Some(crosshair) = find(".dappr-chart-crosshair") {
                    set(&crosshair, "x1", &format!("{:.1}", row.x));
                    set(&crosshair, "x2", &format!("{:.1}", row.x));
                    set(&crosshair, "visibility", "visible");
                }
            } else if let Some(band) = find(".dappr-chart-band") {
                let attribute = if layout.horizontal { "y" } else { "x" };
                set(
                    &band,
                    attribute,
                    &format!("{:.1}", row.x - layout.band / 2.0),
                );
                set(&band, "visibility", "visible");
            }
            child(&parts.tooltip, "div", "dappr-chart-tooltip-title")
                .set_text_content(Some(&row.label));
            for (series_index, series) in layout.series.iter().enumerate() {
                let point = series.points[index];
                if let Some(dot) = hover_dots
                    .item(series_index as u32)
                    .and_then(|n| n.dyn_into::<Element>().ok())
                {
                    match point {
                        Some(point) => {
                            set(&dot, "cx", &format!("{:.1}", row.x));
                            set(&dot, "cy", &format!("{:.1}", point.y));
                            set(&dot, "visibility", "visible");
                        }
                        None => set(&dot, "visibility", "hidden"),
                    }
                }
                let value = point.map_or("\u{2013}".to_string(), |p| format_value(p.value));
                tooltip_row(&parts.tooltip, series_index, &value, &series.name);
            }
            if layout.horizontal {
                // Beside the end of the row's longest bar, level with the row.
                anchor_x = layout
                    .series
                    .iter()
                    .filter_map(|series| series.points[index])
                    .map(|point| point.y)
                    .fold(layout.left, f64::max);
                anchor_y = (row.x - layout.band / 2.0).max(0.0);
            } else {
                anchor_x = row.x;
            }
        }
    }

    // Beside the position, on whichever side has more room.
    let style = parts.tooltip.style();
    let on_left = anchor_x > (layout.left + layout.right) / 2.0;
    let _ = style.set_property("top", &format!("{anchor_y:.0}px"));
    let _ = style.set_property("left", &format!("{anchor_x:.0}px"));
    let _ = style.set_property(
        "transform",
        if on_left {
            "translateX(calc(-100% - 12px))"
        } else {
            "translateX(12px)"
        },
    );
    parts.tooltip.set_hidden(false);
}

/// Draws the chart from the current state.
fn redraw(state: &Rc<RefCell<State>>, parts: &Parts) {
    let selected = {
        let mut state = state.borrow_mut();
        let plot = render(&state.spec, &state.data, state.width);
        // Our own SVG, with every piece of data text escaped by `render`.
        parts.drawing.set_inner_html(&plot.svg);
        state.layout = plot.layout;

        parts.title.set_hidden(state.title.is_empty());
        parts.title.set_text_content(Some(&state.title));
        parts.note.set_hidden(plot.note.is_none());
        parts.note.set_text_content(plot.note.as_deref());

        // The table view: what was plotted, as numbers.
        let table = match state.spec.kind {
            ChartKind::Histogram => {
                histogram_table(&state.data, &state.spec.x, state.spec.bins).unwrap_or_default()
            }
            _ => {
                let mut columns = vec![state.spec.x.as_str()];
                columns.extend(state.spec.drawn_series().iter().map(String::as_str));
                state.data.select(&columns)
            }
        };
        parts.table.set(table);

        // A legend names the series when there are several. With one, the
        // title already says what is plotted.
        parts.legend.set_text_content(None);
        let series = state.spec.drawn_series();
        parts.legend.set_hidden(series.len() < 2);
        if series.len() >= 2 {
            let swatch = match state.spec.kind {
                ChartKind::Line => "dappr-chart-swatch-line",
                ChartKind::Scatter => "dappr-chart-swatch-dot",
                _ => "dappr-chart-swatch-bar",
            };
            for (index, name) in series.iter().enumerate() {
                let item = child(&parts.legend, "span", "dappr-chart-legend-item");
                child(&item, "span", &format!("{swatch} dappr-s{index}"));
                child(&item, "span", "").set_text_content(Some(name));
            }
        }
        // Keep the read-out where it was only if it is currently showing.
        state.selected.filter(|_| !parts.tooltip.hidden())
    };
    parts.tooltip.set_hidden(true);
    if selected.is_some() {
        select(state, parts, selected);
    }
}
