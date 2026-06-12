#![allow(unused)]
use std::{collections::BTreeMap, path::PathBuf};

use proc_macro2::{Delimiter, Span, TokenStream, TokenTree};

use crate::Result;

pub struct SpanLocation {
    fs_crate_root: PathBuf,
    /// Path from crate root to the current module file
    fs_module_path: PathBuf,
    /// Extra parts of module path, that are not part of the file path.
    module_path_postfix: syn::Path,
}
impl SpanLocation {
    /// Construct `SpanLocation` info from proc-macro span.
    /// This function will effectively:
    /// - get crate root path from `CARGO_MANIFEST_PATH`
    /// - get module path from `local_file()`
    /// - reparse `local_file()` with simplified parser, that will extract extra module information.
    pub fn recover(span: proc_macro::Span) -> Result<Self> {
        let fs_crate_root = Self::get_crate_root_path()?;
        let fs_module_path = span
            .local_file()
            .ok_or_else(|| error!(Span::call_site() => "Failed to get local file"))?;
        let _ = fs_module_path;
        todo!()
    }
    /// Convert location to rust compatible module path.
    pub fn module_path(&self) -> syn::Path {
        todo!()
    }
    pub fn file_path(&self) -> PathBuf {
        self.fs_crate_root.join(&self.fs_module_path)
    }

    fn get_crate_root_path() -> Result<PathBuf> {
        let crate_root = std::env::var("CARGO_MANIFEST_DIR")
            .map_err(|_| error!(Span::call_site() => "CARGO_MANIFEST_DIR is not set"))?;
        Ok(PathBuf::from(crate_root))
    }
}

/// Information about modules in file,
/// in format of map:
/// `<byte_location> -> <module_path>`
struct ModInfo {
    modules: BTreeMap<usize, syn::Path>,
}
impl ModInfo {
    fn micro_parse(file: &str) -> Result<ModInfo> {
        let tokens: TokenStream = file
            .parse()
            .map_err(|e| error!(Span::call_site() => "failed to tokenize source: {e}"))?;
        let mut modules = BTreeMap::new();
        modules.insert(0, Self::path_from_idents(&[]));
        let mut path = Vec::new();
        Self::scan_tokens(tokens, &mut path, &mut modules);
        Ok(Self { modules })
    }

    fn path_at_offset(&self, offset: usize) -> Option<&syn::Path> {
        self.modules
            .range(..=offset)
            .next_back()
            .map(|(_, path)| path)
    }

    fn scan_tokens(
        tokens: TokenStream,
        path: &mut Vec<proc_macro2::Ident>,
        modules: &mut BTreeMap<usize, syn::Path>,
    ) {
        let mut iter = tokens.into_iter().peekable();

        macro_rules! peek_and_parse {
            ($path:ident ($val:ident) $($body:tt)*) => {
                let Some(TokenTree::$path($val)) = iter.peek() else {
                    continue;
                };
                $($body)*

                let Some(TokenTree::$path($val)) = iter.next() else {
                    unreachable!();
                };
            };
        }
        while let Some(token) = iter.next() {
            match token {
                TokenTree::Ident(mod_kw) => {
                    if mod_kw == "mod" {
                        peek_and_parse!(Ident(name));
                        peek_and_parse!(Group(group) if group.delimiter() != Delimiter::Brace {
                            continue;
                        });

                        let after_open = group.span_open().byte_range().end;
                        path.push(name);
                        modules.insert(after_open, Self::path_from_idents(path));

                        Self::scan_tokens(group.stream(), path, modules);

                        path.pop();
                        let after_close = group.span_close().byte_range().end;
                        modules.insert(after_close, Self::path_from_idents(path));
                    }
                }
                TokenTree::Group(group) if group.delimiter() == Delimiter::Brace => {
                    Self::scan_tokens(group.stream(), path, modules);
                }
                _ => {}
            }
        }
    }

    fn path_from_idents(idents: &[proc_macro2::Ident]) -> syn::Path {
        let mut segments = syn::punctuated::Punctuated::<syn::PathSegment, syn::Token![::]>::new();
        for ident in idents {
            segments.push(syn::PathSegment {
                ident: ident.clone(),
                arguments: syn::PathArguments::None,
            });
        }
        syn::Path {
            leading_colon: None,
            segments,
        }
    }
}

#[cfg(test)]
#[allow(clippy::similar_names)]
mod tests {
    use super::ModInfo;

    fn path_str(info: &ModInfo, offset: usize) -> String {
        info.path_at_offset(offset)
            .map(|p| {
                p.segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::")
            })
            .unwrap_or_default()
    }

    #[test]
    fn nested_modules() {
        let src = r"
mod foo {
    mod bar {
    }
}
mod baz {
}
";
        let info = ModInfo::micro_parse(src).unwrap();
        let bar_keyword = src.find("mod bar").unwrap();
        let bar_open = bar_keyword + src[bar_keyword..].find('{').unwrap() + 1;
        let bar_close = bar_open + src[bar_open..].find('}').unwrap() + 1;
        let baz_keyword = src.find("mod baz").unwrap();
        let baz_open = baz_keyword + src[baz_keyword..].find('{').unwrap() + 1;

        assert_eq!(path_str(&info, 0), "");
        assert_eq!(path_str(&info, bar_open), "foo::bar");
        assert_eq!(path_str(&info, bar_close), "foo");
        assert_eq!(path_str(&info, baz_open), "baz");
    }

    #[test]
    fn ignores_mod_in_comments_and_literals() {
        let src = r#"
// mod fake { }
/* mod nested { } */
const S: &str = "mod str { }";
mod real {
}
"#;
        let info = ModInfo::micro_parse(src).unwrap();
        let real_open = src.find("mod real").unwrap() + "mod real ".len() + 1;
        assert_eq!(path_str(&info, real_open), "real");
    }

    #[test]
    fn ignores_external_mod() {
        let src = "mod external;\nmod inline {\n}\n";
        let info = ModInfo::micro_parse(src).unwrap();
        let inline_open = src.find("mod inline").unwrap() + "mod inline ".len() + 1;
        assert_eq!(path_str(&info, inline_open), "inline");
        let external_pos = src.find("external").unwrap();
        assert_eq!(path_str(&info, external_pos), "");
    }

    #[test]
    fn raw_ident_module_name() {
        let src = "mod r#type {\n}\n";
        let info = ModInfo::micro_parse(src).unwrap();
        let open = src.find('{').unwrap() + 1;
        let path = info.path_at_offset(open).unwrap();
        assert!(path.segments[0].ident.to_string().starts_with("r#"));
    }
}
