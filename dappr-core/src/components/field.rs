//! ## Field: typed, labelled inputs
//!
//! A [`Field<T>`] is a form control together with its label and a
//! [`Signal<T>`] holding its current value, already converted to the right
//! type: `f64` for numbers and sliders, `bool` for checkboxes and switches,
//! `String` for text, and your own type for selects and radios.
//!
//! The binding goes both ways: when the user changes the control the Signal
//! is set, and when your code sets the Signal the control shows the new value.
use crate::communication::signals::Signal;
use crate::data::load::{parse_table, Loadable};
use crate::data::table::DTable;
use crate::data::DataError;
use crate::framework::dom;
use crate::framework::traits::Component;
use std::cell::Cell;
use std::fmt::Display;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{HtmlElement, HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement};

thread_local! {
    /// Source of unique `name`s, so each radio group is its own group.
    static NEXT_GROUP: Cell<u32> = const { Cell::new(0) };
}

/// Calls `on_event` every time `event` fires on `el`, for as long as `el`
/// is on the page.
fn listen(el: &HtmlElement, event: &str, mut on_event: impl FnMut() + 'static) {
    dom::bind(el, dom::on(el, event, move |_| on_event()));
}

fn input_el(input_type: &str) -> HtmlInputElement {
    let input: HtmlInputElement = dom::create_element("input").unchecked_into();
    input.set_type(input_type);
    input
}

/// `<label><span>text</span>control</label>`, or with the control first for
/// checkboxes and switches, where the text goes to the right of the box.
fn labelled(label: &str, control: &HtmlElement, control_first: bool) -> (HtmlElement, HtmlElement) {
    let wrapper = dom::create_element("label");
    let text = dom::create_element("span");
    text.set_text_content(Some(label));
    let (first, second) = if control_first {
        (control, &text)
    } else {
        (&text, control)
    };
    wrapper.append_child(first).expect("Failed to build field");
    wrapper.append_child(second).expect("Failed to build field");
    (wrapper, text)
}

/// A labelled form control bound to a [`Signal<T>`].
///
/// # Example
/// ```ignore
/// use dappr::prelude::*;
/// use wasm_bindgen::prelude::*;
///
/// #[wasm_bindgen(main)]
/// pub fn main() {
///     dapp!(dapp, {
///         let age = dapp.number("Age", 30.0).min(0.0).max(120.0);
///         text!("In ten years you will be {}", age.value().map(|a| a + 10.0));
///     });
/// }
/// ```
pub struct Field<T> {
    el: HtmlElement,
    control: HtmlElement,
    signal: Signal<T>,
}

impl<T: Clone + 'static> Field<T> {
    /// The reactive value of the field. Pass it to `text!`, `map` it, or
    /// `subscribe` to it.
    pub fn value(&self) -> Signal<T> {
        self.signal.clone()
    }

    /// The current value.
    pub fn get(&self) -> T {
        self.signal.get()
    }

    /// Sets the value. The control on the page updates to match.
    pub fn set(&self, value: T) {
        self.signal.set(value);
    }

    /// Chainable form of [`set`](Field::set), to give a field its starting
    /// value: `dapp.text_input("Name").with_value("Ada".to_string())`.
    pub fn with_value(self, value: T) -> Self {
        self.set(value);
        self
    }

    /// Disables (or re-enables) the control. Chainable.
    pub fn disabled(self, disabled: bool) -> Self {
        self.set_disabled(disabled);
        self
    }

    /// Disables (or re-enables) the control, e.g. from a callback.
    pub fn set_disabled(&self, disabled: bool) {
        // On a radio group the control is the <fieldset>, which disables
        // every radio inside it.
        if disabled {
            self.control
                .set_attribute("disabled", "")
                .expect("Failed to disable the field");
        } else {
            self.control
                .remove_attribute("disabled")
                .expect("Failed to enable the field");
        }
    }

    /// The control itself (`<input>`, `<select>`, `<textarea>`, or the
    /// `<fieldset>` of a radio group), as opposed to
    /// [`get_element`](Component::get_element), which is the whole field
    /// including its label.
    pub fn control(&self) -> &HtmlElement {
        &self.control
    }

    fn control_attribute(self, name: &str, value: &str) -> Self {
        self.control
            .set_attribute(name, value)
            .unwrap_or_else(|_| panic!("Cannot set the attribute: {} = {}", name, value));
        self
    }
}

impl<T> Component for Field<T> {
    fn get_element(&self) -> &HtmlElement {
        &self.el
    }
}

// ---- Text -----------------------------------------------------------------

