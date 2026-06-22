# Module export path

This example shows that a path-imported `some.rs`
can include other modules.

This allows us to split macro implementation into multiple files.

```
#[token_goblin::munch]
mod my_module {
  mod other; // mod `other` in the current crate namespace is provided to the macro impl with a small conversion:
             // ```
             // #[path = "path_to_current/other.rs"]
             // mod other;
             // ```
  // entry fn
}
```