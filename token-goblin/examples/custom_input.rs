#[token_goblin::munch]
mod custom_input {
    pub struct CustomInput {
        pub x: syn::Lit,
        pub y: syn::Lit,
    }

    impl syn::parse::Parse for CustomInput {
        fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
            let x = input.parse()?;
            let _: syn::Token![,] = input.parse()?;
            let y = input.parse()?;
            Ok(CustomInput { x, y })
        }
    }

    pub fn add(input: CustomInput) -> TokenStream {
        let x = input.x;
        let y: syn::Lit = input.y;
        quote! {
            #x + #y
        }
    }
}
fn main() {
    let result = custom_input::add!(1, 2);
    println!("result: {result}");
}
