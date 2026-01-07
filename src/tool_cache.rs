use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

#[derive(Debug)]
pub struct ToolCache {
    base_dir: PathBuf,
}

impl ToolCache {
    pub fn new() -> Option<Self> {
        dirs::home_dir().map(|home| {
            let base = home.join(".bu").join("cache");
            ToolCache { base_dir: base }
        })
    }

    #[cfg(test)]
    pub fn with_dir(base_dir: PathBuf) -> Self {
        ToolCache { base_dir }
    }

    pub fn cache_dir(&self) -> &Path {
        &self.base_dir
    }

    pub fn get_tool_path(&self, tool_name: &str, version: &str) -> PathBuf {
        let mut path = self.base_dir.join(tool_name).join(version).join(tool_name);

        // On Windows, append .exe
        if cfg!(windows) {
            path.set_extension("exe");
        }
        path
    }

    pub fn is_installed(&self, tool_name: &str, version: &str) -> bool {
        let path = self.get_tool_path(tool_name, version);
        let installed = path.exists();
        debug!(
            "Checking if {}@{} is at {:?}: {}",
            tool_name, version, path, installed
        );
        installed
    }

    pub fn install<F>(&self, tool_name: &str, version: &str, downloader: F) -> io::Result<PathBuf>
    where
        F: FnOnce(&Path) -> io::Result<()>,
    {
        let tool_path = self.get_tool_path(tool_name, version);

        if let Some(parent) = tool_path.parent() {
            fs::create_dir_all(parent)?;
        }

        info!("Installing {}@{} to {:?}", tool_name, version, tool_path);
        downloader(&tool_path)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&tool_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&tool_path, perms)?;
        }

        Ok(tool_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_tool_path_structure() {
        let dir = tempdir().unwrap();
        let cache = ToolCache::with_dir(dir.path().to_path_buf());

        let path = cache.get_tool_path("buck2", "2024-01-01");

        let expected_name = if cfg!(windows) { "buck2.exe" } else { "buck2" };
        assert_eq!(path.file_name().unwrap().to_str().unwrap(), expected_name);
        assert!(path.starts_with(dir.path()));
    }

    #[test]
    fn test_install_mock_tool() {
        let dir = tempdir().unwrap();
        let cache = ToolCache::with_dir(dir.path().to_path_buf());

        let result = cache.install("test-tool", "1.2.3", |path| {
            File::create(path)?;
            Ok(())
        });

        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.exists());
        assert!(cache.is_installed("test-tool", "1.2.3"));
    }
}
