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

    /// Like [`Subscription::new`], for a detach action that *owns* what the
    /// subscription keeps alive: the `wasm_bindgen` `Closure` behind a DOM
    /// event listener, or the handle to a `Signal`.
    ///
    /// For such a subscription, [`Subscription::detach`] leaks the action
    /// instead of dropping it, because "for the remainder of the program"
    /// has to cover what it owns too. Dropping a listener's `Closure` would
    /// make every later event fail with "closure invoked recursively or
    /// after being dropped"; dropping a Signal's handle could free a derived
    /// Signal that nothing else holds, silently ending the subscription.
    pub(crate) fn new_owning(detach: impl FnOnce() + 'static) -> Self {
        Subscription {
            detach: Some(Box::new(detach)),
            owns_resource: true,
        }
    }

    /// Keeps this subscription alive for the remainder of the program,
    /// together with whatever it is subscribed to.
    ///
    /// **NOTE**: this is a deliberate, permanent leak. Prefer holding onto
    /// the `Subscription`, or giving it to something with a lifetime of its
    /// own: `Signal::keep` for a Signal fed by another, or
    /// `dom::bind` for anything that should last while an element is on
    /// the page.
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
