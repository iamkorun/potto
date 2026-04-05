use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

/// Update .env.example by appending keys that are missing from it.
/// Values are stripped — only KEY= is written.
/// Uses atomic write (temp file + rename) to prevent data loss on crash.
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

    // Build the new content in memory
    let mut new_content = String::new();
    if !current_content.is_empty() {
        new_content.push_str(&current_content);
        if !current_content.ends_with('\n') {
            new_content.push('\n');
        }
    }

    let mut added = Vec::new();
    for key in missing_keys {
        if !env.contains_key(key) && !example.contains_key(key) {
            continue;
        }
        new_content.push_str(key);
        new_content.push_str("=\n");
        added.push(key.clone());
    }

    // Atomic write: write to temp file, then rename
    let parent = example_path.parent().unwrap_or(Path::new("."));
    let tmp_path = parent.join(".env.example.tmp");
    fs::write(&tmp_path, &new_content)?;
    fs::rename(&tmp_path, example_path)?;

    Ok(added)
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

}
