//! Mix of declarative and proc-macro macros.

#[token_goblin::munch]
pub fn stringify_any(input: TokenStream) -> TokenStream {
    let string = input.to_string();
    quote! {
        #string
    }
}


macro_rules! stringify_ident {
    ($ident:ident) => {
        stringify_any!($ident)
    };
}


fn main() {
    // this will fail at compile time, due to wrong input pattern
    // let result = stringify_ident!("non ident");
    // let result = stringify_ident!(foo asd);
    let result = stringify_ident!(foo);
    println!("result: {result}");
}