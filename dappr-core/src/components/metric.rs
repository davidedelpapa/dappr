//! ## Metric: one number that matters
//!
//! A label, a large value, and optionally how much it changed. When the
//! point is a single figure -- a total, a count, a rate -- this says it
//! better than a chart with one bar.
use crate::charts::format_value;
use crate::communication::signals::Signal;
use crate::framework::dom;
use crate::framework::traits::{Component, NumberSource, TextSource};
use std::cell::Cell;
use std::rc::Rc;
use web_sys::HtmlElement;

/// A headline figure on the page.
///
/// # Example
/// ```ignore
/// use dappr::prelude::*;
///
/// // A fixed figure.
/// dapp.metric("Orders", "1,284");
///
/// // One that follows the data, with its change since last month.
/// dapp.metric("Revenue", revenue.map(|r| format_compact(*r)))
///     .delta(change, "% vs last month");
/// ```
pub struct Metric {
    el: HtmlElement,
    delta_el: HtmlElement,
    /// Whether a rise is bad news (costs, errors, waiting time).
    lower_is_better: Rc<Cell<bool>>,
    /// Re-draws the delta; set by [`Metric::delta`].
    refresh: Rc<dyn Fn()>,
}

impl Metric {
    /// A metric called `label` showing `value`: text, or a `Signal<String>`
    /// to follow. Format the value as you want it read; see
    /// [`format_compact`](crate::charts::format_compact) for large numbers.
    pub fn new(label: &str, value: impl TextSource) -> Self {
        let value = value.into_text_signal();
        let el = dom::create_element("div");
        el.set_class_name("dappr-metric");
        let part = |class: &str| {
            let part = dom::create_element("div");
            part.set_class_name(class);
            el.append_child(&part).expect("Failed to build the metric");
            part
        };
        part("dappr-metric-label").set_text_content(Some(label));
        let value_el = part("dappr-metric-value");
        value_el.set_text_content(Some(&value.get()));
        let delta_el = part("dappr-metric-delta");
        delta_el.set_hidden(true);

        value
            .subscribe(move |text| value_el.set_text_content(Some(text)))
            .detach();
        Metric {
            el,
            delta_el,
            lower_is_better: Rc::new(Cell::new(false)),
            refresh: Rc::new(|| {}),
        }
    }

    /// Shows how much the value changed: a number, or a `Signal<f64>` to
    /// follow. `suffix` is written right after it, to give the unit and what
    /// it is compared with: `"% vs last month"`, `" since Monday"`.
    ///
    /// A rise is marked with an up arrow and shown as good news, a fall
    /// with a down arrow as bad news (see
    /// [`lower_is_better`](Metric::lower_is_better) to swap that). The arrow
    /// and the sign carry the direction, so it never depends on colour
    /// alone. Chainable.
    pub fn delta(mut self, change: impl NumberSource, suffix: &str) -> Self {
        let change = change.into_number_signal();
        let (delta_el, lower_is_better, suffix) = (
            self.delta_el.clone(),
            Rc::clone(&self.lower_is_better),
            suffix.to_string(),
        );
        let source = change.clone();
        let refresh: Rc<dyn Fn()> = Rc::new(move || {
            let change = source.get();
            delta_el.set_hidden(false);
            let (arrow, direction) = if change > 0.0 {
                ("\u{25B2}", "up")
            } else if change < 0.0 {
                ("\u{25BC}", "down")
            } else {
                ("\u{2013}", "flat")
            };
            let good = (change > 0.0) != lower_is_better.get();
            let tone = match direction {
                "flat" => "dappr-metric-flat",
                _ if good => "dappr-metric-good",
                _ => "dappr-metric-bad",
            };
            delta_el.set_class_name(&format!("dappr-metric-delta {tone}"));
            let sign = if change > 0.0 { "+" } else { "" };
            delta_el.set_text_content(Some(&format!(
                "{arrow} {sign}{}{suffix}",
                format_value((change * 100.0).round() / 100.0)
            )));
        });
        refresh();
        let on_change = Rc::clone(&refresh);
        change.subscribe(move |_| on_change()).detach();
        self.refresh = refresh;
        self
    }

    /// Marks a fall as the good direction, for figures such as costs,
    /// errors or waiting time. Chainable, before or after
    /// [`delta`](Metric::delta).
    pub fn lower_is_better(self) -> Self {
        self.lower_is_better.set(true);
        (self.refresh)();
        self
    }
}

impl Component for Metric {
    fn get_element(&self) -> &HtmlElement {
        &self.el
    }
}

/// So a `Signal` of numbers can be shown as a metric's value directly.
impl TextSource for Signal<f64> {
    fn into_text_signal(self) -> Signal<String> {
        self.map(|value| format_value(*value))
    }
}
