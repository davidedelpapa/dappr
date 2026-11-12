//! `style!` and `style_file!` -- generate scoped Rust class-name constants
//! from CSS.
//!
//! These macros used to *also* be responsible for staging CSS into a `pkg/`
//! directory as a side effect of macro expansion, which `prepare_dapp!()`
//! then bundled at some other, unspecified point during the same crate's
//! compilation. That coupling is gone: these macros now do exactly one
//! thing -- parse CSS and emit `pub const` names for its classes -- and
//! never touch the filesystem for writing. The actual CSS *bundling* into
//! `static/styles.min.css` is a separate, well-defined step that runs in
//! `build.rs`, before `rustc` even starts (see `dappr-styles::bundle` and
//! this crate's `build.rs` for the app-side half of this).
//!
//! Both halves independently discover the same `style!`/`style_file!`
//! declarations (this file, at macro-expansion time, to generate constants;
//! `build.rs`, before compilation, to generate the bundle) using the same
//! `dappr-styles` crate. That's a deliberate bit of duplicated *reading*
//! work in exchange for removing all cross-macro *write* coordination --
//! reads are cheap and side-effect-free, so there's nothing to get wrong by
//! doing it twice.

use dappr_styles::extract_class_names;
use proc_macro2::TokenStream as TokenStream2;
use proc_macro2::{Ident, Span};
use quote::quote;
use std::collections::HashSet;
use std::path::Path;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{self, parse2, punctuated::Punctuated, Expr, ExprLit, Lit, Token};

#[doc(hidden)]
struct Args {
    values: Punctuated<Expr, Token![,]>,
}
impl Parse for Args {
    #[doc(hidden)]
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Args {
            values: Punctuated::parse_terminated(input)?,
        })
    }
}

#[doc(hidden)]
fn generate_class_constants(
    scope: String,
    class_names: HashSet<String>,
) -> proc_macro2::TokenStream {
    let class_constants = class_names.iter().map(|class_name| {
        let rust_safe_ident = class_name.replace('-', "_"); // Convert to valid Rust identifier
        let ident = Ident::new(&rust_safe_ident, Span::call_site());
        let scoped_name = format!("{}-{}", scope, class_name); // Keep original class name for the value

        quote! {
            pub const #ident: &str = #scoped_name;
        }
    });

    let scope_ident = Ident::new(scope.as_str(), Span::call_site());

    quote! {
        pub mod #scope_ident {
            #(#class_constants)*
        }
    }
}

/// Parses the two-string-literal argument shape shared by `style!` and
/// `style_file!`. Returns a `syn::Error` (rather than panicking) on
/// misuse, so callers can turn it into a proper `compile_error!` pointing
/// at the macro invocation, instead of a raw proc-macro panic.
#[doc(hidden)]
fn parse_two_str_args(input: TokenStream2, usage: &str) -> syn::Result<(syn::LitStr, syn::LitStr)> {
    let call_site = input.span();
    let args = parse2::<Args>(input)?;
    let args_vec: Vec<_> = args.values.into_iter().collect();

    match <[Expr; 2]>::try_from(args_vec) {
        Ok(
            [Expr::Lit(ExprLit {
                lit: Lit::Str(a), ..
            }), Expr::Lit(ExprLit {
                lit: Lit::Str(b), ..
            })],
        ) => Ok((a, b)),
        Ok([a, b]) => Err(syn::Error::new(
            a.span().join(b.span()).unwrap_or_else(|| a.span()),
            usage,
        )),
        Err(_) => Err(syn::Error::new(call_site, usage)),
    }
}

/// Validates that a scope name will actually work as the Rust module name
/// `generate_class_constants` turns it into (`pub mod #scope { ... }`).
///
/// Without this check, a scope like `"my-app"` -- a very plausible real
/// choice, hyphens are everywhere in crate/package naming -- would reach
/// `Ident::new(scope, ...)` and panic outright with a raw, confusing
/// proc-macro panic instead of a clean diagnostic pointing at the `style!`
/// call. (Class *names* already get their hyphens sanitized to underscores
/// before becoming identifiers; the scope name itself never got the same
/// treatment.)
#[doc(hidden)]
fn validate_scope(scope_lit: &syn::LitStr) -> syn::Result<String> {
    let scope = scope_lit.value();
    if syn::parse_str::<Ident>(&scope).is_err() {
        return Err(syn::Error::new(
            scope_lit.span(),
            format!(
                "style!/style_file!: scope '{scope}' can't be used as a Rust identifier \
                 (it becomes a module name: `pub mod {scope} {{ ... }}`). Hyphens in \
                 particular aren't allowed -- try an underscore instead, e.g. '{}'.",
                scope.replace('-', "_")
            ),
        ));
    }
    Ok(scope)
}

