use std::{fmt::Debug, path::PathBuf, str::FromStr};

use proc_macro2::{Delimiter, Group, Span, TokenStream};
use quote::{ToTokens, format_ident, quote, quote_spanned};
use syn::{Token, spanned::Spanned};

use crate::{
    Result,
    dylib::{self, BuildProfile, GeneratedCrate},
    ide_support::{self, is_lazy},
    metadata, path,
    template::{self, TemplateContext},
};

pub enum ProxyMode {
    Precompiled { dylib_path: syn::LitStr },
    //TODO: avoid double parse
    Lazy { config: Group, src: Group },
}

pub struct ProxyArgs {
    pub _brace: syn::token::Brace,
    pub mode: ProxyMode,
    pub _comma: Token![,],
    pub macro_name: syn::Ident,
}
impl ProxyArgs {
    fn compiled(dylib_path: syn::LitStr, macro_name: syn::Ident) -> Self {
        Self {
            _brace: syn::token::Brace::default(),
            mode: ProxyMode::Precompiled { dylib_path },
            _comma: syn::token::Comma::default(),
            macro_name,
        }
    }
    pub fn lazy(macro_name: syn::Ident, config: &TokenStream, src: &TokenStream) -> Self {
        Self {
            _brace: syn::token::Brace::default(),
            mode: ProxyMode::Lazy {
                config: Group::new(Delimiter::Brace, config.clone()),
                src: Group::new(Delimiter::Brace, src.clone()),
            },
            _comma: syn::token::Comma::default(),
            macro_name,
        }
    }
}

impl syn::parse::Parse for ProxyMode {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(syn::LitStr) {
            let dylib_path = input.parse()?;
            Ok(ProxyMode::Precompiled { dylib_path })
        } else {
            let config = input.parse()?;
            let src = input.parse()?;
            Ok(ProxyMode::Lazy { config, src })
        }
    }
}
impl syn::parse::Parse for ProxyArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            _brace: syn::braced!(content in input),
            mode: ProxyMode::parse(&content)?,
            _comma: content.parse()?,
            macro_name: content.parse()?,
        })
    }
}
impl ToTokens for ProxyMode {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            ProxyMode::Precompiled { dylib_path } => {
                tokens.extend(dylib_path.to_token_stream());
            }
            ProxyMode::Lazy { config, src } => {
                tokens.extend(config.to_token_stream());
                tokens.extend(src.to_token_stream());
            }
        }
    }
}
impl ToTokens for ProxyArgs {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let brace = syn::token::Brace::default();
        brace.surround(tokens, |tokens| {
            tokens.extend(self.mode.to_token_stream());
            tokens.extend(syn::token::Comma::default().to_token_stream());
            tokens.extend(self.macro_name.to_token_stream());
        });
    }
}

impl Debug for ProxyMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProxyMode::Precompiled { dylib_path } => {
                write!(f, "Precompiled {{ dylib_path: {} }}", dylib_path.value())
            }
            ProxyMode::Lazy { config, src } => {
                write!(f, "Lazy {{ config: {config}, src: {src} }}")
            }
        }
    }
}

impl Debug for ProxyArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ProxyArgs {{ macro_name: {:?}, mode: {:?} }}",
            self.macro_name, self.mode
        )
    }
}

#[derive(Debug)]
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Lazieness {
    Enforced,
    Disabled,
    // By default enabled for IDE only
    #[default]
    Default,
}
#[derive(Copy, Clone, Debug)]
pub struct Config {
    // If set to false, we add source-hash to output path
    // This enforces recompilation of the macro for each change in the source code.
    pub incremental: bool,

    // If set to lazy, build is done on proxy side.
    // This is default for IDE expansion, and can be (experimentally) enforced per macro.
    pub lazy: Lazieness,
    // whether we need to use per crate `build-dir`
    pub split_cache: bool,
    // Cargo build profile
    pub profile: BuildProfile,
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
                "incremental" => config.incremental = lit_to_bool(value)?,
                "split_cache" => config.split_cache = lit_to_bool(value)?,
                "lazy" => {
                    config.lazy = if lit_to_bool(value)? {
                        Lazieness::Enforced
                    } else {
                        Lazieness::Disabled
                    }
                }
                "profile" => {
                    config.profile =
                        lit_to_string(value).and_then(|s| BuildProfile::from_str(&s))?;
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
            incremental: true,
            split_cache: false,
            lazy: Lazieness::default(),
            profile: BuildProfile::default(),
        }
    }
}
#[allow(clippy::needless_pass_by_value, reason = "better api")]
pub fn munch_impl(args: TokenStream, item_tts: TokenStream) -> Result<TokenStream> {
    let config = Config::from_attrs(args.clone())?;

    let item = syn::parse2::<syn::Item>(item_tts.clone())?;
    let context = build_template(item)?;

    if is_lazy(config) {
        // return Ok(quote! {
        //     // #ide_helper_mod
        //     // #compile_info_docs
        //     const _: () = (); // add new item, to prevent `cargo expand` cleanup  (in case where we have only one macro_rules!).
        //     #out_mod
        // });
        let mod_name = context
            .mod_name
            .as_ref()
            .map_or_else(|| format_ident!("global"), |(_, name)| name.clone());
        let fn_entries = entries_impl(&mod_name, &context.entries, |e| {
            ProxyArgs::lazy(e.clone(), &args, &item_tts)
        });

        let ide_helper_mod = ide_support::emit_ide_helper_mod(&context);
        let out = if let Some((vis, _)) = &context.mod_name {
            quote! {
                #ide_helper_mod

                #vis mod #mod_name {
                    #(#fn_entries)*
                }
            }
        } else {
            quote! {
                #ide_helper_mod

                #(#fn_entries)*
            }
        };
        return Ok(out);
    }

    let build_result = BuildContext::render_and_compile(context, config)?;

    Ok(build_result.emit())
}

