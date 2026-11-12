//! Dappr: Data Apps in Rust, made easy
//!
//! Dappr is a Rust wasm framework that aims to simplify the creation of data-centric apps.
//!
//! # Examples
//! The following is a hello-world example:
//! ```ignore
//! use dappr::prelude::*;
//! use wasm_bindgen::prelude::*;
//!
//! #[wasm_bindgen(start)]
//! pub fn run() {
//!     dapp!(dapp, {
//!         text!("Hello Dappr!", _title);
//!         text!("Data apps in Rust made easy.", _subtitle);
//!         let input = dapp.input("What's your name?");
//!         text!("Hello, {}!", input.text());
//!     });
//! }
//! ```
//! The cratemust be a library (`crate-type = ["cdylib", "rlib"]`) inside *Cargo.toml*.
//! To run it, add the following to your *index.html*:
//! ```html
//! <!doctype html>
//! <html lang="en">
//!     <head>
//!         <meta charset="UTF-8" />
//!         <meta name="viewport" content="width=device-width, initial-scale=1.0" />
//!         <title>Hello Dappr</title>
//!         <link data-trunk rel="rust" />
//!         <link data-trunk rel="css" href="static/styles.min.css" />
//!     </head>
//!     <body>
//!         <main id="dapp" class="container"></main>
//!     </body>
//! </html>
//! ```
//! Build the above with: `wasm-pack build --target web` and run it with `trunk serve`.
//!
//! More examples in the [examples](../examples/) directory
//!
//! # Info
//! For more information and lots of examples, start with:
//! - [`Dapp`](crate::app::Dapp) struct
//! - [`text!`](crate::text) macro
pub use dappr_core::*;
pub use dappr_macros::*;

/// The main Dappr app, the entry point for all the components
pub mod app;

/// Prelude: all the needed imports to run a data app
pub mod prelude {
    pub use crate::app::Dapp;
    #[doc(hidden)]
    pub use crate::app::DAPP_INSTANCE;
    pub use crate::dapp;
    pub use crate::text;

    pub use dappr_core::communication::dispatcher::Dispatcher;
    pub use dappr_core::communication::signals::Signal;
    pub use dappr_core::communication::subscription::Subscription; // <-- added
    pub use dappr_core::components::button::*;
    pub use dappr_core::components::html::*;
    pub use dappr_core::components::input::*;
    pub use dappr_core::components::text::*;
    pub use dappr_core::framework::traits::*;
    pub use dappr_core::*;
    pub use dappr_macros::*;
    #[doc(hidden)]
    pub use std::cell::RefCell;
    #[doc(hidden)]
    pub use std::rc::Rc;
}
// Default stle from Pico CSS ✨ v2.0.6 (https://picocss.com)
// See also dappr-macros/src/assets/vendor/pico.license.
sys_style_file!("PICO_STYLE", "pico.css");