#[doc(hidden)]
pub fn style_file_impl(input: TokenStream2, is_library_style: bool) -> TokenStream2 {
    let (scope_lit, path_lit) = match parse_two_str_args(
        input,
        "style_file! expects two string literals: a scope name and a file path, \
         e.g. style_file!(\"button\", \"src/components/button.css\")",
    ) {
        Ok(pair) => pair,
        Err(e) => return e.to_compile_error(),
    };
    let scope = match validate_scope(&scope_lit) {
        Ok(scope) => scope,
        Err(e) => return e.to_compile_error(),
    };
    let file_path = path_lit.value();

    let css_content = if is_library_style {
        match file_path.as_str() {
            // Pico is embedded exactly once, in `dappr-styles`, and shared
            // by this macro and by `build.rs`'s bundling step -- see that
            // crate's `pico_css()` for why that single source of truth
            // matters.
            "pico.css" => dappr_styles::pico_css().to_string(),
            other => {
                return syn::Error::new(
                    path_lit.span(),
                    format!("Unknown library style file: {other}"),
                )
                .to_compile_error();
            }
        }
    } else {
        // Resolved relative to the *calling* crate's manifest directory,
        // not the process's current working directory. `CARGO_MANIFEST_DIR`
        // is set by Cargo for every rustc invocation, so this is robust
        // regardless of where `cargo build` happens to be run from -- unlike
        // the previous implementation, which read a CWD-relative path and
        // only worked by convention when building from this exact
        // workspace's root.
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let resolved = Path::new(&manifest_dir).join(&file_path);
        match std::fs::read_to_string(&resolved) {
            Ok(content) => content,
            Err(e) => {
                return syn::Error::new(
                    path_lit.span(),
                    format!("style_file!: could not read '{}': {e}", resolved.display()),
                )
                .to_compile_error();
            }
        }
    };

    match extract_class_names(&scope, Path::new(&file_path), &css_content) {
        Ok(class_names) => {
            generate_class_constants(scope, class_names.into_iter().collect::<HashSet<_>>())
        }
        Err(e) => syn::Error::new(
            path_lit.span(),
            format!("style_file!: invalid CSS in '{file_path}': {e}"),
        )
        .to_compile_error(),
    }
}