impl Field<String> {
    /// A single-line text input.
    pub fn text(label: &str) -> Self {
        let input = input_el("text");
        let signal = Signal::new(String::new());

        let (input_in, signal_in) = (input.clone(), signal.clone());
        listen(&input, "input", move || signal_in.set(input_in.value()));
        let input_out = input.clone();
        crate::framework::dom::bind(
            &input,
            signal.subscribe(move |value| {
                // Only touch the control when it differs, so typing does not
                // move the cursor.
                if input_out.value() != *value {
                    input_out.set_value(value);
                }
            }),
        );

        let (el, _) = labelled(label, &input, false);
        Field {
            el,
            control: input.into(),
            signal,
        }
    }

    /// A multi-line text input.
    pub fn textarea(label: &str) -> Self {
        let area: HtmlTextAreaElement = dom::create_element("textarea").unchecked_into();
        let signal = Signal::new(String::new());

        let (area_in, signal_in) = (area.clone(), signal.clone());
        listen(&area, "input", move || signal_in.set(area_in.value()));
        let area_out = area.clone();
        crate::framework::dom::bind(
            &area,
            signal.subscribe(move |value| {
                if area_out.value() != *value {
                    area_out.set_value(value);
                }
            }),
        );

        let (el, _) = labelled(label, &area, false);
        Field {
            el,
            control: area.into(),
            signal,
        }
    }

    /// Greyed-out hint shown while the field is empty. Chainable.
    pub fn placeholder(self, placeholder: &str) -> Self {
        self.control_attribute("placeholder", placeholder)
    }

    /// Number of visible text lines, for a [`textarea`](Field::textarea).
    /// Chainable.
    pub fn rows(self, rows: u32) -> Self {
        self.control_attribute("rows", &rows.to_string())
    }
}

// ---- Numbers --------------------------------------------------------------

impl Field<f64> {
    /// Shared by `number` and `slider`: an `<input>` whose value is a number.
    fn numeric(input: &HtmlInputElement, initial: f64) -> Signal<f64> {
        input.set_value_as_number(initial);
        let signal = Signal::new(initial);

        let (input_in, signal_in) = (input.clone(), signal.clone());
        listen(input, "input", move || {
            let value = input_in.value_as_number();
            // Empty or half-typed ("-", "1e") reads as NaN: keep the last
            // good value until the text is a number again.
            if !value.is_nan() {
                signal_in.set(value);
            }
        });
        let input_out = input.clone();
        crate::framework::dom::bind(
            input,
            signal.subscribe(move |value| {
                if input_out.value_as_number() != *value {
                    input_out.set_value_as_number(*value);
                }
            }),
        );
        signal
    }

    /// A number input. Accepts decimals; use [`step`](Field::step) to change
    /// what the arrows add, and [`min`](Field::min) / [`max`](Field::max) for
    /// the allowed range.
    ///
    /// While the box is empty or not a number, the value stays what it last was.
    pub fn number(label: &str, initial: f64) -> Self {
        let input = input_el("number");
        // Without this, browsers flag any decimal as invalid.
        input.set_step("any");
        let signal = Self::numeric(&input, initial);
        let (el, _) = labelled(label, &input, false);
        Field {
            el,
            control: input.into(),
            signal,
        }
    }

    /// A slider from `min` to `max`. The current value is shown next to the
    /// label. Between whole numbers it moves in steps of 1; otherwise it
    /// moves freely. Use [`step`](Field::step) to choose the step yourself.
    pub fn slider(label: &str, min: f64, max: f64, initial: f64) -> Self {
        let input = input_el("range");
        // Range before value: the browser clamps the value to the range it
        // knows about at the time.
        input.set_min(&min.to_string());
        input.set_max(&max.to_string());
        // A slider steps by 1 by default, which would silently move a
        // starting value such as 0.8 (on 0..1) to a whole number. When the
        // range or the start is not whole, allow any value until `.step()`
        // says otherwise.
        if [min, max, initial].iter().any(|v| v.fract() != 0.0) {
            input.set_step("any");
        }
        let signal = Self::numeric(&input, initial);

        let (el, text) = labelled(label, &input, false);
        // A <span>, not an <output>: <output> is a labelable element, and
        // coming first it would become what the <label> labels, instead of
        // the slider.
        let output = dom::create_element("span");
        output.set_class_name("dappr-field-value");
        output.set_text_content(Some(&initial.to_string()));
        text.after_with_node_1(&output)
            .expect("Failed to build slider");
        crate::framework::dom::bind(
            &input,
            signal.subscribe(move |value| output.set_text_content(Some(&value.to_string()))),
        );

        Field {
            el,
            control: input.into(),
            signal,
        }
    }

