use crate::communication::subscription::Subscription;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

type Callback<T> = Rc<RefCell<Box<dyn FnMut(&T)>>>;

/// What every handle to one Signal shares.
struct Inner<T> {
    value: RefCell<T>,
    callbacks: RefCell<Vec<(u64, Callback<T>)>>,
    next_id: Cell<u64>,
    /// Bumped by every `set`, so a `set` can tell that a newer one happened
    /// while it was still notifying.
    version: Cell<u64>,
    /// Subscriptions this Signal owns: for a derived Signal, the ones that
    /// feed it from its sources. They end when the Signal does.
    kept: RefCell<Vec<Subscription>>,
}

/// The Signal struct provides reacivity to the Components.
///
/// The difference between the Signal and the Dispatcher is that the Signal
/// is a wrapper around a value that can be updated and subscribed to,
/// while the Dispatcher is a wrapper around a list of callbacks that can be dispatched.
///
/// The Signal is used to wrap around a type and update and sync all components with the new value.
///
/// ## How long a Signal lives
///
/// Cloning a Signal gives another handle to the same value. The Signal
/// lasts while at least one handle exists, or while something is subscribed
/// to it (a [`Subscription`] counts as a handle).
///
/// A Signal derived from others ([`map`](Signal::map), [`zip`](Signal::zip),
/// `computed!`) keeps its sources alive, never the other way round. So when
/// the last thing using a derived Signal goes away -- a component removed
/// from the page, a handle dropped -- the derived Signal stops listening to
/// its sources and is freed, and nothing is left behind on them.
pub struct Signal<T> {
    inner: Rc<Inner<T>>,
}

impl<T> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Signal {
            inner: Rc::clone(&self.inner),
        }
    }
}

/// A handle to a Signal that does not keep it alive. Get one with
/// [`Signal::downgrade`]; turn it back into a Signal with
/// [`upgrade`](WeakSignal::upgrade), which gives `None` once the Signal is
/// gone.
///
/// This is how a callback on one Signal can update another without tying
/// their lifetimes together.
pub struct WeakSignal<T> {
    inner: Weak<Inner<T>>,
}

impl<T> Clone for WeakSignal<T> {
    fn clone(&self) -> Self {
        WeakSignal {
            inner: Weak::clone(&self.inner),
        }
    }
}

impl<T> WeakSignal<T> {
    /// The Signal, if it still exists.
    pub fn upgrade(&self) -> Option<Signal<T>> {
        self.inner.upgrade().map(|inner| Signal { inner })
    }
}

