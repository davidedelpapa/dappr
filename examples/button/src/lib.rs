use dappr::prelude::*;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn run() {
    dapp!(dapp, {
        dapp.title_static("Dappr buttons");
        let button = dapp.button("Press me!");
        button.pressed(move || {
            text!("Button pressed");
        });
        text!(
            "Button has been pressed",
            button.get_signal(),
            "button is not yet pressed"
        );
    });
}
