//! # Component lifecycle example
//!
//! Most apps build their page once. This one keeps adding and removing
//! components while it runs, which shows three things:
//!
//! 1. **Scoped styles**: `style!` below generates a `notification` module
//!    holding the scoped class name (`notification::chip ==
//!    "notification-chip"`) and the matching CSS (`notification::CSS`).
//!    `dapp.style(notification::CSS)` adds that CSS to the page.
//!
//! 2. **Building components after startup**: new components are created
//!    from inside a button's click handler, by opening a `dapp!` block in
//!    the handler (see the comment on `post_button.pressed` for why).
//!
//! 3. **Removing components**: clicking a notification calls `unmount()`
//!    on it. That takes it off the page *and* ends everything that was
//!    tied to it: its click listener, its link to the `count` signal, and
//!    the signal derived just for its label. Nothing is left behind,
//!    however many notifications come and go. "Clear all" does the same
//!    for a whole container with `clear()`.
//!
//! You never manage those subscriptions yourself: they last exactly as
//! long as the component is on the page.

use dappr::prelude::*;
use wasm_bindgen::prelude::*;

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
        let clear_button = dapp.button("Clear all");
        // Notifications go in their own container, so they can be cleared
        // together.
        let board = Rc::new(dapp.container());

        // NOTE on why this closure opens its own `dapp!` block, instead of
        // capturing `dapp`:
        //
        // `dapp` here is borrowed from the global Dapp for the *duration of
        // this `dapp!` block only* -- it can't be moved into a `'static`
        // closure that outlives the block (which this one does: it keeps
        // creating new notifications every time the button is clicked, long
        // after this block has finished running). A container, on the other
        // hand, is a handle you can keep: `board.with(..)` targets it, and
        // inside it `text!` and `dapp!` add to the board.
        let posted = Signal::new(0usize);
        let (count_for_post, board_for_post) = (count.clone(), Rc::clone(&board));
        post_button.pressed(move || {
            let number = posted.get() + 1;
            posted.set(number);
            count_for_post.set(count_for_post.get() + 1);

            board_for_post.with(|board| {
                // Each notification is a small container holding a button.
                let chip = board.container();
                let active = count_for_post.clone();
                // A signal derived only for this label: it follows `count`
                // while the notification is shown, and is freed with it.
                let label = active.map(move |active| {
                    format!("Notification #{number} of {active} \u{2014} click to dismiss")
                });
                let button = chip.button(&label.get());
                button.add_class(notification::chip);
                button.bind_signal(label, None, None);

                // Clicking removes the whole notification. `unmount` ends
                // its listener and its link to `count` as well -- and it is
                // fine to do that from inside the very click being handled.
                let count_for_dismiss = count_for_post.clone();
                button.pressed(move || {
                    chip.unmount();
                    count_for_dismiss.set(count_for_dismiss.get().saturating_sub(1));
                });
            });
        });

        let count_for_clear = count.clone();
        clear_button.pressed(move || {
            board.clear();
            count_for_clear.set(0);
        });
    });
}