fn build_template(item: syn::Item) -> Result<TemplateContext> {
    let template = timed!("template_context", {
        match item {
            syn::Item::Fn(item) => TemplateContext::from_fn(item),
            syn::Item::Mod(item) => TemplateContext::from_mod(item),
            // In case we need to support `macro foo {}` items
            // syn::Item::Verbatim(item) => macro_impl(config, item),
            // for macro_rules! syntax (both looks useless, since it's always easier
            // to implement custom `macro_rules!` wrapper )
            // syn::Item::Macro(item) => macro_impl(config, item),
            v => Err(error!(v.span() => "Expected function or module" )),
        }
    })?;

    if crate::DEBUG_ENV {
        debug!("env vars: {}", get_env_vars());
        let span: proc_macro::Span = template.name_span().unwrap();
        debug!(
            "span_source_file: {}, {:?}, line: {}",
            span.file(),
            span.local_file(),
            span.line()
        );
    }
    Ok(template)
}

pub fn proxy_impl(input: proc_macro2::TokenStream) -> Result<proc_macro2::TokenStream> {
    debug!("proxy input: {}", input);
    let input: ProxyInput = syn::parse2(input)?;

    debug!("proxy input parsed: {:?}", input);
    let dylib_path = match input.proxy_args.mode {
        ProxyMode::Precompiled { dylib_path } => PathBuf::from(dylib_path.value()),
        ProxyMode::Lazy { config, src } => {
            let config = Config::from_attrs(config.stream())?;
            let input: syn::Item = syn::parse2(src.stream())?;
            let template = build_template(input)?;
            let build_result = BuildContext::render_and_compile(template, config)?;

            build_result.dylib_path
        }
    };

    dylib::load_and_run_entry(
        &dylib_path,
        &input.proxy_args.macro_name.to_string(),
        input.tokens,
    )
}

impl TemplateContext {
    fn from_fn(item: syn::ItemFn) -> Result<Self> {
        let name = &item.sig.ident;
        let package_name = format!("token-goblin-{}", name.to_string().replace('_', "-"));

        let generated_content = {
            let mut item2 = item.clone();
            item2.vis = syn::Visibility::Public(syn::token::Pub::default());
            item2.to_token_stream()
        };

        let context = TemplateContext {
            package_name: package_name.clone(),
            package_extra: String::new(),
            source_metadata: metadata::load_dependencies()?,
            generated_content,
            entries: vec![item],

            mod_name: None,
        };

        Ok(context)
    }

    // Only exportable if pub or pub(crate)/pub(super)
    fn is_exportable(vis: &syn::Visibility) -> bool {
        match vis {
            syn::Visibility::Public(_) => true,
            syn::Visibility::Restricted(restricted) => {
                restricted.path.segments.len() == 1
                    && (restricted.path.segments[0].ident == "crate"
                        || restricted.path.segments[0].ident == "super")
            }
            syn::Visibility::Inherited => false,
        }
    }

    fn from_mod(mod_item: syn::ItemMod) -> Result<Self> {
        let name = &mod_item.ident;
        let package_name = format!("token-goblin-{}", name.to_string().replace('_', "-"));

        let Some((b, content)) = mod_item.content else {
            return Err(error!(mod_item.span() => "Expected module content"));
        };

        let mut entries = Vec::new();

        for item in &content {
            if let syn::Item::Fn(item) = item
                && Self::is_exportable(&item.vis)
            {
                // Only public functions are considered as entry points to token-goblin.
                entries.push(item.clone());
            }
        }

        if entries.is_empty() {
            return Err(error!(b.span.join() => "Expected at least one function"));
        }
        Ok(TemplateContext {
            package_name: package_name.clone(),
            package_extra: String::new(),
            source_metadata: metadata::load_dependencies()?,
            entries,
            generated_content: quote! { #(#content)* },

            mod_name: Some((mod_item.vis.clone(), name.clone())),
        })
    }
}

struct BuildContext {
    config: Config,
    template_context: TemplateContext,

