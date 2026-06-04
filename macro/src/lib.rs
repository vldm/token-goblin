use proc_macro::TokenStream;

mod metadata;
#[proc_macro]
pub fn proxy(input: TokenStream) -> TokenStream {
    input
}

#[proc_macro_attribute]
pub fn munch(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[proc_macro_derive(Snif)]
pub fn snif(input: TokenStream) -> TokenStream {
    input
}
