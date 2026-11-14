# Dappr

Data Apps made easy (in Rust)

Dappr aims to be for Rust what [Streamlit](https://streamlit.io/) is for Python:
you describe a page top to bottom, widgets hand you reactive values, and the
framework takes care of the rest. Apps compile to WebAssembly and run entirely
in the browser.

```rust
use dappr::prelude::*;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn run() {
    dapp!(dapp, {
        text!("Hello Dappr!", _title);
        let input = dapp.input("What's your name?");
        text!("Hello, {}!", input.text());
    });
}
```

## Data

`DTable` is a small table of typed cells. Load one from CSV or JSON (or build
it row by row), reshape it with `filter` / `sort_by` / `select` / `head`, and
show it with `dapp.table`. Give `dapp.table` a `Signal<DTable>` and the table
on the page follows your inputs:

```rust
dapp!(dapp, {
    let orders = DTable::from_csv(include_str!("../orders.csv")).unwrap();

    let region = dapp.select("Region", &["North", "South"]).value();
    let min_units = dapp.slider("Minimum units", 1.0, 12.0, 1.0).value();

    // Recomputed whenever any listed signal changes.
    let shown = computed!(region, min_units => {
        orders
            .filter(|row| row.text("region") == Some(region) && row.number("units") >= Some(min_units))
            .sort_by("units", true)
    });

    dapp.text("{} orders", shown.map(|table| table.n_rows()), None);
    dapp.table(&shown);
});
```

- **Loading:** `DTable::from_csv`, `from_tsv`, `from_json`; `CsvOptions` for
  other delimiters, files without headers, strict row lengths. Numbers and
  booleans are recognised; empty fields are missing values.
- **Columns:** `sum`, `mean`, `min`, `max`, `unique`.
- **At runtime:** `fetch_table(url)` downloads a table, and
  `dapp.file_upload("Your data").table()` reads a file the user picks (it
  stays in the browser). Both give a `Signal<Loadable<DTable>>`:
  `dapp.load_status(&data)` shows a spinner or the error, and
  `dapp.table(&data)` shows the table once it is there. Pass `fetch_table` a
  `Signal<String>` to fetch again whenever the URL changes. See
  `just example-loading`.
- **Cargo features:** `csv` and `json` are on by default; `ndarray` adds
  `DTable::from_ndarray`. Use `default-features = false` to drop the parsers
  you do not need (they cost nothing in an app that never calls them, but
  they do take time to compile).

See `just example-data`.

## Charts

Each chart takes a table, the column for the x-axis, and the columns to plot.
Like `dapp.table`, they follow a `Signal<DTable>`:

```rust
dapp.line_chart(&monthly, "month", &["north", "south", "east"])
    .title("Units sold per month")
    .y_label("Units");

// The same numbers as parts of a whole.
dapp.bar_chart(&monthly, "month", &["north", "south", "east"]).stacked();

// Dates on x (2026-03-05, 2026-03, 2026-03-05 14:30) get a time axis.
dapp.area_chart(&per_day, "date", &["revenue"]);

// Bars compare totals: add up the orders of each region first.
let by_region = selected.map(|orders| orders.sum_by("region", &["revenue"]));
dapp.bar_chart(by_region, "region", &["revenue"]);

dapp.scatter_chart(&selected, "units", &["revenue"]);

// How one column's values are spread.
dapp.histogram(&selected, "revenue").bins(10);

// Long category names: sideways bars, largest first.
dapp.bar_chart(&by_product, "product", &["revenue"]).horizontal();

// When the point is one number, a figure says it better than a chart.
dapp.metric("Revenue", format_compact(48_250.0)).delta(12.5, "% vs last month");
```

| Chart | Use it for |
|---|---|
| `line_chart` | change along an ordered x: time, steps |
| `area_chart` | the same, for an amount; best with one or two columns |
| `bar_chart` | comparing categories; `.stacked()` for parts of a whole, `.horizontal()` for long names |
| `scatter_chart` | how two numeric columns relate |
| `histogram` | how the values of one column are spread |
| `metric` | one headline number, with its change |

Every chart sizes itself to its place on the page, follows the light or dark
theme, and comes with a legend (for several series), a read-out that follows
the pointer or the arrow keys, and a "Show data" table. Colours are assigned in
a fixed order chosen to stay distinguishable for colour-blind readers, which is
why a line, area or bar chart draws at most 8 series and a scatter chart 3.

Dates are recognised when they cannot be misread: `2026-03-05`, `05.03.2026`,
`5 Mar 2026`, with or without a time. For anything else, such as
`03/05/2026`, say how it is written: `.date_format("%m/%d/%Y")`.

For anything these do not cover, use a plotting crate: `dapp.svg(text)` shows
SVG output (for example from `poloto`), and `dapp.canvas(w, h)` gives you a
canvas to draw on (for example with `plotters`). See `just example-charts` and
`just example-chart-crates`.

## Markdown and LaTeX

```rust
dapp.markdown("## Method\n\nWe fit a **linear** model, $y = ax + b$.");
dapp.latex(r"\hat{a} = \frac{\sum (x_i - \bar{x})(y_i - \bar{y})}{\sum (x_i - \bar{x})^2}");

// Both follow a Signal<String>: a live preview of what the user types.
let notes = dapp.textarea("Notes (Markdown)");
dapp.markdown(notes.value());

// text! shorthands, with a template and a signal if you like.
text!("Total: **{}**", total, _md);
```

- **Markdown** covers headings, emphasis, lists, links, images, code, tables,
  task lists and footnotes; `$...$` and `$$...$$` are math.
- **Safe by default.** HTML written inside Markdown is shown as text, links
  that would run script go nowhere, and LaTeX cannot add markup, so text from
  users, files and APIs can be shown as is. `dapp.markdown_with_html` allows
  HTML, for text that is part of your app.
- **Nothing to add to `index.html`.** Math is drawn as MathML, which browsers
  display themselves. It looks best where a math font is installed (Cambria
  Math, STIX Two Math, Latin Modern Math); most desktops have one.
- **Cargo features:** `markdown` and `latex`, both on by default. Together
  they add roughly 300 KB to the wasm of an app that uses them, and nothing to
  one that does not.

See `just example-prose`.

## Inputs

Inputs are typed: each returns a `Field<T>` whose `.value()` is a reactive
`Signal<T>`, so you compute with numbers and booleans, not strings.

```rust
dapp!(dapp, {
    let price = dapp.number("Price", 9.5).min(0.0).step(0.5);     // f64
    let quantity = dapp.slider("Quantity", 1.0, 20.0, 3.0);       // f64
    let gift = dapp.checkbox("Gift wrap (+2)", false);            // bool
    let unit = dapp.select("Currency", &["EUR", "USD"]);          // &str

    let total = price.value()
        .zip(&quantity.value())
        .zip(&gift.value())
        .map(|((p, q), gift)| p * q + if *gift { 2.0 } else { 0.0 });
    text!("Total: {}", total);
});
```

Available: `text_input`, `textarea`, `number`, `slider`, `checkbox`, `switch`,
`select`, `radio`. Options of `select` and `radio` can be any type that is
`Display + Clone + PartialEq`, including your own enums. `Signal::map` and
`Signal::zip` derive new values from inputs. See `just example-inputs`.

## Layout

Layout methods return a `Container`, which has every method the page has, so
layouts nest:

```rust
dapp!(dapp, {
    let sidebar = dapp.sidebar();
    let name = sidebar.input("Your name");

    let [left, right] = dapp.columns();          // or dapp.columns_weighted([2, 1])
    left.card_titled("Greeting").text("Hello, {}!", name.text(), None);

    // `with` makes text! write into a container, like Streamlit's `with col:`
    right.with(|col| {
        text!("Right column", _h3);
        col.button("Click me");
    });

    dapp.expander("Details").p_static("Hidden until opened.");
});
```

Available: `container`, `section`, `columns`, `columns_weighted`, `card`,
`card_titled`, `expander`, `expander_open`, `sidebar`. See `just example-layout`.

## Styling

Apps are styled out of the box. [Pico CSS](https://picocss.com) is vendored in
the `dappr-styles` crate and added to the page when the app starts, so your
`index.html` needs no stylesheet link and your crate needs no `build.rs`.

For your own CSS, `style!` scopes it and generates matching class names:

```rust
style!("badge", ".pill { border-radius: 999px; }");
// badge::pill == "badge-pill", badge::CSS == ".badge-pill{border-radius:999px}"

dapp.style(badge::CSS);
dapp.button("New").add_class(badge::pill);
```

## Dependencies and useful tools

Dappr's workflow employs the following tools:

- the `wasm32-unknown-unknown` Rust target (`rustup target add wasm32-unknown-unknown`)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/) ([repo](https://github.com/rustwasm/wasm-pack)) to build the library and run the browser tests
- [trunk](https://trunkrs.dev/) ([repo](https://github.com/trunk-rs/trunk)) to run examples and dapps locally
- [just](https://just.systems/) ([repo](https://github.com/casey/just) to automate builds

## Build

Dappr ships wih a .justfile, so you just run `just` as *build* is the default recipe:

```bash
just
```

## Run examples

Using `just` all examples recipes start with *example-* and use `trunk` to run.
List all recipes and select the example you want

```bash
just --list
```

For example:

```bash
just example-hello
```

## Test

```bash
just test
```

This runs the native tests and then the browser tests (headless Firefox, so
`geckodriver` must be installed).

## License

Copyright (c) 2024-2026 Davide Del Papa under the terms of the MIT License (see LICENSE file attached).