    generated: GeneratedCrate,
    dylib_path: PathBuf,
    compile_error: TokenStream,
}
impl BuildContext {
    /// Build crate from template as dylib.
    /// Use name to calculate output path, and include source hash if needed.
    pub fn render_and_compile(template_context: TemplateContext, config: Config) -> Result<Self> {
        let generated = render_template(&template_context, config)?;
        let dylib_error = timed!("compile_crate", {
            dylib::compile_crate(&generated, config.profile)
        });

        let (dylib_path, compile_error) = match dylib_error {
            Ok(dylib) => (dylib.dylib_path, TokenStream::new()),
            Err(e) => (PathBuf::new(), e.to_compile_error()),
        };

        debug!("generated crate: {}", generated.source_dir.display());

        Ok(Self {
            config,
            template_context,
            generated,
            dylib_path,
            compile_error,
        })
    }
    pub fn emit(self) -> TokenStream {
        let mod_name = self
            .template_context
            .mod_name
            .as_ref()
            .map_or_else(|| format_ident!("global"), |(_, name)| name.clone());

        let fn_entries = entries_impl(&mod_name, &self.template_context.entries, |name| {
            ProxyArgs::compiled(
                syn::LitStr::new(&self.dylib_path.display().to_string(), Span::call_site()),
                name.clone(),
            )
        });

        let (ide_helper_mod, compile_info_docs) = self.emit_helpers();
        let compile_error = self.compile_error;

        if let Some((vis, _)) = &self.template_context.mod_name {
            quote! {
                #ide_helper_mod
                #compile_error
                #compile_info_docs

                #vis mod #mod_name {
                    #(#fn_entries)*
                }
            }
        } else {
            quote! {
                #ide_helper_mod
                #compile_error

                #compile_info_docs
                const _: () = (); // add new item, to prevent `cargo expand` cleanup  (in case where we have only one macro_rules!).
                #(#fn_entries)*
            }
        }
    }
    fn emit_helpers(&self) -> (TokenStream, TokenStream) {
        // Build doc comments for compile info
        let compile_info_docs = {
            use std::fmt::Write;
            let mut comments = String::new();
            writeln!(&mut comments, "/// Compile info:").ok();
            writeln!(&mut comments, "///   Profile: {}", self.config.profile).ok();
            writeln!(
                &mut comments,
                "///   Split cache: {}",
                self.config.split_cache
            )
            .ok();
            writeln!(
                &mut comments,
                "///   Incremental: {}",
                self.config.incremental
            )
            .ok();
            writeln!(&mut comments, "///   Lazy: {:?}", self.config.lazy).ok();
            writeln!(
                &mut comments,
                "///   Generated crate: {}",
                self.generated.source_dir.display()
            )
            .ok();
            // writeln!(&mut comments, "///   envs: \n").ok();
            // get_env_vars().split('\n').for_each(|line| {
            //     writeln!(&mut comments, "///   {line}").ok();
            // });
            TokenStream::from_str(&comments).unwrap()
        };
        let ide_helper_mod = ide_support::emit_ide_helper_mod(&self.template_context);
        (ide_helper_mod, compile_info_docs)
    }
}

fn render_template(context: &TemplateContext, config: Config) -> Result<GeneratedCrate> {
    let (output_dir, stable) = path::calculate_generated_path(context.name_span());

    debug!(
        "path_is_stable: {}, config.cache: {}",
        stable, config.incremental
    );

    // If user enforces no-cache, or we cannot find macro declaration path
    // we need to include the source hash
    let include_source_hash = !(config.incremental && stable);

    template::render_crate(
        &output_dir,
        context,
        config.split_cache,
        include_source_hash,
    )
}

fn entries_impl(
    mod_name: &syn::Ident,
    entries: &[syn::ItemFn],
    proxy_input: impl Fn(&syn::Ident) -> ProxyArgs,
) -> Vec<TokenStream> {
    // Using mixed site to resolve `$crate`.
    let crate_proxy = quote_spanned! { Span::mixed_site() =>
        $crate::proxy!
    };

    let mut out = vec![];
    for entry in entries {
        let proxy_input = proxy_input(&entry.sig.ident);
        let visibility = &entry.vis;

        // Global macro can be only in pub mods, or if fn without mod.
        let macro_glob = if matches!(visibility, syn::Visibility::Public(_)) {
            quote! {#[macro_export]}
        } else {
            quote! {}
        };

        let name = &entry.sig.ident;
        let postfix = postfix_hash(name.span());
        let mod_name_str = mod_name.to_string();
        let macro_name = format_ident!("{}_{}_{}", mod_name_str, name, postfix);
        out.push(quote! {
            #macro_glob
            #[doc(hidden)]
            #[allow(unused)]
            macro_rules! #macro_name {
                ($($args:tt)*) => {
                    #crate_proxy{#proxy_input, $($args)*}
                };
            }

            #visibility use #macro_name as #name;
        });
    }
    out
}

// Location hash to prevent collisions in macro names
// (when used pub crate, and require #[macro_export] to be visible).
fn postfix_hash(span: Span) -> String {
    let span = span.unwrap();
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"postfix");
    hasher.update(span.file().as_bytes());
    hasher.update(&span.line().to_le_bytes());
    hasher.update(&span.column().to_le_bytes());
    hasher.finalize().to_hex().to_string()
}

fn get_env_vars() -> String {
    std::env::vars()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("\n")
}
