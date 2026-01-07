use std::fs;
use std::io;
use std::path::Path;

/// Reads Node version from version files in order of preference.
/// Checks .nvmrc first, then .node-version.
/// Returns "latest" if no version file is found.
/// Handles "v" prefix in version strings (e.g., "v18.17.0").
pub fn get_node_version(path: &Path) -> io::Result<String> {
    // Check .nvmrc first
    let nvmrc_path = path.join(".nvmrc");
    if nvmrc_path.exists() {
        let content = fs::read_to_string(nvmrc_path)?;
        return Ok(normalize_version(content.trim()));
    }

    // Check .node-version second
    let node_version_path = path.join(".node-version");
    if node_version_path.exists() {
        let content = fs::read_to_string(node_version_path)?;
        return Ok(normalize_version(content.trim()));
    }

    // Default to "latest" if no version file exists
    Ok("latest".to_string())
}

/// Normalizes version string by removing "v" prefix if present
fn normalize_version(version: &str) -> String {
    version.strip_prefix('v').unwrap_or(version).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_read_from_nvmrc() {
        let dir = tempdir().unwrap();
        let nvmrc_path = dir.path().join(".nvmrc");
        let mut file = File::create(&nvmrc_path).unwrap();
        writeln!(file, "18.17.0").unwrap();

        let version = get_node_version(dir.path()).unwrap();
        assert_eq!(version, "18.17.0");
    }

    #[test]
    fn test_read_from_node_version() {
        let dir = tempdir().unwrap();
        let node_version_path = dir.path().join(".node-version");
        let mut file = File::create(&node_version_path).unwrap();
        writeln!(file, "20.10.0").unwrap();

        let version = get_node_version(dir.path()).unwrap();
        assert_eq!(version, "20.10.0");
    }

    #[test]
    fn test_prefer_nvmrc_when_both_exist() {
        let dir = tempdir().unwrap();

        // Create .nvmrc
        let nvmrc_path = dir.path().join(".nvmrc");
        let mut nvmrc_file = File::create(&nvmrc_path).unwrap();
        writeln!(nvmrc_file, "18.17.0").unwrap();

        // Create .node-version
        let node_version_path = dir.path().join(".node-version");
        let mut node_version_file = File::create(&node_version_path).unwrap();
        writeln!(node_version_file, "20.10.0").unwrap();

        let version = get_node_version(dir.path()).unwrap();
        assert_eq!(
            version, "18.17.0",
            ".nvmrc should be preferred over .node-version"
        );
    }

    #[test]
    fn test_handle_v_prefix() {
        let dir = tempdir().unwrap();
        let nvmrc_path = dir.path().join(".nvmrc");
        let mut file = File::create(&nvmrc_path).unwrap();
        writeln!(file, "v18.17.0").unwrap();

        let version = get_node_version(dir.path()).unwrap();
        assert_eq!(version, "18.17.0", "v prefix should be removed");
    }

    #[test]
    fn test_trim_whitespace() {
        let dir = tempdir().unwrap();
        let nvmrc_path = dir.path().join(".nvmrc");
        let mut file = File::create(&nvmrc_path).unwrap();
        writeln!(file, "  18.17.0  ").unwrap();

        let version = get_node_version(dir.path()).unwrap();
        assert_eq!(version, "18.17.0", "whitespace should be trimmed");
    }

    #[test]
    fn test_trim_whitespace_with_v_prefix() {
        let dir = tempdir().unwrap();
        let nvmrc_path = dir.path().join(".nvmrc");
        let mut file = File::create(&nvmrc_path).unwrap();
        writeln!(file, "  v18.17.0  ").unwrap();

        let version = get_node_version(dir.path()).unwrap();
        assert_eq!(
            version, "18.17.0",
            "whitespace should be trimmed and v prefix removed"
        );
    }

    #[test]
    fn test_default_to_latest_when_no_files_exist() {
        let dir = tempdir().unwrap();

        let version = get_node_version(dir.path()).unwrap();
        assert_eq!(
            version, "latest",
            "should default to 'latest' when no version files exist"
        );
    }

    #[test]
    fn test_node_version_file_with_v_prefix() {
        let dir = tempdir().unwrap();
        let node_version_path = dir.path().join(".node-version");
        let mut file = File::create(&node_version_path).unwrap();
        writeln!(file, "v20.10.0").unwrap();

        let version = get_node_version(dir.path()).unwrap();
        assert_eq!(
            version, "20.10.0",
            "v prefix should be removed from .node-version"
        );
    }
}
