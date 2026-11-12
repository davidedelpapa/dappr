use crate::communication::subscription::Subscription;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

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
    callbacks: Rc<RefCell<Vec<(u64, Box<dyn FnMut(&T)>)>>>,
    next_id: Rc<RefCell<u64>>,
}

impl<T: Clone + 'static> Signal<T> {
    /// Signal constructor
    pub fn new(initial: T) -> Self {
        Signal {
            value: Rc::new(RefCell::new(initial)),
            callbacks: Rc::new(RefCell::new(vec![])),
            next_id: Rc::new(RefCell::new(0)),
        }
    }

    /// Set the value of the Signal. This will trigger all the subscribed callbacks.
    pub fn set(&self, new_value: T) {
        *self.value.borrow_mut() = new_value.clone();
        for (_, callback) in self.callbacks.borrow_mut().iter_mut() {
            callback(&new_value);
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
        self.callbacks.borrow_mut().push((id, Box::new(callback)));

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
}
