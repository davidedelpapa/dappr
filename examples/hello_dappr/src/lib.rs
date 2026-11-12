use dappr::prelude::*;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn run() {
    dapp!(dapp, {
        text!("Hello Dappr!", _title); // Equivalent to: dapp.title_static("Hello Dappr!");
        text!("Data apps in Rust made easy.", _subtitle); // Equivalent to: dapp.subtitle_static("Data apps in Rust made easy.");
        let input = dapp.input("What's your name?");
        text!("Hello, {}!", input.text()); // Equivalent to: dapp.text("Hello, {}!", input.text(), None);
    });
}
