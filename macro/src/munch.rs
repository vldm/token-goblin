use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};

use crate::Result;

/// How to emit debug information during macro expansion.
pub enum DebugMode {
    /// Source macro is expected to produce items and we can emit extra items with debug information.
    Items,
    /// Source macro is expected to produce expression so we need to wrap extra items into a block.
    Expression,
}
pub struct Config {
    pub cache: bool,
    pub split_cache: bool,
    pub debug: Option<DebugMode>,
}
impl Config {
    fn from_attrs(args: TokenStream) -> Result<Self> {
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

fn function_impl(config: Config, item: syn::ItemFn) -> Result<TokenStream> {
    let name = &item.sig.ident;

    // Using mixed site to resolve `$crate`.
    let crate_proxy = quote_spanned! { Span::mixed_site() =>
        $crate::proxy!
    };
    let out = quote! {
        macro_rules! #name {
            ($($args:tt)*) => {
                #crate_proxy{#item, $($args)*}
            };
        }
    };

    debug!("out: {}", out);
    debug!("env vars: {}", get_env_vars()?);
    Ok(out)
}

fn get_env_vars() -> Result<String> {
    let env_vars = std::env::vars()
        .map(|(key, value)| format!("{}={}", key, value))
        .collect::<Vec<_>>()
        .join("\n");
    Ok(env_vars)
}
