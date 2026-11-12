use dappr::prelude::*;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn run() {
    dapp!(dapp, {
        // Simple text! with component specification: this reduces to the same function on the component specified with '_static'
        // Example: the following is infact the same as:
        // dapp.title_static("Dappr text examples");
        text!("Dappr text examples", _title);
        text!(
            "Here are many example for the <pre>text!</pre> macro.",
            _html
        ); // _html component: inserts raw HTML inside the app without any check; potentially dangerous.
        text!("H1 example", _h1);
        text!("H2 example", _h2);
        text!("H3 example", _h3);
        text!("H4 example", _h4);
        text!("H5 example", _h5); // So far no h6 implemented
        text!("p example", _p);
        text!("div example", _div);

        // Here we insert an input component whose signal will influence all other text! down below:
        let input = dapp.input("Insert some text to render here");

        // text! with no component specified is a shorthand for _p
        text!("As soon as you start to type in the input, the text below will be changed and pushed further down to make space for the reactive, non initialied components");

        // Below are the components with no initial state:
        //  they are rendered only when the Signal coming from the input button gets dispached.
        text!(
            "<p>This has appeared as soon as you started to type: {}</p>",
            input.text(),
            _html
        ); // This is more dangerous than the preceding one, as some of the html inected can come from the use though the Signal.
        text!(
            "Responsive H1 example template; text inserted: {}",
            input.text(),
            _h1
        );
        text!(
            "Responsive H2 example template; text inserted: {}",
            input.text(),
            _h2
        );
        text!(
            "Responsive H3 example template; text inserted: {}",
            input.text(),
            _h3
        );
        text!(
            "Responsive H4 example template; text inserted: {}",
            input.text(),
            _h4
        );
        text!(
            "Responsive H5 example template; text inserted: {}",
            input.text(),
            _h5
        );
        text!(
            "Responsive p example template; text inserted: {}",
            input.text(),
            _p
        );
        text!(
            "Responsive div example template; text inserted: {}",
            input.text(),
            _div
        );

        // Components with initial state. The state will be rendered at the beginning, but ges changed when the Signal gets dispatched.
        text!("<p>This was written in code after the responsive ones,<br />but you could see its <b>initial state</b>; text inserted:  {}</p>",
            input.text(),
            "<p>This one has an <b>initial state</b>.</p>",
            _html
        ); // Still dangerous
        text!(
            "Responsive h1 example template with initial state; text inserted: {}",
            input.text(),
            "Responsive h1 example template with initial state; this text will change",
            _h1
        );
        text!(
            "Responsive h2 example template with initial state; text inserted: {}",
            input.text(),
            "Responsive h2 example template with initial state; this text will change",
            _h2
        );
        text!(
            "Responsive h3 example template with initial state; text inserted: {}",
            input.text(),
            "Responsive h3 example template with initial state; this text will change",
            _h3
        );
        text!(
            "Responsive h4 example template with initial state; text inserted: {}",
            input.text(),
            "Responsive h4 example template with initial state; this text will change",
            _h4
        );
        text!(
            "Responsive h5 example template with initial state; text inserted: {}",
            input.text(),
            "Responsive h5 example template with initial state; this text will change",
            _h5
        );
        text!(
            "Responsive p example template with initial state; text inserted: {}",
            input.text(),
            "Responsive p example template with initial state; this text will change",
            _p
        );
        text!(
            "Responsive div example template with initial state; text inserted: {}",
            input.text(),
            "Responsive div example template with initial state; this text will change",
            _div
        );
    });
}
