use dappr::prelude::*;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn run() {
    dapp!(dapp, {
        dapp.title_static("Raw HTML example");
        dapp.html_static("<h2>This is an example of injected raw HTML</h2>");
        dapp.html_static("<p>This is another example of injected raw HTML.</p>");
        #[cfg(feature = "sanitizehtml")]
        dapp.sanitized_html_static(
            "<h1>This is an example of sanitized HTML instead.</h1>",
            Sanitize::Basic,
        );
        #[cfg(feature = "sanitizehtml")]
        dapp.sanitized_html_static(
            "This is another example of sanitized HTML which removes <a href='#'>links</a> and <span>blocks</span> but leaves <em>inline markup</em>.",
            Sanitize::Restricted,
        );
        let input = dapp.input("Insert some HTML code");
        #[cfg(feature = "sanitizehtml")]
        text!(
            "Responsive div example template with initial state; text inserted: {}",
            input.text(),
            "Responsive div example template with initial state; this text will change",
            _html,
            Sanitize::RemoveAll
        );
    });
}
