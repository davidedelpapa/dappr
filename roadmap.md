# Tentative Roadmap

Feb. 09, 2025 -- Davide Del Papa

Updated Nov. 2026. Items marked *(rustream)* existed in the first version of
this idea, [rustream](https://codeberg.org/davidedelpapa/rustream) (built on
Yew), and are to be brought over. Ideas parked for later are in
`backburner.md`.

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

- File upload (load a CSV or JSON picked by the user)

Reactivity

- Derived values -- OK (`Signal::map`, `Signal::zip`, `computed!`)

- Subscriptions tied to a component's lifetime, so derived signals created
  in callbacks do not leak

Data types

- RawHtml -- OK

- RawSvg *(rustream)*

- Canvas *(rustream: `<Canvas>` + `<CanvasElement>`)*

- Markdown *(rustream: via `pulldown-cmark`)*

- LaTeX *(rustream: via `katex`)*

- Csv (Tsv, etc) -- OK (`DTable::from_csv`, `from_tsv`, `CsvOptions`) *(rustream: delimiter / headers / flexible / trim options)*

- Json -- OK for tables (`DTable::from_json`); a pretty-printed JSON view is still to do *(rustream: `<ViewJson>`)*

- DTable (With support for ndarrays) -- OK (`ndarray` feature) *(rustream: `DTable` / `DColumn`)*

- Table view of a DTable -- OK *(rustream: `<ViewTable>`, `<ViewCsv>`)*

- Charts
  - integrate existing Rust chart crates first *(rustream: `poloto` through
    RawSvg, `plotters` through Canvas)*
  - mermaid

- REST APIs (fetch)

Packaging

- One Cargo feature per heavy component (csv, json, markdown, latex, ...) --
  started (`csv`, `json`, `ndarray`, `sanitizehtml`), so
  apps only pay in wasm size for what they use *(rustream; its README notes
  3-4 MB wasm files as the problem this addresses)*
