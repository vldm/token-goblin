use std::path::PathBuf;
use std::process::Command;

const VISIBILITY_FIXTURES: &[&str] = &[
    "tests/module_visibility_pass.rs",
    "tests/module_visibility_fail.rs",
];

const TRYBUILD_PASS_EMUL_DIR: &str = "tests/trybuild-pass-emul";

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("token-goblin manifest has no parent directory")
        .to_path_buf()
}

fn fixtures_root() -> PathBuf {
    repo_root().join("fixtures")
}

fn example_readme_dir() -> PathBuf {
    repo_root().join("example_readme")
}

fn trybuild_pass_emul_dir() -> PathBuf {
    fixtures_root().join(TRYBUILD_PASS_EMUL_DIR)
}

fn trybuild_pass_emul_manifest() -> PathBuf {
    trybuild_pass_emul_dir().join("Cargo.toml")
}

fn module_path_fixture_dir() -> PathBuf {
    fixtures_root().join("tests/module-path")
}

fn module_path_fixture_manifest() -> PathBuf {
    module_path_fixture_dir().join("Cargo.toml")
}

fn run_cargo_expand(manifest: &PathBuf, fixture_dir: &PathBuf, args: &[&str]) -> String {
    let mut command_args = vec!["expand"];
    command_args.extend(args);
    let output = run_cargo_in_fixture(manifest, fixture_dir, &command_args);
    assert!(
        output.status.success(),
        "cargo expand failed for {}:\nstdout:\n{}\nstderr:\n{}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn module_path_for_alias(expanded: &str, alias: &str) -> String {
    let marker = format!(" as {alias};");
    let idx = expanded
        .find(&marker)
        .unwrap_or_else(|| panic!("macro alias `{alias}` not found in expanded output"));
    let prefix = &expanded[..idx];
    let line_start = prefix
        .rfind("///   Module path:")
        .unwrap_or_else(|| panic!("module path doc not found before `{alias}`"));
    let line = prefix[line_start..]
        .lines()
        .next()
        .unwrap_or_else(|| panic!("module path line missing before `{alias}`"));
    line.strip_prefix("///   Module path:")
        .unwrap_or_else(|| panic!("unexpected module path line: {line}"))
        .trim()
        .to_string()
}

fn assert_module_paths(expanded: &str, expected: &[(&str, &str)]) {
    for (alias, expected_path) in expected {
        assert_eq!(
            module_path_for_alias(expanded, alias),
            *expected_path,
            "module path mismatch for `{alias}`"
        );
    }
}

fn run_cargo_in_fixture(
    manifest: &PathBuf,
    fixture_dir: &PathBuf,
    args: &[&str],
) -> std::process::Output {
    let mut command = Command::new("cargo");
    command.current_dir(fixture_dir);
    for arg in args {
        command.arg(arg);
    }
    command.arg("--manifest-path").arg(manifest);
    command
        .output()
        .unwrap_or_else(|err| panic!("failed to spawn cargo in {}: {err}", fixture_dir.display()))
}

#[test]
fn example_readme_cargo_test() {
    let testbed_dir = example_readme_dir();
    let manifest = testbed_dir.join("Cargo.toml");
    assert!(
        manifest.is_file(),
        "example_readme manifest missing: {}",
        manifest.display()
    );

    let target_dir = repo_root().join("target/example_readme");
    let output = Command::new("cargo")
        .arg("test")
        .current_dir(&testbed_dir)
        .env("CARGO_TARGET_DIR", &target_dir)
        .args(["--", "--nocapture"])
        .output()
        .unwrap_or_else(|err| panic!("failed to spawn cargo in {}: {err}", testbed_dir.display()));

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        println!("stdout:\n{stdout}");
        eprintln!("stderr:\n{stderr}");
    }
    assert!(
        output.status.success(),
        "example_readme cargo test failed (status={}):\n{stdout}{stderr}",
        output.status
    );
}

#[test]
fn module_visibility_ui() {
    let tests = trybuild::TestCases::new();
    let root = fixtures_root();
    tests.pass(root.join(VISIBILITY_FIXTURES[0]));
    tests.compile_fail(root.join(VISIBILITY_FIXTURES[1]));
}

// Simulate trybuild for better error debugging
#[test]
fn trybuild_pass_emul_builds_external_bin() {
    let fixture_dir = trybuild_pass_emul_dir();
    let manifest = trybuild_pass_emul_manifest();
    assert!(
        manifest.is_file(),
        "trybuild pass emulator manifest missing: {}",
        manifest.display()
    );

    let output = run_cargo_in_fixture(
        &manifest,
        &fixture_dir,
        &["build", "--bin", "module_visibility_pass"],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        eprintln!("stdout:\n{stdout}");
        eprintln!("stderr:\n{stderr}");
    }

    assert!(
        output.status.success(),
        "expected external trybuild-style bin to build successfully"
    );
    let combined = format!("{stdout}{stderr}");
    assert!(
        !combined.contains("prefix not found") && !combined.contains("custom attribute panicked"),
        "unexpected SpanLocation failure:\n{combined}"
    );
}

#[test]
fn module_path_fixture_expand_output() {
    let fixture_dir = module_path_fixture_dir();
    let manifest = module_path_fixture_manifest();
    assert!(
        manifest.is_file(),
        "module path fixture manifest missing: {}",
        manifest.display()
    );

    let lib = run_cargo_expand(&manifest, &fixture_dir, &["--lib"]);
    assert_module_paths(
        &lib,
        &[
            ("lib_root", ""),
            ("lib_nested", "nested"),
            ("lib_shared", "shared_name"),
            ("shared_mod_root", "shared_mod"),
        ],
    );

    let bin = run_cargo_expand(&manifest, &fixture_dir, &["--bin", "module_path_fixture"]);
    assert_module_paths(
        &bin,
        &[
            ("bin_root", ""),
            ("bin_nested", "nested"),
            ("bin_shared", "shared_name"),
            ("shared_mod_root", "shared_mod"),
        ],
    );

    let test = run_cargo_expand(&manifest, &fixture_dir, &["--test", "integration"]);
    assert_module_paths(&test, &[("test_root", ""), ("test_nested", "nested")]);

    let example = run_cargo_expand(&manifest, &fixture_dir, &["--example", "demo"]);
    assert_module_paths(&example, &[("example_root", "")]);

    let bench = run_cargo_expand(&manifest, &fixture_dir, &["--bench", "bench"]);
    assert_module_paths(&bench, &[("bench_root", "")]);
}
