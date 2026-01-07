//! .NET SDK version detection from global.json.

use std::fs;
use std::io;
use std::path::Path;

/// Reads .NET SDK version from global.json.
///
/// The global.json file specifies which .NET SDK version to use:
/// ```json
/// {
///   "sdk": {
///     "version": "8.0.100"
///   }
/// }
/// ```
///
/// Returns "latest" if no global.json is found.
pub fn get_dotnet_version(path: &Path) -> io::Result<String> {
    let global_json = path.join("global.json");
    if !global_json.exists() {
        return Ok("latest".to_string());
    }

    let content = fs::read_to_string(global_json)?;

    // Simple JSON parsing without external dependency
    // Look for "version": "X.Y.Z" pattern
    if let Some(version) = extract_sdk_version(&content) {
        return Ok(version);
    }

    Ok("latest".to_string())
}

/// Extracts SDK version from global.json content.
fn extract_sdk_version(content: &str) -> Option<String> {
    // Find "sdk" section and then "version" within it
    // Handle both formatted and minified JSON

    // First, find the "sdk" key
    let sdk_start = content.find("\"sdk\"")?;
    let after_sdk = &content[sdk_start..];

    // Find the opening brace of the sdk object
    let brace_start = after_sdk.find('{')?;
    let sdk_content = &after_sdk[brace_start..];

    // Find the closing brace (simple nesting not handled, but works for typical global.json)
    let brace_end = sdk_content.find('}')?;
    let sdk_object = &sdk_content[..brace_end];

    // Find "version" within the sdk object
    let version_start = sdk_object.find("\"version\"")?;
    let after_version = &sdk_object[version_start..];

    // Find the colon
    let colon_pos = after_version.find(':')?;
    let after_colon = &after_version[colon_pos + 1..];

    // Find the quoted version value
    let quote_start = after_colon.find('"')?;
    let after_quote = &after_colon[quote_start + 1..];
    let quote_end = after_quote.find('"')?;

    Some(after_quote[..quote_end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_read_global_json() {
        let dir = tempdir().unwrap();
        let mut file = File::create(dir.path().join("global.json")).unwrap();
        writeln!(
            file,
            r#"{{
  "sdk": {{
    "version": "8.0.100"
  }}
}}"#
        )
        .unwrap();

        let version = get_dotnet_version(dir.path()).unwrap();
        assert_eq!(version, "8.0.100");
    }

    #[test]
    fn test_no_global_json_returns_latest() {
        let dir = tempdir().unwrap();
        let version = get_dotnet_version(dir.path()).unwrap();
        assert_eq!(version, "latest");
    }

    #[test]
    fn test_global_json_with_additional_fields() {
        let dir = tempdir().unwrap();
        let mut file = File::create(dir.path().join("global.json")).unwrap();
        writeln!(
            file,
            r#"{{
  "sdk": {{
    "version": "7.0.400",
    "rollForward": "latestMinor"
  }},
  "msbuild-sdks": {{
    "Microsoft.Build.Traversal": "4.0.0"
  }}
}}"#
        )
        .unwrap();

        let version = get_dotnet_version(dir.path()).unwrap();
        assert_eq!(version, "7.0.400");
    }

    #[test]
    fn test_extract_sdk_version_minified() {
        let content = r#"{"sdk":{"version":"6.0.300"}}"#;
        assert_eq!(extract_sdk_version(content), Some("6.0.300".to_string()));
    }

    #[test]
    fn test_extract_sdk_version_formatted() {
        let content = r#"
{
  "sdk": {
    "version": "8.0.100"
  }
}
"#;
        assert_eq!(extract_sdk_version(content), Some("8.0.100".to_string()));
    }
}
