//! Loading data from a URL.
//!
//! ```ignore
//! use dappr::prelude::*;
//!
//! // Fetched once, when the app starts.
//! let prices = fetch_table("prices.csv");
//! dapp.load_status(&prices);
//! dapp.table(&prices);
//! ```
//!
//! The URL can also be a `Signal<String>`: the data is fetched again every
//! time it changes, which is how an app queries an API with parameters taken
//! from its inputs.
//!
//! The browser decides what a page may fetch. Files served next to the app
//! always work; another site must allow it (CORS), or the request fails.
use super::load::{parse_table, Loadable};
use super::table::DTable;
use crate::communication::signals::Signal;
use std::cell::Cell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::Response;

/// Where to fetch from: a fixed URL (`&str`, `String`), or a
/// `Signal<String>` to fetch again whenever the URL changes.
pub trait UrlSource {
    /// The URL as a Signal.
    fn into_signal(self) -> Signal<String>;
}
impl UrlSource for &str {
    fn into_signal(self) -> Signal<String> {
        Signal::new(self.to_string())
    }
}
impl UrlSource for String {
    fn into_signal(self) -> Signal<String> {
        Signal::new(self)
    }
}
impl UrlSource for &String {
    fn into_signal(self) -> Signal<String> {
        Signal::new(self.clone())
    }
}
impl UrlSource for Signal<String> {
    fn into_signal(self) -> Signal<String> {
        self
    }
}
impl UrlSource for &Signal<String> {
    fn into_signal(self) -> Signal<String> {
        self.clone()
    }
}

/// The body of a successful response, with its `Content-Type`.
struct Fetched {
    text: String,
    content_type: String,
}

fn js_message(error: wasm_bindgen::JsValue) -> String {
    error
        .dyn_ref::<js_sys::Error>()
        .map(|e| String::from(e.message()))
        .or_else(|| error.as_string())
        .unwrap_or_else(|| "request failed".to_string())
}

async fn get(url: &str) -> Result<Fetched, String> {
    let window = web_sys::window().ok_or("no browser window")?;
    let response: Response = JsFuture::from(window.fetch_with_str(url))
        .await
        .map_err(|e| format!("could not reach {url}: {}", js_message(e)))?
        .unchecked_into();
    if !response.ok() {
        return Err(format!(
            "{url} answered {} {}",
            response.status(),
            response.status_text()
        ));
    }
    let content_type = response
        .headers()
        .get("content-type")
        .ok()
        .flatten()
        .unwrap_or_default();
    let text = JsFuture::from(response.text().map_err(js_message)?)
        .await
        .map_err(js_message)?
        .as_string()
        .unwrap_or_default();
    Ok(Fetched { text, content_type })
}

/// Fetches the text at `url`. For use in your own `async` code; most apps
/// want [`fetch_text`] or [`fetch_table`] instead.
pub async fn get_text(url: &str) -> Result<String, String> {
    get(url).await.map(|fetched| fetched.text)
}

/// The engine behind `fetch_text` and `fetch_table`: fetches whenever the
/// URL changes and publishes each state. `finish` turns a response into the
/// final value.
fn fetch_with<T: Clone + 'static>(
    url: impl UrlSource,
    finish: impl Fn(&str, Fetched) -> Loadable<T> + 'static,
) -> Signal<Loadable<T>> {
    let url = url.into_signal();
    let output = Signal::new(Loadable::Idle);
    // Each request gets a number; a response is used only if no newer
    // request has started since, so a slow old answer cannot overwrite a
    // fast new one.
    let latest = Rc::new(Cell::new(0u64));
    let finish = Rc::new(finish);

    // The URL signal reaches the output through a weak handle: the output
    // keeps the URL alive, not the other way round, so an output nobody
    // uses any more stops fetching and is freed.
    let start = {
        let target = output.downgrade();
        move |url: &String| {
            let Some(output) = target.upgrade() else {
                return;
            };
            let request = latest.get() + 1;
            latest.set(request);
            let url = url.trim().to_string();
            if url.is_empty() {
                output.set(Loadable::Idle);
                return;
            }
            output.set(Loadable::Loading);
            let (target, latest, finish) = (target.clone(), Rc::clone(&latest), Rc::clone(&finish));
            spawn_local(async move {
                let result = get(&url).await;
                // A newer request has started, or nobody is waiting any more.
                let Some(output) = target.upgrade().filter(|_| latest.get() == request) else {
                    return;
                };
                output.set(match result {
                    Ok(fetched) => finish(&url, fetched),
                    Err(message) => Loadable::Failed(message),
                });
            });
        }
    };

    start(&url.get());
    output.keep(url.subscribe(start));
    output
}

/// Fetches text from a URL.
///
/// Returns at once with a Signal that goes [`Loading`](Loadable::Loading),
/// then [`Ready`](Loadable::Ready) with the text or
/// [`Failed`](Loadable::Failed) with the reason. An empty URL is
/// [`Idle`](Loadable::Idle).
pub fn fetch_text(url: impl UrlSource) -> Signal<Loadable<String>> {
    fetch_with(url, |_, fetched| Loadable::Ready(fetched.text))
}

/// Fetches a table from a URL: JSON if the server says so or the URL ends
/// in `.json`, tab-separated for `.tsv`, otherwise CSV (see
/// [`parse_table`]).
///
/// ```ignore
/// // Refetched each time the user picks another year.
/// let year = dapp.select("Year", &[2024, 2025, 2026]).value();
/// let url = year.map(|year| format!("/api/sales?year={year}"));
/// let sales = fetch_table(url);
/// dapp.load_status(&sales);
/// dapp.table(&sales);
/// ```
pub fn fetch_table(url: impl UrlSource) -> Signal<Loadable<DTable>> {
    fetch_with(url, |url, fetched| {
        parse_table(url, &fetched.content_type, &fetched.text).into()
    })
}
