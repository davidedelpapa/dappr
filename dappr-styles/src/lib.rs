//! Build-time CSS discovery and bundling for Dappr apps.
//!
//! This crate is deliberately *not* a proc-macro crate. It's an ordinary
//! library so it can be used from two very different places that both need
//! the same logic:
//!
//! - `dappr-macros`' `style!`/`style_file!` proc macros call [`extract_class_names`]
//!   to generate the `pub const` names for each CSS class (a pure, read-only
//!   operation -- no file writes).
//! - A consuming app's `build.rs` calls [`scan_dir`] + [`bundle`] to find every
//!   `style!`/`style_file!` declaration in the crate's own source and produce
//!   `static/styles.min.css` *before* `rustc` even starts compiling.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use lightningcss::rules::CssRule;
use lightningcss::selector::Component as SelectorComponent;
use lightningcss::stylesheet::{ParserOptions, PrinterOptions, StyleSheet};
use lightningcss::traits::ToCss;
use syn::visit::{self, Visit};
use syn::{Expr, ExprLit, Lit, Macro};

#[derive(Debug, thiserror::Error)]
pub enum StyleError {
    #[error("failed to read {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse Rust source in {path}: {source}")]
    RustParse {
        path: PathBuf,
        #[source]
        source: syn::Error,
    },
    #[error("failed to parse CSS for scope '{scope}' (declared in {declared_in}): {message}")]
    CssParse {
        scope: String,
        declared_in: PathBuf,
        message: String,
    },
    #[error("CSS conflicts detected:\n{0}")]
    Conflicts(String),
}

/// Pico's default CSS, embedded once, here, via `include_str!` (resolved
/// relative to *this source file*, at compile time -- robust regardless of
/// the caller's current working directory).
///
/// Previously this file was embedded in two different places, through two
/// different mechanisms: a `lazy_static` in `dappr-macros::bundler` (via
/// `include_str!`, correctly), and a *second*, separate read in
/// `dappr-macros::styles` via `std::fs::read_to_string("dappr-macros/src/assets/vendor/pico.min.css")`
/// -- a path that only resolves if the build happens to be run from this
/// exact monorepo's workspace root. Having exactly one copy, exposed by one
/// function, removes both the duplication and the fragile path.
pub fn pico_css() -> &'static str {
    include_str!("assets/vendor/pico.min.css")
}

/// A single `style!`/`style_file!` declaration found in the source tree.
#[derive(Debug, Clone)]
pub struct DiscoveredStyle {
    /// The scope name, e.g. `"button"` in `style!("button", "...")`.
    pub scope: String,
    /// The raw CSS text (already read from disk, for `style_file!`).
    pub css: String,
    /// Where this declaration lives, for error messages and `rerun-if-changed`.
    pub declared_in: PathBuf,
    /// For `style_file!`, the CSS file that was read (also worth watching
    /// for changes). `None` for inline `style!` declarations.
    pub css_file: Option<PathBuf>,
}

/// Recursively scans `root` for `.rs` files and extracts every
/// `style!("scope", "...")` / `style_file!("scope", "path/to/file.css")`
/// declaration, using a real syntax-tree walk (not string matching), so it
/// correctly handles nested modules, raw string literals, formatting, etc.
///
/// `style_file!` paths are resolved relative to `root` (pass
/// `env::var("CARGO_MANIFEST_DIR")` from `build.rs` -- see the crate docs).
pub fn scan_dir(root: &Path) -> Result<Vec<DiscoveredStyle>, StyleError> {
    let mut found = Vec::new();
    for entry in walk_rs_files(root)? {
        let source = fs::read_to_string(&entry).map_err(|e| StyleError::Io {
            path: entry.clone(),
            source: e,
        })?;
        let file = syn::parse_file(&source).map_err(|e| StyleError::RustParse {
            path: entry.clone(),
            source: e,
        })?;

        let mut visitor = StyleVisitor {
            root,
            declared_in: &entry,
            found: Vec::new(),
            error: None,
        };
        visitor.visit_file(&file);
        if let Some(err) = visitor.error {
            return Err(err);
        }
        found.extend(visitor.found);
    }
    Ok(found)
}

