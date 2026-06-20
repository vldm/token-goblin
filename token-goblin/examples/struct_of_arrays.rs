//! This example shows, how token-goblin can help you building struct of arrays.
//!
//! It implements something similar to `MultiArrayList` from zig.
//!
//! This example convert a struct with named fields into a struct of arrays.
//! And implements push and pop methods for it.


// The macro `multi_array_vec` converts a struct with named fields into a struct of arrays.
// ```
// struct SoaFoo {
//     name: Vec<String>,
//     x: Vec<f64>,
//     y: Vec<f64>,
//     health: Vec<u16>,
// }
// ```

/// Some docs to Foo.
#[derive(token_goblin::Snif)]
pub struct Foo {
    name: String,
    x: f64,
    y: f64,
    health: u16,
}


#[token_goblin::munch]
mod soa {
    use syn::braced;

    struct SoaInput {
        name: syn::Ident,
        // we only interested in structures.
        item: syn::ItemStruct,
    }

    impl syn::parse::Parse for SoaInput {
        // The argument for macro is in form of `name {itemdefinition..}`
        fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
            let name = input.parse()?;
            let content;
            let _brace_token = braced!(content in input);
            let item = content.parse()?;
            Ok(SoaInput { name, item })
        }
    }
    /// 1. Parse structure definition and name for new structure.
    /// 2. Generate new structure in a form of
    /// ```
    /// struct #name {
    ///     name: Vec<String>,
    ///     x: Vec<f64>,
    ///     y: Vec<f64>,
    ///     health: Vec<u16>,
    /// }
    /// ```
    pub fn multi_array_vec(input: TokenStream) -> TokenStream {
        let SoaInput { name, item } = syn::parse2(input).expect("Failed to parse input");

        let syn::Fields::Named(named_fields) = item.fields else {
            panic!("Expected struct with named fields");
        };

        let (field_defs, field_inits): (Vec<TokenStream>, Vec<TokenStream>) = named_fields
            .named
            .iter()
            .map(|field| {
                let field_name = field.ident.clone().expect("field should exist");
                let ty = field.ty.clone();
                (
                    quote! {
                        #field_name: Vec<#ty>
                    },
                    quote! {
                        #field_name: Vec::<#ty>::new()
                    },
                )
            })
            .unzip();

        let visibility = &item.vis;

        quote! {
            // TODO: Extra derive based on fields.
            #[derive(Default)]
            #visibility struct #name {
                #(#field_defs),*
            }
            impl #name {
                pub fn new() -> Self {
                    Self {
                        #(#field_inits),*
                    }
                }
            }
        }
    }

    // The snif macro allows made implementations more modular.
    // e.g. by extracting some of impl methods into separate macros.

    /// Implement push for original structure, and pop into original structure.
    pub fn push_pop_impl(input: TokenStream) -> TokenStream {
        let SoaInput { name, item } = syn::parse2(input).expect("Failed to parse input");

        let syn::Fields::Named(named_fields) = item.fields else {
            panic!("Expected struct with named fields");
        };
        let item_name = item.ident.clone();

        let field_names = named_fields
            .named
            .iter()
            .map(|field| field.ident.clone().expect("field should exist"))
            .collect::<Vec<_>>();

        let any_field = field_names.first().cloned().into_iter();
        quote! {
            impl #name {
                pub fn push(&mut self, item: #item_name) {
                    #(self.#field_names.push(item.#field_names));*
                }
                pub fn pop(&mut self) -> Option<#item_name> {
                    #( if self.#any_field.is_empty()  {
                        return None;
                    })*

                    #(let #field_names = self.#field_names.pop().unwrap();)*
                    Some(#item_name {
                        #(#field_names),*
                    })
                }
            }
        }
    }
}

token_goblin::snif!(Foo in soa::multi_array_vec!(SoaFoo));
token_goblin::snif!(Foo in soa::push_pop_impl!(SoaFoo));

#[allow(clippy::float_cmp)]
fn main() {
    let mut collection = SoaFoo::new();
    collection.push(Foo {
        name: "John".to_string(),
        x: 1.0,
        y: 2.0,
        health: 100,
    });
    collection.push(Foo {
        name: "Jane".to_string(),
        x: 3.0,
        y: 4.0,
        health: 200,
    });

    let item = collection.pop().expect("Expected item to be present");
    assert_eq!(item.name, "Jane");
    assert_eq!(item.x, 3.0);
    assert_eq!(item.y, 4.0);
    assert_eq!(item.health, 200);
}
