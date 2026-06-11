//! Support of rust-analyzer macro expansion:
//!
//! - Check if we under r-a expansion,
//! - emit special helper module with original source text as it would be written into impl module,
//! - ensure that same libraries are used through `extern crate ...`,
//!
//!

use std::path::Path;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::template::TemplateContext;

/// Returns true if proc-server is IDE.
/// (Currently only `rust-analyzer` is supported)
fn is_ide() -> bool {
    // current process is rust-analyzer
    std::env::current_exe()
        .ok()
        .as_deref()
        .and_then(Path::file_name)
        .map_or_else(|| "unknown".into(), |s| s.to_string_lossy().into_owned())
        .contains("rust-analyzer")
    || // RUST_ANALYZER_INTERNALS_DO_NOT_USE=this is unstable"
    std::env::var("RUST_ANALYZER_INTERNALS_DO_NOT_USE").unwrap_or_default() == "this is unstable"
}

fn format_ide_helper_mod(template_context: &TemplateContext) -> TokenStream {
    let deps = template_context
        .source_metadata
        .dependencies
        .iter()
        .map(|dep| {
            let name = format_ident!("{}", &dep.name);

            quote! {
                extern crate #name;
            }
        })
        .collect::<Vec<_>>();
    let content = &template_context.generated_content;
    quote! {
        mod __ide_tg_helper {
            extern crate token_goblin_runtime;
            use token_goblin_runtime::prelude::*;
            #(#deps)*
            #content
        }
    }
}

pub fn emit_ide_helper_mod(template_context: &TemplateContext) -> TokenStream {
    if is_ide() {
        format_ide_helper_mod(template_context)
    } else {
        TokenStream::new()
    }
}

// ///
// /// Returns true if we are under IDE and input tokenstream is in edit (carret contain 'intellijRulezz' ident)
// ///
// pub fn skip_compile_hack(template_context: &TemplateContext) -> bool {
//     if !is_ide() {
//         return false;
//     }
//     template_context.generated_content
//     input.contains("intellijRulezz")
// }
