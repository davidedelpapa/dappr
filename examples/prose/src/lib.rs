//! # Markdown and LaTeX example
//!
//! - `dapp.markdown(text)` shows Markdown; `$...$` and `$$...$$` inside it
//!   are math.
//! - `dapp.latex(text)` shows one equation.
//! - Both take a `Signal<String>` to follow the inputs, and `text!` has the
//!   `_md` and `_latex` specifiers as a shorthand.
//!
//! Markdown is safe with text from anywhere: try typing `<b>bold</b>` or a
//! `[link](javascript:alert(1))` in the editor below.
use dappr::prelude::*;
use wasm_bindgen::prelude::*;

const REPORT: &str = r#"
## Compound interest

Money left to grow earns interest on its interest. With a yearly rate $r$,
paid $n$ times a year, a sum $P$ becomes after $t$ years:

$$A = P \left(1 + \frac{r}{n}\right)^{nt}$$

| Paid | $n$ |
|---|---|
| Yearly | 1 |
| Monthly | 12 |
| Daily | 365 |

- [x] Markdown tables, lists and task lists
- [x] Math, inline and on its own line
- [ ] ~~JavaScript to load~~ none needed
"#;

const STARTING_NOTES: &str = "### My notes\n\nWrite **Markdown** here, with math like $e^{i\\pi} + 1 = 0$.\n\n1. It updates as you type\n2. HTML such as <b>this</b> stays text";

#[wasm_bindgen(start)]
pub fn run() {
    dapp!(dapp, {
        text!("Markdown and LaTeX", _title);

        // ---- Fixed text ---------------------------------------------------
        dapp.markdown(REPORT);

        // ---- An equation and a result that follow the inputs --------------
        let [inputs, result] = dapp.columns();
        let principal = inputs.number("Starting sum (P)", 1000.0).min(0.0).value();
        let rate = inputs
            .slider("Yearly rate in % (r)", 0.0, 15.0, 5.0)
            .step(0.5)
            .value();
        let years = inputs.slider("Years (t)", 1.0, 40.0, 10.0).value();

        result.with(|_| {
            // The same formula, with the chosen numbers in it.
            let equation = computed!(principal, rate, years => {
                let amount = principal * (1.0 + rate / 100.0 / 12.0).powf(12.0 * years);
                format!(
                    r"A = {principal} \left(1 + \frac{{{:.3}}}{{12}}\right)^{{12 \cdot {years}}} = {amount:.2}",
                    rate / 100.0
                )
            });
            text!("Paid monthly:", _h5);
            dapp!(result, {
                result.latex(equation);
            });

            // `text!` with a template and a signal, rendered as Markdown.
            let gain = computed!(principal, rate, years => {
                let amount = principal * (1.0 + rate / 100.0 / 12.0).powf(12.0 * years);
                format!("{:.2}", amount - principal)
            });
            text!("That is **{}** earned in interest.", gain, _md);
        });

        // ---- A live Markdown preview --------------------------------------
        text!("Try it", _h2);
        let [editor, preview] = dapp.columns();
        let notes = editor
            .textarea("Markdown")
            .rows(8)
            .with_value(STARTING_NOTES.to_string());
        preview.card().markdown(notes.value());
    });
}
