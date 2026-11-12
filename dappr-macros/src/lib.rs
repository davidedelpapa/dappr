//mod bundler;
mod styles;
mod utils;
//use bundler::{bundle_styles_impl, BundleStyleConfig};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use styles::{style_file_impl, style_impl};

/// Parses a CSS file and generates a module with constants for each class name.
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

/// Define CSS inline and generate a module with constants for each class name.
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
