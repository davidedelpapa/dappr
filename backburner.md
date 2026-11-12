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