#[doc(hidden)]
pub fn style_impl(input: TokenStream2) -> TokenStream2 {
    let (scope_lit, styles_lit) = match parse_two_str_args(
        input,
        "style! expects two string literals: a scope name and a block of CSS, \
         e.g. style!(\"button\", \".btn { color: red; }\")",
    ) {
        Ok(pair) => pair,
        Err(e) => return e.to_compile_error(),
    };
    let scope = match validate_scope(&scope_lit) {
        Ok(scope) => scope,
        Err(e) => return e.to_compile_error(),
    };
    let styles = styles_lit.value();

    match extract_class_names(&scope, Path::new("<inline style!>"), &styles) {
        Ok(class_names) => {
            generate_class_constants(scope, class_names.into_iter().collect::<HashSet<_>>())
        }
        Err(e) => syn::Error::new(
            styles_lit.span(),
            format!("style!: invalid CSS for scope '{scope}': {e}"),
        )
        .to_compile_error(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;
    use std::fs;
    use syn::File;

    /// Pulls the sorted `pub const` items out of the single module a
    /// `style!`/`style_file!` expansion produces, *and* returns the
    /// module's own name -- so tests can catch a scope/module-name mismatch
    /// directly, instead of only comparing the constants inside it.
    ///
    /// (The previous version of this test helper only compared inner items
    /// and never looked at the module's ident at all, so a bug that made
    /// `generate_class_constants` emit the wrong module name -- unrelated
    /// to the scope it was given -- would have passed silently. Comparing
    /// `module.ident` here closes that gap.)
    fn module_name_and_sorted_consts(tokens: TokenStream2) -> (String, Vec<(String, String)>) {
        let file: File = syn::parse2(tokens).expect("generated output should be valid Rust");
        let item_mod = match file.items.first() {
            Some(syn::Item::Mod(m)) => m,
            other => panic!("expected a single module item, got {other:?}"),
        };
        let mut consts: Vec<(String, String)> = item_mod
            .content
            .as_ref()
            .map(|(_, items)| {
                items
                    .iter()
                    .filter_map(|item| match item {
                        syn::Item::Const(c) => {
                            let value = match &*c.expr {
                                Expr::Lit(ExprLit {
                                    lit: Lit::Str(s), ..
                                }) => s.value(),
                                _ => panic!("expected a string literal const value"),
                            };
                            Some((c.ident.to_string(), value))
                        }
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default();
        consts.sort();
        (item_mod.ident.to_string(), consts)
    }

    #[test]
    fn test_style_macro() {
        let input = quote! {
            "testscope1", r#".class1 { color: red; } .class2 { color: blue; }"#
        };
        let output = style_impl(input);

        let (module_name, consts) = module_name_and_sorted_consts(output);
        assert_eq!(module_name, "testscope1");
        assert_eq!(
            consts,
            vec![
                ("class1".to_string(), "testscope1-class1".to_string()),
                ("class2".to_string(), "testscope1-class2".to_string()),
            ]
        );
    }

    #[test]
    fn test_style_macro_reports_bad_css_as_compile_error() {
        // A leading-digit class name is an invalid CSS identifier, so this
        // is a genuine parse error -- verified directly against the real
        // `lightningcss` crate (not just this crate's own logic): it
        // returns `Err("Invalid empty selector...")` for exactly this
        // input.
        //
        // Note what does NOT belong here: a missing closing brace, an
        // unterminated string, or an invalid property *value* (e.g.
        // `color: not-a-color`) all still parse successfully. Per the CSS
        // Syntax spec, a block left open at end-of-input is implicitly
        // closed rather than treated as an error -- every browser does
        // this too. Only a malformed *selector* reliably produces a hard
        // parse error; see `test_style_macro_tolerates_unterminated_block`
        // below for the (equally real) other side of that.
        let input = quote! {
            "testscope_broken", ".1invalid { color: red; }"
        };
        let output = style_impl(input).to_string();
        assert!(
            output.contains("compile_error"),
            "expected a compile_error! for an invalid selector, got: {output}"
        );
    }

    /// The flip side of the test above: CSS's grammar is lenient about
    /// blocks left open at end-of-input (it implicitly closes them rather
    /// than erroring), so a missing closing brace does NOT produce a
    /// `compile_error!` -- it produces a perfectly valid, successfully
    /// extracted class. This is correct, spec-compliant parser behavior,
    /// not a gap in `style!`'s error handling; this test exists so nobody
    /// "fixes" that assumption back the other way by mistake.
    #[test]
    fn test_style_macro_tolerates_unterminated_block() {
        let input = quote! {
            "testscope_unterminated", ".btn { color: red;"
        };
        let output = style_impl(input);
        let (module_name, consts) = module_name_and_sorted_consts(output);
        assert_eq!(module_name, "testscope_unterminated");
        assert_eq!(
            consts,
            vec![("btn".to_string(), "testscope_unterminated-btn".to_string())]
        );
    }

    #[test]
    fn test_style_macro_reports_wrong_arity_as_compile_error() {
        let input = quote! { "only-one-arg" };
        let output = style_impl(input).to_string();
        assert!(output.contains("compile_error"));
    }

    #[test]
    fn test_style_macro_reports_invalid_scope_as_compile_error() {
        // A hyphen isn't a valid character in a Rust identifier, and the
        // scope becomes one (`pub mod #scope { ... }`).
        let input = quote! {
            "my-app", ".btn { color: red; }"
        };
        let output = style_impl(input).to_string();
        assert!(
            output.contains("compile_error"),
            "expected a compile_error! for an invalid scope identifier, got: {output}"
        );
    }

    #[test]
    fn test_style_file_macro() {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let file_path = Path::new(&manifest_dir).join("test_style_file.css");
        fs::write(
            &file_path,
            r#"
    .class1 { color: red; }
    .class2 { color: blue; }
"#,
        )
        .expect("Failed to write test CSS file");

        let input = quote! {
            "testscope2", "test_style_file.css"
        };
        let output = style_file_impl(input, false);

        let (module_name, consts) = module_name_and_sorted_consts(output);
        assert_eq!(module_name, "testscope2");
        assert_eq!(
            consts,
            vec![
                ("class1".to_string(), "testscope2-class1".to_string()),
                ("class2".to_string(), "testscope2-class2".to_string()),
            ]
        );
        fs::remove_file(&file_path).expect("Failed to remove test CSS file");
    }

    #[test]
    fn test_style_file_macro_reports_missing_file_as_compile_error() {
        let input = quote! {
            "testscope3", "this/file/does/not/exist.css"
        };
        let output = style_file_impl(input, false).to_string();
        assert!(output.contains("compile_error"));
    }

    #[test]
    fn test_sys_style_file_pico() {
        let input = quote! {
            "PICO_STYLE", "pico.css"
        };
        let output = style_file_impl(input, true);
        let (module_name, consts) = module_name_and_sorted_consts(output);
        assert_eq!(module_name, "PICO_STYLE");
        // Pico ships a lot of classes -- just check we actually found some,
        // without hardcoding the exact set (that would just be re-testing
        // Pico's own CSS, and would break every time Pico is upgraded).
        assert!(!consts.is_empty());
    }
}