impl<T: Clone + 'static> Signal<T> {
    /// Signal constructor
    pub fn new(initial: T) -> Self {
        Signal {
            inner: Rc::new(Inner {
                value: RefCell::new(initial),
                callbacks: RefCell::new(Vec::new()),
                next_id: Cell::new(0),
                version: Cell::new(0),
                kept: RefCell::new(Vec::new()),
            }),
        }
    }

    /// Set the value of the Signal. This will trigger all the subscribed callbacks.
    ///
    /// Callbacks are free to use this same Signal: they can `get` it,
    /// `subscribe` to it, drop a [`Subscription`] to it (their own included),
    /// or `set` it again.
    ///
    /// - A callback subscribed while a `set` is running is first called on
    ///   the next `set`.
    /// - A callback unsubscribed while a `set` is running is not called
    ///   again, not even later in that same `set`.
    /// - If a callback calls `set` on this Signal, the nested `set` notifies
    ///   every *other* callback, but not the one that is still running (it
    ///   can read the new value with `get`). This is what stops a callback
    ///   that writes back to its own Signal from recursing forever. The
    ///   outer `set` then stops notifying, so no callback is handed the
    ///   older value after the newer one.
    pub fn set(&self, new_value: T) {
        let inner = &self.inner;
        *inner.value.borrow_mut() = new_value.clone();
        let version = inner.version.get().wrapping_add(1);
        inner.version.set(version);

        // Work on a snapshot, with no borrow of the list held while user
        // code runs, so callbacks can subscribe/unsubscribe freely.
        let snapshot: Vec<(u64, Callback<T>)> = inner.callbacks.borrow().clone();
        for (id, callback) in snapshot {
            if inner.version.get() != version {
                // A callback called `set` again: that newer `set` has
                // already notified everyone with the newer value.
                break;
            }
            let still_subscribed = inner
                .callbacks
                .borrow()
                .iter()
                .any(|(cb_id, _)| *cb_id == id);
            if !still_subscribed {
                continue;
            }
            // `try_borrow_mut` fails only if this very callback is already
            // running further up the stack (a nested `set`): skip it.
            if let Ok(mut callback) = callback.try_borrow_mut() {
                callback(&new_value);
            };
        }
    }

    /// Get the current value of the Signal
    pub fn get(&self) -> T {
        self.inner.value.borrow().clone()
    }

    /// Subscribe a new callback to the Signal.
    ///
    /// Returns a [`Subscription`]: drop it to unsubscribe this callback, or
    /// call `.detach()` to keep it subscribed for the remainder of the program.
    ///
    /// While the Subscription exists, so does the Signal -- which matters
    /// for a derived Signal nobody else holds:
    /// `total.map(format).subscribe(show)` keeps working for as long as the
    /// returned Subscription is kept.
    #[must_use = "dropping the returned Subscription immediately unsubscribes this callback -- \
                  see Subscription's docs"]
    pub fn subscribe<F>(&self, callback: F) -> Subscription
    where
        F: FnMut(&T) + 'static,
    {
        let inner = &self.inner;
        let id = inner.next_id.get();
        inner.next_id.set(id + 1);
        let callback: Box<dyn FnMut(&T)> = Box::new(callback);
        inner
            .callbacks
            .borrow_mut()
            .push((id, Rc::new(RefCell::new(callback))));

        // The Subscription holds the Signal (a strong handle), so the
        // Signal cannot disappear from under a live subscription.
        let signal = self.clone();
        Subscription::new_owning(move || {
            // Take the callback out before dropping it: dropping may run
            // code (the callback's own captures) that touches this list.
            let removed: Vec<(u64, Callback<T>)> = {
                let mut callbacks = signal.inner.callbacks.borrow_mut();
                let (gone, kept) = callbacks.drain(..).partition(|(cb_id, _)| *cb_id == id);
                *callbacks = kept;
                gone
            };
            drop(removed);
        })
    }

    /// A handle that does not keep this Signal alive. See [`WeakSignal`].
    pub fn downgrade(&self) -> WeakSignal<T> {
        WeakSignal {
            inner: Rc::downgrade(&self.inner),
        }
    }

    /// Makes `subscription` last exactly as long as this Signal: it ends
    /// when the last handle to the Signal is dropped.
    ///
    /// This is the building block for a Signal that is fed by another. The
    /// subscription on the source is kept *by the derived Signal*, and its
    /// callback reaches the derived Signal through a [`WeakSignal`], so the
    /// source never keeps the derived Signal alive:
    ///
    /// ```
    /// # use dappr_core::prelude::*;
    /// let celsius = Signal::new(20.0);
    /// let fahrenheit = Signal::new(68.0);
    /// let target = fahrenheit.downgrade();
    /// fahrenheit.keep(celsius.subscribe(move |c| {
    ///     if let Some(fahrenheit) = target.upgrade() {
    ///         fahrenheit.set(c * 9.0 / 5.0 + 32.0);
    ///     }
    /// }));
    /// celsius.set(100.0);
    /// assert_eq!(fahrenheit.get(), 212.0);
    /// ```
    ///
    /// ([`map`](Signal::map) does exactly this for you.)
    pub fn keep(&self, subscription: Subscription) {
        self.inner.kept.borrow_mut().push(subscription);
    }

    /// A new Signal whose value is always `f` applied to this one's.
    ///
    /// This is how you compute things from inputs:
    ///
    /// ```ignore
    /// let celsius = dapp.number("Celsius", 20.0);
    /// let fahrenheit = celsius.value().map(|c| c * 9.0 / 5.0 + 32.0);
    /// text!("That is {} F", fahrenheit);
    /// ```
    ///
    /// The derived Signal follows this one for as long as the derived
    /// Signal is in use. When nothing holds or subscribes to it any more,
    /// it stops listening and is freed.
    pub fn map<U, F>(&self, f: F) -> Signal<U>
    where
        U: Clone + 'static,
        F: Fn(&T) -> U + 'static,
    {
        let derived = Signal::new(f(&self.get()));
        let target = derived.downgrade();
        derived.keep(self.subscribe(move |value| {
            if let Some(derived) = target.upgrade() {
                derived.set(f(value));
            }
        }));
        derived
    }

    /// A new Signal holding this one's value paired with `other`'s, updated
    /// when either changes. Combine it with [`map`](Signal::map) to compute
    /// from two inputs (and nest `zip` for more):
    ///
    /// ```ignore
    /// let total = price.value().zip(&quantity.value()).map(|(p, q)| p * q);
    /// ```
    pub fn zip<U: Clone + 'static>(&self, other: &Signal<U>) -> Signal<(T, U)> {
        let derived = Signal::new((self.get(), other.get()));

        let (target, other_in) = (derived.downgrade(), other.clone());
        derived.keep(self.subscribe(move |value| {
            if let Some(derived) = target.upgrade() {
                derived.set((value.clone(), other_in.get()));
            }
        }));
        let (target, self_in) = (derived.downgrade(), self.clone());
        derived.keep(other.subscribe(move |value| {
            if let Some(derived) = target.upgrade() {
                derived.set((self_in.get(), value.clone()));
            }
        }));
        derived
    }

    /// The number of callbacks currently subscribed. Useful in tests, to
    /// check that something stopped listening.
    pub fn subscriber_count(&self) -> usize {
        self.inner.callbacks.borrow().len()
    }

    /// Update the Signal after a delay in milliseconds
    pub fn update_after_delay(&self, new_value: T, delay_ms: i32) {
        let signal = self.clone();
        // A one-shot closure: freed by the browser side once it has run.
        let closure = Closure::once_into_js(move || signal.set(new_value));
        web_sys::window()
            .expect("Could not find the window")
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                closure.unchecked_ref(),
                delay_ms,
            )
            .expect("Failed to set timeout");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_update() {
        let signal = Signal::new(String::from("initial"));
        assert_eq!(signal.get(), "initial");

        signal.set(String::from("updated"));
        assert_eq!(signal.get(), "updated");
    }

    #[test]
    fn test_signal_subscription() {
        let signal = Signal::new(0usize);
        let value = Rc::new(RefCell::new(0usize));

        let value_clone = Rc::clone(&value);
        signal
            .subscribe(move |v: &usize| {
                *value_clone.borrow_mut() = *v;
            })
            .detach();

        signal.set(42);
        assert_eq!(*value.borrow(), 42);
    }

    /// Dropping the returned `Subscription` should actually remove the callback
    #[test]
    fn test_dropping_subscription_unsubscribes() {
        let signal = Signal::new(0);
        let seen = Rc::new(RefCell::new(Vec::new()));
        let seen_clone = Rc::clone(&seen);

        {
            let _sub = signal.subscribe(move |v| seen_clone.borrow_mut().push(*v));
            signal.set(1);
            assert_eq!(signal.subscriber_count(), 1);
        } // `_sub` dropped here

        assert_eq!(signal.subscriber_count(), 0);
        signal.set(2); // must not be observed
        assert_eq!(*seen.borrow(), vec![1]);
    }

    /// Unsubscribing one callback must not disturb any other, independently
    /// of subscribe/unsubscribe order
    #[test]
    fn test_unsubscribe_is_independent_per_callback() {
        let signal = Signal::new(0);
        let count_a = Rc::new(RefCell::new(0));
        let count_b = Rc::new(RefCell::new(0));
        let (ca, cb) = (Rc::clone(&count_a), Rc::clone(&count_b));

        let sub_a = signal.subscribe(move |_| *ca.borrow_mut() += 1);
        let _sub_b = signal.subscribe(move |_| *cb.borrow_mut() += 1);

        signal.set(1);
        drop(sub_a);
        signal.set(2);

        assert_eq!(*count_a.borrow(), 1);
        assert_eq!(*count_b.borrow(), 2);
    }

    /// A callback may drop its own Subscription while it is running.
    #[test]
    fn test_callback_can_unsubscribe_itself() {
        let signal = Signal::new(0);
        let calls = Rc::new(RefCell::new(0));
        let slot: Rc<RefCell<Option<Subscription>>> = Rc::new(RefCell::new(None));

        let (calls_c, slot_c) = (Rc::clone(&calls), Rc::clone(&slot));
        let sub = signal.subscribe(move |_| {
            *calls_c.borrow_mut() += 1;
            slot_c.borrow_mut().take(); // drops this callback's own Subscription
        });
        *slot.borrow_mut() = Some(sub);

        signal.set(1);
        signal.set(2);
        assert_eq!(*calls.borrow(), 1);
        assert_eq!(signal.subscriber_count(), 0);
    }

    /// A callback may subscribe a new callback; it starts on the next `set`.
    /// One unsubscribed mid-`set` is not called later in that same `set`.
    #[test]
    fn test_callback_can_subscribe_and_unsubscribe_others() {
        let signal = Signal::new(0);
        let late_calls = Rc::new(RefCell::new(Vec::new()));
        let victim_calls = Rc::new(RefCell::new(0));
        let victim: Rc<RefCell<Option<Subscription>>> = Rc::new(RefCell::new(None));

        let (signal_c, late_c, victim_c) =
            (signal.clone(), Rc::clone(&late_calls), Rc::clone(&victim));
        signal
            .subscribe(move |v| {
                victim_c.borrow_mut().take();
                if *v == 1 {
                    let late_c = Rc::clone(&late_c);
                    signal_c
                        .subscribe(move |v| late_c.borrow_mut().push(*v))
                        .detach();
                }
            })
            .detach();
        let victim_calls_c = Rc::clone(&victim_calls);
        *victim.borrow_mut() = Some(signal.subscribe(move |_| *victim_calls_c.borrow_mut() += 1));

        signal.set(1);
        signal.set(2);
        assert_eq!(*late_calls.borrow(), vec![2]);
        assert_eq!(*victim_calls.borrow(), 0);
    }

    /// A callback that writes back to its own Signal does not recurse, and
    /// the other subscribers see the value it wrote.
    #[test]
    fn test_callback_can_set_its_own_signal() {
        let signal = Signal::new(0);
        let seen = Rc::new(RefCell::new(Vec::new()));

        let signal_c = signal.clone();
        signal
            .subscribe(move |v| {
                if *v > 10 {
                    signal_c.set(10); // clamp
                }
            })
            .detach();
        let seen_c = Rc::clone(&seen);
        signal
            .subscribe(move |v| seen_c.borrow_mut().push(*v))
            .detach();

        signal.set(50);
        assert_eq!(signal.get(), 10);
        assert_eq!(seen.borrow().last(), Some(&10));
    }

    #[test]
    fn test_map_follows_its_source() {
        let celsius = Signal::new(0.0);
        let fahrenheit = celsius.map(|c| c * 9.0 / 5.0 + 32.0);
        assert_eq!(fahrenheit.get(), 32.0);
        celsius.set(100.0);
        assert_eq!(fahrenheit.get(), 212.0);
    }

    #[test]
    fn test_zip_updates_when_either_side_changes() {
        let price = Signal::new(2.5);
        let quantity = Signal::new(4u32);
        let total = price.zip(&quantity).map(|(p, q)| p * f64::from(*q));
        let seen = Rc::new(RefCell::new(Vec::new()));
        let seen_c = Rc::clone(&seen);
        total
            .subscribe(move |t| seen_c.borrow_mut().push(*t))
            .detach();

        assert_eq!(total.get(), 10.0);
        quantity.set(10);
        price.set(1.0);
        assert_eq!(*seen.borrow(), vec![25.0, 10.0]);
    }

    // ---- Lifetimes ----

    /// A derived Signal nobody uses any more leaves nothing on its source.
    #[test]
    fn test_dropping_a_derived_signal_unsubscribes_from_its_source() {
        let source = Signal::new(1);
        assert_eq!(source.subscriber_count(), 0);
        let doubled = source.map(|v| v * 2);
        assert_eq!(source.subscriber_count(), 1);
        // Another handle keeps it going.
        let handle = doubled.clone();
        drop(doubled);
        source.set(5);
        assert_eq!(handle.get(), 10);
        assert_eq!(source.subscriber_count(), 1);
        drop(handle);
        assert_eq!(source.subscriber_count(), 0);
        source.set(6); // nothing left to notify, and nothing breaks
    }

    /// The scenario that used to leak: deriving again and again from a
    /// long-lived Signal, as a callback that rebuilds part of a page does.
    #[test]
    fn test_deriving_repeatedly_does_not_accumulate() {
        let source = Signal::new(0);
        let other = Signal::new(0);
        for round in 0..100 {
            let mapped = source.map(|v| v + 1);
            let zipped = source.zip(&other).map(|(a, b)| a + b);
            source.set(round);
            assert_eq!(mapped.get(), round + 1);
            assert_eq!(zipped.get(), round);
        }
        assert_eq!(source.subscriber_count(), 0);
        assert_eq!(other.subscriber_count(), 0);
    }

    /// A subscription keeps the whole chain behind it alive, and releases
    /// all of it when it ends.
    #[test]
    fn test_a_subscription_keeps_a_derived_chain_alive() {
        let source = Signal::new(1);
        let seen = Rc::new(RefCell::new(Vec::new()));
        let seen_c = Rc::clone(&seen);
        // No handle to either derived Signal is kept: only the Subscription.
        let subscription = source
            .map(|v| v * 10)
            .map(|v| v + 1)
            .subscribe(move |v| seen_c.borrow_mut().push(*v));
        source.set(2);
        source.set(3);
        assert_eq!(*seen.borrow(), vec![21, 31]);
        assert_eq!(source.subscriber_count(), 1);

        drop(subscription);
        assert_eq!(source.subscriber_count(), 0);
        source.set(4);
        assert_eq!(*seen.borrow(), vec![21, 31]);
    }

    /// `.detach()` means "for the rest of the program", derived chain included.
    #[test]
    fn test_detach_keeps_a_derived_chain_for_good() {
        let source = Signal::new(1);
        let seen = Rc::new(RefCell::new(Vec::new()));
        let seen_c = Rc::clone(&seen);
        source
            .map(|v| v * 10)
            .subscribe(move |v| seen_c.borrow_mut().push(*v))
            .detach();
        source.set(2);
        assert_eq!(*seen.borrow(), vec![20]);
        assert_eq!(source.subscriber_count(), 1);
    }

    #[test]
    fn test_zip_releases_both_sources() {
        let (a, b) = (Signal::new(1), Signal::new(2));
        let pair = a.zip(&b);
        assert_eq!((a.subscriber_count(), b.subscriber_count()), (1, 1));
        b.set(5);
        assert_eq!(pair.get(), (1, 5));
        drop(pair);
        assert_eq!((a.subscriber_count(), b.subscriber_count()), (0, 0));
    }

    /// A source can go away first; what was derived from it keeps its last value.
    #[test]
    fn test_weak_signal_and_sources_dropped_first() {
        let source = Signal::new(3);
        let weak = source.downgrade();
        assert_eq!(weak.upgrade().map(|s| s.get()), Some(3));
        let doubled = source.map(|v| v * 2);
        drop(source);
        // The derived Signal still holds its source.
        assert!(weak.upgrade().is_some());
        assert_eq!(doubled.get(), 6);
        drop(doubled);
        assert!(weak.upgrade().is_none());
    }

    /// Ending a derived Signal from inside a callback of its own source.
    #[test]
    fn test_derived_signal_can_be_dropped_during_a_set() {
        let source = Signal::new(0);
        let slot: Rc<RefCell<Option<Signal<i32>>>> = Rc::new(RefCell::new(None));
        *slot.borrow_mut() = Some(source.map(|v| v + 1));
        let slot_c = Rc::clone(&slot);
        let _clear = source.subscribe(move |v| {
            if *v == 2 {
                slot_c.borrow_mut().take();
            }
        });
        source.set(1);
        assert_eq!(slot.borrow().as_ref().map(|s| s.get()), Some(2));
        source.set(2);
        assert!(slot.borrow().is_none());
        assert_eq!(source.subscriber_count(), 1); // only `_clear`
        source.set(3);
    }
}
