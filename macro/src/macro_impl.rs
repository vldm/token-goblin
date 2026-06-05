use std::path::PathBuf;

use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};

use crate::{
    Result,
    dylib::{self, BuildProfile},
    metadata, path,
    template::{self, TemplateContext},
};

pub struct ProxyInput {
    pub dylib_path: syn::LitStr,
    pub source_hash: syn::LitStr,
    pub tokens: proc_macro2::TokenStream,
}

impl syn::parse::Parse for ProxyInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let dylib_path = input.parse()?;
        input.parse::<syn::Token![,]>()?;
        let source_hash = input.parse()?;
        let tokens = if input.is_empty() {
            proc_macro2::TokenStream::new()
        } else {
            input.parse::<syn::Token![,]>()?;
            input.parse()?
        };
        Ok(Self {
            dylib_path,
            source_hash,
            tokens,
        })
    }
}
/// How to emit debug information during macro expansion.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DebugMode {
    /// Source macro is expected to produce items and we can emit extra items with debug information.
    Items,
    /// Source macro is expected to produce expression so we need to wrap extra items into a block.
    Expression,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub cache: bool,
    pub split_cache: bool,
    pub debug: Option<DebugMode>,
}
impl Config {
    fn from_attrs(args: TokenStream) -> Result<Self> {
        debug!("args: {}", args);
        let config = Config::default();
        Ok(config)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            cache: true,
            split_cache: false,
            debug: None,
        }
    }
}

pub fn munch_impl(args: TokenStream, item: TokenStream) -> Result<TokenStream> {
    let item = syn::parse2::<syn::Item>(item)?;
    let config = Config::from_attrs(args)?;
    match item {
        syn::Item::Fn(item) => function_impl(config, item),
        _ => Err(error!(Span::call_site() => "Expected function")),
    }
}

fn function_impl(config: Config, mut item: syn::ItemFn) -> Result<TokenStream> {
    let name = &item.sig.ident;
    let package_name = format!("token-goblin-{}", name.to_string().replace('_', "-"));

    let source_metadata = metadata::load_dependencies()?;

    item.vis = syn::Visibility::Public(syn::token::Pub::default());
    let impls = quote! { #item }.to_string();
    let entry = format!("impls::{name}(input)");

    let context = TemplateContext {
        package_name: package_name.clone(),
        package_extra: String::new(),
        source_metadata,
        entry,
        impls,
    };

    let output_dir = PathBuf::from(path::OUT_DIR)
        .join("generated")
        .join(name.to_string());
    let generated = template::render_crate(&output_dir, &context, config.split_cache)?;
    let dylib = dylib::compile_crate(&generated, BuildProfile::Release)?;

    debug!("generated crate: {}", generated.source_dir.display());

    let path = proc_macro2::Literal::string(&dylib.dylib_path.display().to_string());
    let source_hash = proc_macro2::Literal::string(&generated.source_hash);

    // Using mixed site to resolve `$crate`.
    let crate_proxy = quote_spanned! { Span::mixed_site() =>
        $crate::proxy!
    };
    let out = quote! {
        macro_rules! #name {
            ($($args:tt)*) => {
                #crate_proxy{#path, #source_hash, $($args)*}
            };
        }
    };

    debug!("out: {}", out);
    // debug!("env vars: {}", get_env_vars()?);
    Ok(out)
}

pub fn proxy_impl(input: proc_macro2::TokenStream) -> Result<proc_macro2::TokenStream> {
    let ProxyInput {
        dylib_path,
        source_hash,
        tokens,
    } = syn::parse2(input)?;
    dylib::load_and_run_entry(
        std::path::Path::new(&dylib_path.value()),
        &source_hash.value(),
        tokens,
    )
}

// fn get_env_vars() -> Result<String> {
//     let env_vars = std::env::vars()
//         .map(|(key, value)| format!("{}={}", key, value))
//         .collect::<Vec<_>>()
//         .join("\n");
//     Ok(env_vars)
// }
