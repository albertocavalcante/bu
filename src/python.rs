//! Python version detection from .python-version and pyproject.toml.

use std::fs;
use std::io;
use std::path::Path;

/// Reads Python version from version files in order of preference.
///
/// Checks:
/// 1. `.python-version` (pyenv/asdf style)
/// 2. `pyproject.toml` (requires-python field)
///
/// Returns "latest" if no version file is found.
pub fn get_python_version(path: &Path) -> io::Result<String> {
    // Check .python-version first (most explicit)
    let python_version_file = path.join(".python-version");
    if python_version_file.exists() {
        let content = fs::read_to_string(python_version_file)?;
        let version = content.trim();
        if !version.is_empty() {
            return Ok(version.to_string());
        }
    }

    // Check pyproject.toml for requires-python
    let pyproject = path.join("pyproject.toml");
    if pyproject.exists() {
        let content = fs::read_to_string(pyproject)?;
        if let Some(version) = extract_requires_python(&content) {
            return Ok(version);
        }
    }

    Ok("latest".to_string())
}

/// Extracts the requires-python version from pyproject.toml content.
fn extract_requires_python(content: &str) -> Option<String> {
    // Look for requires-python = ">=3.8" or similar
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("requires-python") {
            // Extract value between quotes
            if let Some(start) = line.find('"')
                && let Some(end) = line[start + 1..].find('"')
            {
                let version_spec = &line[start + 1..start + 1 + end];
                // Clean up version specifier (remove >=, ~=, etc.)
                return Some(clean_version_spec(version_spec));
            }
            if let Some(start) = line.find('\'')
                && let Some(end) = line[start + 1..].find('\'')
            {
                let version_spec = &line[start + 1..start + 1 + end];
                return Some(clean_version_spec(version_spec));
            }
        }
    }
    None
}

/// Cleans version specifier by removing comparison operators.
fn clean_version_spec(spec: &str) -> String {
    spec.trim_start_matches(">=")
        .trim_start_matches("<=")
        .trim_start_matches("==")
        .trim_start_matches("~=")
        .trim_start_matches('>')
        .trim_start_matches('<')
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_read_python_version_file() {
        let dir = tempdir().unwrap();
        let mut file = File::create(dir.path().join(".python-version")).unwrap();
        writeln!(file, "3.11.4").unwrap();

        let version = get_python_version(dir.path()).unwrap();
        assert_eq!(version, "3.11.4");
    }

    #[test]
    fn test_read_pyproject_requires_python() {
        let dir = tempdir().unwrap();
        let mut file = File::create(dir.path().join("pyproject.toml")).unwrap();
        writeln!(
            file,
            r#"[project]
name = "myproject"
requires-python = ">=3.9"
"#
        )
        .unwrap();

        let version = get_python_version(dir.path()).unwrap();
        assert_eq!(version, "3.9");
    }

    #[test]
    fn test_python_version_file_takes_precedence() {
        let dir = tempdir().unwrap();

        let mut pv = File::create(dir.path().join(".python-version")).unwrap();
        writeln!(pv, "3.12").unwrap();

        let mut pp = File::create(dir.path().join("pyproject.toml")).unwrap();
        writeln!(pp, r#"requires-python = ">=3.9""#).unwrap();

        let version = get_python_version(dir.path()).unwrap();
        assert_eq!(version, "3.12");
    }

    #[test]
    fn test_no_version_file_returns_latest() {
        let dir = tempdir().unwrap();
        let version = get_python_version(dir.path()).unwrap();
        assert_eq!(version, "latest");
    }

    #[test]
    fn test_clean_version_spec() {
        assert_eq!(clean_version_spec(">=3.9"), "3.9");
        assert_eq!(clean_version_spec("==3.11.0"), "3.11.0");
        assert_eq!(clean_version_spec("~=3.10"), "3.10");
        assert_eq!(clean_version_spec("3.9"), "3.9");
    }

    #[test]
    fn test_extract_requires_python_single_quotes() {
        let content = "requires-python = '>=3.8'";
        assert_eq!(extract_requires_python(content), Some("3.8".to_string()));
    }

    #[test]
    fn test_extract_requires_python_double_quotes() {
        let content = r#"requires-python = ">=3.10""#;
        assert_eq!(extract_requires_python(content), Some("3.10".to_string()));
    }
}
