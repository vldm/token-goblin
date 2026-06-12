#![allow(unused)]
use std::{
    collections::BTreeMap,
    path::{Component, Path, PathBuf},
};

use proc_macro2::{Delimiter, Span, TokenStream, TokenTree};

use crate::{Result, metadata};

pub struct SpanLocation {
    pub fs_workspace_root: PathBuf,
    pub fs_crate_root: PathBuf,
    /// Path from crate root to the current module file
    pub fs_module_path: PathBuf,
    /// Extra parts of module path, that are not part of the file path.
    pub module_path_postfix: syn::Path,
}
impl SpanLocation {
    /// Construct `SpanLocation` info from proc-macro span.
    /// This function will effectively:
    /// - get crate root path from `CARGO_MANIFEST_PATH`
    /// - get module path from `local_file()`
    /// - reparse `local_file()` with simplified parser, that will extract extra module information.
    pub fn recover(span: proc_macro::Span) -> Result<Self> {
        let fs_crate_root = Self::get_crate_root_path()?;
        let (_, mut fs_workspace_manifest) = metadata::find_workspace_manifest(&fs_crate_root)?;
        fs_workspace_manifest.pop();
        let fs_workspace_root = fs_workspace_manifest;
        debug!("workspace_path: {}", fs_workspace_root.display());
        debug!("fs_crate_root: {}", fs_crate_root.display());

        let fs_module_path_absolute_or_relative = span
            .local_file()
            .ok_or_else(|| error!(Span::call_site() => "SpanLocation: Failed to get local file"))?;

        debug!(
            "fs_module_path_absolute_or_relative: {}",
            fs_module_path_absolute_or_relative.display()
        );
        let fs_module_path = Self::get_module_path(
            &fs_workspace_root,
            &fs_crate_root,
            &fs_module_path_absolute_or_relative,
        )
        .map_err(|e| error!(Span::call_site() => "SpanLocation: Failed to get module path: {e}"))?;

        let mod_info = ModInfo::micro_parse_file(&fs_module_path_absolute_or_relative)?;
        let module_path_postfix = mod_info
            .path_at_line_column(span.line(), span.column())
            .ok_or_else(
                || error!(Span::call_site() => "SpanLocation: Failed to get module path postfix"),
            )?
            .clone();
        Ok(Self {
            fs_workspace_root,
            fs_crate_root,
            fs_module_path,
            module_path_postfix,
        })
    }

    /// Return path to module relative to crate root.
    fn get_module_path(
        fs_workspace_root: &Path,
        fs_crate_root: &Path,
        fs_module_path_absolute_or_relative: &Path,
    ) -> Result<PathBuf, std::io::Error> {
        let fs_module_path_absolute = fs_workspace_root.join(fs_module_path_absolute_or_relative);
        debug!(
            "fs_module_path_absolute: {}",
            fs_module_path_absolute.display()
        );
        let fs_module_path_absolute = fs_module_path_absolute.canonicalize()?;

        let fs_module_path = fs_module_path_absolute
            .strip_prefix(fs_crate_root)
            .map_err(std::io::Error::other)?;
        Ok(fs_module_path.to_path_buf())
    }
    /// Convert location to rust compatible module path.
    pub fn module_path(&self) -> syn::Path {
        // fs_module_path to syn::Path
        // join with module_path_postfix
        let mut result = syn::Path {
            leading_colon: None,
            segments: syn::punctuated::Punctuated::new(),
        };
        let file = self.fs_module_path.extension();

        let mut components = self.fs_module_path.components();
        while let Some(Component::Normal(component)) = components.next() {
            if component.eq_ignore_ascii_case("mod.rs") {
                break;
            }
            let is_rs = std::path::Path::new(&component)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("rs"));

            let component_str = component.to_string_lossy().replace('-', "_");

            let mod_part = if is_rs {
                &component_str[..component_str.len() - 3]
            } else {
                &component_str
            };

            result.segments.push(syn::PathSegment {
                ident: syn::Ident::new(mod_part, Span::call_site()),
                arguments: syn::PathArguments::None,
            });

            // .rs should be last part
            if is_rs {
                break;
            }
        }

