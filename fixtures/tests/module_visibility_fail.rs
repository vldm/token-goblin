#![allow(dead_code, unused_imports, unused_macros)]

// private in module: not visible outside module
mod sealed {
    use proc_macro2::TokenStream;

    #[token_goblin::munch]
    fn secret(input: TokenStream) -> TokenStream {
        input
    }
}

fn from_outside_sealed() {
    let _: i32 = sealed::secret!(1);
    //~^ ERROR cannot find `secret`
}

// private in sibling module: not visible from sibling
mod a {
    use proc_macro2::TokenStream;

    #[token_goblin::munch]
    fn hidden(input: TokenStream) -> TokenStream {
        input
    }
}

mod b {
    fn sibling() {
        let _: i32 = super::a::hidden!(1);
        //~^ ERROR cannot find `hidden`
    }
}

// pub(super): not visible outside parent module
mod grandparent {
    mod parent {
        mod child {
            use proc_macro2::TokenStream;

            #[token_goblin::munch]
            pub(super) fn super_only(input: TokenStream) -> TokenStream {
                input
            }
        }
    }

    mod other_branch {
        fn cousin() {
            let _: i32 = super::parent::child::super_only!(1);
            //~^ ERROR private
        }
    }
}

// testbed-style private macro: not visible outside module
mod bar {
    use proc_macro2::TokenStream;

    #[token_goblin::munch]
    fn bar(input: TokenStream) -> TokenStream {
        input
    }
}

fn from_outside_bar() {
    let _: i32 = bar::bar!(1);
    //~^ ERROR cannot find `bar`
}

fn main() {}
