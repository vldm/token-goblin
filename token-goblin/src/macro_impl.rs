use std::{path::PathBuf, str::FromStr};

use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote, quote_spanned};
use syn::{Token, spanned::Spanned};

use crate::{
    Result,
    dylib::{self, BuildProfile},
    metadata, path,
    template::{self, TemplateContext},
};

pub struct ProxyArgs {
    pub dylib_path: syn::LitStr,
    pub source_hash: syn::LitStr,
}
impl syn::parse::Parse for ProxyArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        syn::braced!(content in input);

        let dylib_path = content.parse()?;
        content.parse::<syn::Token![,]>()?;
        let source_hash = content.parse()?;
        Ok(Self {
            dylib_path,
            source_hash,
        })
    }
}
impl ToTokens for ProxyArgs {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let brace = syn::token::Brace::default();
        brace.surround(tokens, |tokens| {
            let comma = syn::token::Comma::default();

            tokens.extend(self.dylib_path.to_token_stream());
            tokens.extend(comma.into_token_stream());
            tokens.extend(self.source_hash.to_token_stream());
        });
    }
}

pub struct ProxyInput {
    pub proxy_args: ProxyArgs,
    pub tokens: proc_macro2::TokenStream,
}

impl syn::parse::Parse for ProxyInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let proxy_args = input.parse()?;

        let tokens = if input.is_empty() {
            proc_macro2::TokenStream::new()
        } else {
            input.parse::<syn::Token![,]>()?;
            input.parse()?
        };
        Ok(Self { proxy_args, tokens })
    }
}

/// How to emit debug information during macro expansion.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DebugMode {
    /// Source macro is expected to produce items and we can emit extra items with debug information.
    Item,
    /// Source macro is expected to produce expression so we need to wrap extra items into a block.
    Expression,
}
impl FromStr for DebugMode {
    type Err = syn::Error;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "item" => Ok(DebugMode::Item),
            "expression" | "expr" => Ok(DebugMode::Expression),
            _ => Err(error!(Span::call_site() => "Unknown debug mode: {}", s)),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Config {
    // If set to false, we add source-hash to output path
    // This enforces recompilation of the macro for each change in the source code.
    pub cache: bool,
    // whether we need to use per crate `build-dir`
    pub split_cache: bool,
    // Cargo build profile
    pub profile: BuildProfile,
    // How to emit debug information during macro expansion.
    pub debug: Option<DebugMode>,
}

impl syn::parse::Parse for Config {
    // parse key=value, comma separated pairs,
    // boolean values can skip arguments
    // debug provided as ident, either `item` or `expr`
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut config = Self::default();
        while !input.is_empty() {
            let key = input.parse::<syn::Ident>()?;
            let value = if input.peek(syn::Token![=]) {
                input.parse::<syn::Token![=]>()?;
                input.parse::<syn::Lit>()?
            } else {
                syn::Lit::Bool(syn::LitBool::new(true, key.span()))
            };

            match key.to_string().as_str() {
                "cache" => config.cache = lit_to_bool(value)?,
                "split_cache" => config.split_cache = lit_to_bool(value)?,
                "profile" => {
                    config.profile =
                        lit_to_string(value).and_then(|s| BuildProfile::from_str(&s))?;
                }
                "debug" => {
                    config.debug =
                        Some(lit_to_string(value).and_then(|s| DebugMode::from_str(&s))?);
                }
                _ => return Err(error!(key.span() => "Unknown key: {}", key)),
            }

            if input.is_empty() {
                break;
            }
            input.parse::<syn::Token![,]>()?;
        }
        Ok(config)
    }
}
fn lit_to_bool(lit: syn::Lit) -> Result<bool> {
    match lit {
        syn::Lit::Bool(lit) => Ok(lit.value()),
        _ => Err(error!(lit.span() => "Expected boolean value")),
    }
}
fn lit_to_string(lit: syn::Lit) -> Result<String> {
    match lit {
        syn::Lit::Str(lit) => Ok(lit.value()),
        _ => Err(error!(lit.span() => "Expected string value")),
    }
}

impl Config {
    fn from_attrs(args: TokenStream) -> Result<Self> {
        debug!("config args: {}", args);
        syn::parse2(args)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            cache: true,
            split_cache: false,
            profile: BuildProfile::Release,
            debug: None,
        }
    }
}

pub fn munch_impl(args: TokenStream, item: TokenStream) -> Result<TokenStream> {
    timed!("munch_impl", {
        let item = syn::parse2::<syn::Item>(item)?;
        let config = Config::from_attrs(args)?;
        match item {
            syn::Item::Fn(item) => function_impl(config, item),
            syn::Item::Mod(item) => module_impl(config, item),
            // In case we need to support `macro foo {}` items
            // syn::Item::Verbatim(item) => macro_impl(config, item),
            // for macro_rules! syntax (both looks useless, since it's always easier
            // to implement custom `macro_rules!` )
            // syn::Item::Macro(item) => macro_impl(config, item),
            v => Err(error!(v.span() => "Expected function or module" )),
        }
    })
}

