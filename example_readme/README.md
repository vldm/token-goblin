Extracted to separate workspace to speedup token-goblin build, and reduce amount of dependencies.
Also having separate workspace is mandatory, for reexports related tests, like `use_reexported_munch` test.

Having it as a crate, rather than as fixture, allows to testing new features dirrectly in this crate.

Since `token-goblin` is macro, expanding of examples is enough for testing.

And test that checks it is located in right place is located in `token-goblin/tests/fixtures.rs:example_readme_cargo_test()`.