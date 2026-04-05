use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

/// Update .env.example by appending keys that are missing from it.
/// Values are stripped — only KEY= is written.
/// Returns the list of keys that were added.
pub fn sync_example(
    env: &HashMap<String, String>,
    example: &HashMap<String, String>,
    example_path: &Path,
    missing_keys: &[String],
) -> io::Result<Vec<String>> {
    if missing_keys.is_empty() {
        return Ok(vec![]);
    }

    // Read current content (if any) to append to it
    let current_content = if example_path.exists() {
        fs::read_to_string(example_path)?
    } else {
        String::new()
    };

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(example_path)?;

    // Write existing content
    if !current_content.is_empty() {
        // Ensure there's a newline before we append
        write!(file, "{}", current_content)?;
        if !current_content.ends_with('\n') {
            writeln!(file)?;
        }
    }

    // Append missing keys with empty values
    let mut added = Vec::new();
    for key in missing_keys {
        if !env.contains_key(key) && !example.contains_key(key) {
            // Shouldn't happen, but skip ghost keys
            continue;
        }
        writeln!(file, "{}=", key)?;
        added.push(key.clone());
    }

    Ok(added)
}

/// Build the content for a fresh .env.example from a given env map.
/// Keys are sorted, values are stripped.
#[allow(dead_code)]
pub fn build_example_content(env: &HashMap<String, String>) -> String {
    let mut keys: Vec<&String> = env.keys().collect();
    keys.sort();
    keys.iter().map(|k| format!("{}=\n", k)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn temp_dir() -> TempDir {
        tempfile::tempdir().expect("Failed to create temp dir")
    }

    fn make_map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn test_sync_adds_missing_keys() {
        let dir = temp_dir();
        let example_path = dir.path().join(".env.example");
        fs::write(&example_path, "FOO=\n").unwrap();

        let env = make_map(&[("FOO", "bar"), ("SECRET", "x")]);
        let example = make_map(&[("FOO", "")]);
        let missing = vec!["SECRET".to_string()];

        let added = sync_example(&env, &example, &example_path, &missing).unwrap();
        assert_eq!(added, vec!["SECRET"]);

        let content = fs::read_to_string(&example_path).unwrap();
        assert!(content.contains("FOO="));
        assert!(content.contains("SECRET="));
        // Should not have the actual secret value
        assert!(!content.contains("SECRET=x"));
    }

    #[test]
    fn test_sync_creates_file_if_missing() {
        let dir = temp_dir();
        let example_path = dir.path().join(".env.example");

        let env = make_map(&[("KEY", "value")]);
        let example = make_map(&[]);
        let missing = vec!["KEY".to_string()];

        let added = sync_example(&env, &example, &example_path, &missing).unwrap();
        assert_eq!(added, vec!["KEY"]);
        assert!(example_path.exists());

        let content = fs::read_to_string(&example_path).unwrap();
        assert!(content.contains("KEY="));
        assert!(!content.contains("KEY=value"));
    }

    #[test]
    fn test_sync_no_missing_keys() {
        let dir = temp_dir();
        let example_path = dir.path().join(".env.example");
        fs::write(&example_path, "FOO=\n").unwrap();

        let env = make_map(&[("FOO", "bar")]);
        let example = make_map(&[("FOO", "")]);

        let added = sync_example(&env, &example, &example_path, &[]).unwrap();
        assert!(added.is_empty());
    }

    #[test]
    fn test_build_example_content() {
        let env = make_map(&[("B_KEY", "secret"), ("A_KEY", "other")]);
        let content = build_example_content(&env);
        // Keys should be sorted
        assert_eq!(content, "A_KEY=\nB_KEY=\n");
        // Values must not appear
        assert!(!content.contains("secret"));
        assert!(!content.contains("other"));
    }
}
