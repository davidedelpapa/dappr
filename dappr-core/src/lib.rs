pub mod charts;
pub mod communication;
pub mod components;
pub mod data;
pub mod framework;

// Thi prelude is needed for doc tests
#[doc(hidden)]
#[allow(unused_imports)]
pub mod prelude {
    pub use crate::charts::{
        format_compact, format_value, parse_date, parse_date_with, ChartKind, ChartSpec,
    };
    pub use crate::communication::dispatcher::Dispatcher;
    pub use crate::communication::signals::{Signal, WeakSignal};
    pub use crate::communication::subscription::Subscription;
    pub use crate::components::button::*;
    pub use crate::components::chart::Chart;
    pub use crate::components::drawing::{Canvas, Svg};
    pub use crate::components::field::{Field, UploadedFile};
    pub use crate::components::html::*;
    pub use crate::components::input::*;
    #[cfg(feature = "latex")]
    pub use crate::components::markdown::Latex;
    #[cfg(feature = "markdown")]
    pub use crate::components::markdown::Markdown;
    pub use crate::components::metric::Metric;
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
