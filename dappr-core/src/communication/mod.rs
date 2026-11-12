//! This module contains the communication logic of the Dappr framework.

/// The Dispatcher struct is used to dispatch events to the components.
pub mod dispatcher;

/// The Signal struct is used to send signals (reactive values) to the components.
pub mod signals;

/// The `Subscription` RAII handle returned by `Signal::subscribe` and
/// `Component::add_event_listener`. Drop it to detach, or call `.detach()`
/// to keep it attached forever.
pub mod subscription;
