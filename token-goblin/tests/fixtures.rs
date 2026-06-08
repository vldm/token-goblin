use std::path::PathBuf;
use std::process::Command;

const FIXTURE_CASES: &[(&str, &str)] = &[
    ("empty", "configs/empty.toml"),
    ("build-dir", "configs/build-dir.toml"),
    ("target-dir", "configs/target-dir.toml"),
];

const VISIBILITY_FIXTURES: &[&str] = &[
    "tests/module_visibility_pass.rs",
    "tests/module_visibility_fail.rs",
];

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("token-goblin manifest has no parent directory")
        .to_path_buf()
}

fn fixtures_root() -> PathBuf {
    repo_root().join("fixtures")
}

fn smoke_fixture_dir() -> PathBuf {
    fixtures_root().join("tests/smoke-macro")
}

fn smoke_manifest() -> PathBuf {
    smoke_fixture_dir().join("Cargo.toml")
}

fn run_fixture_case(name: &str, config_rel: &str) -> Result<(), String> {
    let config_path = fixtures_root().join(config_rel);
    let manifest = smoke_manifest();
    let fixture_dir = smoke_fixture_dir();

    for path in [&config_path, &manifest] {
        if !path.is_file() {
            return Err(format!("fixture path not found: {}", path.display()));
        }
    }

    let output = Command::new("cargo")
        .arg("+stable")
        .arg("--config")
        .arg(&config_path)
        .arg("test")
        .arg("--all-targets") // skip doctests
        .arg("--manifest-path")
        .arg(&manifest)
        .current_dir(&fixture_dir)
        .args(["--", "--nocapture"])
        .output()
        .map_err(|err| format!("failed to spawn cargo for fixture `{name}`: {err}"))?;

    println!("output: {}", String::from_utf8_lossy(&output.stdout));
    eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    if output.status.success() {
        return Ok(());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(format!(
        "fixture `{name}` failed (status={}):\n{stdout}{stderr}",
        output.status
    ))
}

#[test]
fn smoke_macro_fixtures_run_with_stable_toolchain() {
    for (name, config_rel) in FIXTURE_CASES {
        run_fixture_case(name, config_rel).unwrap_or_else(|err| panic!("{err}"));
    }
}

#[test]
fn module_visibility_ui() {
    let tests = trybuild::TestCases::new();
    let root = fixtures_root();
    tests.pass(root.join(VISIBILITY_FIXTURES[0]));
    tests.compile_fail(root.join(VISIBILITY_FIXTURES[1]));
}

#[test]
fn fixture_paths_exist() {
    let fixtures = fixtures_root();
    assert!(
        fixtures.is_dir(),
        "fixtures directory missing: {}",
        fixtures.display()
    );

    let manifest = smoke_manifest();
    assert!(
        manifest.is_file(),
        "smoke fixture manifest missing: {}",
        manifest.display()
    );

    for (name, config_rel) in FIXTURE_CASES {
        let config = fixtures.join(config_rel);
        assert!(
            config.is_file(),
            "fixture config `{name}` missing: {}",
            config.display()
        );
    }

    for rel in VISIBILITY_FIXTURES {
        let path = fixtures.join(rel);
        assert!(
            path.is_file(),
            "visibility fixture missing: {}",
            path.display()
        );
    }
}
