use std::path::{Path, PathBuf};

/// Walk up the directory tree from `start` to find `.env` and `.env.example`.
/// Returns (env_path, example_path), each Some if found.
pub fn find_env_files(start: &Path) -> (Option<PathBuf>, Option<PathBuf>) {
    let mut current = start.to_path_buf();

    loop {
        let env = current.join(".env");
        let example = current.join(".env.example");

        let found_env = env.exists().then_some(env);
        let found_example = example.exists().then_some(example);

        // If we found at least one file at this level, return both (even if one is missing)
        if found_env.is_some() || found_example.is_some() {
            return (found_env, found_example);
        }

        // Go up one level
        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => break,
        }
    }

    (None, None)
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn temp_dir() -> TempDir {
        tempfile::tempdir().expect("Failed to create temp dir")
    }

    #[test]
    fn test_finds_both_in_same_dir() {
        let dir = temp_dir();
        fs::write(dir.path().join(".env"), "FOO=bar").unwrap();
        fs::write(dir.path().join(".env.example"), "FOO=").unwrap();

        let (env, example) = find_env_files(dir.path());
        assert!(env.is_some());
        assert!(example.is_some());
    }

    #[test]
    fn test_finds_in_parent_dir() {
        let parent = temp_dir();
        fs::write(parent.path().join(".env"), "FOO=bar").unwrap();
        fs::write(parent.path().join(".env.example"), "FOO=").unwrap();

        let child = parent.path().join("subdir");
        fs::create_dir(&child).unwrap();

        let (env, example) = find_env_files(&child);
        assert!(env.is_some(), "Should find .env in parent");
        assert!(example.is_some(), "Should find .env.example in parent");
    }

    #[test]
    fn test_returns_none_when_no_files() {
        let dir = temp_dir();
        let (env, example) = find_env_files(dir.path());
        assert!(env.is_none());
        assert!(example.is_none());
    }

    #[test]
    fn test_finds_only_env() {
        let dir = temp_dir();
        fs::write(dir.path().join(".env"), "FOO=bar").unwrap();

        let (env, example) = find_env_files(dir.path());
        assert!(env.is_some());
        assert!(example.is_none());
    }

}
