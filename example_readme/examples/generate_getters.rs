#[derive(token_goblin::Snif)]
struct Foo {
    x: i32,
}
#[token_goblin::munch(lazy)]
fn generate_getters(input: SnifedEntries) -> TokenStream {
    let syn::Item::Struct(item) = &input.entries[0].item else {
        return syn::Error::new(input.span(), "Expected struct").to_compile_error();
    };
    let name = &item.ident;
    let (fields, types): (Vec<syn::Ident>, Vec<syn::Type>) = item
        .fields
        .iter()
        .cloned()
        .map(|field| (field.ident.unwrap(), field.ty))
        .unzip();
    quote! {
        impl #name {
            #(
                pub fn #fields(&self) -> &#types {
                    &self.#fields
                }
            )*
        }
    }
}

token_goblin::snif!(Foo in generate_getters!(extra args));

fn main() {
    let foo = Foo { x: 42 };
    println!("x: {}", foo.x());
}
