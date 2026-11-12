use crate::communication::subscription::Subscription;
use std::{cell::RefCell, rc::Rc};

/// Dispatcher: subscribe to and dispatch events.
#[derive(Clone)]
pub struct Dispatcher {
    callbacks: Rc<RefCell<Vec<(u64, Box<dyn FnMut()>)>>>,
    next_id: Rc<RefCell<u64>>,
}

impl Dispatcher {
    /// Dispatcher constructor
    pub fn new() -> Self {
        Dispatcher {
            callbacks: Rc::new(RefCell::new(vec![])),
            next_id: Rc::new(RefCell::new(0)),
        }
    }
    /// Dispatch all the subscribed events
    pub fn dispatch(&self) {
        for (_, callback) in self.callbacks.borrow_mut().iter_mut() {
            callback();
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
        self.callbacks.borrow_mut().push((id, Box::new(callback)));

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
}
