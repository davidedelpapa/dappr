//! # Chart crates example
//!
//! Dappr's own `line_chart` / `bar_chart` / `scatter_chart` cover the common
//! cases. For anything else, use a plotting crate and hand Dappr the result:
//!
//! - a crate that outputs **SVG text** (here [poloto]) goes through `dapp.svg`;
//! - a crate that draws on a **canvas** (here [plotters]) goes through
//!   `dapp.canvas`.
//!
//! Both are reactive here: each drawing is redone when its slider moves.
//!
//! [poloto]: https://crates.io/crates/poloto
//! [plotters]: https://crates.io/crates/plotters
// The two integrations come from rustream's `poloto-integration` and
// `plotters-integration` examples; what is new is that they follow a slider.
use dappr::prelude::*;
use plotters::prelude::*;
use plotters_canvas::CanvasBackend;
use wasm_bindgen::prelude::*;

/// A bell curve as SVG text, drawn by poloto.
/// Adapted from the "gaussian" example in poloto's README.
fn bell_curve_svg(sigma: f64) -> String {
    use std::f64::consts::TAU;
    let curve = move |x: f64| {
        let k = (sigma * TAU).sqrt().recip();
        [x, (-0.5 * x.powi(2) / sigma.powi(2)).exp() * k]
    };
    let xs = poloto::util::range_iter([-5.0, 5.0], 200);
    let plot = poloto::build::plot(format!("sigma = {sigma}")).line(xs.map(curve));

    poloto::frame_build()
        .data(poloto::plots!(poloto::build::origin(), plot))
        .build_and_label(("Bell curve (poloto)", "x", "y"))
        .append_to(poloto::header().light_theme())
        .render_string()
        .unwrap_or_else(|_| String::from("<svg></svg>"))
}

/// Draws y = x^power on the canvas with plotters.
/// Adapted from plotters' quick-start example.
fn draw_power_curve(canvas: &Canvas, power: i32) -> Option<()> {
    let backend = CanvasBackend::with_canvas_object(canvas.canvas().clone())?;
    let root = backend.into_drawing_area();
    root.fill(&WHITE).ok()?;
    let mut chart = ChartBuilder::on(&root)
        .caption(format!("y = x^{power} (plotters)"), ("sans-serif", 22))
        .margin(10)
        .x_label_area_size(30)
        .y_label_area_size(40)
        .build_cartesian_2d(-1f32..1f32, -1.1f32..1.1f32)
        .ok()?;
    chart.configure_mesh().draw().ok()?;
    chart
        .draw_series(LineSeries::new(
            (-50..=50)
                .map(|x| x as f32 / 50.0)
                .map(|x| (x, x.powi(power))),
            &RED,
        ))
        .ok()?;
    root.present().ok()
}

#[wasm_bindgen(start)]
pub fn run() {
    dapp!(dapp, {
        text!("Charts from other crates", _title);
        let [left, right] = dapp.columns();

        // ---- SVG text: poloto -> dapp.svg ---------------------------------
        left.h4_static("SVG, with poloto");
        let sigma = left.slider("Sigma", 0.3, 2.0, 1.0).step(0.1).value();
        // A Signal<String> of SVG: redrawn each time sigma changes.
        left.svg(sigma.map(|sigma| bell_curve_svg(*sigma)));

        // ---- A canvas: plotters -> dapp.canvas ----------------------------
        right.h4_static("Canvas, with plotters");
        let power = right.slider("Power", 1.0, 6.0, 2.0).value();
        right.canvas(640, 480).draw_on(&power, |canvas, power| {
            let _ = draw_power_curve(canvas, *power as i32);
        });
    });
}
