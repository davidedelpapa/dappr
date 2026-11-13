//! Charts drawn from a [`DTable`]: line, bar and scatter.
//!
//! This module is the drawing half: [`render`] turns a [`ChartSpec`] and a
//! table into an SVG string plus a [`Layout`] describing where every value
//! ended up. It touches no browser API, so it is tested natively. The
//! [`Chart`](crate::components::chart::Chart) component puts the result on
//! the page and adds the legend, hover read-out and data table.
//!
//! The look follows a few fixed rules: thin marks, a recessive hairline
//! grid, one y-axis, text in text colours (never in a series colour), and a
//! palette of eight colours assigned in a fixed order so a series keeps its
//! colour whatever the data is filtered to.
use crate::data::table::{Cell, DTable};
use std::fmt::Write;

/// How many series a line or bar chart draws. The palette has eight colours
/// that stay tellable apart (including for colour-blind readers) when they
/// sit next to each other; a ninth would not be.
pub const MAX_SERIES: usize = 8;
/// How many series a scatter chart draws. In a scatter every colour can end
/// up beside every other, and only the first three stay distinct that way.
pub const MAX_SCATTER_SERIES: usize = 3;

const FONT_SIZE: f64 = 12.0;
/// Rough width of one character of tick text, to reserve room for labels
/// without measuring them in a browser.
const CHAR_WIDTH: f64 = 6.6;
const MAX_LABEL_CHARS: usize = 18;

/// The kind of chart.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChartKind {
    /// Lines, for change along an ordered x (time, steps).
    Line,
    /// Vertical bars, for comparing amounts across categories.
    Bar,
    /// Dots, for the relationship between two numeric columns.
    Scatter,
}

/// What to draw: which columns, and how.
#[derive(Debug, Clone, PartialEq)]
pub struct ChartSpec {
    /// The kind of chart.
    pub kind: ChartKind,
    /// The column along the x-axis.
    pub x: String,
    /// The columns to plot, one series each.
    pub y: Vec<String>,
    /// Text under the x-axis (empty for none).
    pub x_label: String,
    /// Text above the y-axis (empty for none).
    pub y_label: String,
    /// Height of the drawing in pixels, axis labels included.
    pub height: f64,
}

impl ChartSpec {
    /// A spec with the default height and no axis labels.
    pub fn new(kind: ChartKind, x: &str, y: &[&str]) -> Self {
        ChartSpec {
            kind,
            x: x.to_string(),
            y: y.iter().map(|name| name.to_string()).collect(),
            x_label: String::new(),
            y_label: String::new(),
            height: 320.0,
        }
    }

    /// The series actually drawn: the `y` columns, up to the limit for this
    /// kind of chart.
    pub fn drawn_series(&self) -> &[String] {
        let limit = match self.kind {
            ChartKind::Scatter => MAX_SCATTER_SERIES,
            _ => MAX_SERIES,
        };
        &self.y[..self.y.len().min(limit)]
    }
}

/// One x position in a drawn chart.
#[derive(Debug, Clone, PartialEq)]
pub struct RowLayout {
    /// Horizontal position, in pixels from the left of the drawing.
    pub x: f64,
    /// The x value as text.
    pub label: String,
}

/// One value in a drawn chart.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointLayout {
    /// Vertical position of the value, in pixels from the top.
    pub y: f64,
    /// The value itself.
    pub value: f64,
}

/// One series in a drawn chart.
#[derive(Debug, Clone, PartialEq)]
pub struct SeriesLayout {
    /// The column name.
    pub name: String,
    /// One entry per row; `None` where the value is missing.
    pub points: Vec<Option<PointLayout>>,
}

/// Where everything is in a drawn chart, for hit-testing the pointer.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    /// The kind of chart.
    pub kind: ChartKind,
    /// The name of the x column.
    pub x_name: String,
    /// Left edge of the plot area.
    pub left: f64,
    /// Right edge of the plot area.
    pub right: f64,
    /// Top edge of the plot area.
    pub top: f64,
    /// Bottom edge of the plot area.
    pub bottom: f64,
    /// Width of one category slot (bar and line charts with text on x).
    pub band: f64,
    /// The x positions, left to right for line and bar charts.
    pub rows: Vec<RowLayout>,
    /// The series, in the order given (which is also their colour order).
    pub series: Vec<SeriesLayout>,
}

/// A drawn chart.
#[derive(Debug, Clone, PartialEq)]
pub struct Plot {
    /// The drawing.
    pub svg: String,
    /// Where things are, or `None` if there was nothing to draw.
    pub layout: Option<Layout>,
    /// Something the reader should know (series left out, bad column).
    pub note: Option<String>,
}

