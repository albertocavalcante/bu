//! Deno version detection.
//!
//! Deno projects don't typically pin versions in config files,
//! so this module primarily exists for consistency.

use std::io;
use std::path::Path;

/// Gets Deno version for the project.
///
/// Currently returns "latest" as Deno projects don't typically
/// pin SDK versions in configuration files.
///
/// In the future, this could read from:
/// - `.dvmrc` (Deno Version Manager)
/// - `deno.json` if it gains version pinning support
pub fn get_deno_version(_path: &Path) -> io::Result<String> {
    // Deno doesn't have a standard version pinning mechanism yet
    // Could support .dvmrc in the future
    Ok("latest".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_returns_latest() {
        let dir = tempdir().unwrap();
        let version = get_deno_version(dir.path()).unwrap();
        assert_eq!(version, "latest");
    }
}
