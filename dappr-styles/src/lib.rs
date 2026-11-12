//! Build-time CSS discovery and bundling for Dappr apps.
//!
//! This crate is deliberately *not* a proc-macro crate. It's an ordinary
//! library so it can be used from two very different places that both need
//! the same logic:
//!
//! - `dappr-macros`' `style!`/`style_file!` proc macros call [`extract_class_names`]
//!   to generate the `pub const` names for each CSS class (a pure, read-only
//!   operation -- no file writes).
//!   and [`scope_css`] to produce the matching, scoped CSS that the app
//!   injects at runtime with `dapp.style(...)`. Nothing is written to disk.
//! - Optionally, an app's `build.rs` can call [`scan_dir`] + [`bundle`] to
//!   collect every `style!`/`style_file!` declaration into one static
//!   stylesheet instead. Dappr itself does not need this.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use lightningcss::css_modules::{Config as CssModulesConfig, Pattern};
use lightningcss::rules::{CssRule, CssRuleList};
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

/// Pico CSS, vendored in this crate (see `assets/vendor/pico.license`).
///
/// This is the single copy in the workspace. The `sys_style_file!` macro
/// embeds it into the `dappr` crate as a string constant, and `Dapp::new()`
/// injects that into the page, so apps get the baseline look with no
/// stylesheet link and no build step.
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

/// Extracts every class name used in a selector anywhere in `css`,
/// including inside `@media` / `@supports` blocks and nested rules.
///
/// Returns a proper error instead of silently swallowing a parse failure
/// into an empty `Vec` -- a CSS typo surfaces where it happened, not as a
/// confusing "no such constant" error somewhere unrelated.
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
    collect_class_names(&stylesheet.rules, &mut class_names);
    class_names.sort();
    class_names.dedup();
    Ok(class_names)
}

fn collect_class_names(rules: &CssRuleList, out: &mut Vec<String>) {
    for rule in &rules.0 {
        match rule {
            CssRule::Style(style_rule) => {
                for selector in &style_rule.selectors.0 {
                    for component in selector.iter_raw_match_order() {
                        if let SelectorComponent::Class(class) = component {
                            out.push(class.to_string());
                        }
                    }
                }
                collect_class_names(&style_rule.rules, out);
            }
            CssRule::Media(media) => collect_class_names(&media.rules, out),
            CssRule::Supports(supports) => collect_class_names(&supports.rules, out),
            _ => {}
        }
    }
}

/// Rewrites every class (and id) selector in `css` from `.name` to
/// `.{scope}-name`, and minifies the result.
///
/// This is what makes `style!`'s scoping real: the constants the macro
/// generates (`scope::name == "{scope}-name"`) and the CSS that reaches the
/// browser are produced from the same rule, so they always match.
///
/// Only selectors are renamed. Animation names, grid areas, container names
/// and custom properties are left alone, so a scoped rule can still refer to
/// things defined elsewhere (for instance one of Pico's `--pico-*` variables).
pub fn scope_css(scope: &str, source: &Path, css: &str) -> Result<String, StyleError> {
    let err = |message: String| StyleError::CssParse {
        scope: scope.to_string(),
        declared_in: source.to_path_buf(),
        message,
    };
    let pattern_src = format!("{scope}-[local]");
    let pattern = Pattern::parse(&pattern_src).map_err(|e| err(e.to_string()))?;
    let options = ParserOptions {
        css_modules: Some(CssModulesConfig {
            pattern,
            dashed_idents: false,
            animation: false,
            grid: false,
            custom_idents: false,
            container: false,
            pure: false,
        }),
        ..Default::default()
    };
    let stylesheet = StyleSheet::parse(css, options).map_err(|e| err(e.to_string()))?;
    let out = stylesheet
        .to_css(PrinterOptions {
            minify: true,
            ..Default::default()
        })
        .map_err(|e| err(e.to_string()))?;
    Ok(out.code)
}

/// Scopes every discovered style (see [`scope_css`]), checks that no two
/// scopes style the very same selector, and returns one minified stylesheet.
///
/// Because class selectors are scoped, two scopes can both declare `.title`
/// without clashing. What is still reported is two scopes styling the same
/// *unscoped* selector, such as a bare `h1`.
///
/// `prepend_css` is emitted first, verbatim. Dappr injects Pico at runtime,
/// so most callers pass `None`.
pub fn bundle(styles: &[DiscoveredStyle], prepend_css: Option<&str>) -> Result<String, StyleError> {
    let mut rule_sources: HashMap<String, Vec<String>> = HashMap::new();
    let mut all_css = String::new();
    if let Some(prepend) = prepend_css {
        all_css.push_str(prepend);
        all_css.push('\n');
    }

    for style in styles {
        let scoped = scope_css(&style.scope, &style.declared_in, &style.css)?;
        let stylesheet = StyleSheet::parse(&scoped, ParserOptions::default()).map_err(|e| {
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
                    .to_css_string(PrinterOptions::default())
                    .unwrap_or_default();
                let scopes = rule_sources.entry(selector).or_default();
                if !scopes.contains(&style.scope) {
                    scopes.push(style.scope.clone());
                }
            }
        }
        all_css.push_str(&scoped);
        all_css.push('\n');
    }

    let mut conflicts: Vec<String> = rule_sources
        .into_iter()
        .filter(|(_, scopes)| scopes.len() > 1)
        .map(|(selector, scopes)| {
            format!("  '{selector}' declared in scopes: {}", scopes.join(", "))
        })
        .collect();
    conflicts.sort();
    if !conflicts.is_empty() {
        return Err(StyleError::Conflicts(conflicts.join("\n")));
    }

    Ok(all_css)
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
    fn bundle_detects_conflicts_on_unscoped_selectors() {
        let a = DiscoveredStyle {
            scope: "a".into(),
            css: "h1 { color: red; }".into(),
            declared_in: PathBuf::from("a.rs"),
            css_file: None,
        };
        let b = DiscoveredStyle {
            scope: "b".into(),
            css: "h1 { color: blue; }".into(),
            declared_in: PathBuf::from("b.rs"),
            css_file: None,
        };
        let err = bundle(&[a, b], None).unwrap_err();
        assert!(matches!(err, StyleError::Conflicts(_)));
    }

    #[test]
    fn bundle_scopes_classes_so_same_name_does_not_conflict() {
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
        let css = bundle(&[a, b], Some(".base{margin:0}")).unwrap();
        assert!(css.starts_with(".base{margin:0}"));
        assert!(css.contains(".a-shared{color:red}"));
        assert!(css.contains(".b-shared{color:#00f}"));
    }

    #[test]
    fn scope_css_renames_selectors_only() {
        let css = scope_css(
            "card",
            Path::new("t.rs"),
            "@media (min-width: 10px) { .box:hover > .title { color: var(--pico-color); animation: spin 1s; } }",
        )
        .unwrap();
        assert!(css.contains(".card-box:hover>.card-title"), "{css}");
        assert!(css.contains("var(--pico-color)"), "{css}");
        assert!(css.contains("spin"), "{css}");
        assert!(!css.contains("card-spin"), "{css}");
    }

    #[test]
    fn extract_class_names_looks_inside_media_queries() {
        let names = extract_class_names(
            "x",
            Path::new("t.rs"),
            ".a { color: red; } @media (min-width: 10px) { .b, .a { color: blue; } }",
        )
        .unwrap();
        assert_eq!(names, vec!["a".to_string(), "b".to_string()]);
    }
}
