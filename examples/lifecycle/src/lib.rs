//! # Component lifecycle example
//!
//! This example exists to tie together, in one place, everything that
//! changed about styles and lifecycle recently:
//!
//! 1. **Styles**: `style!` below is a *pure* macro. It writes nothing to
//!    disk: it generates a `notification` module holding the scoped class
//!    name (`notification::chip == "notification-chip"`) and the matching
//!    scoped CSS (`notification::CSS`). `dapp.style(notification::CSS)`
//!    adds that CSS to the page. No `build.rs`, no stylesheet link in
//!    `index.html`; Dappr's baseline look (Pico) is added by Dappr itself.
//!
//! 2. **Building components after startup**: every previous example only
//!    ever creates components once, up front, inside the `dapp!` block.
//!    This one creates new components later, from inside a button's click
//!    handler -- by opening a new `dapp!` block inside the handler
//!    (see the comment on `post_button.pressed` below for why).
//!
//! 3. **Detaching components**: clicking a notification removes it, for
//!    real -- both the visible element (`Component::unmount`) and its own
//!    click listener (dropping the `Subscription` `add_event_listener`
//!    returns). Previously, listeners were `.forget()`'d unconditionally
//!    and could never be removed; this example is the one place in the
//!    whole crate that deliberately does NOT `.detach()` a `Subscription`,
//!    specifically to show the other half of that API.

use dappr::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

// Generates `notification::chip` (the class name) and `notification::CSS`
// (the CSS below, with `.chip` rewritten to `.notification-chip`).
style!(
    "notification",
    r#"
    .chip {
    display: inline-block;
    margin: 0.25rem 0.25rem 0.25rem 0;
    padding: 0.5rem 1rem;
    border-radius: 999px;
    background: var(--pico-primary-background, #4b6bfb);
color: white;
cursor: pointer;
border: none;
}
"#
);

#[wasm_bindgen(start)]
pub fn run() {
    dapp!(dapp, {
        dapp.style(notification::CSS);
        dapp.title_static("Component lifecycle");
        dapp.p_static(
            "Click \"Post notification\" a few times, then click a notification \
to dismiss it. Each dismiss really removes the element and its \
listener -- it isn't just hidden.",
        );

        let count = Signal::new(0usize);
        text!(
            "Active notifications: {}",
            count.clone(),
            "Active notifications: 0"
        );

        let post_button = dapp.button("Post notification");

        // NOTE on why this closure opens its own `dapp!` block, instead of
        // capturing `dapp`:
        //
        // `dapp` here is borrowed from the global Dapp for the *duration of
        // this `dapp!` block only* -- it can't be moved into a `'static`
        // closure that outlives the block (which this one does: it keeps
        // creating new notifications every time the button is clicked, long
        // after this block has finished running). A nested `dapp!` borrows
        // the same global Dapp again, at the moment it is actually needed.
        // `text!` does the same thing internally.
        let count_for_click = count.clone();
        post_button.pressed(move || {
            let n = count_for_click.get() + 1;
            count_for_click.set(n);
            let label = format!("Notification #{n} \u{2014} click to dismiss");

            dapp!(dapp, {
                let notification = dapp.button(&label);

                notification.add_class(notification::chip);

                // We manage this Subscription ourselves -- rather than
                // `Button::pressed`, which always `.detach()`s internally
                // and gives no way back -- specifically so we CAN drop it
                // when the notification is dismissed.
                //
                // It's held in an `Rc<RefCell<Option<Subscription>>>`
                // because the closure that eventually drops it is the
                // *same* closure this Subscription owns (see below for why
                // that needs an extra step, not a direct `drop(sub)` here).
                let held_subscription: Rc<RefCell<Option<Subscription>>> =
                    Rc::new(RefCell::new(None));
                let held_subscription_for_click = Rc::clone(&held_subscription);

                let count_for_dismiss = count_for_click.clone();
                let element_for_dismiss = notification.get_element().clone();

                let subscription = notification.add_event_listener("click", move |_event| {
                    // 1. Remove the element from the page right away. This
                    //    is safe to do synchronously, from within its own
                    //    click handler -- a DOM node removing itself during
                    //    its own event dispatch is a well-supported, common
                    //    pattern in the browser.
                    element_for_dismiss.remove();
                    count_for_dismiss.set(count_for_dismiss.get().saturating_sub(1));

                    // 2. Do NOT drop our own Subscription synchronously,
                    //    right here. We are currently executing *inside*
                    //    the very closure that this Subscription owns --
                    //    dropping it now would free that closure's captured
                    //    environment while it's still running on the call
                    //    stack. Instead, defer the actual drop to a
                    //    microtask via `spawn_local`: by the time that runs,
                    //    this click handler has already returned, so
                    //    there's nothing unsafe about tearing it down.
                    let to_drop = Rc::clone(&held_subscription_for_click);
                    spawn_local(async move {
                        to_drop.borrow_mut().take(); // dropped here
                    });
                });

                *held_subscription.borrow_mut() = Some(subscription);
            });
        });
    });
}
