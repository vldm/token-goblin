use std::{collections::HashSet, fmt::Debug, iter, path::PathBuf, str::FromStr};

use proc_macro2::{Delimiter, Group, Span, TokenStream, TokenTree};
use quote::{ToTokens, format_ident, quote, quote_spanned};
use syn::{Attribute, Token, parse::Parser, punctuated::Punctuated, spanned::Spanned};

use crate::{
    Result,
    dylib::{self, BuildProfile, GeneratedCrate},
    ide_support::{self, is_lazy},
    metadata, path,
    rust_mod_fs::SpanLocation,
    syn_items,
    template::{self, TemplateContext},
};

// ===============================
// Parse objects
// ===============================

///
/// The way how proxy macro is used.
///
pub enum ProxyMode {
    /// Proc-macro code is already compiled into dylib.
    Precompiled { dylib_path: syn::LitStr },
    //TODO: avoid double parse
    /// Proc-macro code need to be compiled before use.
    Lazy { config: Group, src: Group },
}

/// Full list of internal arguments to `proxy` macro.
/// `{$mode, $macro_name}`
pub struct ProxyArgs {
    /// The whole arguments list is braced.
    pub _brace: syn::token::Brace,
    /// The way how proxy macro is used.
    pub mode: ProxyMode,
    pub _comma: Token![,],
    /// Name of the macro:
    /// used in case, where dylib contain multiple macros.
    pub macro_name: syn::Ident,
}
/// Typed version of params to `proxy!{$($proxy_input)*}` macro.
pub struct ProxyInput {
    /// Internal arguments.
    pub proxy_args: ProxyArgs,
    /// The input to the charm.
    pub tokens: proc_macro2::TokenStream,
}

/// Whether to enforce lazy expansion of `munch` macro.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Lazieness {
    Enforced,
    Disabled,
    /// By default enabled for IDE only
    #[default]
    Default,
}

/// Part of config.
/// Specify how to extend `Cargo.toml` for generated crate.

#[derive(Clone, Debug, Default)]
pub struct ExtraMetadata {
    ///
    /// List of dependencies to include in generated crate.
    ///
    /// By default we use only `[dev-dependencies]` in generated crate,
    /// and any entry in this list will be additionally searched in `[dependencies]` section.
    ///
    pub dependencies: HashSet<String>,
    /// If set to true, only set `dependencies` will be used.
    pub strict_dependencies: bool,
    /// If set, duplicate dependencies would be filtered out.
    /// In case if depdendency was declared in dev-dependencies, and in dependencies,
    /// only dev-dependency will be used.
    pub skip_duplicate: bool,
}
/// Configuration of `munch` macro.
/// provided as extra arguments:
/// ```
/// #[token_goblin::munch(lazy = true, incremental = false, split_cache = true, profile = "release")]
/// fn inner_function(_: TokenStream) -> TokenStream {
///   //..
///   # todo!()
/// }
///
/// ```
#[derive(Clone, Debug)]
pub struct Config {
    /// If set to false, we add source-hash to output path
    /// This enforces recompilation of the macro for each change in the source code.
    pub incremental: bool,
    /// If set to lazy, build is done on proxy side.
    /// This is default for IDE expansion, and can be (experimentally) enforced per macro.
    pub lazy: Lazieness,
    /// whether we need to use per crate `build-dir`
    pub split_cache: bool,
    /// Cargo build profile
    pub profile: BuildProfile,

    /// If set to true, we do not emit ide helper module.
    pub no_ide_helper: bool,

    /// Extra metadata to include in generated crate.
    ///
    /// Extend config with next options:
    /// - `dependencies` - list of dependencies to include in generated crate.
    /// - `strict_dependencies` - if set, only dependencies from `dependencies` section will be used.
    /// - `skip_duplicate_dependencies` - if set, duplicate dependencies will be filtered out.
    ///   In case if depdendency was declared in dev-dependencies, and in dependencies,
    ///   only dev-dependency will be used.
    pub extra_metadata: ExtraMetadata,
}

