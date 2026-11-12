pub mod communication;
pub mod components;
pub mod framework;

// Thi prelude is needed for doc tests
#[doc(hidden)]
#[allow(unused_imports)]
pub mod prelude {
    pub use crate::communication::dispatcher::Dispatcher;
    pub use crate::communication::signals::Signal;
    pub use crate::communication::subscription::Subscription;
    pub use crate::components::button::*;
    pub use crate::components::field::Field;
    pub use crate::components::html::*;
    pub use crate::components::input::*;
    pub use crate::components::text::*;
    pub use crate::framework::dom::*;
    pub use crate::framework::traits::*;
    pub use crate::*;
}
