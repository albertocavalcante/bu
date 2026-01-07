use std::fs;
use std::io;
use std::path::Path;

pub fn get_buck2_version(path: &Path) -> io::Result<String> {
    let version_file = path.join(".buckversion");
    if version_file.exists() {
        let content = fs::read_to_string(version_file)?;
        return Ok(content.trim().to_string());
    }
    Ok("latest".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_read_specific_version() {
        let dir = tempdir().unwrap();
        let mut file = File::create(dir.path().join(".buckversion")).unwrap();
        writeln!(file, "2023-10-15").unwrap();

        let version = get_buck2_version(dir.path()).unwrap();
        assert_eq!(version, "2023-10-15");
    }

    #[test]
    fn test_read_latest_version() {
        let dir = tempdir().unwrap();
        let mut file = File::create(dir.path().join(".buckversion")).unwrap();
        writeln!(file, "latest").unwrap();

        let version = get_buck2_version(dir.path()).unwrap();
        assert_eq!(version, "latest");
    }

    #[test]
    fn test_trim_whitespace() {
        let dir = tempdir().unwrap();
        let mut file = File::create(dir.path().join(".buckversion")).unwrap();
        writeln!(file, "  2023-12-01  \n").unwrap();

        let version = get_buck2_version(dir.path()).unwrap();
        assert_eq!(version, "2023-12-01");
    }

    #[test]
    fn test_no_version_file_defaults_to_latest() {
        let dir = tempdir().unwrap();
        // No .buckversion file
        let version = get_buck2_version(dir.path()).unwrap();
        assert_eq!(version, "latest");
    }
}