fn walk_rs_files(root: &Path) -> Result<Vec<PathBuf>, StyleError> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = fs::read_dir(&dir).map_err(|e| StyleError::Io {
            path: dir.clone(),
            source: e,
        })?;
        for entry in entries {
            let entry = entry.map_err(|e| StyleError::Io {
                path: dir.clone(),
                source: e,
            })?;
            let path = entry.path();
            // Skip build output / vcs / dependency dirs -- never contains
            // *our* source, and can be enormous.
            if path.is_dir() {
                if matches!(
                    path.file_name().and_then(|n| n.to_str()),
                    Some("target") | Some(".git") | Some("node_modules")
                ) {
                    continue;
                }
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                out.push(path);
            }
        }
    }
    Ok(out)
}

/// Same CSS-class extraction previously in `dappr-macros`, but returning a
/// proper error instead of silently swallowing a parse failure into an empty
/// `Vec` -- a CSS typo now surfaces where it happened, not as a confusing
/// "no such constant" error somewhere unrelated.
pub fn extract_class_names(
    scope: &str,
    source: &Path,
    css: &str,
) -> Result<Vec<String>, StyleError> {
    let stylesheet =
        StyleSheet::parse(css, ParserOptions::default()).map_err(|e| StyleError::CssParse {
            scope: scope.to_string(),
            declared_in: source.to_path_buf(),
            message: e.to_string(),
        })?;

    let mut class_names = Vec::new();
    for rule in &stylesheet.rules.0 {
        if let CssRule::Style(style_rule) = rule {
            for selector in &style_rule.selectors.0 {
                for component in selector.iter() {
                    if let SelectorComponent::Class(class) = component {
                        class_names.push(class.to_string());
                    }
                }
            }
        }
    }
    Ok(class_names)
}

/// Concatenates every discovered scope's CSS (optionally prepended with
/// framework defaults such as Pico), checks for cross-scope selector
/// conflicts, and returns the minified result ready to write to
/// `static/styles.min.css`.
pub fn bundle(styles: &[DiscoveredStyle], prepend_css: Option<&str>) -> Result<String, StyleError> {
    let mut rule_sources: HashMap<String, Vec<String>> = HashMap::new();
    for style in styles {
        let stylesheet = StyleSheet::parse(&style.css, ParserOptions::default()).map_err(|e| {
            StyleError::CssParse {
                scope: style.scope.clone(),
                declared_in: style.declared_in.clone(),
                message: e.to_string(),
            }
        })?;
        for rule in &stylesheet.rules.0 {
            if let CssRule::Style(style_rule) = rule {
                let selector = style_rule
                    .selectors
                    .to_css_string(PrinterOptions {
                        minify: false,
                        ..Default::default()
                    })
                    .unwrap_or_default();
                rule_sources
                    .entry(selector)
                    .or_default()
                    .push(style.scope.clone());
            }
        }
    }

    let mut seen: HashSet<&str> = HashSet::new();
    let conflicts: Vec<String> = rule_sources
        .into_iter()
        .filter(|(_, scopes)| scopes.len() > 1)
        .map(|(selector, scopes)| {
            format!("  '{selector}' declared in scopes: {}", scopes.join(", "))
        })
        .collect();
    // (`seen` intentionally unused beyond the filter above -- kept for a
    // future improvement where near-duplicate selectors across scopes with
    // different specificity get flagged too, not just exact matches.)
    let _ = &mut seen;

    if !conflicts.is_empty() {
        return Err(StyleError::Conflicts(conflicts.join("\n")));
    }

    let mut all_css = String::new();
    if let Some(prepend) = prepend_css {
        all_css.push_str(prepend);
        all_css.push('\n');
    }
    for style in styles {
        all_css.push_str(&style.css);
        all_css.push('\n');
    }

    let stylesheet = StyleSheet::parse(&all_css, ParserOptions::default()).map_err(|e| {
        StyleError::CssParse {
            scope: "<bundle>".to_string(),
            declared_in: PathBuf::from("<in-memory bundle>"),
            message: e.to_string(),
        }
    })?;
    let minified = stylesheet
        .to_css(PrinterOptions {
            minify: true,
            ..Default::default()
        })
        .map_err(|e| StyleError::CssParse {
            scope: "<bundle>".to_string(),
            declared_in: PathBuf::from("<in-memory bundle>"),
            message: e.to_string(),
        })?;

    Ok(minified.code)
}

