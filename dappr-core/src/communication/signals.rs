use crate::communication::subscription::Subscription;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

type Callback<T> = Rc<RefCell<Box<dyn FnMut(&T)>>>;
type Callbacks<T> = Rc<RefCell<Vec<(u64, Callback<T>)>>>;

/// The Signal struct provides reacivity to the Components.
///
/// The difference between the Signal and the Dispatcher is that the Signal
/// is a wrapper around a value that can be updated and subscribed to,
/// while the Dispatcher is a wrapper around a list of callbacks that can be dispatched.
///
/// The Signal is used to wrap around a type and update and sync all components with the new value.
#[derive(Clone)]
pub struct Signal<T> {
    value: Rc<RefCell<T>>,
    callbacks: Callbacks<T>,
    next_id: Rc<RefCell<u64>>,
    /// Bumped by every `set`, so a `set` can tell that a newer one happened
    /// while it was still notifying.
    version: Rc<Cell<u64>>,
}

impl<T: Clone + 'static> Signal<T> {
    /// Signal constructor
    pub fn new(initial: T) -> Self {
        Signal {
            value: Rc::new(RefCell::new(initial)),
            callbacks: Rc::new(RefCell::new(vec![])),
            next_id: Rc::new(RefCell::new(0)),
            version: Rc::new(Cell::new(0)),
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
        *self.value.borrow_mut() = new_value.clone();
        let version = self.version.get().wrapping_add(1);
        self.version.set(version);

        // Work on a snapshot, with no borrow of the list held while user
        // code runs, so callbacks can subscribe/unsubscribe freely.
        let snapshot: Vec<(u64, Callback<T>)> = self.callbacks.borrow().clone();
        for (id, callback) in snapshot {
            if self.version.get() != version {
                // A callback called `set` again: that newer `set` has
                // already notified everyone with the newer value.
                break;
            }
            let still_subscribed = self
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
        self.value.borrow().clone()
    }

    /// Subscribe a new callback to the Signal.
    ///
    /// Returns a [`Subscription`]: drop it to unsubscribe this callback, or
    /// call `.detach()` to keep it subscribed for the remainder of the program
    #[must_use = "dropping the returned Subscription immediately unsubscribes this callback -- \
                  see Subscription's docs"]
    pub fn subscribe<F>(&self, callback: F) -> Subscription
    where
        F: FnMut(&T) + 'static,
    {
        let id = {
            let mut next_id = self.next_id.borrow_mut();
            let id = *next_id;
            *next_id += 1;
            id
        };
        let callback: Box<dyn FnMut(&T)> = Box::new(callback);
        self.callbacks
            .borrow_mut()
            .push((id, Rc::new(RefCell::new(callback))));

        let callbacks = Rc::clone(&self.callbacks);
        Subscription::new(move || {
            callbacks.borrow_mut().retain(|(cb_id, _)| *cb_id != id);
        })
    }

    /// The number of currently-live subscriptions. Mostly useful for tests
    /// asserting that `Subscription`s actually detach.
    #[cfg(test)]
    pub(crate) fn subscriber_count(&self) -> usize {
        self.callbacks.borrow().len()
    }

    /// Update the Signal after a delay in milliseconds
    pub fn update_after_delay(&self, new_value: T, delay_ms: i32) {
        let signal = self.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let closure = Closure::wrap(Box::new(move || {
                signal.set(new_value.clone());
            }) as Box<dyn FnMut()>);

            web_sys::window()
                .expect("Could not find the window")
                .set_timeout_with_callback_and_timeout_and_arguments_0(
                    closure.as_ref().unchecked_ref(),
                    delay_ms,
                )
                .expect("Failed to set timeout");

            // Prevent the closure from being dropped prematurely
            closure.forget();
        });
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
}
