#[token_goblin::munch]
fn generate_enums(components: Vec<String>) -> TokenStream {
    let mut result = vec![];
    for dim in 1..=components.len() {
        let cons = components[0..dim].join(",");
        result.push(format!("#[derive(Debug)] enum Enum{dim} {{ {cons} }}"));
    }
    TokenStream::from_str(&result.join("\n")).unwrap()
}

generate_enums!["X", "Y", "Z", "W", "V", "U", "T", "S", "R", "Q"];

fn main() {
    let x = Enum4::W;
    println!("x: {x:?}");
}