// ---- AST walk -------------------------------------------------------------

struct StyleVisitor<'a> {
    root: &'a Path,
    declared_in: &'a Path,
    found: Vec<DiscoveredStyle>,
    error: Option<StyleError>,
}

fn as_str_lit(e: &Expr) -> Option<String> {
    if let Expr::Lit(ExprLit {
        lit: Lit::Str(s), ..
    }) = e
    {
        Some(s.value())
    } else {
        None
    }
}

impl<'a, 'ast> Visit<'ast> for StyleVisitor<'a> {
    fn visit_macro(&mut self, mac: &'ast Macro) {
        if self.error.is_some() {
            return;
        }

        let name = mac.path.segments.last().map(|s| s.ident.to_string());
        match name.as_deref() {
            Some("style") | Some("style_file") => {
                let is_file = name.as_deref() == Some("style_file");
                if let Ok(args) = mac.parse_body::<StyleArgs>() {
                    let values: Vec<_> = args.values.iter().collect();
                    if let [scope_expr, second_expr] = values.as_slice() {
                        if let (Some(scope), Some(second)) =
                            (as_str_lit(scope_expr), as_str_lit(second_expr))
                        {
                            if is_file {
                                let css_path = self.root.join(&second);
                                match fs::read_to_string(&css_path) {
                                    Ok(css) => self.found.push(DiscoveredStyle {
                                        scope,
                                        css,
                                        declared_in: self.declared_in.to_path_buf(),
                                        css_file: Some(css_path),
                                    }),
                                    Err(e) => {
                                        self.error = Some(StyleError::Io {
                                            path: css_path,
                                            source: e,
                                        });
                                    }
                                }
                            } else {
                                self.found.push(DiscoveredStyle {
                                    scope,
                                    css: second,
                                    declared_in: self.declared_in.to_path_buf(),
                                    css_file: None,
                                });
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        visit::visit_macro(self, mac);
    }
}

struct StyleArgs {
    values: syn::punctuated::Punctuated<Expr, syn::Token![,]>,
}
impl syn::parse::Parse for StyleArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(StyleArgs {
            values: syn::punctuated::Punctuated::parse_terminated(input)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_finds_inline_and_file_styles() {
        let dir = std::env::temp_dir().join(format!("dappr_styles_test_{}", std::process::id()));
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(
            dir.join("src/main.rs"),
            r##"
                fn setup() {
                    style!("button", ".btn { color: red; }");
                    style_file!("card", "src/card.css");
                }
            "##,
        )
        .unwrap();
        fs::write(dir.join("src/card.css"), ".card { color: blue; }").unwrap();

        let found = scan_dir(&dir).unwrap();
        assert_eq!(found.len(), 2);
        assert!(found
            .iter()
            .any(|s| s.scope == "button" && s.css.contains("red")));
        assert!(found
            .iter()
            .any(|s| s.scope == "card" && s.css.contains("blue")));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn bundle_detects_conflicts() {
        let a = DiscoveredStyle {
            scope: "a".into(),
            css: ".shared { color: red; }".into(),
            declared_in: PathBuf::from("a.rs"),
            css_file: None,
        };
        let b = DiscoveredStyle {
            scope: "b".into(),
            css: ".shared { color: blue; }".into(),
            declared_in: PathBuf::from("b.rs"),
            css_file: None,
        };
        let err = bundle(&[a, b], None).unwrap_err();
        assert!(matches!(err, StyleError::Conflicts(_)));
    }

    #[test]
    fn bundle_minifies_and_prepends() {
        let a = DiscoveredStyle {
            scope: "a".into(),
            css: ".a { color: red; }".into(),
            declared_in: PathBuf::from("a.rs"),
            css_file: None,
        };
        let css = bundle(&[a], Some(".base { margin: 0; }")).unwrap();
        assert!(css.contains(".base{margin:0}"));
        assert!(css.contains(".a{color:red}"));
    }
}