    /// Sets a range attribute, then makes the control and the value agree
    /// again: the browser snaps a slider to its range and step, so the value
    /// is re-applied under the new rule, and whatever the control then shows
    /// becomes the value.
    fn range_attribute(self, name: &str, value: f64) -> Self {
        let field = self.control_attribute(name, &value.to_string());
        let input: &HtmlInputElement = field.control.unchecked_ref();
        let wanted = field.signal.get();
        input.set_value_as_number(wanted);
        let shown = input.value_as_number();
        if !shown.is_nan() && shown != wanted {
            field.signal.set(shown);
        }
        field
    }

    /// Smallest allowed value. Chainable.
    pub fn min(self, min: f64) -> Self {
        self.range_attribute("min", min)
    }

    /// Largest allowed value. Chainable.
    pub fn max(self, max: f64) -> Self {
        self.range_attribute("max", max)
    }

    /// Distance between allowed values: `1.0` for whole numbers, `0.01` for
    /// cents. Chainable.
    pub fn step(self, step: f64) -> Self {
        self.range_attribute("step", step)
    }
}

// ---- Booleans -------------------------------------------------------------

impl Field<bool> {
    fn boolean(label: &str, initial: bool, switch: bool) -> Self {
        let input = input_el("checkbox");
        if switch {
            input
                .set_attribute("role", "switch")
                .expect("Failed to build switch");
        }
        input.set_checked(initial);
        let signal = Signal::new(initial);

        let (input_in, signal_in) = (input.clone(), signal.clone());
        listen(&input, "change", move || signal_in.set(input_in.checked()));
        let input_out = input.clone();
        crate::framework::dom::bind(
            &input,
            signal.subscribe(move |value| input_out.set_checked(*value)),
        );

        let (el, _) = labelled(label, &input, true);
        // Pico spaces text-like controls but not a lone checkbox; this class
        // gives it the same room below (see dappr.css).
        el.set_class_name("dappr-field-check");
        Field {
            el,
            control: input.into(),
            signal,
        }
    }

    /// A checkbox.
    pub fn checkbox(label: &str, initial: bool) -> Self {
        Self::boolean(label, initial, false)
    }

    /// An on/off switch: a checkbox drawn as a toggle.
    pub fn switch(label: &str, initial: bool) -> Self {
        Self::boolean(label, initial, true)
    }
}

// ---- Files ---------------------------------------------------------------

/// A file the user picked with a [`file`](Field::file) field, already read
/// into memory.
#[derive(Debug, Clone, PartialEq)]
pub struct UploadedFile {
    name: String,
    mime: String,
    bytes: Rc<Vec<u8>>,
}

impl UploadedFile {
    /// A file from its parts. The browser builds these for you; this is for
    /// tests and for data that comes from somewhere else.
    pub fn new(name: &str, mime: &str, bytes: Vec<u8>) -> Self {
        UploadedFile {
            name: name.to_string(),
            mime: mime.to_string(),
            bytes: Rc::new(bytes),
        }
    }

    /// The file's name, without any path.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The MIME type the browser reports (`text/csv`, ...); may be empty.
    pub fn mime(&self) -> &str {
        &self.mime
    }

    /// The file's content.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Size in bytes.
    pub fn size(&self) -> usize {
        self.bytes.len()
    }

    /// The content as text. Bytes that are not valid UTF-8 are replaced
    /// with the "unknown character" mark rather than causing an error.
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.bytes).into_owned()
    }

    /// The content as a table: JSON for `.json` files, tab-separated for
    /// `.tsv`, comma-separated otherwise (see [`parse_table`]).
    pub fn table(&self) -> Result<DTable, DataError> {
        parse_table(&self.name, &self.mime, &self.text())
    }
}

impl Field<Option<UploadedFile>> {
    /// A file picker. The value is `None` until the user picks a file, then
    /// the file, read into memory. The file stays in the browser: nothing
    /// is sent anywhere.
    ///
    /// Setting the value to `None` clears the picker.
    pub fn file(label: &str) -> Self {
        let input = input_el("file");
        let signal: Signal<Option<UploadedFile>> = Signal::new(None);
        // Reading is asynchronous. If the user picks again before a read
        // finishes, only the newest pick may set the value.
        let latest = Rc::new(Cell::new(0u64));

        let (input_in, signal_in) = (input.clone(), signal.clone());
        listen(&input, "change", move || {
            let pick = latest.get() + 1;
            latest.set(pick);
            let Some(file) = input_in.files().and_then(|files| files.get(0)) else {
                signal_in.set(None);
                return;
            };
            let (signal, latest) = (signal_in.clone(), Rc::clone(&latest));
            spawn_local(async move {
                let Ok(buffer) = JsFuture::from(file.array_buffer()).await else {
                    return;
                };
                if latest.get() != pick {
                    return;
                }
                let bytes = js_sys::Uint8Array::new(&buffer).to_vec();
                signal.set(Some(UploadedFile::new(&file.name(), &file.type_(), bytes)));
            });
        });
        let input_out = input.clone();
        crate::framework::dom::bind(
            &input,
            signal.subscribe(move |value| {
                if value.is_none() {
                    input_out.set_value("");
                }
            }),
        );

        let (el, _) = labelled(label, &input, false);
        Field {
            el,
            control: input.into(),
            signal,
        }
    }

