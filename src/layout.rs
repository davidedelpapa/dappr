//! Layout containers.
//!
//! A [`Container`] is a region of the page that you fill exactly like the
//! page itself: it has every method [`Dapp`] has (`button`, `input`, `h2`,
//! `columns`, ...), so layouts nest freely.
//!
//! ```no_run
//! use dappr::prelude::*;
//! use wasm_bindgen::prelude::*;
//!
//! #[wasm_bindgen(main)]
//! pub fn main() {
//!     dapp!(dapp, {
//!         let sidebar = dapp.sidebar();
//!         let name = sidebar.input("Your name");
//!
//!         let [left, right] = dapp.columns();
//!         left.h3_static("Left");
//!         right.with(|_| text!("Hello, {}!", name.text()));
//!
//!         let details = dapp.expander("More");
//!         details.p_static("Hidden until opened.");
//!     });
//! }
//! ```
use crate::app::Dapp;
use dappr_core::framework::dom::{create_element, document};
use dappr_core::framework::traits::Component;
use std::ops::Deref;
use wasm_bindgen::JsCast;
use web_sys::HtmlElement;

/// The `id` of the sidebar element created by [`Dapp::sidebar`].
pub const SIDEBAR_ID: &str = "dappr-sidebar";

/// A region of the page, returned by the layout methods of [`Dapp`].
///
/// It dereferences to [`Dapp`], so everything you can add to the page you can
/// add to a `Container`, including more containers. It is also a
/// [`Component`], so `add_class`, `id`, `set_attribute` and `unmount` apply to
/// the container's own element.
///
/// To make [`text!`](crate::text) write into a container, use
/// [`with`](Dapp::with).
pub struct Container {
    dapp: Dapp,
}

impl Container {
    fn new(root: HtmlElement) -> Self {
        Container {
            dapp: Dapp::from_root(root),
        }
    }
}

impl Deref for Container {
    type Target = Dapp;
    fn deref(&self) -> &Dapp {
        &self.dapp
    }
}

impl Component for Container {
    fn get_element(&self) -> &HtmlElement {
        &self.dapp.root
    }
}

/// Layouts
impl Dapp {
    /// Appends a new element with the given tag and returns it as a Container.
    fn child_container(&self, tag: &str) -> Container {
        let el = create_element(tag);
        self.append(&el);
        Container::new(el)
    }

    /// A plain, invisible container (a `<div>`).
    ///
    /// Useful to reserve a place on the page and fill it later, or to group
    /// components so they can be styled or removed together.
    ///
    /// ```no_run
    /// # use dappr::prelude::*;
    /// # let dapp = Dapp::new();
    /// let results = dapp.container();
    /// dapp.p_static("This paragraph comes after the container...");
    /// results.p_static("...but this one is shown before it.");
    /// ```
    pub fn container(&self) -> Container {
        self.child_container("div")
    }

    /// A `<section>`: like [`container`](Dapp::container), with some space
    /// after it to set it apart from what follows.
    pub fn section(&self) -> Container {
        self.child_container("section")
    }

    /// Side-by-side columns of equal width. The number of columns is the
    /// number of names you bind.
    ///
    /// On narrow screens the columns stack vertically.
    ///
    /// ```no_run
    /// # use dappr::prelude::*;
    /// # let dapp = Dapp::new();
    /// let [left, middle, right] = dapp.columns();
    /// left.h3_static("One");
    /// middle.h3_static("Two");
    /// right.h3_static("Three");
    /// ```
    pub fn columns<const N: usize>(&self) -> [Container; N] {
        self.columns_weighted([1; N])
    }