        assert!(
            components.next().is_none(),
            "BUG: fs_module_path has unexpected part {:?} in {}",
            components,
            self.fs_module_path.display()
        );

        // extend with module_path_postfix
        for segment in &self.module_path_postfix.segments {
            result.segments.push(segment.clone());
        }
        result
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
    /// Byte offset where each source line starts (`lines[0]` is line 1).
    lines: Vec<usize>,
}
impl ModInfo {
    pub fn micro_parse_file(file: &Path) -> Result<ModInfo> {
        proc_macro2::fallback::force();
        let result = (|| {
            let content = std::fs::read_to_string(file).map_err(
                |e| error!(Span::call_site() => "SpanLocation: failed to read file: {e}"),
            )?;
            Self::micro_parse(&content)
        })();
        proc_macro2::fallback::unforce();
        result
    }

    pub fn micro_parse(content: &str) -> Result<ModInfo> {
        let lines = Self::build_line_starts(content);
        let tokens: TokenStream = content.parse().map_err(
            |e| error!(Span::call_site() => "SpanLocation: failed to tokenize source: {e}"),
        )?;
        let mut modules = BTreeMap::new();
        modules.insert(0, Self::path_from_idents(&[]));
        let mut path = Vec::new();
        Self::scan_tokens(tokens, &mut path, &mut modules);
        Ok(Self { modules, lines })
    }
    pub fn path_at_offset(&self, offset: usize) -> Option<&syn::Path> {
        self.modules
            .range(..=offset)
            .next_back()
            .map(|(_, path)| path)
    }

    pub fn path_at_line_column(&self, line: usize, column: usize) -> Option<&syn::Path> {
        self.path_at_offset(self.byte_offset(line, column)?)
    }

    fn build_line_starts(file: &str) -> Vec<usize> {
        let mut lines = vec![0];
        for (i, b) in file.bytes().enumerate() {
            if b == b'\n' {
                lines.push(i + 1);
            }
        }
        lines
    }

    fn byte_offset(&self, line: usize, column: usize) -> Option<usize> {
        let line_start = *self.lines.get(line.checked_sub(1)?)?;
        Some(line_start + column.checked_sub(1)?)
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

    fn path_str_at_line_column(info: &ModInfo, line: usize, column: usize) -> String {
        info.path_at_line_column(line, column)
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

    #[test]
    fn line_starts_table() {
        let src = "mod a {\n    mod b {\n    }\n}\n";
        let info = ModInfo::micro_parse(src).unwrap();
        assert_eq!(info.lines, vec![0, 8, 20, 26, 28]);
    }

    #[test]
    fn path_at_line_column_matches_byte_offset() {
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

        let line_column = |pos: usize| {
            let line = src[..pos].matches('\n').count() + 1;
            (line, pos - info.lines[line - 1] + 1)
        };

        let (bar_open_line, bar_open_column) = line_column(bar_open);
        let (bar_close_line, bar_close_column) = line_column(bar_close);
        let (baz_open_line, baz_open_column) = line_column(baz_open);

        assert_eq!(path_str_at_line_column(&info, 1, 1), "");
        assert_eq!(
            path_str_at_line_column(&info, bar_open_line, bar_open_column),
            path_str(&info, bar_open)
        );
        assert_eq!(
            path_str_at_line_column(&info, bar_open_line, bar_open_column),
            "foo::bar"
        );
        assert_eq!(
            path_str_at_line_column(&info, bar_close_line, bar_close_column),
            path_str(&info, bar_close)
        );
        assert_eq!(
            path_str_at_line_column(&info, bar_close_line, bar_close_column),
            "foo"
        );
        assert_eq!(
            path_str_at_line_column(&info, baz_open_line, baz_open_column),
            path_str(&info, baz_open)
        );
        assert_eq!(
            path_str_at_line_column(&info, baz_open_line, baz_open_column),
            "baz"
        );
    }
}
