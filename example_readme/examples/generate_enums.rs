#[token_goblin::munch]
fn generate_enums(components: CommaSeparated<Token>) {
    let components: Vec<String> = components.into();
    for dim in 1..=components.len() {
        let cons = components[0..dim].join(",");
        output_str! {
            "#[derive(Debug)]
            enum Enum{dim} {{
                {cons}
            }}"
        }
    }
}

generate_enums!["X", "Y", "Z", "W", "V", "U", "T", "S", "R", "Q"];

fn main() {
    let x = Enum8::W;
    println!("x: {x:?}");
}
