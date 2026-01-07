use std::fs;
use std::io;
use std::path::Path;

pub fn get_gradle_version(path: &Path) -> io::Result<String> {
    let wrapper_file = path.join("gradle/wrapper/gradle-wrapper.properties");

    if !wrapper_file.exists() {
        return Ok("latest".to_string());
    }

    let content = fs::read_to_string(wrapper_file)?;

    // Parse the distributionUrl property
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("distributionUrl")
            && let Some(url) = line.split('=').nth(1)
        {
            // Extract version from URL like:
            // https://services.gradle.org/distributions/gradle-8.5-bin.zip
            // or https://services.gradle.org/distributions/gradle-8.5-all.zip
            if let Some(version) = extract_version_from_url(url.trim()) {
                return Ok(version);
            }
        }
    }

    // If we can't parse the version, return "latest"
    Ok("latest".to_string())
}

fn extract_version_from_url(url: &str) -> Option<String> {
    // Look for pattern: gradle-X.Y-bin.zip or gradle-X.Y-all.zip
    // The URL might be escaped (contains \:)
    let url = url.replace("\\:", ":");

    // Find "gradle-" in the URL
    if let Some(start_idx) = url.find("gradle-") {
        let after_gradle = &url[start_idx + 7..]; // Skip "gradle-"

        // Find the next "-" which separates version from distribution type (bin/all)
        if let Some(end_idx) = after_gradle.find('-') {
            let version = &after_gradle[..end_idx];
            return Some(version.to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_parse_distribution_url_with_bin() {
        let dir = tempdir().unwrap();
        let wrapper_dir = dir.path().join("gradle/wrapper");
        fs::create_dir_all(&wrapper_dir).unwrap();

        let mut file = File::create(wrapper_dir.join("gradle-wrapper.properties")).unwrap();
        writeln!(file, "distributionBase=GRADLE_USER_HOME").unwrap();
        writeln!(file, "distributionPath=wrapper/dists").unwrap();
        writeln!(file, "distributionUrl=https\\://services.gradle.org/distributions/gradle-8.5-bin.zip").unwrap();
        writeln!(file, "zipStoreBase=GRADLE_USER_HOME").unwrap();
        writeln!(file, "zipStorePath=wrapper/dists").unwrap();

        let version = get_gradle_version(dir.path()).unwrap();
        assert_eq!(version, "8.5");
    }

    #[test]
    fn test_parse_distribution_url_with_all() {
        let dir = tempdir().unwrap();
        let wrapper_dir = dir.path().join("gradle/wrapper");
        fs::create_dir_all(&wrapper_dir).unwrap();

        let mut file = File::create(wrapper_dir.join("gradle-wrapper.properties")).unwrap();
        writeln!(file, "distributionBase=GRADLE_USER_HOME").unwrap();
        writeln!(file, "distributionUrl=https\\://services.gradle.org/distributions/gradle-7.6.1-all.zip").unwrap();
        writeln!(file, "zipStoreBase=GRADLE_USER_HOME").unwrap();

        let version = get_gradle_version(dir.path()).unwrap();
        assert_eq!(version, "7.6.1");
    }

    #[test]
    fn test_parse_distribution_url_without_escaped_colon() {
        let dir = tempdir().unwrap();
        let wrapper_dir = dir.path().join("gradle/wrapper");
        fs::create_dir_all(&wrapper_dir).unwrap();

        let mut file = File::create(wrapper_dir.join("gradle-wrapper.properties")).unwrap();
        writeln!(file, "distributionUrl=https://services.gradle.org/distributions/gradle-8.0-bin.zip").unwrap();

        let version = get_gradle_version(dir.path()).unwrap();
        assert_eq!(version, "8.0");
    }

    #[test]
    fn test_handle_missing_wrapper_file() {
        let dir = tempdir().unwrap();
        // No gradle/wrapper directory created

        let version = get_gradle_version(dir.path()).unwrap();
        assert_eq!(version, "latest");
    }

    #[test]
    fn test_handle_malformed_properties_no_distribution_url() {
        let dir = tempdir().unwrap();
        let wrapper_dir = dir.path().join("gradle/wrapper");
        fs::create_dir_all(&wrapper_dir).unwrap();

        let mut file = File::create(wrapper_dir.join("gradle-wrapper.properties")).unwrap();
        writeln!(file, "distributionBase=GRADLE_USER_HOME").unwrap();
        writeln!(file, "zipStoreBase=GRADLE_USER_HOME").unwrap();
        // No distributionUrl property

        let version = get_gradle_version(dir.path()).unwrap();
        assert_eq!(version, "latest");
    }

    #[test]
    fn test_handle_malformed_properties_invalid_url() {
        let dir = tempdir().unwrap();
        let wrapper_dir = dir.path().join("gradle/wrapper");
        fs::create_dir_all(&wrapper_dir).unwrap();

        let mut file = File::create(wrapper_dir.join("gradle-wrapper.properties")).unwrap();
        writeln!(file, "distributionUrl=invalid-url").unwrap();

        let version = get_gradle_version(dir.path()).unwrap();
        assert_eq!(version, "latest");
    }

    #[test]
    fn test_extract_version_from_url_bin() {
        assert_eq!(
            extract_version_from_url("https://services.gradle.org/distributions/gradle-8.5-bin.zip"),
            Some("8.5".to_string())
        );
    }

    #[test]
    fn test_extract_version_from_url_all() {
        assert_eq!(
            extract_version_from_url("https://services.gradle.org/distributions/gradle-7.6.1-all.zip"),
            Some("7.6.1".to_string())
        );
    }

    #[test]
    fn test_extract_version_from_url_escaped() {
        assert_eq!(
            extract_version_from_url("https\\://services.gradle.org/distributions/gradle-8.0-bin.zip"),
            Some("8.0".to_string())
        );
    }

    #[test]
    fn test_extract_version_from_url_invalid() {
        assert_eq!(extract_version_from_url("invalid-url"), None);
    }

    #[test]
    fn test_trim_whitespace_in_properties() {
        let dir = tempdir().unwrap();
        let wrapper_dir = dir.path().join("gradle/wrapper");
        fs::create_dir_all(&wrapper_dir).unwrap();

        let mut file = File::create(wrapper_dir.join("gradle-wrapper.properties")).unwrap();
        writeln!(file, "  distributionUrl = https://services.gradle.org/distributions/gradle-8.5-bin.zip  ").unwrap();

        let version = get_gradle_version(dir.path()).unwrap();
        assert_eq!(version, "8.5");
    }
}
