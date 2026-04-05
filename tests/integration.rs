use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn potto_bin() -> String {
    // Use the debug binary built by cargo test
    let manifest = env!("CARGO_MANIFEST_DIR");
    format!("{}/target/debug/potto", manifest)
}

fn run_potto(args: &[&str], cwd: &Path) -> std::process::Output {
    Command::new(potto_bin())
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("Failed to run potto binary")
}

fn temp_dir() -> TempDir {
    tempfile::tempdir().expect("Failed to create temp dir")
}

// ─── Parser integration ───────────────────────────────────────────────────────

#[test]
fn test_parse_complex_env_file() {
    use potto::parser::parse_env_content;

    let content = r#"
# This is a comment
FOO=bar

BAZ="hello world"
QUOTED='single quoted'
EMPTY=
WITH_COMMENT=value # inline comment
export EXPORTED=yes
SPECIAL="p#ssw0rd"
"#;
    let map = parse_env_content(content);
    assert_eq!(map.get("FOO").map(String::as_str), Some("bar"));
    assert_eq!(map.get("BAZ").map(String::as_str), Some("hello world"));
    assert_eq!(map.get("QUOTED").map(String::as_str), Some("single quoted"));
    assert_eq!(map.get("EMPTY").map(String::as_str), Some(""));
    assert_eq!(map.get("WITH_COMMENT").map(String::as_str), Some("value"));
    assert_eq!(map.get("EXPORTED").map(String::as_str), Some("yes"));
    assert_eq!(map.get("SPECIAL").map(String::as_str), Some("p#ssw0rd"));
    // Comments and blank lines should not become keys
    assert!(!map.contains_key("#"));
    assert!(!map.contains_key(""));
}

// ─── Checker integration ──────────────────────────────────────────────────────

#[test]
fn test_checker_both_directions() {
    use potto::checker::compare_maps;
    use std::collections::HashMap;

    let mut env: HashMap<String, String> = HashMap::new();
    env.insert("FOO".into(), "bar".into());
    env.insert("SECRET".into(), "xxx".into());

    let mut example: HashMap<String, String> = HashMap::new();
    example.insert("FOO".into(), "".into());
    example.insert("REQUIRED".into(), "".into());

    let result = compare_maps(&env, &example);
    assert!(!result.is_in_sync());
    assert_eq!(result.missing_from_example, vec!["SECRET"]);
    assert_eq!(result.missing_from_env, vec!["REQUIRED"]);
    assert_eq!(result.in_sync_count, 1);
}

// ─── Discovery integration ────────────────────────────────────────────────────

#[test]
fn test_discovery_walks_up_to_parent() {
    use potto::discovery::find_env_files;

    let root = temp_dir();
    fs::write(root.path().join(".env"), "X=1").unwrap();
    fs::write(root.path().join(".env.example"), "X=").unwrap();

    let nested = root.path().join("a/b/c");
    fs::create_dir_all(&nested).unwrap();

    let (env_path, example_path) = find_env_files(&nested);
    assert!(env_path.is_some(), "Should discover .env from parent");
    assert!(example_path.is_some(), "Should discover .env.example from parent");
}

// ─── Sync integration ─────────────────────────────────────────────────────────

#[test]
fn test_sync_strips_values() {
    use potto::sync::sync_example;
    use std::collections::HashMap;

    let dir = temp_dir();
    let example_path = dir.path().join(".env.example");

    let mut env: HashMap<String, String> = HashMap::new();
    env.insert("API_KEY".into(), "super_secret".into());
    env.insert("DATABASE_URL".into(), "postgres://user:pass@host/db".into());

    let example: HashMap<String, String> = HashMap::new();
    let missing = vec!["API_KEY".to_string(), "DATABASE_URL".to_string()];

    sync_example(&env, &example, &example_path, &missing).unwrap();

    let content = fs::read_to_string(&example_path).unwrap();
    assert!(content.contains("API_KEY="), "Should contain KEY=");
    assert!(content.contains("DATABASE_URL="), "Should contain DATABASE_URL=");
    // Values must not appear
    assert!(!content.contains("super_secret"), "Should not contain secret value");
    assert!(!content.contains("postgres://"), "Should not contain db url");
}

// ─── Binary integration (exit codes) ─────────────────────────────────────────