// ---- Text and numbers -------------------------------------------------------

/// Escapes text for use in SVG/HTML content and attribute values.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

fn shorten(text: &str) -> String {
    if text.chars().count() <= MAX_LABEL_CHARS {
        text.to_string()
    } else {
        let head: String = text.chars().take(MAX_LABEL_CHARS - 1).collect();
        format!("{head}\u{2026}")
    }
}

fn text_width(text: &str) -> f64 {
    text.chars().count() as f64 * CHAR_WIDTH
}

/// Adds thousands separators to the integer part of a plain decimal number.
fn group_thousands(number: &str) -> String {
    let (sign, rest) = match number.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", number),
    };
    let (int, frac) = match rest.split_once('.') {
        Some((int, frac)) => (int, Some(frac)),
        None => (rest, None),
    };
    let mut grouped = String::new();
    for (i, digit) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    match frac {
        Some(frac) => format!("{sign}{grouped}.{frac}"),
        None => format!("{sign}{grouped}"),
    }
}

/// A value as shown in a read-out: without floating-point noise, with
/// thousands separators (`1,234.5`).
pub fn format_value(value: f64) -> String {
    if !value.is_finite() {
        return value.to_string();
    }
    let cleaned = (value * 1e10).round() / 1e10;
    if cleaned.abs() >= 1e15 {
        return cleaned.to_string();
    }
    group_thousands(&cleaned.to_string())
}

/// A tick label. Axes that reach 10,000 use compact labels (`10K`, `1.5M`)
/// throughout; smaller ones are written in full (`2,500`, `0.25`).
fn format_tick(value: f64, step: f64, compact: bool) -> String {
    if compact {
        let (scaled, suffix) = match value.abs() {
            v if v >= 1e9 => (value / 1e9, "B"),
            v if v >= 1e6 => (value / 1e6, "M"),
            v if v >= 1e3 => (value / 1e3, "K"),
            _ => (value, ""),
        };
        let rounded = (scaled * 100.0).round() / 100.0;
        return format!("{rounded}{suffix}");
    }
    // As many decimals as the step needs: 0 for 50, 1 for 2.5, 2 for 0.25.
    let decimals = (0..=6usize)
        .find(|d| {
            let scaled = step * 10f64.powi(*d as i32);
            (scaled - scaled.round()).abs() < 1e-6
        })
        .unwrap_or(6);
    group_thousands(&format!("{value:.decimals$}"))
}

/// Round tick values covering `min..=max`: returns the first tick, the step
/// and how many ticks there are. The ticks land on 1, 2, 2.5 or 5 times a
/// power of ten.
fn nice_ticks(min: f64, max: f64, target: usize) -> (f64, f64, usize) {
    let (min, max) = if min == max {
        let pad = if min == 0.0 { 1.0 } else { min.abs() * 0.1 };
        (min - pad, max + pad)
    } else {
        (min, max)
    };
    let raw_step = (max - min) / target.max(1) as f64;
    let magnitude = 10f64.powf(raw_step.log10().floor());
    let step = [1.0, 2.0, 2.5, 5.0, 10.0]
        .iter()
        .map(|m| m * magnitude)
        .find(|step| *step >= raw_step)
        .unwrap_or(10.0 * magnitude);
    let first = (min / step).floor() * step;
    let last = (max / step).ceil() * step;
    let count = ((last - first) / step).round() as usize + 1;
    (first, step, count)
}

struct Scale {
    domain: (f64, f64),
    range: (f64, f64),
}

impl Scale {
    fn at(&self, value: f64) -> f64 {
        let (d0, d1) = self.domain;
        let (r0, r1) = self.range;
        if d1 == d0 {
            return (r0 + r1) / 2.0;
        }
        r0 + (value - d0) / (d1 - d0) * (r1 - r0)
    }
}

// ---- Drawing ----------------------------------------------------------------

fn message(width: f64, height: f64, text: &str) -> Plot {
    let svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" class=\"dappr-chart-svg\" width=\"{width}\" \
         height=\"{height}\" viewBox=\"0 0 {width} {height}\" role=\"img\" aria-label=\"{label}\">\
         <text class=\"dappr-chart-message\" x=\"{cx}\" y=\"{cy}\" text-anchor=\"middle\">{label}</text></svg>",
        label = escape(text),
        cx = width / 2.0,
        cy = height / 2.0,
    );
    Plot {
        svg,
        layout: None,
        note: None,
    }
}

