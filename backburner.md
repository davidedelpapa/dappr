# Backburner

Things we decided are worth doing, but not yet. Revisit at the end of the
current round of work.

## Tabs container

**What:** `dapp.tabs(["Chart", "Data"])` returning one `Container` per tab,
the counterpart of Streamlit's `st.tabs`. Only the selected tab's content is
visible.

**Why it is here and not done:** tabs are common in Streamlit apps, but they
organise a page rather than enable one. Typed inputs came first.

**Decision (Nov 2026): do not patch `pico.css` for this.** Pico has no tab
component, and editing the vendored file would mean re-applying the patch on
every Pico upgrade. Put the rules in `dappr-styles/src/assets/dappr.css`,
next to the columns and sidebar rules, using Pico's variables
(`--pico-primary`, `--pico-muted-border-color`, ...) so tabs follow the theme.

**Sketch:**

- Markup: a `<div role="tablist">` of `<button role="tab">`, followed by one
  `<div role="tabpanel">` per tab; inactive panels get the `hidden` attribute.
- Behaviour: a `Signal<usize>` for the selected index; clicking a tab sets it;
  one subscriber updates `aria-selected` and `hidden`. Expose the signal so
  apps can react to, or change, the selected tab.
- API shape: same const-generic pattern as `columns`, so
  `let [chart, data] = dapp.tabs(["Chart", "Data"]);`.
- Accessibility: arrow-key navigation between tabs.

## From rustream, not yet brought over

Rustream (https://codeberg.org/davidedelpapa/rustream) was the first version
of this project, built on Yew. Most of it now exists in Dappr in some form
(tables, CSV, JSON, Markdown, LaTeX, SVG, canvas, cards, sections). What is
still missing:

### Navigation bar

**What rustream had:** `<NavBar sticky={true}>` holding a `<Logo>` and a
`<NavMenu>` of links, collapsing into a menu button on narrow screens
(`src/components/layout.rs`, styles in `vendor/milligram.nav.css`).

**Sketch for Dappr:** `dapp.navbar("App name")` returning a `Container` for
the links, placed above the sidebar and the page. Pico already styles
`<nav><ul>...</ul></nav>`; the collapse on narrow screens and the sticky
option need a few rules in `dappr.css`. A navbar mostly makes sense together
with several pages, which Dappr does not have yet -- decide that first.

### Page with an automatic table of contents

**What rustream had:** `<Page>` collected every `<Title>` and `<Header>`
inside it (id, text, level) into a context, and `<Toc>` rendered that list as
links (`src/components/layout.rs`: `PageState`, `TOCEntry`, `PageAction`).

**Sketch for Dappr:** `dapp.toc()` returning a component that lists the
headings of the page (or of a container), each linking to its heading. Give
headings an `id` derived from their text when they are created, and keep the
list in a `Signal` so headings added later appear too. A natural home for it
is the sidebar. Markdown headings should be included.

### A pretty-printed JSON view

**What rustream had:** `<ViewJson>` showed a parsed JSON value indented in a
`<pre>` (`src/components/dataview.rs`).

**Sketch for Dappr:** `dapp.json(text)` -- indented, in a `<pre><code>`, with
a clear message when the text is not valid JSON. Streamlit's `st.json` also
lets the reader collapse objects and arrays; nested `<details>` would do it.

### A generic data view

**What rustream had:** `<DataView<T>>` showed any `Display` value in a `<pre>`
or a `<div>`.

**Probably not needed:** `text!` and `dapp.text` already show any `Display`
value. Listed so it is a decision, not an oversight.

## Other things parked along the way

- **A bundled math font** for LaTeX, as an opt-in Cargo feature, for viewers
  whose machine has none (about 400 KB). Without one, brackets do not stretch.
- **POST, headers and authentication** for `fetch_table` / `fetch_text`
  (GET only today).
- **Subscriptions tied to a component's lifetime**, so signals derived inside
  callbacks (`map`, `zip`, `computed!`) do not stay subscribed for ever.
- **A sparkline in the metric tile**, showing the recent trend.
- **Syntax highlighting** for code blocks in Markdown.
- **Removing `scan_dir` / `bundle`** from `dappr-styles`: nothing uses them
  since styles are injected at runtime.
