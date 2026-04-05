use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

/// Parse an .env file into a map of key -> value.
/// Handles:
/// - Comments (lines starting with #)
/// - Blank lines
/// - KEY=VALUE pairs
/// - Quoted values (single and double quotes)
/// - Inline comments after values
/// - KEY= (empty value)
/// - export KEY=VALUE prefix
pub fn parse_env_file(path: &Path) -> io::Result<HashMap<String, String>> {
    let content = fs::read_to_string(path)?;
    Ok(parse_env_content(&content))
}

/// Parse .env content from a string.
pub fn parse_env_content(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();

    for line in content.lines() {
        let line = line.trim();

        // Skip blank lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Strip optional "export " prefix
        let line = line.strip_prefix("export ").unwrap_or(line).trim();

        // Must contain '='
        if let Some(eq_pos) = line.find('=') {
            let key = line[..eq_pos].trim().to_string();
            let raw_value = line[eq_pos + 1..].to_string();

            // Skip keys that are empty or contain spaces (malformed)
            if key.is_empty() || key.contains(' ') {
                continue;
            }

            let value = parse_value(&raw_value);
            map.insert(key, value);
        }
    }

    map
}

/// Extract the value from the raw right-hand side of KEY=VALUE.
/// Strips surrounding quotes and handles inline comments for unquoted values.
fn parse_value(raw: &str) -> String {
    let raw = raw.trim();

    if raw.is_empty() {
        return String::new();
    }

    // Double-quoted value: "value with spaces" or "value # not a comment"
    if let Some(stripped) = raw.strip_prefix('"') {
        if let Some(end) = stripped.find('"') {
            return stripped[..end].to_string();
        }
        // Unclosed quote — return everything after the opening quote
        return stripped.to_string();
    }

    // Single-quoted value: 'value'
    if let Some(stripped) = raw.strip_prefix('\'') {
        if let Some(end) = stripped.find('\'') {
            return stripped[..end].to_string();
        }
        return stripped.to_string();
    }

    // Unquoted: strip inline comment (# preceded by whitespace)
    let value = if let Some(comment_pos) = find_inline_comment(raw) {
        raw[..comment_pos].trim_end()
    } else {
        raw
    };

    value.to_string()
}

/// Find the position of an inline comment: a '#' preceded by whitespace.
fn find_inline_comment(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'#' && i > 0 && bytes[i - 1] == b' ' {
            return Some(i - 1); // include the space in the trim
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_key_value() {
        let content = "FOO=bar\nBAZ=qux\n";
        let map = parse_env_content(content);
        assert_eq!(map.get("FOO"), Some(&"bar".to_string()));
        assert_eq!(map.get("BAZ"), Some(&"qux".to_string()));
    }

    #[test]
    fn test_comments_and_blank_lines() {
        let content = "\n# This is a comment\nFOO=bar\n\n# Another comment\nBAZ=qux\n";
        let map = parse_env_content(content);
        assert_eq!(map.len(), 2);
        assert_eq!(map.get("FOO"), Some(&"bar".to_string()));
        assert_eq!(map.get("BAZ"), Some(&"qux".to_string()));
    }

    #[test]
    fn test_empty_value() {
        let content = "KEY=\n";
        let map = parse_env_content(content);
        assert_eq!(map.get("KEY"), Some(&"".to_string()));
    }

    #[test]
    fn test_double_quoted_value() {
        let content = "MESSAGE=\"hello world\"\n";
        let map = parse_env_content(content);
        assert_eq!(map.get("MESSAGE"), Some(&"hello world".to_string()));
    }

    #[test]
    fn test_single_quoted_value() {
        let content = "TOKEN='abc123'\n";
        let map = parse_env_content(content);
        assert_eq!(map.get("TOKEN"), Some(&"abc123".to_string()));
    }

    #[test]
    fn test_inline_comment_stripped() {
        let content = "HOST=localhost # the host\n";
        let map = parse_env_content(content);
        assert_eq!(map.get("HOST"), Some(&"localhost".to_string()));
    }

    #[test]
    fn test_quoted_value_preserves_hash() {
        let content = "PASS=\"p#ssw0rd\"\n";
        let map = parse_env_content(content);
        assert_eq!(map.get("PASS"), Some(&"p#ssw0rd".to_string()));
    }

    #[test]
    fn test_export_prefix() {
        let content = "export API_KEY=secret\n";
        let map = parse_env_content(content);
        assert_eq!(map.get("API_KEY"), Some(&"secret".to_string()));
    }

    #[test]
    fn test_spaces_around_equals() {
        let content = "FOO = bar\n";
        let map = parse_env_content(content);
        assert_eq!(map.get("FOO"), Some(&"bar".to_string()));
    }

    #[test]
    fn test_no_equals_line_ignored() {
        let content = "NOTAKEYVALUE\nFOO=bar\n";
        let map = parse_env_content(content);
        assert_eq!(map.len(), 1);
        assert_eq!(map.get("FOO"), Some(&"bar".to_string()));
    }

    #[test]
    fn test_duplicate_keys_last_wins() {
        let content = "FOO=first\nFOO=second\n";
        let map = parse_env_content(content);
        assert_eq!(map.get("FOO"), Some(&"second".to_string()));
    }

    #[test]
    fn test_only_comments_and_blanks() {
        let content = "# comment\n\n# another comment\n   \n";
        let map = parse_env_content(content);
        assert!(map.is_empty());
    }

    #[test]
    fn test_value_with_equals_sign() {
        let content = "DATABASE_URL=postgres://user:pass@host/db?sslmode=require\n";
        let map = parse_env_content(content);
        assert_eq!(
            map.get("DATABASE_URL"),
            Some(&"postgres://user:pass@host/db?sslmode=require".to_string())
        );
    }

    #[test]
    fn test_unclosed_double_quote() {
        let content = "KEY=\"unclosed value\n";
        let map = parse_env_content(content);
        assert_eq!(map.get("KEY"), Some(&"unclosed value".to_string()));
    }

    #[test]
    fn test_unclosed_single_quote() {
        let content = "KEY='unclosed value\n";
        let map = parse_env_content(content);
        assert_eq!(map.get("KEY"), Some(&"unclosed value".to_string()));
    }

    #[test]
    fn test_tabs_around_equals() {
        let content = "FOO\t=\tbar\n";
        let map = parse_env_content(content);
        assert_eq!(map.get("FOO"), Some(&"bar".to_string()));
    }

    #[test]
    fn test_key_with_numbers_and_underscores() {
        let content = "APP_V2_KEY_123=value\n";
        let map = parse_env_content(content);
        assert_eq!(map.get("APP_V2_KEY_123"), Some(&"value".to_string()));
    }

    #[test]
    fn test_empty_file() {
        let map = parse_env_content("");
        assert!(map.is_empty());
    }

    #[test]
    fn test_value_with_multiple_hashes() {
        let content = "URL=http://host #port #comment\n";
        let map = parse_env_content(content);
        // First space-hash is the inline comment boundary
        assert_eq!(map.get("URL"), Some(&"http://host".to_string()));
    }

    #[test]
    fn test_export_with_extra_spaces() {
        let content = "export  FOO=bar\n";
        let map = parse_env_content(content);
        // "export " is stripped, leaving " FOO=bar", key is " FOO" trimmed to "FOO"
        // Actually the trim on the line handles leading spaces after export
        assert_eq!(map.get("FOO"), Some(&"bar".to_string()));
    }
}