/// Arguments to `Spit` derive macro.
/// provided as extra attributes: `#[charm(path_to_macro)]`
/// ```
/// # macro_rules! path_to_macro {
/// #   ($($tt:tt)*) => { }
/// # }
///
/// #[derive(token_goblin::Spit)]
/// #[charm(path_to_macro)]
/// struct MyStruct {
///   field: i32,
/// }
/// ```
struct SpitArgs {
    pub list_of_macros: Vec<syn::Path>,
}

// The input to `snif` macro.
pub struct SnifInput {
    /// User called macro like this: `snif!(MyStruct, OtherStruct in some_macro!("extra tokens"))`
    chain: Punctuated<syn::Path, Token![,]>,
    _in_token: Token![in],
    macro_path: syn::Path,
    _exclamation: Token![!],
    macro_args: proc_macro2::Group,
}

// ===============================
// Macro impls
// ===============================
#[allow(clippy::needless_pass_by_value, reason = "better api")]
pub fn munch_impl(args: TokenStream, item_tts: TokenStream) -> Result<TokenStream> {
    let config = Config::from_attrs(args.clone())?;

    let item = syn::parse2::<syn_items::Item>(item_tts.clone())?;
    let source_metadata = metadata::load_dependencies(&config.extra_metadata)?;
    let context = build_template(item, source_metadata)?;

    if is_lazy(&config) {
        let mod_name = context
            .mod_name
            .as_ref()
            .map_or_else(|| format_ident!("global"), |(_, name)| name.clone());
        let fn_entries = expand_entries(&mod_name, &context.entries, |e| {
            ProxyArgs::lazy(e.clone(), &args, &item_tts)
        });

        let ide_helper_mod = ide_support::emit_ide_helper_mod(&context, &config);
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

pub fn proxy_impl(input: proc_macro2::TokenStream) -> Result<proc_macro2::TokenStream> {
    debug!("proxy input: {}", input);
    let input: ProxyInput = syn::parse2(input)?;

    debug!("proxy input parsed: {:?}", input);
    let dylib_path = match input.proxy_args.mode {
        ProxyMode::Precompiled { dylib_path } => PathBuf::from(dylib_path.value()),
        ProxyMode::Lazy { config, src } => {
            let config = Config::from_attrs(config.stream())?;
            let input: syn_items::Item = syn::parse2(src.stream())?;
            let source_metadata = metadata::load_dependencies(&config.extra_metadata)?;
            let template = build_template(input, source_metadata)?;
            let build_result = BuildContext::render_and_compile(template, config)?;

            if !build_result.compile_error.is_empty() {
                let compile_error = &build_result.compile_error;
                return Ok(quote! {
                    #compile_error
                });
            }
            build_result.dylib_path
        }
    };

    dylib::load_and_run_entry(&dylib_path, &input.proxy_args.macro_name, input.tokens)
}

#[allow(clippy::needless_pass_by_value, reason = "consistent api")]
#[allow(clippy::unnecessary_wraps, reason = "consistent api")]
pub fn spit_impl(attr: TokenStream, item: TokenStream) -> Result<TokenStream> {
    Ok(quote! {
        #attr!{#item}
    })
}
/// Extract `spit` attributes from input, and return token stream with them called.
pub fn spit_derive_impl(input: TokenStream) -> Result<TokenStream> {
    let original_input = input.clone();
    let list_of_macros = syn::parse2::<SpitArgs>(input)?.list_of_macros;

    Ok(quote! {
        #(#list_of_macros!{#original_input})*
    })
}
/// Generate macro with same name
#[allow(clippy::needless_pass_by_value, reason = "consistent api")]
pub fn derive_snif_impl(input: TokenStream) -> Result<TokenStream> {
    // TODO: Give user a way to customize the macro name.
    // TODO: Rewrite to some simpler form, cause we only need a name and visibility of item.
    let any_item = syn::parse2::<syn::Item>(input.clone())?;

    let (visibility, name) = match any_item {
        syn::Item::Fn(item) => (item.vis, item.sig.ident),
        syn::Item::Mod(item) => (item.vis, item.ident),
        syn::Item::Struct(item) => (item.vis, item.ident),
        syn::Item::Enum(item) => (item.vis, item.ident),
        syn::Item::Union(item) => (item.vis, item.ident),
        syn::Item::Trait(item) => (item.vis, item.ident),
        _ => bail!(any_item.span() => "Expected function, module, struct, enum, or trait"),
    };
    let macro_glob = if matches!(visibility, syn::Visibility::Public(_)) {
        quote! {#[macro_export]}
    } else {
        quote! {}
    };

    let macro_name = format_ident!("{}_{}", name, postfix_hash(name.span()));

    let res = quote! {
        #macro_glob
        #[doc(hidden)]
        #[allow(unused)]
        macro_rules! #macro_name {
            (@token_goblin [($($next:tt)+) $(=> $rest:tt)*] [$($other:tt)*]) => {
                $($next)+! (@token_goblin
                    [$($rest)*]
                    [
                        $($other)*
                        {#input}
                    ]
                )
            };
            // empty input, just return input tokens.
            () => {
                #input
            };
            ($($any:tt)*) => {core::compile_error!("This macro should be used only from token-goblin::snif")};
        }

        #visibility use #macro_name as #name;
    };
    Ok(res)
}

// attribute `#[derive_snif]` - that work like `#[derive(Snif)]` but allows any item.
#[allow(clippy::needless_pass_by_value, reason = "consistent api")]
pub fn derive_snif_attr_impl(input: TokenStream) -> Result<TokenStream> {
    let resulted_macro = derive_snif_impl(input.clone())?;

    // Since attribute macro consumes tokens, return original input as well.
    // Note: if macro returns `Err` - original input will persist as well.
    Ok(quote::quote! {
        #input
        #resulted_macro
    })
}

pub fn snif_impl(input: TokenStream) -> Result<TokenStream> {
    debug!("snif expand input: {}", input);
    let SnifInput {
        chain,
        macro_path,
        macro_args,
        ..
    }: SnifInput = syn::parse2(input)?;

    let chain_of_macros = chain
        .iter() // all macros in users chain
        .map(ToTokens::to_token_stream)
        // the end macro itself
        // repeat it once more, to allow macro_call itself with normalized arguments
        .chain(iter::repeat_n(macro_path.to_token_stream(), 2))
        .collect::<Vec<_>>();

    let (first, rest) = chain_of_macros
        .split_first()
        .expect("at least snif should exist");

    let macro_args = macro_args.stream();

    let x = quote! {
        #first!
         {
            @token_goblin
            [#( (#rest) ) => *] // the list of macros to chain
            [#macro_args ] // collected arguments
        }
    };

    debug!("snif expanded: {}", x);
    Ok(x)
}

// ===============================
// Integration glue of multiple components
// ===============================
fn build_template(
    item: syn_items::Item,
    source_metadata: metadata::Metadata,
) -> Result<TemplateContext> {
    let template = timed!("build_template", {
        match item {
            syn_items::Item::Fn(item) => TemplateContext::from_fn(item, source_metadata),
            syn_items::Item::Mod(item) => TemplateContext::from_mod(item, source_metadata),
            v @ syn_items::Item::Verbatim(_) => {
                bail!(v.span() => "Expected function or module" )
            }
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

fn render_template(context: &TemplateContext, config: &Config) -> Result<GeneratedCrate> {
    timed!("render_template", {
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
    })
}

impl TemplateContext {
    #[allow(clippy::unnecessary_wraps, reason = "consistent api")]
    fn from_fn(item: syn_items::ItemFn, source_metadata: metadata::Metadata) -> Result<Self> {
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
            source_metadata,
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

    fn from_mod(mod_item: syn_items::ItemMod, source_metadata: metadata::Metadata) -> Result<Self> {
        let name = &mod_item.ident;
        let package_name = format!("token-goblin-{}", name.to_string().replace('_', "-"));

        let Some((b, content)) = mod_item.content else {
            bail!(mod_item.span() => "Expected module content");
        };

        let mut entries = Vec::new();

        for item in &content {
            if let syn_items::Item::Fn(item) = item
                && Self::is_exportable(&item.vis)
            {
                // Only public functions are considered as entry points to token-goblin.
                entries.push(item.clone());
            }
        }

        if entries.is_empty() {
            bail!(b.span.join() => "Expected at least one function")
        }
        Ok(TemplateContext {
            package_name: package_name.clone(),
            package_extra: String::new(),
            source_metadata,
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
        let generated = render_template(&template_context, &config)?;
        let dylib_error = timed!("compile_crate", {
            dylib::compile_crate(&generated, config.profile)
        });

        let (dylib_path, compile_error) = match dylib_error {
            Ok(dylib) => {
                debug!("generated: {}", dylib.dylib_path.display());
                (dylib.dylib_path, TokenStream::new())
            }
            Err(e) => {
                debug!("generated: failed to compile: {}", e);
                (PathBuf::new(), e.to_compile_error())
            }
        };

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

        let fn_entries = expand_entries(&mod_name, &self.template_context.entries, |name| {
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
            let span_location =
                SpanLocation::recover(self.template_context.name_span().unwrap()).unwrap();
            writeln!(
                &mut comments,
                "///   Module path: {}",
                span_location.module_path().to_token_stream()
            )
            .ok();
            writeln!(
                &mut comments,
                "///   File path: {}",
                span_location.file_path().display()
            )
            .ok();
            // writeln!(&mut comments, "///   envs: \n").ok();
            // get_env_vars().split('\n').for_each(|line| {
            //     writeln!(&mut comments, "///   {line}").ok();
            // });
            TokenStream::from_str(&comments).unwrap()
        };
        let ide_helper_mod = ide_support::emit_ide_helper_mod(&self.template_context, &self.config);
        (ide_helper_mod, compile_info_docs)
    }
}

fn expand_entries(
    mod_name: &syn::Ident,
    entries: &[syn_items::ItemFn],
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

        let snif_branch = quote! {
            // The task of this branch is to normalize the input
            // (@token_goblin [($($next:tt)+) $(=> $rest:tt)*] [$($other:tt)*]) => {
            (@token_goblin [($($me:tt)*)] // the list of macros to chain
            [$($macro_args:tt)*] ) => {
                $($me)*! {$($macro_args)*}
            }; // collected arguments
            (@token_goblin [$($more:tt)*] $($any:tt)*) => {
                core::compile_error!(
                    concat!("Unexpected input in token-goblin::snif", "got extra chains: ", stringify!($($more:tt)*)))
            };
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
                #snif_branch
                ($($args:tt)*) => {
                    #crate_proxy{#proxy_input, $($args)*}
                };
            }

            #visibility use #macro_name as #name;
        });
    }
    out
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

impl Config {
    fn from_attrs(args: TokenStream) -> Result<Self> {
        debug!("config args: {}", args);
        syn::parse2(args)
    }
}

// ===============================
// Trait impls
// ===============================
// Parse
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

impl syn::parse::Parse for Config {
    // parse key=value, comma separated pairs,
    // boolean values can skip arguments
    // debug provided as ident, either `item` or `expr`
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut config = Self::default();
        while !input.is_empty() {
            let key = input.parse::<syn::Ident>()?;
            let value: Option<TokenTree> = if input.peek(syn::Token![=]) {
                input.parse::<syn::Token![=]>()?;
                Some(input.parse::<TokenTree>()?)
            } else {
                None
            };

            match key.to_string().as_str() {
                "incremental" => config.incremental = parse_lit_bool(value)?,
                "split_cache" => config.split_cache = parse_lit_bool(value)?,
                "lazy" => {
                    config.lazy = if parse_lit_bool(value)? {
                        Lazieness::Enforced
                    } else {
                        Lazieness::Disabled
                    }
                }
                "dependencies" => {
                    config
                        .extra_metadata
                        .dependencies
                        .extend(parse_array_lit_str(value)?);
                }
                "strict_dependencies" => {
                    config.extra_metadata.strict_dependencies = parse_lit_bool(value)?;
                }
                "skip_duplicate_dependencies" => {
                    config.extra_metadata.skip_duplicate = parse_lit_bool(value)?;
                }
                "no_ide_helper" => {
                    config.no_ide_helper = parse_lit_bool(value)?;
                }
                "profile" => {
                    config.profile =
                        parse_lit_str(value).and_then(|s| BuildProfile::from_str(&s))?;
                }
                _ => bail!(key.span() => "Unknown key: {}", key),
            }

            if input.is_empty() {
                break;
            }
            input.parse::<syn::Token![,]>()?;
        }
        Ok(config)
    }
}

impl syn::parse::Parse for SpitArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let attrs = Attribute::parse_outer(input)?;

        let mut list_of_macros: Vec<syn::Path> = Vec::new();
        for attr in attrs {
            if !attr.path().is_ident("charm") {
                continue;
            }
            let syn::Meta::List(list) = attr.meta else {
                continue;
            };
            list_of_macros.push(syn::parse2(list.tokens.clone())?);
        }
        debug!(
            "list_of_macros: {}",
            list_of_macros
                .iter()
                .map(|attr| attr.to_token_stream().to_string())
                .collect::<Vec<_>>()
                .join("\n")
        );
        while !input.is_empty() {
            // consume item
            let _ = input.parse::<TokenTree>()?;
        }
        Ok(Self { list_of_macros })
    }
}

impl syn::parse::Parse for SnifInput {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let parse_punctuated_until_in =
            |input: syn::parse::ParseStream| -> Result<Punctuated<syn::Path, Token![,]>> {
                let mut punctuated = Punctuated::new();

                loop {
                    let value = syn::Path::parse(input)?;
                    punctuated.push_value(value);
                    if input.peek(Token![in]) {
                        break;
                    }
                    let punct = input.parse()?;
                    punctuated.push_punct(punct);
                }

                Ok(punctuated)
            };

        Ok(SnifInput {
            chain: parse_punctuated_until_in(input)?,
            _in_token: input.parse()?,
            macro_path: input.parse()?,
            _exclamation: input.parse()?,
            macro_args: input.parse()?,
        })
    }
}
// Parse boolean value from token tree
fn parse_lit_bool(lit: Option<TokenTree>) -> Result<bool> {
    let Some(lit) = lit else {
        return Ok(true);
    };
    let lit: syn::LitBool = syn::parse2(lit.into_token_stream())?;
    Ok(lit.value())
}

fn parse_lit_str(lit: Option<TokenTree>) -> Result<String> {
    let Some(lit) = lit else {
        return Ok(String::new());
    };
    let lit: syn::LitStr = syn::parse2(lit.into_token_stream())?;
    Ok(lit.value())
}

fn parse_array_lit_str(group: Option<TokenTree>) -> Result<Vec<String>> {
    let Some(group) = group else {
        return Ok(Vec::new());
    };
    let out = match group {
        TokenTree::Group(group) => {
            let parser = Punctuated::<syn::LitStr, Token![,]>::parse_terminated;
            parser
                .parse2(group.stream())?
                .into_iter()
                .map(|lit| lit.value())
                .collect()
        }
        _ => bail!(group.span() => "Expected array literal"),
    };
    Ok(out)
}
// ToTokens
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

// Std traits
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

impl Debug for ProxyInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ProxyInput {{ proxy_args: {:?}, tokens: \"{}\" }}",
            self.proxy_args, self.tokens
        )
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            incremental: true,
            split_cache: false,
            no_ide_helper: false,
            lazy: Lazieness::default(),
            profile: BuildProfile::default(),
            extra_metadata: ExtraMetadata::default(),
        }
    }
}

// Location hash to prevent collisions in macro names
// (when used pub crate, and require #[macro_export] to be visible).
fn postfix_hash(span: Span) -> String {
    let span = span.unwrap();

    let span_location = SpanLocation::recover(span).unwrap();
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"postfix");
    hasher.update(span_location.crate_name().as_bytes());
    hasher.update(
        span_location
            .module_path()
            .to_token_stream()
            .to_string()
            .as_bytes(),
    );
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