fn module_impl(config: Config, item: syn::ItemMod) -> Result<TokenStream> {
    let name = item.ident.clone();
    let context = TemplateContext::from_mod(item)?;
    build_and_compile_crate(&name, &context, config)
}

fn function_impl(config: Config, item: syn::ItemFn) -> Result<TokenStream> {
    debug!(
        "function attrs: {:?}",
        item.attrs
            .iter()
            .map(|attr| format!("{}", attr.to_token_stream()))
            .collect::<Vec<_>>()
    );
    let name = item.sig.ident.clone();

    let context = timed!("template_context", { TemplateContext::from_fn(item)? });
    build_and_compile_crate(&name, &context, config)
}

pub fn proxy_impl(input: proc_macro2::TokenStream) -> Result<proc_macro2::TokenStream> {
    timed!("proxy_impl", {
        let input: ProxyInput = syn::parse2(input)?;

        dylib::load_and_run_entry(
            std::path::Path::new(&input.proxy_args.dylib_path.value()),
            &input.proxy_args.source_hash.value(),
            input.tokens,
        )
    })
}

impl TemplateContext {
    fn from_fn(mut item: syn::ItemFn) -> Result<Self> {
        let name = &item.sig.ident;
        let package_name = format!("token-goblin-{}", name.to_string().replace('_', "-"));

        item.vis = syn::Visibility::Public(syn::token::Pub::default());

        let context = TemplateContext {
            package_name: package_name.clone(),
            package_extra: String::new(),
            source_metadata: metadata::load_dependencies()?,
            entry: format!("impls::{name}(input)"),
            impls: quote! { #item }.to_string(),
        };

        Ok(context)
    }

    fn from_mod(mut item: syn::ItemMod) -> Result<Self> {
        let name = &item.ident;
        let package_name = format!("token-goblin-{}", name.to_string().replace('_', "-"));

        /// rebuild all sub items to public
        let Some((b, mut content)) = item.content else {
            return Err(error!(item.span() => "Expected module content"));
        };

        let mut entries = Vec::new();

        for item in &mut content {
            match item {
                syn::Item::Fn(item) => {
                    item.vis = syn::Visibility::Public(syn::token::Pub::default());
                    entries.push(quote! { #item });
                }
                _ => {
                    return Err(error!(item.span() => "Expected function"));
                }
            }
        }

        if entries.is_empty() {
            return Err(error!(b.span.join() => "Expected at least one function"));
        }

        todo!("Convert items to match impls block");

        Ok(TemplateContext {
            package_name: package_name.clone(),
            package_extra: String::new(),
            source_metadata: metadata::load_dependencies()?,
            entry: format!("impls::{name}(input)"),
            impls: quote! { #(#content)* }.to_string(),
        })
    }
}

/// Build crate from template, and compile it to dylib.
/// Use name to calculate output path, and include source hash if needed.
fn build_and_compile_crate(
    name: &syn::Ident,
    template_context: &TemplateContext,
    config: Config,
) -> Result<TokenStream> {
    let (output_dir, stable) = path::calculate_generated_path(name);

    debug!("path_is_stable: {}, config.cache: {}", stable, config.cache);

    // If user enforces no-cache, or we cannot find macro declaration path
    // we need to include the source hash
    let include_source_hash = !(config.cache && stable);

    let generated = template::render_crate(
        &output_dir,
        template_context,
        config.split_cache,
        include_source_hash,
    )?;

    let dylib = timed!("compile_crate", {
        dylib::compile_crate(&generated, config.profile)?
    });

    debug!("generated crate: {}", generated.source_dir.display());

    let proxy_input = ProxyArgs {
        dylib_path: syn::LitStr::new(&dylib.dylib_path.display().to_string(), Span::call_site()),
        source_hash: syn::LitStr::new(&generated.source_hash, Span::call_site()),
    };

    // Using mixed site to resolve `$crate`.
    let crate_proxy = quote_spanned! { Span::mixed_site() =>
        $crate::proxy!
    };
    let out = quote! {
        macro_rules! #name {
            ($($args:tt)*) => {
                #crate_proxy{#proxy_input, $($args)*}
            };
        }
    };

    debug!("out: {}", out);
    if crate::DEBUG_ENV {
        debug!("env vars: {}", get_env_vars());
    }
    let span: proc_macro::Span = name.span().unwrap();
    debug!(
        "span_source_file: {}, {:?}, line: {}",
        span.file(),
        span.local_file(),
        span.line()
    );
    Ok(out)
}

fn get_env_vars() -> String {
    std::env::vars()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("\n")
}