    /// Which files the picker offers: extensions and/or MIME types,
    /// comma-separated, e.g. `".csv,.tsv,.json"`. Chainable.
    ///
    /// This is a convenience for the user, not a check: they can still pick
    /// any file.
    pub fn accept(self, accept: &str) -> Self {
        self.control_attribute("accept", accept)
    }

    /// The picked file as a table: [`Idle`](Loadable::Idle) until a file is
    /// picked, then [`Ready`](Loadable::Ready) with the table, or
    /// [`Failed`](Loadable::Failed) if the file cannot be read as one.
    ///
    /// ```ignore
    /// let upload = dapp.file_upload("Your data").accept(".csv,.json");
    /// let data = upload.table();
    /// dapp.load_status(&data);
    /// dapp.table(&data);
    /// ```
    pub fn table(&self) -> Signal<Loadable<DTable>> {
        self.signal.map(|file| match file {
            None => Loadable::Idle,
            Some(file) => file.table().into(),
        })
    }
}

// ---- Choices --------------------------------------------------------------

impl<T: Display + Clone + PartialEq + 'static> Field<T> {
    fn require_options(kind: &str, options: &[T]) {
        assert!(
            !options.is_empty(),
            "Field::{kind} needs at least one option to choose from"
        );
    }

    /// A dropdown to pick one of `options`. Starts on the first one.
    ///
    /// The value is the option itself, not its text, so `options` can be
    /// numbers, strings, or any type of yours that is `Display + Clone +
    /// PartialEq`. Setting the value to something that is not one of the
    /// options leaves the dropdown where it was.
    ///
    /// # Panics
    /// If `options` is empty.
    pub fn select(label: &str, options: &[T]) -> Self {
        Self::require_options("select", options);
        let select: HtmlSelectElement = dom::create_element("select").unchecked_into();
        for option in options {
            let el = dom::create_element("option");
            el.set_text_content(Some(&option.to_string()));
            select.append_child(&el).expect("Failed to build select");
        }
        let signal = Signal::new(options[0].clone());

        let (select_in, signal_in, options_in) = (select.clone(), signal.clone(), options.to_vec());
        listen(&select, "change", move || {
            if let Some(option) = options_in.get(select_in.selected_index() as usize) {
                signal_in.set(option.clone());
            }
        });
        let (select_out, options_out) = (select.clone(), options.to_vec());
        crate::framework::dom::bind(
            &select,
            signal.subscribe(move |value| {
                if let Some(index) = options_out.iter().position(|option| option == value) {
                    select_out.set_selected_index(index as i32);
                }
            }),
        );

        let (el, _) = labelled(label, &select, false);
        Field {
            el,
            control: select.into(),
            signal,
        }
    }

    /// Radio buttons to pick one of `options`. Starts on the first one.
    ///
    /// Like [`select`](Field::select), the value is the option itself.
    ///
    /// # Panics
    /// If `options` is empty.
    pub fn radio(label: &str, options: &[T]) -> Self {
        Self::require_options("radio", options);
        let group = NEXT_GROUP.with(|next| {
            let id = next.get();
            next.set(id + 1);
            format!("dappr-radio-{id}")
        });
        let fieldset = dom::create_element("fieldset");
        let legend = dom::create_element("legend");
        legend.set_text_content(Some(label));
        fieldset
            .append_child(&legend)
            .expect("Failed to build radio group");

        let signal = Signal::new(options[0].clone());
        let mut inputs = Vec::with_capacity(options.len());
        for (index, option) in options.iter().enumerate() {
            let input = input_el("radio");
            input.set_name(&group);
            input.set_checked(index == 0);

            let (input_in, signal_in, option_in) = (input.clone(), signal.clone(), option.clone());
            listen(&input, "change", move || {
                if input_in.checked() {
                    signal_in.set(option_in.clone());
                }
            });

            let (row, _) = labelled(&option.to_string(), &input, true);
            fieldset
                .append_child(&row)
                .expect("Failed to build radio group");
            inputs.push(input);
        }

        let options_out = options.to_vec();
        crate::framework::dom::bind(
            &fieldset,
            signal.subscribe(move |value| {
                if let Some(index) = options_out.iter().position(|option| option == value) {
                    inputs[index].set_checked(true);
                }
            }),
        );

        Field {
            el: fieldset.clone(),
            control: fieldset,
            signal,
        }
    }
}
