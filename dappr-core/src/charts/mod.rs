//! Charts drawn from a [`DTable`]: line, area, bar, scatter and histogram.
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
    /// Lines with the space down to zero lightly filled, for an amount that
    /// changes along an ordered x. Best with one or two series.
    Area,
    /// Vertical bars, for comparing amounts across categories.
    Bar,
    /// Dots, for the relationship between two numeric columns.
    Scatter,
    /// Bars counting how many rows fall in each range of one numeric
    /// column, to see how its values are spread.
    Histogram,
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
    /// Bar charts: stack the series on top of each other instead of placing
    /// them side by side.
    pub stacked: bool,
    /// Histograms: roughly how many bars to aim for; 0 picks a number from
    /// the amount of data. The ranges are rounded to tidy numbers, so the
    /// exact count can differ.
    pub bins: usize,
    /// Bar charts: draw the bars sideways, with the categories down the
    /// left. The right choice for long category names, or many categories.
    pub horizontal: bool,
    /// How dates on x are written, for [`parse_date_with`], when they are
    /// not in a form that is recognised on its own (see [`parse_date`]).
    pub date_format: Option<String>,
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
            stacked: false,
            bins: 0,
            horizontal: false,
            date_format: None,
        }
    }

    /// The series actually drawn: the `y` columns, up to the limit for this
    /// kind of chart.
    pub fn drawn_series(&self) -> &[String] {
        let limit = match self.kind {
            ChartKind::Scatter => MAX_SCATTER_SERIES,
            // A histogram counts one column: it has no y columns.
            ChartKind::Histogram => 0,
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
    /// Whether the chart is turned on its side (horizontal bars). Then the
    /// categories run top to bottom: a row's `x` is its *vertical* position
    /// and a point's `y` is its *horizontal* one.
    pub horizontal: bool,
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

/// A number shortened for a headline figure: `1,284` in full below ten
/// thousand, then `12.9K`, `4.2M`, `1.3B`.
pub fn format_compact(value: f64) -> String {
    if !value.is_finite() {
        return value.to_string();
    }
    let (scaled, suffix) = match value.abs() {
        v if v >= 1e9 => (value / 1e9, "B"),
        v if v >= 1e6 => (value / 1e6, "M"),
        v if v >= 1e4 => (value / 1e3, "K"),
        _ => return format_value((value * 100.0).round() / 100.0),
    };
    // Three significant digits: 12.9K, 129K, 1.29M.
    let decimals = match scaled.abs() {
        v if v >= 100.0 => 0,
        v if v >= 10.0 => 1,
        _ => 2,
    };
    let text = format!("{scaled:.decimals$}");
    let text = if text.contains('.') {
        text.trim_end_matches('0').trim_end_matches('.')
    } else {
        &text
    };
    format!("{text}{suffix}")
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

// ---- Dates ------------------------------------------------------------------

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Days from 1970-01-01 to the given date (proleptic Gregorian calendar).
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let year_of_era = y.rem_euclid(400);
    let month_index = (i64::from(month) + 9) % 12;
    let day_of_year = (153 * month_index + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// The date that is `days` after 1970-01-01, as (year, month, day).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_index + 2) / 5 + 1) as u32;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    } as u32;
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

fn days_in_month(year: i64, month: u32) -> u32 {
    (days_from_civil(year + i64::from(month == 12), month % 12 + 1, 1)
        - days_from_civil(year, month, 1)) as u32
}

/// The month (1-12) a name stands for: `Mar`, `march`, `MARCH`. At least
/// the first three letters, and nothing that is not part of the name.
fn month_from_name(name: &str) -> Option<u32> {
    const FULL: [&str; 12] = [
        "january",
        "february",
        "march",
        "april",
        "may",
        "june",
        "july",
        "august",
        "september",
        "october",
        "november",
        "december",
    ];
    let name = name.trim_end_matches('.').to_ascii_lowercase();
    if name.len() < 3 {
        return None;
    }
    FULL.iter()
        .position(|full| full.starts_with(&name))
        .map(|index| index as u32 + 1)
}

/// A number written with exactly `len` digits.
fn fixed_digits(part: &str, len: usize) -> Option<u32> {
    (part.len() == len && part.bytes().all(|b| b.is_ascii_digit()))
        .then(|| part.parse().ok())
        .flatten()
}

/// A number written with one or two digits.
fn short_digits(part: &str) -> Option<u32> {
    ((1..=2).contains(&part.len()) && part.bytes().all(|b| b.is_ascii_digit()))
        .then(|| part.parse().ok())
        .flatten()
}

/// Days since 1970-01-01 for a date, or `None` if there is no such day.
fn checked_days(year: i64, month: u32, day: u32) -> Option<f64> {
    ((1..=12).contains(&month) && day >= 1 && day <= days_in_month(year, month))
        .then(|| days_from_civil(year, month, day) as f64)
}

/// A time of day (`14:30`, `14:30:15`, `14:30:15.5`) as a fraction of a day.
/// A trailing `Z` or offset (`+01:00`, `-0500`) is applied, so the result is
/// in UTC.
fn parse_time(text: &str) -> Option<f64> {
    let text = text.trim();
    let (clock, offset_minutes) = if let Some(clock) = text.strip_suffix('Z') {
        (clock, 0.0)
    } else if let Some(position) = text.rfind(['+', '-']) {
        let (clock, offset) = text.split_at(position);
        let sign = if offset.starts_with('-') { -1.0 } else { 1.0 };
        let digits: String = offset[1..].chars().filter(|c| *c != ':').collect();
        let (hours, minutes) = match digits.len() {
            2 => (fixed_digits(&digits, 2)?, 0),
            4 => (
                fixed_digits(&digits[..2], 2)?,
                fixed_digits(&digits[2..], 2)?,
            ),
            _ => return None,
        };
        (clock, sign * f64::from(hours * 60 + minutes))
    } else {
        (text, 0.0)
    };
    let parts: Vec<&str> = clock.split(':').collect();
    let (hours, minutes, seconds) = match parts.as_slice() {
        [h, m] => (fixed_digits(h, 2)?, fixed_digits(m, 2)?, 0.0),
        [h, m, s] => (
            fixed_digits(h, 2)?,
            fixed_digits(m, 2)?,
            s.parse::<f64>().ok()?,
        ),
        _ => return None,
    };
    if hours > 23 || minutes > 59 || !(0.0..60.0).contains(&seconds) {
        return None;
    }
    let seconds_of_day = f64::from(hours) * 3600.0 + f64::from(minutes) * 60.0 + seconds;
    Some((seconds_of_day - offset_minutes * 60.0) / 86_400.0)
}

/// Reads a date or date-time as days since 1970-01-01 (with the time of day
/// as a fraction). Only forms that cannot be misread are understood:
///
/// - year first: `2026-03-05`, `2026/03/05`, `2026-03` (the first of the month);
/// - dotted, day first: `05.03.2026`;
/// - with the month named: `5 Mar 2026`, `Mar 5, 2026`, `March 2026`;
/// - any of the first two followed by `T` or a space and a time: `14:30`,
///   `14:30:15`, optionally with `Z` or an offset such as `+01:00`.
///
/// Slashed dates with the year last (`03/05/2026`) are deliberately not
/// read: they mean different days in different countries. Use
/// [`parse_date_with`] for those.
pub fn parse_date(text: &str) -> Option<f64> {
    let text = text.trim();

    // With the month named: no time of day in these forms.
    if text
        .chars()
        .any(|c| c.is_ascii_alphabetic() && c != 'T' && c != 'Z')
    {
        let parts: Vec<&str> = text
            .split([' ', ','])
            .filter(|part| !part.is_empty())
            .collect();
        let year = |part: &str| fixed_digits(part, 4).map(i64::from);
        return match parts.as_slice() {
            [d, m, y] if short_digits(d).is_some() => {
                checked_days(year(y)?, month_from_name(m)?, short_digits(d)?)
            }
            [m, d, y] => checked_days(year(y)?, month_from_name(m)?, short_digits(d)?),
            [m, y] => checked_days(year(y)?, month_from_name(m)?, 1),
            _ => None,
        };
    }

    let (date, time) = match text.split_once(['T', ' ']) {
        Some((date, time)) => (date, Some(time)),
        None => (text, None),
    };
    let days = if date.contains('.') {
        let parts: Vec<&str> = date.split('.').collect();
        match parts.as_slice() {
            [d, m, y] => checked_days(
                i64::from(fixed_digits(y, 4)?),
                fixed_digits(m, 2)?,
                fixed_digits(d, 2)?,
            )?,
            _ => return None,
        }
    } else {
        let parts: Vec<&str> = date.split(['-', '/']).collect();
        match parts.as_slice() {
            [y, m, d] => checked_days(
                i64::from(fixed_digits(y, 4)?),
                fixed_digits(m, 2)?,
                fixed_digits(d, 2)?,
            )?,
            [y, m] if time.is_none() => {
                checked_days(i64::from(fixed_digits(y, 4)?), fixed_digits(m, 2)?, 1)?
            }
            _ => return None,
        }
    };
    match time {
        Some(time) => Some(days + parse_time(time)?),
        None => Some(days),
    }
}

/// Reads a date written in a format you describe, as days since 1970-01-01.
///
/// The format is the text with these stand-ins: `%Y` a four-digit year,
/// `%y` a two-digit year (2000-2099), `%m` the month as a number, `%b` its
/// name (`Mar`, `March`), `%d` the day, `%H` `%M` `%S` hours, minutes and
/// seconds. Month and day may be written with one or two digits. Everything
/// else in the format must appear as is.
///
/// ```
/// use dappr_core::charts::{parse_date, parse_date_with};
///
/// // The 3rd of May in the US, the 5th of March elsewhere: say which.
/// assert_eq!(parse_date_with("03/05/2026", "%m/%d/%Y"), parse_date("2026-03-05"));
/// assert_eq!(parse_date_with("03/05/2026", "%d/%m/%Y"), parse_date("2026-05-03"));
/// ```
pub fn parse_date_with(text: &str, format: &str) -> Option<f64> {
    let mut rest = text.trim();
    let (mut year, mut month, mut day) = (None, 1u32, 1u32);
    let (mut hours, mut minutes, mut seconds) = (0u32, 0u32, 0u32);

    // Takes the leading run of characters matching `keep`, up to `max` long.
    fn take<'a>(rest: &mut &'a str, max: usize, keep: fn(char) -> bool) -> &'a str {
        let end = rest
            .char_indices()
            .take(max)
            .take_while(|(_, c)| keep(*c))
            .last()
            .map_or(0, |(i, c)| i + c.len_utf8());
        let (taken, after) = rest.split_at(end);
        *rest = after;
        taken
    }
    let digit = |c: char| c.is_ascii_digit();

    let mut format = format.trim().chars();
    while let Some(c) = format.next() {
        if c != '%' {
            rest = rest.strip_prefix(c)?;
            continue;
        }
        match format.next()? {
            'Y' => year = Some(i64::from(fixed_digits(take(&mut rest, 4, digit), 4)?)),
            'y' => year = Some(2000 + i64::from(fixed_digits(take(&mut rest, 2, digit), 2)?)),
            'm' => month = short_digits(take(&mut rest, 2, digit))?,
            'd' => day = short_digits(take(&mut rest, 2, digit))?,
            'H' => hours = short_digits(take(&mut rest, 2, digit))?,
            'M' => minutes = short_digits(take(&mut rest, 2, digit))?,
            'S' => seconds = short_digits(take(&mut rest, 2, digit))?,
            'b' => month = month_from_name(take(&mut rest, 9, |c| c.is_ascii_alphabetic()))?,
            '%' => rest = rest.strip_prefix('%')?,
            _ => return None,
        }
    }
    if !rest.is_empty() || hours > 23 || minutes > 59 || seconds > 59 {
        return None;
    }
    let days = checked_days(year?, month, day)?;
    Some(days + f64::from(hours * 3600 + minutes * 60 + seconds) / 86_400.0)
}

/// Tick positions (in days) and labels for a time axis from `min` to `max`,
/// aiming for about `target` ticks on round moments: whole hours, days,
/// the first of a month, the first of a year.
fn time_ticks(min: f64, max: f64, target: usize) -> Vec<(f64, String)> {
    #[derive(Clone, Copy)]
    enum Unit {
        Hours(i64),
        Days(i64),
        Months(i64),
        Years(i64),
    }
    let candidates = [
        Unit::Hours(1),
        Unit::Hours(3),
        Unit::Hours(6),
        Unit::Hours(12),
        Unit::Days(1),
        Unit::Days(2),
        Unit::Days(7),
        Unit::Days(14),
        Unit::Months(1),
        Unit::Months(3),
        Unit::Months(6),
        Unit::Years(1),
        Unit::Years(2),
        Unit::Years(5),
        Unit::Years(10),
        Unit::Years(20),
        Unit::Years(50),
        Unit::Years(100),
        Unit::Years(500),
        Unit::Years(1000),
    ];
    let length = |unit: Unit| match unit {
        Unit::Hours(n) => n as f64 / 24.0,
        Unit::Days(n) => n as f64,
        Unit::Months(n) => n as f64 * 30.44,
        Unit::Years(n) => n as f64 * 365.25,
    };
    let span = max - min;
    let target = target.max(2) as f64;
    let unit = candidates
        .into_iter()
        .find(|unit| span / length(*unit) <= target)
        .unwrap_or(Unit::Years(1000));

    let day_label = |day: i64| {
        let (_, month, day) = civil_from_days(day);
        format!("{} {day}", MONTHS[month as usize - 1])
    };
    let mut ticks = Vec::new();
    match unit {
        Unit::Hours(n) => {
            let step = n as f64 / 24.0;
            let mut tick = (min / step).ceil() * step;
            while tick <= max + 1e-9 {
                let hour = ((tick - tick.floor()) * 24.0).round() as i64 % 24;
                // Midnight is labelled with the day it starts.
                let label = if hour == 0 {
                    day_label(tick.round() as i64)
                } else {
                    format!("{hour:02}:00")
                };
                ticks.push((tick, label));
                tick += step;
            }
        }
        Unit::Days(n) => {
            let mut day = min.ceil() as i64;
            while day as f64 <= max + 1e-9 {
                ticks.push((day as f64, day_label(day)));
                day += n;
            }
        }
        Unit::Months(n) => {
            let (mut year, month, _) = civil_from_days(min.floor() as i64);
            // 0-based month, moved back to a multiple of the step.
            let mut month = (i64::from(month) - 1) / n * n;
            loop {
                let tick = days_from_civil(year, month as u32 + 1, 1) as f64;
                if tick > max + 1e-9 {
                    break;
                }
                if tick >= min - 1e-9 {
                    // The year is shown on the first tick and each January.
                    let label = if month == 0 || ticks.is_empty() {
                        format!("{} {year}", MONTHS[month as usize])
                    } else {
                        MONTHS[month as usize].to_string()
                    };
                    ticks.push((tick, label));
                }
                month += n;
                if month >= 12 {
                    month -= 12;
                    year += 1;
                }
            }
        }
        Unit::Years(n) => {
            let (first_year, _, _) = civil_from_days(min.floor() as i64);
            let mut year = first_year.div_euclid(n) * n;
            loop {
                let tick = days_from_civil(year, 1, 1) as f64;
                if tick > max + 1e-9 {
                    break;
                }
                if tick >= min - 1e-9 {
                    ticks.push((tick, year.to_string()));
                }
                year += n;
            }
        }
    }
    ticks
}

// ---- Histogram bins ---------------------------------------------------------

/// The ranges a histogram counts in: `counts[i]` rows fall in
/// `first + i * step .. first + (i + 1) * step`.
struct Bins {
    first: f64,
    step: f64,
    counts: Vec<usize>,
}

fn bins_of(values: &[f64], wanted: usize) -> Bins {
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    // With no number given: more bars for more data (Sturges' rule), within
    // sensible limits.
    let target = if wanted > 0 {
        wanted
    } else {
        ((values.len() as f64).log2().ceil() as usize + 1).clamp(5, 30)
    };
    // Tidy ranges: a width of 1, 2, 2.5 or 5 times a power of ten -- the
    // one that gives a number of bars closest to the number wanted.
    let (min, max) = if min == max {
        let pad = if min == 0.0 { 1.0 } else { min.abs() * 0.1 };
        (min - pad, max + pad)
    } else {
        (min, max)
    };
    let magnitude = 10f64.powf(((max - min) / target as f64).log10().floor());
    let (first, step, count) = [0.5, 1.0, 2.0, 2.5, 5.0, 10.0]
        .iter()
        .map(|multiple| {
            let step = multiple * magnitude;
            let first = (min / step).floor() * step;
            // The last range includes its upper edge, so a largest value
            // sitting exactly on an edge does not need a range of its own.
            let count = ((((max - first) / step) - 1e-9).ceil() as usize).max(1);
            (first, step, count)
        })
        .min_by_key(|(_, _, count)| count.abs_diff(target))
        .expect("there are candidate widths");
    let mut counts = vec![0; count];
    for value in values {
        // The largest value belongs to the last range, not one past it.
        let index = (((value - first) / step).floor() as usize).min(count - 1);
        counts[index] += 1;
    }
    Bins {
        first,
        step,
        counts,
    }
}

/// The table a histogram of `column` draws: one row per range, with columns
/// `from`, `to` and `rows` (how many rows of `data` fall in the range).
/// `None` if the column is missing or holds no numbers.
pub fn histogram_table(data: &DTable, column: &str, bins: usize) -> Option<DTable> {
    let values: Vec<f64> = data
        .column(column)?
        .numbers()
        .into_iter()
        .filter(|v| v.is_finite())
        .collect();
    if values.is_empty() {
        return None;
    }
    let bins = bins_of(&values, bins);
    let mut table = DTable::with_columns(&["from", "to", "rows"]);
    for (index, count) in bins.counts.iter().enumerate() {
        let from = bins.first + bins.step * index as f64;
        table.push_row([from, from + bins.step, *count as f64]);
    }
    Some(table)
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

/// Where each segment of a stacked bar starts and ends, in data units:
/// positive values pile up from zero, negative ones pile down from it.
fn stack(values: &[Vec<Option<f64>>], rows: usize) -> Vec<Vec<Option<(f64, f64)>>> {
    let mut up = vec![0.0; rows];
    let mut down = vec![0.0; rows];
    values
        .iter()
        .map(|series| {
            series
                .iter()
                .enumerate()
                .map(|(row, value)| {
                    value.map(|value| {
                        let base = if value >= 0.0 {
                            &mut up[row]
                        } else {
                            &mut down[row]
                        };
                        let from = *base;
                        *base += value;
                        (from, *base)
                    })
                })
                .collect()
        })
        .collect()
}

/// A bar segment from `from` to `to` (both y positions). With `round_end`,
/// the `to` end is rounded: that is the end showing the value. The `from`
/// end -- the baseline, or the segment below in a stack -- stays square.
fn bar_path(x: f64, width: f64, from: f64, to: f64, round_end: bool) -> String {
    let height = (from - to).abs();
    let r = if round_end {
        4f64.min(width / 2.0).min(height)
    } else {
        0.0
    };
    // `d` is +1 when the bar goes up from `from`, -1 when down.
    let d = if to <= from { 1.0 } else { -1.0 };
    format!(
        "M{x0:.1},{b:.1}L{x0:.1},{yr:.1}Q{x0:.1},{y:.1} {xr0:.1},{y:.1}L{xr1:.1},{y:.1}Q{x1:.1},{y:.1} {x1:.1},{yr:.1}L{x1:.1},{b:.1}Z",
        x0 = x,
        x1 = x + width,
        xr0 = x + r,
        xr1 = x + width - r,
        b = from,
        y = to,
        yr = to + d * r,
    )
}

/// A sideways bar segment at height `y`, from `from` to `to` (both x
/// positions), `thickness` tall. As in [`bar_path`], only the `to` end may
/// be rounded.
fn horizontal_bar_path(y: f64, thickness: f64, from: f64, to: f64, round_end: bool) -> String {
    let length = (to - from).abs();
    let r = if round_end {
        4f64.min(thickness / 2.0).min(length)
    } else {
        0.0
    };
    // `d` is +1 when the bar goes right from `from`, -1 when left.
    let d = if to >= from { 1.0 } else { -1.0 };
    format!(
        "M{b:.1},{y0:.1}L{xr:.1},{y0:.1}Q{x:.1},{y0:.1} {x:.1},{yr0:.1}L{x:.1},{yr1:.1}Q{x:.1},{y1:.1} {xr:.1},{y1:.1}L{b:.1},{y1:.1}Z",
        b = from,
        x = to,
        xr = to - d * r,
        y0 = y,
        y1 = y + thickness,
        yr0 = y + r,
        yr1 = y + thickness - r,
    )
}

/// A bar chart turned on its side: categories down the left, values along
/// the bottom. Unlike the other charts it grows taller with the number of
/// categories, so every one keeps a readable row.
fn render_horizontal_bars(spec: &ChartSpec, data: &DTable, width: f64) -> Plot {
    let width = width.max(160.0);
    let Some(category_column) = data.column(&spec.x) else {
        return message(
            width,
            spec.height,
            &format!("No column named \"{}\"", spec.x),
        );
    };
    let drawn = spec.drawn_series();
    if drawn.is_empty() {
        return message(width, spec.height, "No columns to plot");
    }
    for name in drawn {
        if data.column(name).is_none() {
            return message(width, spec.height, &format!("No column named \"{name}\""));
        }
    }
    let note = (drawn.len() < spec.y.len()).then(|| {
        format!(
            "Showing the first {} of {} series",
            drawn.len(),
            spec.y.len()
        )
    });
    let n = data.n_rows();
    if n == 0 {
        return message(width, spec.height, "No data to plot");
    }
    let values: Vec<Vec<Option<f64>>> = drawn
        .iter()
        .map(|name| {
            data.column(name)
                .map(|column| {
                    column
                        .cells()
                        .iter()
                        .map(|cell| cell.as_f64().filter(|v| v.is_finite()))
                        .collect()
                })
                .unwrap_or_default()
        })
        .collect();
    let stacked = spec.stacked && drawn.len() > 1;
    let stacks = if stacked {
        stack(&values, n)
    } else {
        Vec::new()
    };
    let extent: Vec<f64> = if stacked {
        stacks
            .iter()
            .flatten()
            .flatten()
            .map(|(_, to)| *to)
            .collect()
    } else {
        values.iter().flatten().flatten().copied().collect()
    };
    if extent.is_empty() {
        return message(width, spec.height, "No numbers to plot");
    }

    // ---- Labels down the left, at most 40% of the width ----
    let labels: Vec<String> = category_column
        .cells()
        .iter()
        .map(|cell| match cell {
            Cell::Number(number) => format_value(*number),
            other => other.to_string(),
        })
        .collect();
    let widest = labels
        .iter()
        .map(|label| text_width(&shorten(label)))
        .fold(0.0, f64::max);
    let left = (widest + 14.0).min(width * 0.4);
    let right = (width - 16.0).max(left + 40.0);
    // How many characters of a label fit in the space on the left.
    let fits = (((left - 14.0) / CHAR_WIDTH).floor() as usize).max(3);
    let fit = |label: &str| -> String {
        if label.chars().count() <= fits {
            label.to_string()
        } else {
            let head: String = label.chars().take(fits - 1).collect();
            format!("{head}\u{2026}")
        }
    };

    // ---- Value scale, along the bottom, always including zero ----
    let value_min = extent.iter().copied().fold(0.0, f64::min);
    let value_max = extent.iter().copied().fold(0.0, f64::max);
    let target = (((right - left) / 90.0).floor() as usize).clamp(2, 8);
    let (first, step, count) = nice_ticks(value_min, value_max, target);
    let last = first + step * (count - 1) as f64;
    let compact = first.abs().max(last.abs()) >= 10_000.0;
    let scale = Scale {
        domain: (first, last),
        range: (left, right),
    };

    // ---- Rows: at least as tall as the bars need, and the chart at least
    // as tall as asked ----
    let top = if spec.y_label.is_empty() { 10.0 } else { 28.0 };
    let below = if spec.x_label.is_empty() { 26.0 } else { 46.0 };
    let series_count = drawn.len() as f64;
    let side_by_side = if stacked { 1.0 } else { series_count };
    let smallest_band = side_by_side * 10.0 + 2.0 * (side_by_side - 1.0) + 10.0;
    let band = ((spec.height.max(120.0) - top - below) / n as f64).max(smallest_band);
    let bottom = top + band * n as f64;
    let height = bottom + below;
    let bar_thickness = ((band * 0.8 - 2.0 * (side_by_side - 1.0)) / side_by_side).clamp(1.0, 24.0);
    let group_thickness = bar_thickness * side_by_side + 2.0 * (side_by_side - 1.0);

    let rows: Vec<RowLayout> = labels
        .iter()
        .enumerate()
        .map(|(position, label)| RowLayout {
            x: top + band * (position as f64 + 0.5),
            label: label.clone(),
        })
        .collect();
    let series: Vec<SeriesLayout> = drawn
        .iter()
        .enumerate()
        .map(|(index, name)| SeriesLayout {
            name: name.clone(),
            points: values[index]
                .iter()
                .enumerate()
                .map(|(row, value)| {
                    value.map(|value| PointLayout {
                        y: match stacks.get(index).and_then(|s| s[row]) {
                            Some((_, to)) => scale.at(to),
                            None => scale.at(value),
                        },
                        value,
                    })
                })
                .collect(),
        })
        .collect();

    // ---- SVG ----
    let mut svg = String::new();
    let _ = write!(
        svg,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" class=\"dappr-chart-svg\" width=\"{width}\" \
         height=\"{height:.0}\" viewBox=\"0 0 {width} {height:.0}\" role=\"img\" aria-label=\"{}\">",
        escape(&format!(
            "{} chart of {} by {}",
            if stacked { "Stacked bar" } else { "Bar" },
            drawn.join(", "),
            spec.x
        ))
    );
    svg.push_str("<g class=\"dappr-chart-grid\">");
    let ticks: Vec<(f64, String)> = (0..count)
        .map(|i| {
            let value = first + step * i as f64;
            (scale.at(value), format_tick(value, step, compact))
        })
        .collect();
    for (x, _) in &ticks {
        let _ = write!(
            svg,
            "<line x1=\"{x:.1}\" x2=\"{x:.1}\" y1=\"{top:.1}\" y2=\"{bottom:.1}\"/>"
        );
    }
    svg.push_str("</g><g class=\"dappr-chart-ticks\">");
    for (x, text) in &ticks {
        let half = text_width(text) / 2.0;
        let (x, anchor) = if x + half > width {
            (x.min(width), "end")
        } else {
            (*x, "middle")
        };
        let _ = write!(
            svg,
            "<text x=\"{x:.1}\" y=\"{:.1}\" text-anchor=\"{anchor}\">{}</text>",
            bottom + 18.0,
            escape(text)
        );
    }
    for row in &rows {
        let _ = write!(
            svg,
            "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"end\">{}</text>",
            left - 8.0,
            row.x + FONT_SIZE / 3.0,
            escape(&fit(&row.label))
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

    // The highlight behind the hovered row; the component moves and shows it.
    let _ = write!(
        svg,
        "<rect class=\"dappr-chart-band\" visibility=\"hidden\" x=\"{left:.1}\" width=\"{:.1}\" height=\"{band:.1}\"/>",
        right - left
    );

    let zero = scale.at(0.0);
    let outermost = |row: usize, index: usize, rightward: bool| {
        !stacks.iter().skip(index + 1).any(|other| {
            other[row].is_some_and(|(from, to)| (to >= from) == rightward && to != from)
        })
    };
    for (index, series) in series.iter().enumerate() {
        let _ = write!(svg, "<g class=\"dappr-chart-bars dappr-s{index}\">");
        for (position, point) in series.points.iter().enumerate() {
            let Some(point) = point else { continue };
            let path = if stacked {
                let Some((from, to)) = stacks[index][position] else {
                    continue;
                };
                if from == to {
                    continue;
                }
                let rightward = to >= from;
                let (mut x_from, x_to) = (scale.at(from), scale.at(to));
                // A 2px gap separates a segment from the one before it.
                if from != 0.0 && (x_to - x_from).abs() > 3.0 {
                    x_from += if rightward { 2.0 } else { -2.0 };
                }
                horizontal_bar_path(
                    rows[position].x - bar_thickness / 2.0,
                    bar_thickness,
                    x_from,
                    x_to,
                    outermost(position, index, rightward),
                )
            } else {
                if point.value == 0.0 {
                    continue;
                }
                let y =
                    rows[position].x - group_thickness / 2.0 + index as f64 * (bar_thickness + 2.0);
                horizontal_bar_path(y, bar_thickness, zero, point.y, true)
            };
            let _ = write!(svg, "<path d=\"{path}\"/>");
        }
        svg.push_str("</g>");
    }
    // The line the bars grow from, over their square ends.
    let _ = write!(
        svg,
        "<line class=\"dappr-chart-baseline\" x1=\"{zero:.1}\" x2=\"{zero:.1}\" y1=\"{top:.1}\" y2=\"{bottom:.1}\"/>"
    );
    svg.push_str("</svg>");

    Plot {
        svg,
        layout: Some(Layout {
            kind: ChartKind::Bar,
            horizontal: true,
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

/// Draws `data` as `spec` describes, `width` pixels wide.
pub fn render(spec: &ChartSpec, data: &DTable, width: f64) -> Plot {
    let width = width.max(160.0);
    let height = spec.height.max(120.0);
    let kind = spec.kind;

    let Some(x_column) = data.column(&spec.x) else {
        return message(width, height, &format!("No column named \"{}\"", spec.x));
    };

    if kind == ChartKind::Bar && spec.horizontal {
        return render_horizontal_bars(spec, data, width);
    }

    // ---- What goes on each axis ----
    // A histogram is drawn from its counts; everything else from the table.
    let histogram = if kind == ChartKind::Histogram {
        let values: Vec<f64> = x_column
            .numbers()
            .into_iter()
            .filter(|v| v.is_finite())
            .collect();
        if values.is_empty() {
            return message(
                width,
                height,
                &format!("A histogram needs numbers in \"{}\"", spec.x),
            );
        }
        Some(bins_of(&values, spec.bins))
    } else {
        None
    };

    let drawn: Vec<String> = match &histogram {
        Some(_) => vec!["rows".to_string()],
        None => spec.drawn_series().to_vec(),
    };
    if drawn.is_empty() {
        return message(width, height, "No columns to plot");
    }
    if histogram.is_none() {
        for name in &drawn {
            if data.column(name).is_none() {
                return message(width, height, &format!("No column named \"{name}\""));
            }
        }
    }
    let mut note = (histogram.is_none() && drawn.len() < spec.y.len()).then(|| {
        format!(
            "Showing the first {} of {} series",
            drawn.len(),
            spec.y.len()
        )
    });

    let read_date = |text: &str| match &spec.date_format {
        Some(format) => parse_date_with(text, format),
        None => parse_date(text),
    };
    // x as numbers, when it can be placed on a number line: numbers as they
    // are, dates as days. (A bar chart never does this: its x is categories.)
    let numeric_x = x_column.is_numeric();
    let cells = x_column.cells();
    let date_x = !numeric_x
        && cells.iter().any(|cell| !cell.is_null())
        && cells.iter().all(|cell| match cell {
            Cell::Null => true,
            Cell::Text(text) => read_date(text).is_some(),
            _ => false,
        });
    if kind == ChartKind::Scatter && !numeric_x && !date_x {
        return message(
            width,
            height,
            &format!("A scatter chart needs numbers or dates in \"{}\"", spec.x),
        );
    }
    let linear_x = histogram.is_some() || ((numeric_x || date_x) && kind != ChartKind::Bar);
    let x_number = |index: usize| -> Option<f64> {
        match &cells[index] {
            Cell::Number(number) => Some(*number).filter(|n| n.is_finite()),
            Cell::Text(text) if date_x => read_date(text),
            _ => None,
        }
    };

    // The rows to draw: one per range for a histogram; otherwise the
    // table's rows, minus those without an x value when x is a number line.
    let row_indices: Vec<usize> = match &histogram {
        Some(bins) => (0..bins.counts.len()).collect(),
        None => (0..data.n_rows())
            .filter(|&i| !linear_x || x_number(i).is_some())
            .collect(),
    };
    if row_indices.is_empty() {
        return message(width, height, "No data to plot");
    }

    // Values per series, aligned with `row_indices`.
    let values: Vec<Vec<Option<f64>>> = match &histogram {
        Some(bins) => vec![bins.counts.iter().map(|c| Some(*c as f64)).collect()],
        None => drawn
            .iter()
            .map(|name| {
                let cells = data.column(name).map(|c| c.cells()).unwrap_or(&[]);
                row_indices
                    .iter()
                    .map(|&i| cells[i].as_f64().filter(|v| v.is_finite()))
                    .collect()
            })
            .collect(),
    };
    let all_values: Vec<f64> = values.iter().flatten().flatten().copied().collect();
    if all_values.is_empty() {
        return message(width, height, "No numbers to plot");
    }

    // Stacked bars: where each segment starts and ends, in data units.
    // Positive values pile upwards from zero, negative ones downwards.
    let stacked = kind == ChartKind::Bar && spec.stacked && drawn.len() > 1;
    let n = row_indices.len();
    let stacks = if stacked {
        stack(&values, n)
    } else {
        Vec::new()
    };

    // ---- y scale ----
    let extent: Vec<f64> = if stacked {
        stacks
            .iter()
            .flatten()
            .flatten()
            .map(|(_, to)| *to)
            .collect()
    } else {
        all_values
    };
    let mut y_min = extent.iter().copied().fold(f64::INFINITY, f64::min);
    let mut y_max = extent.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if kind != ChartKind::Line && kind != ChartKind::Scatter {
        // Bars and areas are read by how far they reach from zero, so zero
        // must be on the axis.
        y_min = y_min.min(0.0);
        y_max = y_max.max(0.0);
    }
    let (mut y_first, mut y_step, mut y_count) = nice_ticks(y_min, y_max, 5);
    if histogram.is_some() && y_step < 1.0 {
        // Counts are whole numbers: no ticks at 0.5 rows.
        (y_first, y_step) = (0.0, 1.0);
        y_count = y_max.ceil() as usize + 1;
    }
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
    let end_labels = kind == ChartKind::Line && (2..=4).contains(&drawn.len());
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
    let mut band = (right - left) / n as f64;
    let x_values: Vec<f64> = match &histogram {
        // The middle of each range.
        Some(bins) => (0..n)
            .map(|i| bins.first + bins.step * (i as f64 + 0.5))
            .collect(),
        None => row_indices
            .iter()
            .map(|&i| x_number(i).unwrap_or(0.0))
            .collect(),
    };
    let (x_scale, linear_ticks) = if linear_x {
        let (x_min, x_max) = match &histogram {
            Some(bins) => (bins.first, bins.first + bins.step * n as f64),
            None => (
                x_values.iter().copied().fold(f64::INFINITY, f64::min),
                x_values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            ),
        };
        let pad = match kind {
            ChartKind::Scatter => 8.0,
            ChartKind::Histogram => 0.0,
            _ => 4.0,
        };
        let scale = Scale {
            domain: (x_min, x_max),
            range: (left + pad, right - pad),
        };
        let target = (((right - left) / 90.0).floor() as usize).clamp(2, 10);
        let ticks: Vec<(f64, String)> = if let Some(bins) = &histogram {
            // A tick on the edges between bars, as many as fit.
            band = (right - left) / n as f64;
            let compact_x = x_min.abs().max(x_max.abs()) >= 1_000_000.0;
            let labels: Vec<String> = (0..=n)
                .map(|i| format_tick(bins.first + bins.step * i as f64, bins.step, compact_x))
                .collect();
            let widest = labels.iter().map(|l| text_width(l)).fold(0.0, f64::max);
            let every = (((widest + 14.0) / band).ceil() as usize).max(1);
            labels
                .into_iter()
                .enumerate()
                .step_by(every)
                .map(|(i, label)| (scale.at(bins.first + bins.step * i as f64), label))
                .collect()
        } else if date_x {
            time_ticks(x_min, x_max, target)
                .into_iter()
                .map(|(value, label)| (scale.at(value), label))
                .collect()
        } else {
            let (first, step, count) = nice_ticks(x_min, x_max, target);
            let compact_x = x_min.abs().max(x_max.abs()) >= 1_000_000.0;
            (0..count)
                .map(|i| first + step * i as f64)
                .filter(|v| *v >= x_min - step * 1e-9 && *v <= x_max + step * 1e-9)
                .map(|v| (scale.at(v), format_tick(v, step, compact_x)))
                .collect()
        };
        (Some(scale), ticks)
    } else {
        (None, Vec::new())
    };
    let rows: Vec<RowLayout> = row_indices
        .iter()
        .enumerate()
        .map(|(position, &i)| {
            let x = match &x_scale {
                Some(scale) => scale.at(x_values[position]),
                None => left + band * (position as f64 + 0.5),
            };
            let label = match &histogram {
                Some(bins) => {
                    let from = bins.first + bins.step * position as f64;
                    format!(
                        "{} \u{2013} {}",
                        format_value(from),
                        format_value(from + bins.step)
                    )
                }
                None => match &cells[i] {
                    Cell::Number(number) => format_value(*number),
                    other => other.to_string(),
                },
            };
            RowLayout { x, label }
        })
        .collect();
    // Category labels: show every k-th so they never run into each other.
    let x_ticks: Vec<(f64, String)> = if linear_x {
        linear_ticks
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
        .enumerate()
        .map(|(index, name)| SeriesLayout {
            name: name.clone(),
            points: values[index]
                .iter()
                .enumerate()
                .map(|(row, value)| {
                    value.map(|value| PointLayout {
                        // In a stack, the point is the top of the segment.
                        y: match stacks.get(index).and_then(|s| s[row]) {
                            Some((_, to)) => y_scale.at(to),
                            None => y_scale.at(value),
                        },
                        value,
                    })
                })
                .collect(),
        })
        .collect();

    // ---- SVG ----
    let mut svg = String::new();
    let label = match kind {
        ChartKind::Histogram => format!("Histogram of {}", spec.x),
        _ => format!(
            "{} chart of {} by {}",
            match kind {
                ChartKind::Line => "Line",
                ChartKind::Area => "Area",
                ChartKind::Bar if stacked => "Stacked bar",
                ChartKind::Bar => "Bar",
                _ => "Scatter",
            },
            drawn.join(", "),
            spec.x
        ),
    };
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
        // Centred under its position, unless that would run off an edge of
        // the drawing: then it is pulled back inside.
        let half = text_width(text) / 2.0;
        let (x, anchor) = if x + half > width {
            (x.min(width), "end")
        } else if x - half < 0.0 {
            (x.max(0.0), "start")
        } else {
            (*x, "middle")
        };
        let _ = write!(
            svg,
            "<text x=\"{x:.1}\" y=\"{:.1}\" text-anchor=\"{anchor}\">{}</text>",
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

    // The baseline: zero if it is in view (bars and areas grow from it),
    // else the bottom of the plot.
    let baseline = if y_first <= 0.0 && y_last >= 0.0 {
        y_scale.at(0.0)
    } else {
        bottom
    };

    // Hover aids drawn under the marks; the component moves and shows them.
    match kind {
        ChartKind::Bar | ChartKind::Histogram => {
            let _ = write!(
                svg,
                "<rect class=\"dappr-chart-band\" visibility=\"hidden\" y=\"{top:.1}\" width=\"{band:.1}\" height=\"{:.1}\"/>",
                bottom - top
            );
        }
        ChartKind::Line | ChartKind::Area => {
            let _ = write!(
                svg,
                "<line class=\"dappr-chart-crosshair\" visibility=\"hidden\" y1=\"{top:.1}\" y2=\"{bottom:.1}\"/>"
            );
        }
        ChartKind::Scatter => {}
    }

    // Marks.
    match kind {
        ChartKind::Line | ChartKind::Area => {
            for (index, series) in series.iter().enumerate() {
                // A missing value breaks the line rather than being bridged:
                // collect the unbroken runs of points.
                let mut runs: Vec<Vec<(f64, f64)>> = Vec::new();
                let mut pen_down = false;
                for (position, point) in series.points.iter().enumerate() {
                    match point {
                        Some(point) => {
                            if !pen_down {
                                runs.push(Vec::new());
                            }
                            runs.last_mut()
                                .expect("a run was just started")
                                .push((rows[position].x, point.y));
                            pen_down = true;
                        }
                        None => pen_down = false,
                    }
                }
                if kind == ChartKind::Area {
                    // The wash under each run, down to the baseline.
                    let mut fill = String::new();
                    for run in runs.iter().filter(|run| run.len() > 1) {
                        let _ = write!(fill, "M{:.1},{baseline:.1}", run[0].0);
                        for (x, y) in run {
                            let _ = write!(fill, "L{x:.1},{y:.1}");
                        }
                        let _ = write!(fill, "L{:.1},{baseline:.1}Z", run[run.len() - 1].0);
                    }
                    let _ = write!(
                        svg,
                        "<path class=\"dappr-chart-area dappr-s{index}\" d=\"{fill}\"/>"
                    );
                }
                let mut path = String::new();
                for run in &runs {
                    for (i, (x, y)) in run.iter().enumerate() {
                        let command = if i == 0 { 'M' } else { 'L' };
                        let _ = write!(path, "{command}{x:.1},{y:.1}");
                    }
                }
                let _ = write!(
                    svg,
                    "<path class=\"dappr-chart-line dappr-s{index}\" d=\"{path}\"/>"
                );
                // A value with gaps on both sides has no line to belong to:
                // draw it as a dot.
                for run in runs.iter().filter(|run| run.len() == 1) {
                    let _ = write!(
                        svg,
                        "<circle class=\"dappr-chart-dot dappr-s{index}\" cx=\"{:.1}\" cy=\"{:.1}\" r=\"4\"/>",
                        run[0].0, run[0].1
                    );
                }
            }
        }
        ChartKind::Bar | ChartKind::Histogram => {
            let count = series.len() as f64;
            let (bar_width, group_width) = if histogram.is_some() {
                // A bar spans its range, less the 2px gap between neighbours.
                let bar = (band - 2.0).max(1.0);
                (bar, bar)
            } else if stacked {
                let bar = (band * 0.8).clamp(1.0, 24.0);
                (bar, bar)
            } else {
                // Thin bars (24px at most) with a 2px gap between neighbours.
                let bar = ((band * 0.8 - 2.0 * (count - 1.0)) / count).clamp(1.0, 24.0);
                (bar, bar * count + 2.0 * (count - 1.0))
            };
            // In a stack, only the outermost segment of each bar is rounded.
            let outermost = |row: usize, index: usize, upward: bool| {
                !stacks.iter().enumerate().skip(index + 1).any(|(_, other)| {
                    other[row].is_some_and(|(from, to)| (to >= from) == upward && to != from)
                })
            };
            for (index, series) in series.iter().enumerate() {
                let _ = write!(svg, "<g class=\"dappr-chart-bars dappr-s{index}\">");
                for (position, point) in series.points.iter().enumerate() {
                    let Some(point) = point else { continue };
                    let path = if stacked {
                        let Some((from, to)) = stacks[index][position] else {
                            continue;
                        };
                        if from == to {
                            continue;
                        }
                        let upward = to >= from;
                        let (mut y_from, y_to) = (y_scale.at(from), y_scale.at(to));
                        // A 2px gap separates a segment from the one before it.
                        if from != 0.0 && (y_from - y_to).abs() > 3.0 {
                            y_from += if upward { -2.0 } else { 2.0 };
                        }
                        bar_path(
                            rows[position].x - bar_width / 2.0,
                            bar_width,
                            y_from,
                            y_to,
                            outermost(position, index, upward),
                        )
                    } else {
                        if point.value == 0.0 {
                            continue;
                        }
                        let x =
                            rows[position].x - group_width / 2.0 + index as f64 * (bar_width + 2.0);
                        bar_path(x, bar_width, baseline, point.y, true)
                    };
                    let _ = write!(svg, "<path d=\"{path}\"/>");
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

    // The baseline goes over the marks, so bars and fills end cleanly on it.
    let _ = write!(
        svg,
        "<line class=\"dappr-chart-baseline\" x1=\"{left:.1}\" x2=\"{right:.1}\" y1=\"{baseline:.1}\" y2=\"{baseline:.1}\"/>"
    );

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
    if matches!(kind, ChartKind::Line | ChartKind::Area | ChartKind::Scatter) {
        for index in 0..series.len() {
            let _ = write!(
                svg,
                "<circle class=\"dappr-chart-hover dappr-s{index}\" visibility=\"hidden\" r=\"5\"/>"
            );
        }
    }
    svg.push_str("</svg>");

    if let Some(text) = &mut note {
        if kind == ChartKind::Scatter {
            text.push_str(" (a scatter chart keeps to three so the colours stay distinct)");
        }
    }

    Plot {
        svg,
        layout: Some(Layout {
            kind,
            horizontal: false,
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

    // ---- Dates ----

    #[test]
    fn dates_convert_to_days_and_back() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(2000, 3, 1), 11_017);
        assert_eq!(civil_from_days(11_017), (2000, 3, 1));
        for days in [-800_000, -1, 0, 59, 60, 19_000, 20_517, 400_000] {
            let (y, m, d) = civil_from_days(days);
            assert_eq!(days_from_civil(y, m, d), days);
        }
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2026, 2), 28);
        assert_eq!(days_in_month(2026, 12), 31);
    }

    #[test]
    fn parse_date_accepts_iso_forms_only() {
        assert_eq!(parse_date("1970-01-02"), Some(1.0));
        assert_eq!(parse_date("2026/03/05"), parse_date("2026-03-05"));
        assert_eq!(parse_date("2026-03"), parse_date("2026-03-01"));
        assert_eq!(parse_date("1970-01-01T12:00"), Some(0.5));
        assert_eq!(parse_date("1970-01-01 06:00:00"), Some(0.25));
        assert_eq!(parse_date(" 1970-01-01T18:00:00Z "), Some(0.75));
        for not_a_date in [
            "",
            "Jan",
            "2026",
            "1-2",
            "12-05",
            "2026-13-01",
            "2026-02-30",
            "2026-3-5",
            "20260305",
            "2026-03-05T25:00",
            "2026-03-05T10",
            "March 5",
            "2026-03 10:00",
        ] {
            assert_eq!(parse_date(not_a_date), None, "{not_a_date}");
        }
    }

    #[test]
    fn time_ticks_fall_on_round_moments() {
        let day = |text: &str| parse_date(text).unwrap();
        let labels = |from: &str, to: &str, target: usize| -> Vec<String> {
            time_ticks(day(from), day(to), target)
                .into_iter()
                .map(|(_, label)| label)
                .collect()
        };
        // Two years: quarters or halves, the year shown at the start and each January.
        let ticks = labels("2025-01-01", "2026-12-31", 6);
        assert_eq!(ticks, vec!["Jan 2025", "Jul", "Jan 2026", "Jul"]);
        // A few weeks: days.
        let ticks = labels("2026-03-01", "2026-03-29", 6);
        assert_eq!(ticks, vec!["Mar 1", "Mar 8", "Mar 15", "Mar 22", "Mar 29"]);
        // Decades: years.
        assert_eq!(
            labels("1990-06-01", "2026-01-01", 5),
            vec!["2000", "2010", "2020"]
        );
        // One day: hours, with midnight named by its day.
        let ticks = labels("2026-03-05T00:00", "2026-03-06T00:00", 5);
        assert_eq!(ticks, vec!["Mar 5", "06:00", "12:00", "18:00", "Mar 6"]);
        // Every tick is inside the range.
        for (value, _) in time_ticks(day("2026-01-15"), day("2026-09-20"), 6) {
            assert!(value >= day("2026-01-15") && value <= day("2026-09-20"));
        }
    }

    #[test]
    fn dates_on_x_are_placed_by_time_not_by_row() {
        let mut table = DTable::with_columns(&["day", "v"]);
        table.push_row(("2026-01-01", 1));
        table.push_row(("2026-01-02", 2));
        table.push_row(("2026-01-11", 3)); // nine days later
        for kind in [ChartKind::Line, ChartKind::Area, ChartKind::Scatter] {
            let plot = render(&ChartSpec::new(kind, "day", &["v"]), &table, 640.0);
            let layout = plot.layout.unwrap();
            let gap_small = layout.rows[1].x - layout.rows[0].x;
            let gap_large = layout.rows[2].x - layout.rows[1].x;
            assert!((gap_large / gap_small - 9.0).abs() < 1e-6, "{kind:?}");
            // The read-out keeps the date as written.
            assert_eq!(layout.rows[2].label, "2026-01-11");
            assert!(
                plot.svg.contains(">Jan "),
                "{kind:?} should have date ticks"
            );
        }
        // A bar chart keeps dates as evenly spaced categories.
        let layout = render(
            &ChartSpec::new(ChartKind::Bar, "day", &["v"]),
            &table,
            640.0,
        )
        .layout
        .unwrap();
        let gaps = (
            layout.rows[1].x - layout.rows[0].x,
            layout.rows[2].x - layout.rows[1].x,
        );
        assert!((gaps.0 - gaps.1).abs() < 1e-6);
    }

    #[test]
    fn text_that_is_only_partly_dates_stays_categories() {
        let mut table = DTable::with_columns(&["when", "v"]);
        table.push_row(("2026-01-01", 1));
        table.push_row(("later", 2));
        table.push_row(("2026-01-11", 3));
        let layout = render(
            &ChartSpec::new(ChartKind::Line, "when", &["v"]),
            &table,
            640.0,
        )
        .layout
        .unwrap();
        let gaps = (
            layout.rows[1].x - layout.rows[0].x,
            layout.rows[2].x - layout.rows[1].x,
        );
        assert!((gaps.0 - gaps.1).abs() < 1e-6);
    }

    // ---- Area ----

    #[test]
    fn area_chart_fills_down_to_zero() {
        let mut table = DTable::with_columns(&["t", "v"]);
        table.push_row(("a", 120));
        table.push_row(("b", 150));
        table.push_row(("c", 90));
        let plot = render(&ChartSpec::new(ChartKind::Area, "t", &["v"]), &table, 600.0);
        // Zero is on the axis even though the smallest value is 90.
        assert!(plot.svg.contains(">0</text>"));
        assert_eq!(plot.svg.matches("class=\"dappr-chart-area").count(), 1);
        assert_eq!(plot.svg.matches("class=\"dappr-chart-line").count(), 1);
        // The fill is a closed shape that touches the baseline at both ends.
        let fill = plot
            .svg
            .split("dappr-chart-area dappr-s0\" d=\"")
            .nth(1)
            .unwrap();
        let fill = &fill[..fill.find('"').unwrap()];
        let baseline: f64 = plot
            .svg
            .split("dappr-chart-baseline")
            .nth(1)
            .unwrap()
            .split("y1=\"")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap()
            .parse()
            .unwrap();
        assert!(fill.starts_with('M') && fill.ends_with('Z'));
        assert_eq!(fill.matches(&format!(",{baseline:.1}")).count(), 2);
        // A line chart of the same data does not force zero.
        let line = render(&ChartSpec::new(ChartKind::Line, "t", &["v"]), &table, 600.0);
        assert!(!line.svg.contains(">0</text>"));
        assert!(!line.svg.contains("dappr-chart-area"));
    }

    // ---- Stacked bars ----

    fn stacked_spec() -> ChartSpec {
        let mut spec = ChartSpec::new(ChartKind::Bar, "month", &["north", "south"]);
        spec.stacked = true;
        spec
    }

    #[test]
    fn stacked_bars_pile_up_and_the_axis_covers_the_total() {
        let plot = render(&stacked_spec(), &sales(), 640.0);
        let layout = plot.layout.unwrap();
        let (north, south) = (&layout.series[0], &layout.series[1]);
        for row in 0..4 {
            let (n, s) = (north.points[row].unwrap(), south.points[row].unwrap());
            // South sits on top of north: its top is higher on screen.
            assert!(s.y < n.y);
            // Each point still reports its own value, not the running total.
            assert!(s.value <= 140.0);
        }
        // Largest total is Apr: 200 + 130 = 330, so the axis reaches past 300.
        assert!(plot.svg.contains(">400</text>") || plot.svg.contains(">350</text>"));
        // Side by side, the same data only needs to reach 200.
        let grouped = render(
            &ChartSpec::new(ChartKind::Bar, "month", &["north", "south"]),
            &sales(),
            640.0,
        );
        assert!(!grouped.svg.contains(">350</text>") && !grouped.svg.contains(">400</text>"));
    }

    #[test]
    fn stacked_segments_share_one_column_with_a_gap_and_one_rounded_end() {
        let plot = render(&stacked_spec(), &sales(), 640.0);
        let layout = plot.layout.clone().unwrap();
        let paths: Vec<&str> = plot
            .svg
            .split("<path d=\"")
            .skip(1)
            .map(|p| &p[..p.find('"').unwrap()])
            .collect();
        assert_eq!(paths.len(), 8);
        let start = |path: &str| -> (f64, f64) {
            let (x, y) = path[1..path.find('L').unwrap()].split_once(',').unwrap();
            (x.parse().unwrap(), y.parse().unwrap())
        };
        let (north_jan, south_jan) = (start(paths[0]), start(paths[4]));
        // Same column...
        assert_eq!(north_jan.0, south_jan.0);
        // ...with the upper segment starting 2px above the top of the lower.
        let north_top = layout.series[0].points[0].unwrap().y;
        assert!((north_top - south_jan.1 - 2.0).abs() < 0.11);
        // The lower segment is square (no curve of any size); the top is rounded.
        let has_curve = |path: &str| {
            let after_q = path.split('Q').nth(1).unwrap();
            let (control, end) = after_q.split_once(' ').unwrap();
            let end = &end[..end.find('L').unwrap()];
            control != end
        };
        assert!(
            !has_curve(paths[0]),
            "lower segment should be square: {}",
            paths[0]
        );
        assert!(
            has_curve(paths[4]),
            "top segment should be rounded: {}",
            paths[4]
        );
    }

    #[test]
    fn stacked_bars_put_negatives_below_zero() {
        let mut table = DTable::with_columns(&["k", "gain", "loss"]);
        table.push_row(("a", 5, -3));
        table.push_row(("b", 2, -6));
        let mut spec = ChartSpec::new(ChartKind::Bar, "k", &["gain", "loss"]);
        spec.stacked = true;
        let plot = render(&spec, &table, 600.0);
        let layout = plot.layout.unwrap();
        let zero: f64 = plot
            .svg
            .split("dappr-chart-baseline")
            .nth(1)
            .unwrap()
            .split("y1=\"")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap()
            .parse()
            .unwrap();
        assert!(layout.series[0].points[0].unwrap().y < zero);
        assert!(layout.series[1].points[0].unwrap().y > zero);
        // The axis reaches below the lowest stack (-6) and above the highest (5).
        let ticks: Vec<f64> = plot
            .svg
            .split("text-anchor=\"end\">")
            .skip(1)
            .filter_map(|t| t[..t.find('<').unwrap()].parse().ok())
            .collect();
        let (lowest, highest) = (
            ticks.iter().copied().fold(f64::INFINITY, f64::min),
            ticks.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        );
        assert!(lowest <= -6.0 && highest >= 5.0, "ticks {ticks:?}");
    }

    // ---- Histogram ----

    #[test]
    fn histogram_counts_rows_in_tidy_ranges() {
        let mut table = DTable::with_columns(&["minutes"]);
        for v in [1, 2, 2, 3, 11, 12, 13, 14, 15, 28, 30] {
            table.push_row([v]);
        }
        let mut spec = ChartSpec::new(ChartKind::Histogram, "minutes", &[]);
        spec.bins = 3;
        let binned = histogram_table(&table, "minutes", 3).unwrap();
        assert_eq!(binned.headers(), vec!["from", "to", "rows"]);
        let rows: Vec<(f64, f64, f64)> = binned
            .rows()
            .map(|r| {
                (
                    r.number("from").unwrap(),
                    r.number("to").unwrap(),
                    r.number("rows").unwrap(),
                )
            })
            .collect();
        assert_eq!(
            rows,
            vec![(0.0, 10.0, 4.0), (10.0, 20.0, 5.0), (20.0, 30.0, 2.0)]
        );
        // Every row is counted exactly once, the largest value included.
        assert_eq!(rows.iter().map(|r| r.2).sum::<f64>(), 11.0);

        let plot = render(&spec, &table, 600.0);
        let layout = plot.layout.unwrap();
        assert_eq!(layout.kind, ChartKind::Histogram);
        assert_eq!(layout.rows.len(), 3);
        assert_eq!(layout.rows[1].label, "10 \u{2013} 20");
        assert_eq!(layout.series[0].name, "rows");
        assert_eq!(layout.series[0].points[1].unwrap().value, 5.0);
        // Bars touch, less a 2px gap: each is as wide as its range.
        let xs: Vec<f64> = plot
            .svg
            .split("<path d=\"M")
            .skip(1)
            .map(|p| p[..p.find(',').unwrap()].parse().unwrap())
            .collect();
        assert_eq!(xs.len(), 3);
        assert!(((xs[1] - xs[0]) - layout.band).abs() < 0.11);
        // The axis is labelled at the edges of the ranges.
        for edge in ["0", "10", "20", "30"] {
            assert!(
                plot.svg.contains(&format!(">{edge}</text>")),
                "missing edge {edge}"
            );
        }
    }

    #[test]
    fn edge_tick_labels_stay_inside_the_drawing() {
        let mut table = DTable::with_columns(&["v"]);
        for v in [100, 900, 1_900] {
            table.push_row([v]);
        }
        let mut spec = ChartSpec::new(ChartKind::Histogram, "v", &[]);
        spec.bins = 2;
        let plot = render(&spec, &table, 300.0);
        // The label at the right edge ends at the edge instead of being
        // centred on it and cut in half.
        let last = plot
            .svg
            .rsplit("text-anchor=\"")
            .find(|t| t.contains(">2,000<"))
            .unwrap();
        assert!(last.starts_with("end\""), "{last}");
    }

    #[test]
    fn histogram_counts_use_whole_number_ticks() {
        let mut table = DTable::with_columns(&["v"]);
        table.push_row([1.0]);
        table.push_row([9.0]);
        let plot = render(
            &ChartSpec::new(ChartKind::Histogram, "v", &[]),
            &table,
            600.0,
        );
        assert!(!plot.svg.contains(">0.5</text>"));
        assert!(plot.svg.contains(">1</text>"));
    }

    #[test]
    fn histogram_handles_odd_inputs() {
        // One distinct value.
        let mut same = DTable::with_columns(&["v"]);
        for _ in 0..4 {
            same.push_row([7]);
        }
        let binned = histogram_table(&same, "v", 0).unwrap();
        assert_eq!(binned.column("rows").unwrap().sum(), 4.0);
        // Text, missing values and missing columns.
        let mut text = DTable::with_columns(&["v"]);
        text.push_row(["a"]);
        assert!(histogram_table(&text, "v", 0).is_none());
        assert!(histogram_table(&text, "nope", 0).is_none());
        let plot = render(
            &ChartSpec::new(ChartKind::Histogram, "v", &[]),
            &text,
            600.0,
        );
        assert!(plot.svg.contains("A histogram needs numbers"));
        // Without a number of bins, more data gets more bars, within limits.
        let mut many = DTable::with_columns(&["v"]);
        for i in 0..1000 {
            many.push_row([f64::from(i) * 0.37]);
        }
        let bars = histogram_table(&many, "v", 0).unwrap().n_rows();
        assert!((5..=30).contains(&bars), "{bars} bars");
    }

    // ---- More date forms ----

    #[test]
    fn parse_date_reads_named_months_dots_and_offsets() {
        let iso = parse_date("2026-03-05");
        for same_day in [
            "5 Mar 2026",
            "05 March 2026",
            "Mar 5, 2026",
            "march 5 2026",
            "05.03.2026",
        ] {
            assert_eq!(parse_date(same_day), iso, "{same_day}");
        }
        assert_eq!(parse_date("March 2026"), parse_date("2026-03-01"));
        assert_eq!(parse_date("Sept 2026"), parse_date("2026-09-01"));
        // Offsets are applied: 14:30 at +01:00 is 13:30 UTC.
        assert_eq!(
            parse_date("2026-03-05T14:30:00+01:00"),
            parse_date("2026-03-05T13:30:00Z")
        );
        assert_eq!(
            parse_date("2026-03-05 23:30-0200"),
            parse_date("2026-03-06T01:30")
        );
        // Ambiguous or malformed: not read.
        for not_read in [
            "03/05/2026",
            "5/3/2026",
            "Ma 5 2026",
            "5 Foo 2026",
            "31.02.2026",
            "5.3.2026",
            "Mar 5",
            "Tuesday",
            "2026-03-05T14:30+1",
            "Mar 32, 2026",
        ] {
            assert_eq!(parse_date(not_read), None, "{not_read}");
        }
    }

    #[test]
    fn parse_date_with_follows_the_format_given() {
        let day = parse_date("2026-03-05");
        assert_eq!(parse_date_with("03/05/2026", "%m/%d/%Y"), day);
        assert_eq!(parse_date_with("5/3/2026", "%d/%m/%Y"), day);
        assert_eq!(parse_date_with("05-Mar-26", "%d-%b-%y"), day);
        assert_eq!(parse_date_with("20260305", "%Y%m%d"), day);
        assert_eq!(
            parse_date_with("05/03/2026 14:30", "%d/%m/%Y %H:%M"),
            parse_date("2026-03-05T14:30")
        );
        assert_eq!(
            parse_date_with("week 3 of 2026: 3%", "week %d of %Y: %m%%"),
            parse_date("2026-03-03")
        );
        for (text, format) in [
            ("03/05/2026", "%d-%m-%Y"),   // wrong separator
            ("03/05/2026 x", "%m/%d/%Y"), // left-over text
            ("31/02/2026", "%d/%m/%Y"),   // no such day
            ("03/05", "%m/%d"),           // no year
            ("03/05/2026", "%m/%d/%Q"),   // unknown stand-in
        ] {
            assert_eq!(parse_date_with(text, format), None, "{text} as {format}");
        }
    }

    #[test]
    fn a_date_format_puts_ambiguous_dates_on_a_time_axis() {
        let mut table = DTable::with_columns(&["day", "v"]);
        table.push_row(("01/02/2026", 1)); // 1 Feb
        table.push_row(("02/02/2026", 2)); // 2 Feb
        table.push_row(("11/02/2026", 3)); // 11 Feb
        let mut spec = ChartSpec::new(ChartKind::Line, "day", &["v"]);
        // Without a format these are not read as dates: evenly spaced.
        let plain = render(&spec, &table, 640.0).layout.unwrap();
        assert!(
            ((plain.rows[2].x - plain.rows[1].x) - (plain.rows[1].x - plain.rows[0].x)).abs()
                < 1e-6
        );
        spec.date_format = Some("%d/%m/%Y".to_string());
        let dated = render(&spec, &table, 640.0).layout.unwrap();
        let ratio = (dated.rows[2].x - dated.rows[1].x) / (dated.rows[1].x - dated.rows[0].x);
        assert!((ratio - 9.0).abs() < 1e-6);
    }

    // ---- Horizontal bars ----

    fn horizontal(y: &[&str], stacked: bool) -> ChartSpec {
        let mut spec = ChartSpec::new(ChartKind::Bar, "month", y);
        spec.horizontal = true;
        spec.stacked = stacked;
        spec
    }

    #[test]
    fn horizontal_bars_run_rightwards_from_a_shared_zero() {
        let plot = render(&horizontal(&["north"], false), &sales(), 640.0);
        let layout = plot.layout.unwrap();
        assert!(layout.horizontal);
        // Categories run top to bottom, in table order.
        assert!(layout.rows.windows(2).all(|w| w[0].x < w[1].x));
        assert_eq!(layout.rows[0].label, "Jan");
        // A larger value reaches further right: 200 (Apr) beyond 90 (Mar).
        let north = &layout.series[0];
        assert!(north.points[3].unwrap().y > north.points[2].unwrap().y);
        // Every bar starts at the same x, the zero line.
        let starts: Vec<f64> = plot
            .svg
            .split("<path d=\"M")
            .skip(1)
            .map(|p| p[..p.find(',').unwrap()].parse().unwrap())
            .collect();
        assert_eq!(starts.len(), 4);
        assert!(starts.iter().all(|x| (x - starts[0]).abs() < 0.11));
        assert!(plot.svg.contains(">0</text>"));
        // Category names sit to the left of the bars.
        assert!(plot.svg.contains("text-anchor=\"end\">Jan</text>"));
    }

    #[test]
    fn horizontal_bars_grow_taller_with_more_categories() {
        let height = |rows: usize| -> f64 {
            let mut table = DTable::with_columns(&["name", "v"]);
            for i in 0..rows {
                table.push_row((format!("Category number {i}"), i as f64));
            }
            let mut spec = ChartSpec::new(ChartKind::Bar, "name", &["v"]);
            spec.horizontal = true;
            let svg = render(&spec, &table, 640.0).svg;
            svg.split("height=\"")
                .nth(1)
                .unwrap()
                .split('"')
                .next()
                .unwrap()
                .parse()
                .unwrap()
        };
        assert_eq!(height(4), 320.0);
        // Forty categories do not fit in 320px at a readable size.
        assert!(height(40) > 800.0);
    }

    #[test]
    fn horizontal_labels_are_shortened_to_leave_room_for_the_bars() {
        let mut table = DTable::with_columns(&["name", "v"]);
        table.push_row(("An extremely long category name that goes on and on", 5));
        table.push_row(("Short", 3));
        let mut spec = ChartSpec::new(ChartKind::Bar, "name", &["v"]);
        spec.horizontal = true;
        let plot = render(&spec, &table, 300.0);
        let layout = plot.layout.unwrap();
        assert!(layout.left <= 300.0 * 0.4 + 1e-9);
        assert!(plot.svg.contains('\u{2026}'));
        // The read-out keeps the full name.
        assert!(layout.rows[0].label.ends_with("on and on"));
    }

    #[test]
    fn horizontal_bars_can_be_grouped_or_stacked() {
        let grouped = render(&horizontal(&["north", "south"], false), &sales(), 640.0);
        let stacked = render(&horizontal(&["north", "south"], true), &sales(), 640.0);
        let first = |plot: &Plot, i: usize| -> (f64, f64) {
            let path = plot.svg.split("<path d=\"M").nth(i + 1).unwrap();
            let (x, y) = path[..path.find('L').unwrap()].split_once(',').unwrap();
            (x.parse().unwrap(), y.parse().unwrap())
        };
        // Grouped: both start at zero, one below the other.
        let (north, south) = (first(&grouped, 0), first(&grouped, 4));
        assert_eq!(north.0, south.0);
        assert!(south.1 > north.1);
        // Stacked: same row, the second starting 2px after the first ends.
        let (north, south) = (first(&stacked, 0), first(&stacked, 4));
        assert_eq!(north.1, south.1);
        let north_end = stacked.layout.as_ref().unwrap().series[0].points[0]
            .unwrap()
            .y;
        assert!((south.0 - north_end - 2.0).abs() < 0.11);
        // The axis covers the total (Apr: 330) when stacked.
        let south_end = stacked.layout.as_ref().unwrap().series[1].points[3].unwrap();
        assert!(south_end.y <= stacked.layout.as_ref().unwrap().right + 1e-9);
        assert_eq!(south_end.value, 130.0);
    }

    #[test]
    fn compact_numbers_for_headline_figures() {
        for (value, text) in [
            (0.0, "0"),
            (7.0, "7"),
            (1284.0, "1,284"),
            (9999.4, "9,999.4"),
            (12_900.0, "12.9K"),
            (129_000.0, "129K"),
            (4_200_000.0, "4.2M"),
            (1_000_000.0, "1M"),
            (1_290_000_000.0, "1.29B"),
            (-15_500.0, "-15.5K"),
            (5.6789, "5.68"),
        ] {
            assert_eq!(format_compact(value), text);
        }
    }
}
