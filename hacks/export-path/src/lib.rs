//! This example shows, that path imported `some.rs`
//! can include other modules.
//!
//! This allows us to split macro implementation into multiple files.
//! ```
//!  #[token_goblin::munch]
//!  mod my_module {
//!    mod other; // mod `other` in current crate namespace is provided to the macro impl with small conversion:
//!               // ```
//!               // #[path = "path_to_current/other.rs"]
//!               // mod other;
//!               // ```
//!  // entry fn
//!  }
//! ```
//!
//!

/// `#[path]` is rust attribute that allow you to specify path to the module,
/// as docs says "the file path is relative to the directory the source file is located", but it allows
/// using `..` to go up in the directory structure (which rust-analyzer doesn't like though)
#[path = "extern-mod/deep/some.rs"]
mod some;

use some::Foo;
