use std::collections::{HashMap, HashSet};

/// Result of comparing two env files.
#[derive(Debug)]
pub struct CheckResult {
    /// Keys present in env but missing from example (dangerous: undocumented secrets)
    pub missing_from_example: Vec<String>,
    /// Keys present in example but missing from env (dangerous: unset required vars)
    pub missing_from_env: Vec<String>,
    /// Number of keys present in both files
    pub in_sync_count: usize,
}

impl CheckResult {
    /// Returns true if the files are perfectly in sync (no missing keys in either direction).
    pub fn is_in_sync(&self) -> bool {
        self.missing_from_example.is_empty() && self.missing_from_env.is_empty()
    }
}

/// Compare two env maps: env (the actual file) vs example (the template).
pub fn compare_maps(env: &HashMap<String, String>, example: &HashMap<String, String>) -> CheckResult {
    let env_keys: HashSet<&String> = env.keys().collect();
    let example_keys: HashSet<&String> = example.keys().collect();

    let mut missing_from_example: Vec<String> = env_keys
        .difference(&example_keys)
        .map(|s| s.to_string())
        .collect();
    missing_from_example.sort();

    let mut missing_from_env: Vec<String> = example_keys
        .difference(&env_keys)
        .map(|s| s.to_string())
        .collect();
    missing_from_env.sort();

    let in_sync_count = env_keys.intersection(&example_keys).count();

    CheckResult {
        missing_from_example,
        missing_from_env,
        in_sync_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn test_in_sync() {
        let env = make_map(&[("FOO", "bar"), ("BAZ", "qux")]);
        let example = make_map(&[("FOO", ""), ("BAZ", "")]);
        let result = compare_maps(&env, &example);
        assert!(result.is_in_sync());
        assert_eq!(result.in_sync_count, 2);
    }

    #[test]
    fn test_missing_from_example() {
        let env = make_map(&[("FOO", "bar"), ("SECRET", "x")]);
        let example = make_map(&[("FOO", "")]);
        let result = compare_maps(&env, &example);
        assert!(!result.is_in_sync());
        assert_eq!(result.missing_from_example, vec!["SECRET"]);
        assert!(result.missing_from_env.is_empty());
    }

    #[test]
    fn test_missing_from_env() {
        let env = make_map(&[("FOO", "bar")]);
        let example = make_map(&[("FOO", ""), ("REQUIRED", "")]);
        let result = compare_maps(&env, &example);
        assert!(!result.is_in_sync());
        assert!(result.missing_from_example.is_empty());
        assert_eq!(result.missing_from_env, vec!["REQUIRED"]);
    }

    #[test]
    fn test_both_directions_missing() {
        let env = make_map(&[("FOO", "bar"), ("EXTRA", "x")]);
        let example = make_map(&[("FOO", ""), ("MISSING", "")]);
        let result = compare_maps(&env, &example);
        assert!(!result.is_in_sync());
        assert_eq!(result.missing_from_example, vec!["EXTRA"]);
        assert_eq!(result.missing_from_env, vec!["MISSING"]);
    }

    #[test]
    fn test_empty_env() {
        let env = make_map(&[]);
        let example = make_map(&[("FOO", ""), ("BAR", "")]);
        let result = compare_maps(&env, &example);
        assert!(!result.is_in_sync());
        assert_eq!(result.missing_from_env.len(), 2);
        assert!(result.missing_from_example.is_empty());
    }

    #[test]
    fn test_empty_example() {
        let env = make_map(&[("FOO", "bar")]);
        let example = make_map(&[]);
        let result = compare_maps(&env, &example);
        assert!(!result.is_in_sync());
        assert_eq!(result.missing_from_example, vec!["FOO"]);
        assert!(result.missing_from_env.is_empty());
    }

    #[test]
    fn test_results_sorted() {
        let env = make_map(&[("ZZZ", "1"), ("AAA", "2"), ("MMM", "3")]);
        let example = make_map(&[]);
        let result = compare_maps(&env, &example);
        let keys = &result.missing_from_example;
        assert_eq!(keys, &vec!["AAA", "MMM", "ZZZ"]);
    }
}
