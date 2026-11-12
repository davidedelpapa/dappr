pub mod communication;
pub mod components;
pub mod data;
pub mod framework;

// Thi prelude is needed for doc tests
#[doc(hidden)]
#[allow(unused_imports)]
pub mod prelude {
    pub use crate::communication::dispatcher::Dispatcher;
    pub use crate::communication::signals::Signal;
    pub use crate::communication::subscription::Subscription;
    pub use crate::components::button::*;
    pub use crate::components::field::{Field, UploadedFile};
    pub use crate::components::html::*;
    pub use crate::components::input::*;
    pub use crate::components::status::LoadStatus;
    pub use crate::components::table::{Table, TableSource};
    pub use crate::components::text::*;
    #[cfg(feature = "csv")]
    pub use crate::data::csv::CsvOptions;
    pub use crate::data::fetch::{fetch_table, fetch_text, UrlSource};
    pub use crate::data::load::{parse_table, Loadable};
    pub use crate::data::table::{Cell, DColumn, DTable, Row};
    pub use crate::data::DataError;
    pub use crate::framework::dom::*;
    pub use crate::framework::traits::*;
    pub use crate::*;
}
