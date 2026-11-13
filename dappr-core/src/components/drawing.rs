//! ## Svg and Canvas: bring your own drawing
//!
//! Two ways to put a drawing made by other code on the page -- most often a
//! chart from a plotting crate:
//!
//! - [`Svg`] shows SVG text (what crates such as `poloto` produce);
//! - [`Canvas`] gives you a `<canvas>` to draw on (what `plotters` needs).
// Both come from rustream: `<RawSvg>` (src/components/rawhtml.rs) and
// `<Canvas>` / `<CanvasElement>` (src/components/canvas.rs). The drawing
// callback of `<CanvasElement>` became `Canvas::draw`; the fixed canvas id
// rustream had a TODO about is now generated, one per canvas.
use crate::communication::signals::Signal;
use crate::framework::dom;
use crate::framework::traits::{Component, TextSource};
use std::cell::Cell;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, HtmlElement};

/// An SVG drawing on the page, from SVG text.
///
/// The text is inserted as markup, so it must come from code you trust (a
/// plotting crate, your own drawing code) -- never from a user or a file
/// they uploaded.
pub struct Svg {
    el: HtmlElement,
    source: Signal<String>,
}

impl Svg {
    /// Shows `source`.
    pub fn new(source: impl TextSource) -> Self {
        let source = source.into_text_signal();
        let el = dom::create_element("div");
        el.set_class_name("dappr-svg");
        el.set_inner_html(&source.get());
        let el_in = el.clone();
        source
            .subscribe(move |svg| el_in.set_inner_html(svg))
            .detach();
        Svg { el, source }
    }

    /// The SVG text being shown. Set it to change the drawing.
    pub fn source(&self) -> Signal<String> {
        self.source.clone()
    }
}

impl Component for Svg {
    fn get_element(&self) -> &HtmlElement {
        &self.el
    }
}

thread_local! {
    static NEXT_CANVAS: Cell<u32> = const { Cell::new(0) };
}

/// A `<canvas>` to draw on.
///
/// # Example
/// ```ignore
/// let canvas = dapp.canvas(320, 120);
/// canvas.draw(|canvas| {
///     let context = canvas.context_2d();
///     context.set_fill_style_str("#2a78d6");
///     context.fill_rect(10.0, 10.0, 100.0, 60.0);
/// });
/// ```
pub struct Canvas {
    el: HtmlElement,
    canvas: HtmlCanvasElement,
}

impl Canvas {
    /// A canvas `width` by `height` pixels, with a unique `id`.
    pub fn new(width: u32, height: u32) -> Self {
        let el = dom::create_element("div");
        el.set_class_name("dappr-canvas");
        let canvas: HtmlCanvasElement = dom::create_element("canvas").unchecked_into();
        canvas.set_width(width);
        canvas.set_height(height);
        let id = NEXT_CANVAS.with(|next| {
            let id = next.get();
            next.set(id + 1);
            id
        });
        canvas.set_id(&format!("dappr-canvas-{id}"));
        el.append_child(&canvas)
            .expect("Failed to build the canvas");
        Canvas { el, canvas }
    }

    /// The `<canvas>` element itself, for libraries that draw on one.
    pub fn canvas(&self) -> &HtmlCanvasElement {
        &self.canvas
    }

    /// The canvas' `id`, for libraries that look a canvas up by id.
    pub fn id(&self) -> String {
        self.canvas.id()
    }

    /// The canvas' 2D drawing context.
    pub fn context_2d(&self) -> CanvasRenderingContext2d {
        self.canvas
            .get_context("2d")
            .ok()
            .flatten()
            .expect("The browser should provide a 2D canvas context")
            .unchecked_into()
    }

    /// Erases the whole canvas.
    pub fn clear(&self) {
        self.context_2d().clear_rect(
            0.0,
            0.0,
            f64::from(self.canvas.width()),
            f64::from(self.canvas.height()),
        );
    }

    /// Runs your drawing code once, now.
    pub fn draw(&self, draw: impl FnOnce(&Canvas)) {
        draw(self);
    }

    /// Runs your drawing code now, and again every time `signal` changes,
    /// passing its value. The canvas is erased before each run.
    ///
    /// ```ignore
    /// let radius = dapp.slider("Radius", 5.0, 60.0, 20.0).value();
    /// dapp.canvas(200, 140).draw_on(&radius, |canvas, radius| {
    ///     let context = canvas.context_2d();
    ///     context.begin_path();
    ///     let _ = context.arc(100.0, 70.0, *radius, 0.0, std::f64::consts::TAU);
    ///     context.fill();
    /// });
    /// ```
    pub fn draw_on<T: Clone + 'static>(
        &self,
        signal: &Signal<T>,
        draw: impl Fn(&Canvas, &T) + 'static,
    ) {
        let canvas = Canvas {
            el: self.el.clone(),
            canvas: self.canvas.clone(),
        };
        draw(&canvas, &signal.get());
        signal
            .subscribe(move |value| {
                canvas.clear();
                draw(&canvas, value);
            })
            .detach();
    }
}

impl Component for Canvas {
    fn get_element(&self) -> &HtmlElement {
        &self.el
    }
}
