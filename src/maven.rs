use std::fs;
use std::io;
use std::path::Path;

pub fn get_maven_version(path: &Path) -> io::Result<String> {
    let wrapper_props = path.join(".mvn/wrapper/maven-wrapper.properties");

    if !wrapper_props.exists() {
        return Ok("latest".to_string());
    }

    let content = fs::read_to_string(wrapper_props)?;

    // Look for distributionUrl line
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("distributionUrl=") {
            let url = line.strip_prefix("distributionUrl=").unwrap_or("");
            if let Some(version) = extract_maven_version(url) {
                return Ok(version);
            }
        }
    }

    Ok("latest".to_string())
}

fn extract_maven_version(url: &str) -> Option<String> {
    // Example URL: https://repo.maven.apache.org/maven2/org/apache/maven/apache-maven/3.9.6/apache-maven-3.9.6-bin.zip
    // We want to extract "3.9.6"

    // Find "apache-maven/" followed by version
    if let Some(pos) = url.find("apache-maven/") {
        let after_maven = &url[pos + "apache-maven/".len()..];
        // The version is the next path component before the next slash
        if let Some(slash_pos) = after_maven.find('/') {
            let version = &after_maven[..slash_pos];
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
    fn test_parse_distribution_url_standard_format() {
        let url = "https://repo.maven.apache.org/maven2/org/apache/maven/apache-maven/3.9.6/apache-maven-3.9.6-bin.zip";
        let version = extract_maven_version(url);
        assert_eq!(version, Some("3.9.6".to_string()));
    }

    #[test]
    fn test_parse_distribution_url_different_version() {
        let url = "https://repo.maven.apache.org/maven2/org/apache/maven/apache-maven/3.8.1/apache-maven-3.8.1-bin.zip";
        let version = extract_maven_version(url);
        assert_eq!(version, Some("3.8.1".to_string()));
    }

    #[test]
    fn test_parse_distribution_url_alternative_repo() {
        let url = "https://example.com/maven/apache-maven/4.0.0/apache-maven-4.0.0-bin.zip";
        let version = extract_maven_version(url);
        assert_eq!(version, Some("4.0.0".to_string()));
    }

    #[test]
    fn test_malformed_url_no_version() {
        let url = "https://repo.maven.apache.org/maven2/somefile.zip";
        let version = extract_maven_version(url);
        assert_eq!(version, None);
    }

    #[test]
    fn test_empty_url() {
        let version = extract_maven_version("");
        assert_eq!(version, None);
    }

    #[test]
    fn test_get_maven_version_from_wrapper_properties() {
        let dir = tempdir().unwrap();
        let mvn_wrapper_dir = dir.path().join(".mvn/wrapper");
        fs::create_dir_all(&mvn_wrapper_dir).unwrap();

        let props_file = mvn_wrapper_dir.join("maven-wrapper.properties");
        let mut file = File::create(props_file).unwrap();
        writeln!(file, "# Maven Wrapper Properties").unwrap();
        writeln!(file, "distributionUrl=https://repo.maven.apache.org/maven2/org/apache/maven/apache-maven/3.9.6/apache-maven-3.9.6-bin.zip").unwrap();
        writeln!(file, "wrapperUrl=https://repo.maven.apache.org/maven2/org/apache/maven/wrapper/maven-wrapper/3.2.0/maven-wrapper-3.2.0.jar").unwrap();

        let version = get_maven_version(dir.path()).unwrap();
        assert_eq!(version, "3.9.6");
    }

    #[test]
    fn test_missing_wrapper_file_defaults_to_latest() {
        let dir = tempdir().unwrap();
        // No .mvn/wrapper/maven-wrapper.properties file
        let version = get_maven_version(dir.path()).unwrap();
        assert_eq!(version, "latest");
    }

    #[test]
    fn test_wrapper_properties_without_distribution_url() {
        let dir = tempdir().unwrap();
        let mvn_wrapper_dir = dir.path().join(".mvn/wrapper");
        fs::create_dir_all(&mvn_wrapper_dir).unwrap();

        let props_file = mvn_wrapper_dir.join("maven-wrapper.properties");
        let mut file = File::create(props_file).unwrap();
        writeln!(file, "# Maven Wrapper Properties").unwrap();
        writeln!(file, "someOtherProperty=value").unwrap();

        let version = get_maven_version(dir.path()).unwrap();
        assert_eq!(version, "latest");
    }

    #[test]
    fn test_malformed_distribution_url_defaults_to_latest() {
        let dir = tempdir().unwrap();
        let mvn_wrapper_dir = dir.path().join(".mvn/wrapper");
        fs::create_dir_all(&mvn_wrapper_dir).unwrap();

        let props_file = mvn_wrapper_dir.join("maven-wrapper.properties");
        let mut file = File::create(props_file).unwrap();
        writeln!(file, "distributionUrl=https://invalid.url/notmaven.zip").unwrap();

        let version = get_maven_version(dir.path()).unwrap();
        assert_eq!(version, "latest");
    }

    #[test]
    fn test_distribution_url_with_whitespace() {
        let dir = tempdir().unwrap();
        let mvn_wrapper_dir = dir.path().join(".mvn/wrapper");
        fs::create_dir_all(&mvn_wrapper_dir).unwrap();

        let props_file = mvn_wrapper_dir.join("maven-wrapper.properties");
        let mut file = File::create(props_file).unwrap();
        writeln!(file, "  distributionUrl=https://repo.maven.apache.org/maven2/org/apache/maven/apache-maven/3.8.5/apache-maven-3.8.5-bin.zip  ").unwrap();

        let version = get_maven_version(dir.path()).unwrap();
        assert_eq!(version, "3.8.5");
    }

    #[test]
    fn test_multiple_properties_extracts_correct_version() {
        let dir = tempdir().unwrap();
        let mvn_wrapper_dir = dir.path().join(".mvn/wrapper");
        fs::create_dir_all(&mvn_wrapper_dir).unwrap();

        let props_file = mvn_wrapper_dir.join("maven-wrapper.properties");
        let mut file = File::create(props_file).unwrap();
        writeln!(file, "property1=value1").unwrap();
        writeln!(file, "property2=value2").unwrap();
        writeln!(file, "distributionUrl=https://repo.maven.apache.org/maven2/org/apache/maven/apache-maven/3.9.0/apache-maven-3.9.0-bin.zip").unwrap();
        writeln!(file, "property3=value3").unwrap();

        let version = get_maven_version(dir.path()).unwrap();
        assert_eq!(version, "3.9.0");
    }
}
