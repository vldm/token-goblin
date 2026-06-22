This was extracted to a separate workspace to speed up the `token-goblin` build and reduce the number of dependencies.
Having a separate workspace is also mandatory for reexport-related tests, like the `use_reexported_munch` test.

Having it as a crate, rather than as a fixture, allows new features to be tested directly in this crate.

Since `token-goblin` is a macro, expanding examples is enough for testing.

The test that checks whether it is located in the right place is `token-goblin/tests/fixtures.rs:example_readme_cargo_test()`.