#[test]
fn test_exit_code_0_when_in_sync() {
    let dir = temp_dir();
    fs::write(dir.path().join(".env"), "FOO=bar\nBAZ=qux\n").unwrap();
    fs::write(dir.path().join(".env.example"), "FOO=\nBAZ=\n").unwrap();

    let output = run_potto(&["check"], dir.path());
    assert_eq!(
        output.status.code(),
        Some(0),
        "Should exit 0 when in sync. stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_exit_code_1_when_out_of_sync() {
    let dir = temp_dir();
    fs::write(dir.path().join(".env"), "FOO=bar\nSECRET=x\n").unwrap();
    fs::write(dir.path().join(".env.example"), "FOO=\n").unwrap();

    let output = run_potto(&["check"], dir.path());
    assert_eq!(
        output.status.code(),
        Some(1),
        "Should exit 1 when out of sync. stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_exit_code_2_when_file_missing() {
    let dir = temp_dir();
    // No .env or .env.example

    let output = run_potto(&["check"], dir.path());
    assert_eq!(
        output.status.code(),
        Some(2),
        "Should exit 2 when files not found"
    );
}

#[test]
fn test_compare_exit_code_0_same_keys() {
    let dir = temp_dir();
    fs::write(dir.path().join("a.env"), "FOO=bar\nBAZ=qux\n").unwrap();
    fs::write(dir.path().join("b.env"), "FOO=other\nBAZ=stuff\n").unwrap();

    let output = run_potto(
        &[
            "compare",
            dir.path().join("a.env").to_str().unwrap(),
            dir.path().join("b.env").to_str().unwrap(),
        ],
        dir.path(),
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "Should exit 0 when same keys. stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn test_compare_exit_code_1_different_keys() {
    let dir = temp_dir();
    fs::write(dir.path().join("a.env"), "FOO=bar\n").unwrap();
    fs::write(dir.path().join("b.env"), "DIFFERENT=val\n").unwrap();

    let output = run_potto(
        &[
            "compare",
            dir.path().join("a.env").to_str().unwrap(),
            dir.path().join("b.env").to_str().unwrap(),
        ],
        dir.path(),
    );
    assert_eq!(
        output.status.code(),
        Some(1),
        "Should exit 1 when different keys"
    );
}

#[test]
fn test_sync_command_adds_keys() {
    let dir = temp_dir();
    fs::write(dir.path().join(".env"), "FOO=bar\nNEW_KEY=secret\n").unwrap();
    fs::write(dir.path().join(".env.example"), "FOO=\n").unwrap();

    let output = run_potto(&["sync"], dir.path());
    assert_eq!(
        output.status.code(),
        Some(0),
        "Sync should succeed. stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let example_content = fs::read_to_string(dir.path().join(".env.example")).unwrap();
    assert!(example_content.contains("NEW_KEY="), "Example should contain NEW_KEY=");
    assert!(!example_content.contains("secret"), "Example should not contain secret value");
}

#[test]
fn test_sync_creates_example_from_scratch() {
    let dir = temp_dir();
    fs::write(dir.path().join(".env"), "DB_URL=postgres://localhost\nAPI_KEY=abc\n").unwrap();
    // No .env.example

    let output = run_potto(&["sync"], dir.path());
    assert_eq!(
        output.status.code(),
        Some(0),
        "Sync should create example. stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );

    let example_path = dir.path().join(".env.example");
    assert!(example_path.exists(), ".env.example should be created");
    let content = fs::read_to_string(&example_path).unwrap();
    assert!(content.contains("DB_URL="));
    assert!(content.contains("API_KEY="));
    assert!(!content.contains("postgres://"));
    assert!(!content.contains("abc"));
}

#[test]
fn test_help_flag_exits_0() {
    let dir = temp_dir();
    let output = run_potto(&["--help"], dir.path());
    assert_eq!(output.status.code(), Some(0), "Should exit 0 for --help");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("potto"), "Help should mention potto");
    assert!(stdout.contains("check"), "Help should mention check command");
}

#[test]
fn test_version_flag_exits_0() {
    let dir = temp_dir();
    let output = run_potto(&["--version"], dir.path());
    assert_eq!(output.status.code(), Some(0), "Should exit 0 for --version");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("potto"), "Version should mention potto");
}

#[test]
fn test_compare_nonexistent_file_exits_2() {
    let dir = temp_dir();
    fs::write(dir.path().join("a.env"), "FOO=bar\n").unwrap();
    let output = run_potto(
        &["compare", dir.path().join("a.env").to_str().unwrap(), "/nonexistent/.env"],
        dir.path(),
    );
    assert_eq!(
        output.status.code(),
        Some(2),
        "Should exit 2 when compare file doesn't exist"
    );
}

#[test]
fn test_check_with_explicit_paths() {
    let dir = temp_dir();
    fs::write(dir.path().join("my.env"), "FOO=bar\nBAZ=qux\n").unwrap();
    fs::write(dir.path().join("my.env.example"), "FOO=\nBAZ=\n").unwrap();

    let output = run_potto(
        &[
            "check",
            "--env", dir.path().join("my.env").to_str().unwrap(),
            "--example", dir.path().join("my.env.example").to_str().unwrap(),
        ],
        dir.path(),
    );
    assert_eq!(output.status.code(), Some(0), "Should work with explicit paths");
}

#[test]
fn test_sync_already_in_sync() {
    let dir = temp_dir();
    fs::write(dir.path().join(".env"), "FOO=bar\n").unwrap();
    fs::write(dir.path().join(".env.example"), "FOO=\n").unwrap();

    let output = run_potto(&["sync"], dir.path());
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Already in sync"), "Should say already in sync");
}

#[test]
fn test_quiet_flag_suppresses_output() {
    let dir = temp_dir();
    fs::write(dir.path().join(".env"), "FOO=bar\nSECRET=x\n").unwrap();
    fs::write(dir.path().join(".env.example"), "FOO=\n").unwrap();

    let output = run_potto(&["--quiet", "check"], dir.path());
    assert_eq!(output.status.code(), Some(1), "Should still exit 1 when out of sync");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.is_empty(), "Quiet mode should suppress stdout, got: {}", stdout);
}

#[test]
fn test_quiet_flag_with_sync() {
    let dir = temp_dir();
    fs::write(dir.path().join(".env"), "FOO=bar\nNEW=val\n").unwrap();
    fs::write(dir.path().join(".env.example"), "FOO=\n").unwrap();

    let output = run_potto(&["-q", "sync"], dir.path());
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.is_empty(), "Quiet sync should suppress stdout");

    // But the file should still be updated
    let content = fs::read_to_string(dir.path().join(".env.example")).unwrap();
    assert!(content.contains("NEW="), "Sync should still write even in quiet mode");
}

#[test]
fn test_sync_with_explicit_paths() {
    let dir = temp_dir();
    fs::write(dir.path().join("custom.env"), "FOO=bar\nNEW=val\n").unwrap();
    fs::write(dir.path().join("custom.example"), "FOO=\n").unwrap();

    let output = run_potto(
        &[
            "sync",
            "--env", dir.path().join("custom.env").to_str().unwrap(),
            "--example", dir.path().join("custom.example").to_str().unwrap(),
        ],
        dir.path(),
    );
    assert_eq!(output.status.code(), Some(0), "Sync with explicit paths should succeed");

    let content = fs::read_to_string(dir.path().join("custom.example")).unwrap();
    assert!(content.contains("NEW="), "Should add missing key");
    assert!(!content.contains("NEW=val"), "Should strip the value");
}

#[test]
fn test_check_env_pointing_to_nonexistent_file() {
    let dir = temp_dir();
    fs::write(dir.path().join(".env.example"), "FOO=\n").unwrap();

    let output = run_potto(
        &["check", "--env", "/nonexistent/.env"],
        dir.path(),
    );
    assert_eq!(output.status.code(), Some(2), "Should exit 2 for missing --env file");
}

#[test]
fn test_check_example_pointing_to_nonexistent_file() {
    let dir = temp_dir();
    fs::write(dir.path().join(".env"), "FOO=bar\n").unwrap();

    let output = run_potto(
        &["check", "--example", "/nonexistent/.env.example"],
        dir.path(),
    );
    assert_eq!(output.status.code(), Some(2), "Should exit 2 for missing --example file");
}

#[test]
fn test_check_from_subdirectory_discovers_parent() {
    let dir = temp_dir();
    fs::write(dir.path().join(".env"), "FOO=bar\n").unwrap();
    fs::write(dir.path().join(".env.example"), "FOO=\n").unwrap();

    let child = dir.path().join("src/components");
    fs::create_dir_all(&child).unwrap();

    let output = run_potto(&["check"], &child);
    assert_eq!(
        output.status.code(),
        Some(0),
        "Should discover files from parent and report in sync"
    );
}

#[test]
fn test_compare_same_file() {
    let dir = temp_dir();
    let path = dir.path().join("same.env");
    fs::write(&path, "FOO=bar\nBAZ=qux\n").unwrap();
    let p = path.to_str().unwrap();

    let output = run_potto(&["compare", p, p], dir.path());
    assert_eq!(output.status.code(), Some(0), "Comparing file with itself should be in sync");
}

#[test]
fn test_large_env_file() {
    let dir = temp_dir();
    let mut env_content = String::new();
    let mut example_content = String::new();
    for i in 0..100 {
        env_content.push_str(&format!("KEY_{}=value_{}\n", i, i));
        example_content.push_str(&format!("KEY_{}=\n", i));
    }
    fs::write(dir.path().join(".env"), &env_content).unwrap();
    fs::write(dir.path().join(".env.example"), &example_content).unwrap();

    let output = run_potto(&["check"], dir.path());
    assert_eq!(output.status.code(), Some(0), "100 keys should all be in sync");
}

#[test]
fn test_default_command_is_check() {
    let dir = temp_dir();
    fs::write(dir.path().join(".env"), "FOO=bar\n").unwrap();
    fs::write(dir.path().join(".env.example"), "FOO=\n").unwrap();

    // Run without subcommand — should behave like check
    let output = run_potto(&[], dir.path());
    assert_eq!(
        output.status.code(),
        Some(0),
        "Default (no subcommand) should exit 0 when in sync"
    );
}
