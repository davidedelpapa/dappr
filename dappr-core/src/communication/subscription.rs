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
    /// Whether the detach action owns something that must outlive a call to
    /// [`Subscription::detach`] (see [`Subscription::new_owning`]).
    owns_resource: bool,
}

impl Subscription {
    /// Wraps a detach action. Internal constructor: components and `Signal`
    /// build a `Subscription` from whatever their own "remove me" logic is.
    pub(crate) fn new(detach: impl FnOnce() + 'static) -> Self {
        Subscription {
            detach: Some(Box::new(detach)),
            owns_resource: false,
        }
    }

    /// Like [`Subscription::new`], for a detach action that *owns* the thing
    /// being kept alive -- in practice the `wasm_bindgen` `Closure` behind a
    /// DOM event listener, which the browser can only call for as long as
    /// the Rust side has not dropped it.
    ///
    /// For such a subscription, [`Subscription::detach`] leaks the action
    /// instead of dropping it: dropping it would free the `Closure` while
    /// the browser still holds the listener, and every later event would
    /// fail with "closure invoked recursively or after being dropped".
    pub(crate) fn new_owning(detach: impl FnOnce() + 'static) -> Self {
        Subscription {
            detach: Some(Box::new(detach)),
            owns_resource: true,
        }
    }

    /// Keeps this subscription alive for the remainder of the program.
    ///
    /// **NOTE**: Prefer holding onto the `Subscription` instead of using
    /// this method, when the thing it's attached to has its own,
    /// shorter lifetime.
    pub fn detach(mut self) {
        let detach = self.detach.take();
        if self.owns_resource {
            // Deliberate leak: "alive for the remainder of the program".
            std::mem::forget(detach);
        }
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        if let Some(detach) = self.detach.take() {
            detach();
        }
    }
}
