//! Helpers for values computed from several signals.

/// A Signal computed from other Signals, recomputed whenever any of them
/// changes.
///
/// List the source signals, then `=>` and an expression. Inside the
/// expression each name is that signal's *current value*.
///
/// ```no_run
/// use dappr::prelude::*;
/// # let dapp = Dapp::new();
/// let price = dapp.number("Price", 10.0).value();
/// let quantity = dapp.slider("Quantity", 1.0, 10.0, 2.0).value();
/// let gift = dapp.checkbox("Gift wrap", false).value();
///
/// let total = computed!(price, quantity, gift => {
///     price * quantity + if gift { 2.0 } else { 0.0 }
/// });
/// dapp.text("Total: {}", total, None);
/// ```
///
/// It does the same job as chaining [`Signal::zip`](crate::prelude::Signal::zip)
/// and [`Signal::map`](crate::prelude::Signal::map), without the nested
/// tuples. Anything else the expression uses (a table to filter, say) is
/// moved into it.
#[macro_export]
macro_rules! computed {
    ($($signal:ident),+ $(,)? => $body:expr) => {{
        let compute = {
            $(let $signal = $signal.clone();)+
            $crate::prelude::Rc::new(move || {
                $(let $signal = $signal.get();)+
                $body
            })
        };
        let output = $crate::prelude::Signal::new(compute());
        $({
            let (output, compute) = (output.clone(), $crate::prelude::Rc::clone(&compute));
            $signal.subscribe(move |_| output.set(compute())).detach();
        })+
        output
    }};
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn computed_follows_every_source() {
        let price = Signal::new(10.0);
        let quantity = Signal::new(2.0);
        let gift = Signal::new(false);
        let label = String::from("Total");

        let total = computed!(price, quantity, gift => {
            format!("{label}: {}", price * quantity + if gift { 2.0 } else { 0.0 })
        });
        assert_eq!(total.get(), "Total: 20");
        quantity.set(3.0);
        assert_eq!(total.get(), "Total: 30");
        gift.set(true);
        assert_eq!(total.get(), "Total: 32");
        price.set(1.0);
        assert_eq!(total.get(), "Total: 5");
    }

    #[test]
    fn computed_works_with_a_single_source() {
        let celsius = Signal::new(100.0);
        let fahrenheit = computed!(celsius => celsius * 9.0 / 5.0 + 32.0);
        assert_eq!(fahrenheit.get(), 212.0);
    }
}