    /// Side-by-side columns with relative widths: `[2, 1]` makes the first
    /// column twice as wide as the second.
    ///
    /// On narrow screens the columns stack vertically.
    ///
    /// ```no_run
    /// # use dappr::prelude::*;
    /// # let dapp = Dapp::new();
    /// let [main, aside] = dapp.columns_weighted([3, 1]);
    /// main.h3_static("Wide");
    /// aside.h3_static("Narrow");
    /// ```
    pub fn columns_weighted<const N: usize>(&self, weights: [u32; N]) -> [Container; N] {
        let row = self.child_container("div");
        row.add_class(crate::DAPPR_STYLE::dappr_columns);
        let tracks: Vec<String> = weights
            .iter()
            .map(|weight| format!("minmax(0, {}fr)", (*weight).max(1)))
            .collect();
        row.get_element()
            .style()
            .set_property("--dappr-columns", &tracks.join(" "))
            .expect("Failed to set the column widths");

        std::array::from_fn(|_| {
            let column = row.child_container("div");
            column.add_class(crate::DAPPR_STYLE::dappr_column);
            column
        })
    }

    /// A card: a bordered, padded box (an `<article>`).
    ///
    /// ```no_run
    /// # use dappr::prelude::*;
    /// # let dapp = Dapp::new();
    /// let card = dapp.card();
    /// card.h4_static("Revenue");
    /// card.p_static("1.2M");
    /// ```
    pub fn card(&self) -> Container {
        self.child_container("article")
    }

    /// A [`card`](Dapp::card) with a title bar.
    pub fn card_titled(&self, title: &str) -> Container {
        let card = self.card();
        let header = create_element("header");
        header.set_text_content(Some(title));
        card.append(&header);
        card
    }

    /// A collapsible section (`<details>`): only `label` shows until the
    /// user clicks it. It starts closed; see
    /// [`expander_open`](Dapp::expander_open).
    ///
    /// ```no_run
    /// # use dappr::prelude::*;
    /// # let dapp = Dapp::new();
    /// let help = dapp.expander("How is this computed?");
    /// help.p_static("Shown only when expanded.");
    /// ```
    pub fn expander(&self, label: &str) -> Container {
        let details = self.child_container("details");
        let summary = create_element("summary");
        summary.set_text_content(Some(label));
        details.append(&summary);
        details
    }

    /// An [`expander`](Dapp::expander) that starts expanded.
    pub fn expander_open(&self, label: &str) -> Container {
        let details = self.expander(label);
        details.set_attribute("open", "");
        details
    }

    /// The app's sidebar: a panel on the left of the page, the usual home for
    /// the inputs that drive what the main area shows. On narrow screens it
    /// sits above the page instead.
    ///
    /// There is one sidebar per app. The first call creates it; every call,
    /// from the page or from any container, returns the same one.
    ///
    /// ```no_run
    /// # use dappr::prelude::*;
    /// # let dapp = Dapp::new();
    /// let sidebar = dapp.sidebar();
    /// sidebar.h4_static("Settings");
    /// let city = sidebar.input("City");
    /// dapp.text("Weather in {}", city.text(), None);
    /// ```
    pub fn sidebar(&self) -> Container {
        if let Some(existing) = document().get_element_by_id(SIDEBAR_ID) {
            return Container::new(existing.unchecked_into());
        }

        // The sidebar goes right before the app's root element, and their
        // shared parent (normally <body>) becomes the two-column layout.
        let app_root = crate::app::DAPP_INSTANCE.with(|dapp| dapp.borrow().root.clone());
        let parent = app_root
            .parent_element()
            .expect("The dapp root should be attached to the page");
        let sidebar = create_element("aside");
        sidebar.set_id(SIDEBAR_ID);
        sidebar.set_class_name(crate::DAPPR_STYLE::dappr_sidebar);
        parent
            .insert_before(&sidebar, Some(&app_root))
            .expect("Failed to add the sidebar to the page");
        parent
            .class_list()
            .add_1(crate::DAPPR_STYLE::dappr_has_sidebar)
            .expect("Failed to mark the page as having a sidebar");
        Container::new(sidebar)
    }
}
