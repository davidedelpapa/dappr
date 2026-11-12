mod styles;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use styles::{style_file_impl, style_impl};

/// Like [`style!`], but reads the CSS from a file.
///
/// The path is relative to the crate's `Cargo.toml`:
/// `style_file!("card", "src/card.css")`.
#[proc_macro]
pub fn style_file(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = TokenStream2::from(input);
    let output = style_file_impl(input, false);
    // Wrap it with #[doc(hidden)] to aoid the generated module appearing in the docs
    let wrapped_output = quote! {
        #[doc(hidden)]
        #output
    };
    proc_macro::TokenStream::from(wrapped_output)
}
// Internal use only.
#[doc(hidden)]
#[proc_macro]
pub fn sys_style_file(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = TokenStream2::from(input);
    let output = style_file_impl(input, true);
    // Wrap it with #[doc(hidden)] to aoid the generated module appearing in the docs
    let wrapped_output = quote! {
        #[doc(hidden)]
        #output
    };
    proc_macro::TokenStream::from(wrapped_output)
}

/// Defines a block of CSS scoped to this app, and generates a module to use it.
///
/// `style!("card", ".title { color: red; }")` generates:
///
/// ```ignore
/// pub mod card {
///     pub const title: &str = "card-title"; // one per class in the CSS
///     pub const CSS: &str = ".card-title{color:red}"; // the scoped CSS
/// }
/// ```
///
/// Every class is prefixed with the scope name, both in the constant and in
/// the CSS, so two scopes can reuse a class name without clashing. Add the
/// CSS to the page once with `dapp.style(card::CSS)`, then apply classes with
/// `component.add_class(card::title)`.
#[proc_macro]
pub fn style(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = TokenStream2::from(input);
    let output = style_impl(input);
    // Wrap it with #[doc(hidden)] to aoid the generated module appearing in the docs
    let wrapped_output = quote! {
        #[doc(hidden)]
        #output
    };
    proc_macro::TokenStream::from(wrapped_output)
}
