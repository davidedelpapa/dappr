//! RAII handle for anything that needs an explicit "detach" step: a DOM
//! event listener, or a [`Signal`](crate::communication::signals::Signal)
//! subscription.

/// A handle to a live subscription/listener.
///
/// - **Drop it** to detach immediately -- useful for anything with a
///   shorter lifetime than the whole app (e.g. a component you create and
///   later remove).
/// - **Call [`Subscription::detach`]** to keep it alive for the remainder of
///   the program.
///
/// ## Usage
///
/// To use the event listener we must either keep it alive for as long as we need it:
///
/// ```ignore
/// let _click_sub = dapp.add_event_listener("click", |_| { /* ... */ });
/// // `_click_sub` (note: NOT a bare `_`) stays alive for the rest of this scope.
/// ```
///
/// or explicitly make it iton detached mode (a "forever" behavior):
///
/// ```ignore
/// dapp.add_event_listener("click", |_| { /* ... */ }).detach();
/// ```
#[must_use = "dropping this immediately detaches the listener/subscription -- \
              bind it to a named variable that outlives what it should stay attached to \
              (e.g. `let _sub = ...`), or call `.detach()` to keep it forever"]
pub struct Subscription {
    detach: Option<Box<dyn FnOnce()>>,
}

impl Subscription {
    /// Wraps a detach action. Internal constructor: components and `Signal`
    /// build a `Subscription` from whatever their own "remove me" logic is.
    pub(crate) fn new(detach: impl FnOnce() + 'static) -> Self {
        Subscription {
            detach: Some(Box::new(detach)),
        }
    }

    /// Keeps this subscription alive for the remainder of the program.
    ///
    /// **NOTE**: Prefer holding onto the `Subscription` instead of using
    /// this method, when the thing it's attached to has its own,
    /// shorter lifetime.
    pub fn detach(mut self) {
        self.detach.take();
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        if let Some(detach) = self.detach.take() {
            detach();
        }
    }
}
