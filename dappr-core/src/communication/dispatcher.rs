use crate::communication::subscription::Subscription;
use std::{cell::RefCell, rc::Rc};

type Callback = Rc<RefCell<Box<dyn FnMut()>>>;

/// Dispatcher: subscribe to and dispatch events.
#[derive(Clone)]
pub struct Dispatcher {
    callbacks: Rc<RefCell<Vec<(u64, Callback)>>>,
    next_id: Rc<RefCell<u64>>,
}

impl Default for Dispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl Dispatcher {
    /// Dispatcher constructor
    pub fn new() -> Self {
        Dispatcher {
            callbacks: Rc::new(RefCell::new(vec![])),
            next_id: Rc::new(RefCell::new(0)),
        }
    }
    /// Dispatch all the subscribed events.
    ///
    /// Callbacks are free to use this same Dispatcher: they can `subscribe`
    /// to it, drop a [`Subscription`] to it (their own included), or
    /// `dispatch` again. The rules are the same as for
    /// [`Signal::set`](crate::communication::signals::Signal::set): a
    /// callback added during a dispatch first runs on the next one, a
    /// callback removed during a dispatch does not run again, and a nested
    /// `dispatch` skips the callback that is still running.
    pub fn dispatch(&self) {
        let snapshot: Vec<(u64, Callback)> = self.callbacks.borrow().clone();
        for (id, callback) in snapshot {
            let still_subscribed = self
                .callbacks
                .borrow()
                .iter()
                .any(|(cb_id, _)| *cb_id == id);
            if !still_subscribed {
                continue;
            }
            if let Ok(mut callback) = callback.try_borrow_mut() {
                callback();
            };
        }
    }

    /// Subscribe a new event (a callback).
    ///
    /// Returns a [`Subscription`]: drop it to unsubscribe, or call
    /// `.detach()` to keep it subscribed forever.
    #[must_use = "dropping the returned Subscription immediately unsubscribes this callback -- \
                  see Subscription's docs"]
    pub fn subscribe(&self, callback: impl FnMut() + 'static) -> Subscription {
        let id = {
            let mut next_id = self.next_id.borrow_mut();
            let id = *next_id;
            *next_id += 1;
            id
        };
        let callback: Box<dyn FnMut()> = Box::new(callback);
        self.callbacks
            .borrow_mut()
            .push((id, Rc::new(RefCell::new(callback))));

        let callbacks = Rc::clone(&self.callbacks);
        Subscription::new(move || {
            callbacks.borrow_mut().retain(|(cb_id, _)| *cb_id != id);
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::communication::signals::Signal;

    use super::*;

    #[test]
    fn test_dispatch_single_callback() {
        let dispatcher = Dispatcher::new();
        let called = Signal::new(false);

        let called_cloned = called.clone();
        dispatcher
            .subscribe(move || {
                called_cloned.set(true);
            })
            .detach();

        dispatcher.dispatch();

        assert!(called.get(), "The callback should have been invoked.");
    }

    #[test]
    fn test_dispatch_multiple_callbacks() {
        let dispatcher = Dispatcher::new();
        let call_count = Signal::new(0usize);

        let call_count_cloned = call_count.clone();
        dispatcher
            .subscribe(move || {
                let count = call_count_cloned.get();
                call_count_cloned.set(count + 1);
            })
            .detach();

        dispatcher.dispatch();
        dispatcher.dispatch();

        assert_eq!(call_count.get(), 2);
    }

    #[test]
    fn test_dropping_subscription_stops_dispatch() {
        let dispatcher = Dispatcher::new();
        let call_count = Signal::new(0usize);

        let call_count_cloned = call_count.clone();
        let sub = dispatcher.subscribe(move || {
            let count = call_count_cloned.get();
            call_count_cloned.set(count + 1);
        });

        dispatcher.dispatch();
        drop(sub);
        dispatcher.dispatch();

        assert_eq!(
            call_count.get(),
            1,
            "callback should not fire after its Subscription is dropped"
        );
    }

    /// A callback may subscribe to, and unsubscribe from, the Dispatcher
    /// that is currently calling it (a button whose handler adds another
    /// handler, or removes itself).
    #[test]
    fn test_callback_can_subscribe_and_unsubscribe_during_dispatch() {
        let dispatcher = Dispatcher::new();
        let added_calls = Signal::new(0usize);
        let own: Rc<RefCell<Option<Subscription>>> = Rc::new(RefCell::new(None));

        let (dispatcher_c, added_c, own_c) =
            (dispatcher.clone(), added_calls.clone(), Rc::clone(&own));
        let sub = dispatcher.subscribe(move || {
            own_c.borrow_mut().take(); // unsubscribe itself
            let added_c = added_c.clone();
            dispatcher_c
                .subscribe(move || added_c.set(added_c.get() + 1))
                .detach();
        });
        *own.borrow_mut() = Some(sub);

        dispatcher.dispatch(); // adds one callback, removes the first
        assert_eq!(added_calls.get(), 0);
        dispatcher.dispatch(); // only the added one runs
        assert_eq!(added_calls.get(), 1);
    }
}
