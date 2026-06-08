//@check-pass
#![allow(dead_code, unused_imports, unused_macros)]

use proc_macro2::TokenStream;

// pub at crate root: visible in child modules
#[token_goblin::munch]
pub fn crate_root_pub(input: TokenStream) -> TokenStream {
    input
}

mod child {
    use crate::crate_root_pub;

    fn uses_public() {
        let _: i32 = crate_root_pub!(42);
    }
}

// private in module: visible within same module
mod local_only {
    use proc_macro2::TokenStream;

    #[token_goblin::munch]
    fn local(input: TokenStream) -> TokenStream {
        input
    }

    fn same_module() {
        let _: i32 = local!(1);
    }
}

// pub(super): visible in parent module
mod outer {
    mod inner {
        use proc_macro2::TokenStream;

        #[token_goblin::munch]
        pub(super) fn super_visible(input: TokenStream) -> TokenStream {
            input
        }

        fn from_inner() {
            let _: i32 = super_visible!(1);
        }
    }

    use inner::super_visible;

    fn from_outer() {
        let _: i32 = super_visible!(1);
    }
}

// pub(crate): visible in sibling modules
#[token_goblin::munch]
pub(crate) fn crate_visible(input: TokenStream) -> TokenStream {
    input
}

mod sibling {
    use crate::crate_visible;

    fn uses_crate_visible() {
        let _: i32 = crate_visible!(1);
    }
}

// testbed-style private macro in module: visible within same module
mod bar {
    use proc_macro2::TokenStream;

    #[token_goblin::munch]
    fn bar(input: TokenStream) -> TokenStream {
        input
    }

    fn same_module() {
        let _: i32 = bar!(1);
    }
}

fn main() {}
