# Tentative Roadmap

Feb. 09, 2025 -- Davide Del Papa

Updated Nov. 2026. Items marked *(rustream)* existed in the first version of
this idea, [rustream](https://codeberg.org/davidedelpapa/rustream) (built on
Yew). Ideas parked for later, including the rustream items not yet brought
over, are in `backburner.md`.

## Features to implement

Basic Elements to set up a page:

Layouts:

- Containers -- OK (`container`, `with`)

- Sectioning -- OK (`section`, `sidebar`)

- Simple Grid -- OK (`columns`, `columns_weighted`)

Content

- Typography -- Improve

- Links

- Buttons (outside forms)

- Tables -- OK (`dapp.table`, static or from a `Signal<DTable>`)

- Toc, built automatically from the page's titles and headers *(rustream: `<Page>` + `<Toc>`)*

Components:

- Accordion -- OK (`expander`)

- Card -- OK (`card`, `card_titled`)

- Dropdown

- Groups

- Modals

- Nav and menus *(rustream: sticky, collapsible `<NavBar>` with `<Logo>` and `<NavMenu>`)*

- Tabs -- see `backburner.md`

- Bars (Progress bar)

- Tooltip

- Badges

Forms

- Inputs and textareas -- OK (`text_input`, `textarea`, `number`)

- Select -- OK (`select`)

- Checkboxes -- OK (`checkbox`)

- Radios -- OK (`radio`)

- Switch -- OK (`switch`)

- Ranges -- OK (`slider`)

- Use of componnts outside of forms

- File upload (load a CSV or JSON picked by the user) -- OK (`file_upload`)

Reactivity

- Derived values -- OK (`Signal::map`, `Signal::zip`, `computed!`)

- Subscriptions tied to a component's lifetime -- OK: what a component
  needs lasts while it is on the page (`unmount`, `clear`), and derived
  signals are freed when unused

Data types

- RawHtml -- OK

- RawSvg -- OK (`dapp.svg`) *(rustream)*

- Canvas -- OK (`dapp.canvas`, `Canvas::draw_on`) *(rustream: `<Canvas>` + `<CanvasElement>`)*

- Markdown -- OK (`dapp.markdown`, `text!(.., _md)`; safe by default) *(rustream: via `pulldown-cmark`)*

- LaTeX -- OK (`dapp.latex`, `text!(.., _latex)`, math in Markdown), as MathML via `pulldown-latex` instead of KaTeX. Still to do: an optional bundled math font, for machines that have none *(rustream: via `katex`)*

- Csv (Tsv, etc) -- OK (`DTable::from_csv`, `from_tsv`, `CsvOptions`) *(rustream: delimiter / headers / flexible / trim options)*

- Json -- OK for tables (`DTable::from_json`); a pretty-printed JSON view is still to do *(rustream: `<ViewJson>`)*

- DTable (With support for ndarrays) -- OK (`ndarray` feature) *(rustream: `DTable` / `DColumn`)*

- Table view of a DTable -- OK *(rustream: `<ViewTable>`, `<ViewCsv>`)*

- Charts
  - built-in charts from a DTable -- OK (`line_chart`, `area_chart`,
    `bar_chart` with `.stacked()` and `.horizontal()`, `scatter_chart`,
    `histogram`), with a time axis for dates (`.date_format()` for forms
    that could be misread)
  - a headline figure -- OK (`metric`, with `.delta()`)
  - integrate existing Rust chart crates -- OK, see `examples/chart_crates`
    *(rustream: `poloto` through RawSvg, `plotters` through Canvas)*
  - mermaid

- REST APIs (fetch) -- OK for GET (`fetch_table`, `fetch_text`, refetch on a URL signal); POST, headers and authentication are still to do

Packaging

- One Cargo feature per heavy component (csv, json, markdown, latex, ...) --
  started (`csv`, `json`, `markdown`, `latex`, `ndarray`, `sanitizehtml`), so
  apps only pay in wasm size for what they use *(rustream; its README notes
  3-4 MB wasm files as the problem this addresses)*