/// A bar from the baseline to its value, with the value end rounded and
/// the baseline end square.
fn bar_path(x: f64, width: f64, baseline: f64, y: f64) -> String {
    let height = (baseline - y).abs();
    let r = 4f64.min(width / 2.0).min(height);
    // `d` is +1 when the bar goes up from the baseline, -1 when down.
    let d = if y <= baseline { 1.0 } else { -1.0 };
    format!(
        "M{x0:.1},{b:.1}L{x0:.1},{yr:.1}Q{x0:.1},{y:.1} {xr0:.1},{y:.1}L{xr1:.1},{y:.1}Q{x1:.1},{y:.1} {x1:.1},{yr:.1}L{x1:.1},{b:.1}Z",
        x0 = x,
        x1 = x + width,
        xr0 = x + r,
        xr1 = x + width - r,
        b = baseline,
        yr = y + d * r,
    )
}

/// Draws `data` as `spec` describes, `width` pixels wide.
pub fn render(spec: &ChartSpec, data: &DTable, width: f64) -> Plot {
    let width = width.max(160.0);
    let height = spec.height.max(120.0);

    let Some(x_column) = data.column(&spec.x) else {
        return message(width, height, &format!("No column named \"{}\"", spec.x));
    };
    let drawn = spec.drawn_series();
    if drawn.is_empty() {
        return message(width, height, "No columns to plot");
    }
    for name in drawn {
        if data.column(name).is_none() {
            return message(width, height, &format!("No column named \"{name}\""));
        }
    }
    let mut note = (drawn.len() < spec.y.len()).then(|| {
        format!(
            "Showing the first {} of {} series",
            drawn.len(),
            spec.y.len()
        )
    });

    let numeric_x = x_column.is_numeric();
    if spec.kind == ChartKind::Scatter && !numeric_x {
        return message(
            width,
            height,
            &format!("A scatter chart needs numbers in \"{}\"", spec.x),
        );
    }
    // A line chart follows numeric x values; with text (dates, names) it
    // spaces the rows evenly, like a bar chart.
    let linear_x = numeric_x && spec.kind != ChartKind::Bar;

    // The rows to draw: all of them, except those without an x value when x
    // is a number line.
    let row_indices: Vec<usize> = (0..data.n_rows())
        .filter(|&i| !linear_x || x_column.cells()[i].as_f64().is_some())
        .collect();
    if row_indices.is_empty() {
        return message(width, height, "No data to plot");
    }

    // Values per series, aligned with `row_indices`.
    let values: Vec<Vec<Option<f64>>> = drawn
        .iter()
        .map(|name| {
            let cells = data.column(name).map(|c| c.cells()).unwrap_or(&[]);
            row_indices
                .iter()
                .map(|&i| cells[i].as_f64().filter(|v| v.is_finite()))
                .collect()
        })
        .collect();
    let all_values: Vec<f64> = values.iter().flatten().flatten().copied().collect();
    if all_values.is_empty() {
        return message(width, height, "No numbers to plot");
    }

    // ---- y scale ----
    let mut y_min = all_values.iter().copied().fold(f64::INFINITY, f64::min);
    let mut y_max = all_values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if spec.kind == ChartKind::Bar {
        // Bars are read by their length, so they must start at zero.
        y_min = y_min.min(0.0);
        y_max = y_max.max(0.0);
    }
    let (y_first, y_step, y_count) = nice_ticks(y_min, y_max, 5);
    let y_last = y_first + y_step * (y_count - 1) as f64;
    let compact = y_first.abs().max(y_last.abs()) >= 10_000.0;
    let y_ticks: Vec<(f64, String)> = (0..y_count)
        .map(|i| {
            let value = y_first + y_step * i as f64;
            (value, format_tick(value, y_step, compact))
        })
        .collect();

    // ---- margins ----
    let tick_width = y_ticks
        .iter()
        .map(|(_, label)| text_width(label))
        .fold(0.0, f64::max);
    let left = tick_width + 14.0;
    let top = if spec.y_label.is_empty() { 10.0 } else { 28.0 };
    let bottom = height - if spec.x_label.is_empty() { 26.0 } else { 46.0 };

    // Line charts name each line at its right end when there are few
    // enough lines for that to stay readable.
    let end_labels = spec.kind == ChartKind::Line && (2..=4).contains(&drawn.len());
    let end_label_width = if end_labels {
        drawn
            .iter()
            .map(|name| text_width(&shorten(name)))
            .fold(0.0, f64::max)
            + 12.0
    } else {
        0.0
    };
    // Never let labels squeeze the plot to less than half the drawing.
    let mut right = width - 12.0 - end_label_width;
    let end_labels = end_labels && right - left >= width / 2.0;
    if !end_labels {
        right = width - 12.0;
    }
    let right = right.max(left + 40.0);

    let y_scale = Scale {
        domain: (y_first, y_last),
        range: (bottom, top),
    };

    // ---- x positions ----
    let n = row_indices.len();
    let band = (right - left) / n as f64;
    let x_values: Vec<f64> = row_indices
        .iter()
        .map(|&i| x_column.cells()[i].as_f64().unwrap_or(0.0))
        .collect();
    let (x_scale, x_ticks) = if linear_x {
        let x_min = x_values.iter().copied().fold(f64::INFINITY, f64::min);
        let x_max = x_values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let pad = if spec.kind == ChartKind::Scatter {
            8.0
        } else {
            4.0
        };
        let scale = Scale {
            domain: (x_min, x_max),
            range: (left + pad, right - pad),
        };
        let target = (((right - left) / 90.0).floor() as usize).clamp(2, 10);
        let (first, step, count) = nice_ticks(x_min, x_max, target);
        let compact_x = x_min.abs().max(x_max.abs()) >= 1_000_000.0;
        let ticks: Vec<(f64, String)> = (0..count)
            .map(|i| first + step * i as f64)
            .filter(|v| *v >= x_min - step * 1e-9 && *v <= x_max + step * 1e-9)
            .map(|v| (scale.at(v), format_tick(v, step, compact_x)))
            .collect();
        (Some(scale), ticks)
    } else {
        (None, Vec::new())
    };
    let rows: Vec<RowLayout> = row_indices
        .iter()
        .enumerate()
        .map(|(position, &i)| {
            let cell: &Cell = &x_column.cells()[i];
            let x = match &x_scale {
                Some(scale) => scale.at(x_values[position]),
                None => left + band * (position as f64 + 0.5),
            };
            let label = match cell {
                Cell::Number(number) => format_value(*number),
                other => other.to_string(),
            };
            RowLayout { x, label }
        })
        .collect();
    // Category labels: show every k-th so they never run into each other.
    let x_ticks: Vec<(f64, String)> = if linear_x {
        x_ticks
    } else {
        let widest = rows
            .iter()
            .map(|row| text_width(&shorten(&row.label)))
            .fold(0.0, f64::max);
        let every = (((widest + 14.0) / band).ceil() as usize).max(1);
        rows.iter()
            .step_by(every)
            .map(|row| (row.x, shorten(&row.label)))
            .collect()
    };

    let series: Vec<SeriesLayout> = drawn
        .iter()
        .zip(&values)
        .map(|(name, values)| SeriesLayout {
            name: name.clone(),
            points: values
                .iter()
                .map(|value| {
                    value.map(|value| PointLayout {
                        y: y_scale.at(value),
                        value,
                    })
                })
                .collect(),
        })
        .collect();

    // ---- SVG ----
    let mut svg = String::new();
    let label = format!(
        "{} chart of {} by {}",
        match spec.kind {
            ChartKind::Line => "Line",
            ChartKind::Bar => "Bar",
            ChartKind::Scatter => "Scatter",
        },
        drawn.join(", "),
        spec.x
    );
    let _ = write!(
        svg,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" class=\"dappr-chart-svg\" width=\"{width}\" \
         height=\"{height}\" viewBox=\"0 0 {width} {height}\" role=\"img\" aria-label=\"{}\">",
        escape(&label)
    );

    // Grid and y ticks.
    svg.push_str("<g class=\"dappr-chart-grid\">");
    for (value, _) in &y_ticks {
        let y = y_scale.at(*value);
        let _ = write!(
            svg,
            "<line x1=\"{left:.1}\" x2=\"{right:.1}\" y1=\"{y:.1}\" y2=\"{y:.1}\"/>"
        );
    }
    svg.push_str("</g><g class=\"dappr-chart-ticks\">");
    for (value, text) in &y_ticks {
        let _ = write!(
            svg,
            "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"end\">{}</text>",
            left - 8.0,
            y_scale.at(*value) + FONT_SIZE / 3.0,
            escape(text)
        );
    }
    for (x, text) in &x_ticks {
        let _ = write!(
            svg,
            "<text x=\"{x:.1}\" y=\"{:.1}\" text-anchor=\"middle\">{}</text>",
            bottom + 18.0,
            escape(text)
        );
    }
    if !spec.y_label.is_empty() {
        let _ = write!(
            svg,
            "<text class=\"dappr-chart-axis-label\" x=\"0\" y=\"12\">{}</text>",
            escape(&spec.y_label)
        );
    }
    if !spec.x_label.is_empty() {
        let _ = write!(
            svg,
            "<text class=\"dappr-chart-axis-label\" x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\">{}</text>",
            (left + right) / 2.0,
            height - 6.0,
            escape(&spec.x_label)
        );
    }
    svg.push_str("</g>");

    // The baseline: zero if it is in view (bars grow from it), else the
    // bottom of the plot.
    let baseline = if y_first <= 0.0 && y_last >= 0.0 {
        y_scale.at(0.0)
    } else {
        bottom
    };
    let _ = write!(
        svg,
        "<line class=\"dappr-chart-baseline\" x1=\"{left:.1}\" x2=\"{right:.1}\" y1=\"{baseline:.1}\" y2=\"{baseline:.1}\"/>"
    );

    // Hover aids drawn under the marks; the component moves and shows them.
    match spec.kind {
        ChartKind::Bar => {
            let _ = write!(
                svg,
                "<rect class=\"dappr-chart-band\" visibility=\"hidden\" y=\"{top:.1}\" width=\"{band:.1}\" height=\"{:.1}\"/>",
                bottom - top
            );
        }
        ChartKind::Line => {
            let _ = write!(
                svg,
                "<line class=\"dappr-chart-crosshair\" visibility=\"hidden\" y1=\"{top:.1}\" y2=\"{bottom:.1}\"/>"
            );
        }
        ChartKind::Scatter => {}
    }

    // Marks.
    match spec.kind {
        ChartKind::Line => {
            for (index, series) in series.iter().enumerate() {
                // A missing value breaks the line rather than being bridged.
                let mut path = String::new();
                let mut pen_down = false;
                let mut lone_points = Vec::new();
                for (position, point) in series.points.iter().enumerate() {
                    match point {
                        Some(point) => {
                            let command = if pen_down { 'L' } else { 'M' };
                            let _ = write!(path, "{command}{:.1},{:.1}", rows[position].x, point.y);
                            let next_missing = series
                                .points
                                .get(position + 1)
                                .is_none_or(|next| next.is_none());
                            if !pen_down && next_missing {
                                // A value with gaps on both sides has no
                                // line to belong to: draw it as a dot.
                                lone_points.push((rows[position].x, point.y));
                            }
                            pen_down = true;
                        }
                        None => pen_down = false,
                    }
                }
                let _ = write!(
                    svg,
                    "<path class=\"dappr-chart-line dappr-s{index}\" d=\"{path}\"/>"
                );
                for (x, y) in lone_points {
                    let _ = write!(svg, "<circle class=\"dappr-chart-dot dappr-s{index}\" cx=\"{x:.1}\" cy=\"{y:.1}\" r=\"4\"/>");
                }
            }
        }
        ChartKind::Bar => {
            let count = series.len() as f64;
            // Thin bars (24px at most) with a 2px gap between neighbours.
            let bar_width = ((band * 0.8 - 2.0 * (count - 1.0)) / count).clamp(1.0, 24.0);
            let group_width = bar_width * count + 2.0 * (count - 1.0);
            for (index, series) in series.iter().enumerate() {
                let _ = write!(svg, "<g class=\"dappr-chart-bars dappr-s{index}\">");
                for (position, point) in series.points.iter().enumerate() {
                    if let Some(point) = point {
                        let x =
                            rows[position].x - group_width / 2.0 + index as f64 * (bar_width + 2.0);
                        let _ = write!(
                            svg,
                            "<path d=\"{}\"/>",
                            bar_path(x, bar_width, baseline, point.y)
                        );
                    }
                }
                svg.push_str("</g>");
            }
        }
        ChartKind::Scatter => {
            for (index, series) in series.iter().enumerate() {
                let _ = write!(svg, "<g class=\"dappr-chart-dots dappr-s{index}\">");
                for (position, point) in series.points.iter().enumerate() {
                    if let Some(point) = point {
                        let _ = write!(
                            svg,
                            "<circle class=\"dappr-chart-dot\" cx=\"{:.1}\" cy=\"{:.1}\" r=\"4\"/>",
                            rows[position].x, point.y
                        );
                    }
                }
                svg.push_str("</g>");
            }
        }
    }

    // Line-end labels, unless two of them would overlap (lines that end
    // close together are left to the legend).
    if end_labels {
        let ends: Vec<(usize, f64, f64)> = series
            .iter()
            .enumerate()
            .filter_map(|(index, series)| {
                let position = series.points.iter().rposition(Option::is_some)?;
                Some((index, rows[position].x, series.points[position]?.y))
            })
            .collect();
        let mut ys: Vec<f64> = ends.iter().map(|end| end.2).collect();
        ys.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let clear = ys
            .windows(2)
            .all(|pair| pair[1] - pair[0] >= FONT_SIZE + 3.0);
        if clear {
            svg.push_str("<g class=\"dappr-chart-end-labels\">");
            for (index, x, y) in ends {
                let _ = write!(
                    svg,
                    "<text x=\"{:.1}\" y=\"{:.1}\">{}</text>",
                    x + 8.0,
                    y + FONT_SIZE / 3.0,
                    escape(&shorten(&series[index].name))
                );
            }
            svg.push_str("</g>");
        }
    }

    // Hover dots drawn over the marks; hidden until the component shows them.
    if spec.kind != ChartKind::Bar {
        for index in 0..series.len() {
            let _ = write!(
                svg,
                "<circle class=\"dappr-chart-hover dappr-s{index}\" visibility=\"hidden\" r=\"5\"/>"
            );
        }
    }
    svg.push_str("</svg>");

    if let Some(text) = &mut note {
        if spec.kind == ChartKind::Scatter {
            text.push_str(" (a scatter chart keeps to three so the colours stay distinct)");
        }
    }

    Plot {
        svg,
        layout: Some(Layout {
            kind: spec.kind,
            x_name: spec.x.clone(),
            left,
            right,
            top,
            bottom,
            band,
            rows,
            series,
        }),
        note,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sales() -> DTable {
        let mut table = DTable::with_columns(&["month", "north", "south"]);
        table.push_row(("Jan", 120, 80));
        table.push_row(("Feb", 150, 95));
        table.push_row(("Mar", 90, 140));
        table.push_row(("Apr", 200, 130));
        table
    }

    #[test]
    fn nice_ticks_land_on_round_numbers() {
        assert_eq!(nice_ticks(0.0, 200.0, 5), (0.0, 50.0, 5));
        assert_eq!(nice_ticks(80.0, 200.0, 5), (75.0, 25.0, 6));
        assert_eq!(nice_ticks(-30.0, 70.0, 5), (-40.0, 20.0, 7));
        let (first, step, count) = nice_ticks(0.12, 0.47, 5);
        assert!((first - 0.1).abs() < 1e-12 && (step - 0.1).abs() < 1e-12 && count == 5);
        // A single value still gets a range around it.
        let (first, step, count) = nice_ticks(5.0, 5.0, 5);
        assert!(first < 5.0 && first + step * (count - 1) as f64 > 5.0);
    }

    #[test]
    fn numbers_are_formatted_for_reading() {
        assert_eq!(format_value(1234567.5), "1,234,567.5");
        assert_eq!(format_value(-1000.0), "-1,000");
        assert_eq!(format_value(12.0 * 34.9), "418.8");
        assert_eq!(format_tick(2500.0, 500.0, false), "2,500");
        assert_eq!(format_tick(0.25, 0.05, false), "0.25");
        // A step of 2.5 must not be rounded to whole numbers ("2", "8").
        assert_eq!(format_tick(2.5, 2.5, false), "2.5");
        assert_eq!(format_tick(5.0, 2.5, false), "5.0");
        assert_eq!(format_tick(15000.0, 5000.0, true), "15K");
        assert_eq!(format_tick(1_500_000.0, 500_000.0, true), "1.5M");
        assert_eq!(format_tick(0.0, 5000.0, true), "0");
    }

    #[test]
    fn text_from_data_is_escaped() {
        let mut table = DTable::with_columns(&["<x>", "a&b"]);
        table.push_row(("<script>alert(1)</script>", 1));
        table.push_row(("\"quoted\"", 2));
        let plot = render(
            &ChartSpec::new(ChartKind::Bar, "<x>", &["a&b"]),
            &table,
            600.0,
        );
        assert!(!plot.svg.contains("<script>"));
        assert!(plot.svg.contains("&lt;script&gt;"));
        assert!(plot.svg.contains("a&amp;b"));
        assert!(!plot.svg.contains("\"quoted\""));
    }

    #[test]
    fn line_chart_places_rows_left_to_right_and_values_bottom_to_top() {
        let plot = render(
            &ChartSpec::new(ChartKind::Line, "month", &["north", "south"]),
            &sales(),
            640.0,
        );
        let layout = plot.layout.unwrap();
        assert_eq!(layout.rows.len(), 4);
        assert!(layout.rows.windows(2).all(|w| w[0].x < w[1].x));
        assert_eq!(layout.rows[0].label, "Jan");
        let north = &layout.series[0];
        assert_eq!(north.name, "north");
        // 200 (Apr) is higher on screen than 90 (Mar): smaller y.
        assert!(north.points[3].unwrap().y < north.points[2].unwrap().y);
        assert_eq!(north.points[1].unwrap().value, 150.0);
        // Two lines, two colours, in the order given.
        assert_eq!(plot.svg.matches("class=\"dappr-chart-line").count(), 2);
        assert!(plot.svg.contains("dappr-chart-line dappr-s0"));
        assert!(plot.svg.contains("dappr-chart-line dappr-s1"));
        assert!(plot.note.is_none());
    }

    #[test]
    fn line_end_labels_appear_only_when_they_fit() {
        let spec = ChartSpec::new(ChartKind::Line, "month", &["north", "south"]);
        // Ends at 200 and 130: far apart, so both lines are named.
        let apart = render(&spec, &sales(), 640.0);
        assert!(apart.svg.contains("dappr-chart-end-labels"));
        assert!(apart.svg.contains(">north</text>"));

        // Same end value: the labels would sit on top of each other.
        let mut close = sales();
        close.push_row(("May", 100, 101));
        assert!(!render(&spec, &close, 640.0)
            .svg
            .contains("dappr-chart-end-labels"));

        // One series needs no name beside it.
        let single = render(
            &ChartSpec::new(ChartKind::Line, "month", &["north"]),
            &sales(),
            640.0,
        );
        assert!(!single.svg.contains("dappr-chart-end-labels"));
    }

    #[test]
    fn missing_values_break_the_line() {
        let mut table = DTable::with_columns(&["t", "v"]);
        table.push_row(("a", 1));
        table.push_row(("b", 2));
        table.push_row(("c", None::<f64>));
        table.push_row(("d", 4));
        let plot = render(&ChartSpec::new(ChartKind::Line, "t", &["v"]), &table, 600.0);
        let layout = plot.layout.unwrap();
        assert!(layout.series[0].points[2].is_none());
        // Two separate strokes (two "move to" commands) and one lone dot.
        let path = plot
            .svg
            .split("dappr-chart-line dappr-s0\" d=\"")
            .nth(1)
            .unwrap();
        let path = &path[..path.find('"').unwrap()];
        assert_eq!(path.matches('M').count(), 2);
        assert_eq!(
            plot.svg
                .matches("class=\"dappr-chart-dot dappr-s0\"")
                .count(),
            1
        );
    }

    #[test]
    fn bars_start_at_zero_stay_thin_and_keep_a_gap() {
        let plot = render(
            &ChartSpec::new(ChartKind::Bar, "month", &["north", "south"]),
            &sales(),
            640.0,
        );
        // The axis includes zero even though the smallest value is 80.
        assert!(plot.svg.contains(">0</text>"));
        let layout = plot.layout.unwrap();
        // One <path> per value.
        assert_eq!(plot.svg.matches("<path d=\"M").count(), 8);
        // First bar of each series in the first category: 24px wide, 2px apart.
        let xs: Vec<f64> = plot
            .svg
            .split("<path d=\"M")
            .skip(1)
            .map(|p| p[..p.find(',').unwrap()].parse().unwrap())
            .collect();
        let (north_jan, south_jan) = (xs[0], xs[4]);
        assert!((south_jan - north_jan - 26.0).abs() < 0.11);
        // The pair is centred on its category.
        assert!(((north_jan + south_jan + 24.0) / 2.0 - layout.rows[0].x).abs() < 0.11);
    }

    #[test]
    fn bars_handle_negative_values() {
        let mut table = DTable::with_columns(&["k", "v"]);
        table.push_row(("a", 5));
        table.push_row(("b", -3));
        let plot = render(&ChartSpec::new(ChartKind::Bar, "k", &["v"]), &table, 600.0);
        let layout = plot.layout.unwrap();
        let (up, down) = (
            layout.series[0].points[0].unwrap().y,
            layout.series[0].points[1].unwrap().y,
        );
        // Zero sits between the two bar ends.
        let zero = plot.svg.split("dappr-chart-baseline").nth(1).unwrap();
        let zero: f64 = zero
            .split("y1=\"")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap()
            .parse()
            .unwrap();
        assert!(up < zero && zero < down);
    }

    #[test]
    fn numeric_x_is_a_number_line_for_lines_and_scatter() {
        let mut table = DTable::with_columns(&["x", "y"]);
        table.push_row((0, 1));
        table.push_row((1, 2));
        table.push_row((10, 3));
        for kind in [ChartKind::Line, ChartKind::Scatter] {
            let layout = render(&ChartSpec::new(kind, "x", &["y"]), &table, 600.0)
                .layout
                .unwrap();
            let gap_small = layout.rows[1].x - layout.rows[0].x;
            let gap_large = layout.rows[2].x - layout.rows[1].x;
            assert!((gap_large / gap_small - 9.0).abs() < 1e-6, "{kind:?}");
        }
        // A bar chart treats the same column as categories: evenly spaced.
        let layout = render(&ChartSpec::new(ChartKind::Bar, "x", &["y"]), &table, 600.0)
            .layout
            .unwrap();
        let gaps = (
            layout.rows[1].x - layout.rows[0].x,
            layout.rows[2].x - layout.rows[1].x,
        );
        assert!((gaps.0 - gaps.1).abs() < 1e-6);
    }

    #[test]
    fn series_limits_are_applied_and_reported() {
        let names: Vec<String> = (0..10).map(|i| format!("s{i}")).collect();
        let mut headers = vec!["x"];
        headers.extend(names.iter().map(String::as_str));
        let mut table = DTable::with_columns(&headers);
        table.push_row((0..11).collect::<Vec<i32>>());
        table.push_row((1..12).collect::<Vec<i32>>());
        let y: Vec<&str> = names.iter().map(String::as_str).collect();

        let line = render(&ChartSpec::new(ChartKind::Line, "x", &y), &table, 600.0);
        assert_eq!(line.layout.unwrap().series.len(), MAX_SERIES);
        assert_eq!(line.note.unwrap(), "Showing the first 8 of 10 series");
        assert!(!line.svg.contains("dappr-s8"));

        let scatter = render(&ChartSpec::new(ChartKind::Scatter, "x", &y), &table, 600.0);
        assert_eq!(scatter.layout.unwrap().series.len(), MAX_SCATTER_SERIES);
        assert!(scatter
            .note
            .unwrap()
            .starts_with("Showing the first 3 of 10 series"));
    }

    #[test]
    fn problems_are_explained_instead_of_drawn() {
        let table = sales();
        let no_x = render(
            &ChartSpec::new(ChartKind::Line, "nope", &["north"]),
            &table,
            600.0,
        );
        assert!(no_x.layout.is_none());
        assert!(no_x.svg.contains("No column named &quot;nope&quot;"));
        let no_y = render(
            &ChartSpec::new(ChartKind::Line, "month", &["nope"]),
            &table,
            600.0,
        );
        assert!(no_y.svg.contains("No column named &quot;nope&quot;"));
        let text_x = render(
            &ChartSpec::new(ChartKind::Scatter, "month", &["north"]),
            &table,
            600.0,
        );
        assert!(text_x.svg.contains("A scatter chart needs numbers"));
        let empty = render(
            &ChartSpec::new(ChartKind::Bar, "month", &["north"]),
            &table.filter(|_| false),
            600.0,
        );
        assert!(empty.svg.contains("No data to plot"));
        let text_y = render(
            &ChartSpec::new(ChartKind::Bar, "north", &["month"]),
            &table,
            600.0,
        );
        assert!(text_y.svg.contains("No numbers to plot"));
    }

    #[test]
    fn crowded_category_labels_are_thinned() {
        let mut table = DTable::with_columns(&["day", "v"]);
        for day in 1..=60 {
            table.push_row((format!("2026-01-{day:02}"), day));
        }
        let plot = render(
            &ChartSpec::new(ChartKind::Line, "day", &["v"]),
            &table,
            500.0,
        );
        let labels = plot.svg.matches(">2026-01-").count();
        assert!(labels > 1 && labels < 10, "{labels} labels drawn");
    }

    #[test]
    fn large_axes_use_compact_labels() {
        let mut table = DTable::with_columns(&["k", "v"]);
        table.push_row(("a", 12_000));
        table.push_row(("b", 48_000));
        let plot = render(&ChartSpec::new(ChartKind::Bar, "k", &["v"]), &table, 600.0);
        assert!(plot.svg.contains(">50K</text>"));
        assert!(!plot.svg.contains("50,000"));
    }
}
