//! Project type detection based on marker files.
//!
//! This module provides automatic detection of build systems by looking for
//! specific configuration files in the project directory.

use std::fmt;
use std::path::Path;

use crate::{bazel, buck2, gradle, maven, npm};

/// Represents a detected build system type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectType {
    Buck2,
    Bazel,
    Cargo,
    Maven,
    Gradle,
    Npm,
    Go,
    Make,
    Pnpm,
    Yarn,
    Unknown,
}

impl ProjectType {
    /// Returns the command-line tool name for this project type.
    ///
    /// # Panics
    /// Panics if called on `ProjectType::Unknown`.
    pub fn tool_name(&self) -> &'static str {
        match self {
            ProjectType::Buck2 => "buck2",
            ProjectType::Bazel => "bazel",
            ProjectType::Cargo => "cargo",
            ProjectType::Maven => "mvn",
            ProjectType::Gradle => "gradle",
            ProjectType::Npm => "npm",
            ProjectType::Go => "go",
            ProjectType::Make => "make",
            ProjectType::Pnpm => "pnpm",
            ProjectType::Yarn => "yarn",
            ProjectType::Unknown => panic!("Cannot get tool name for Unknown project type"),
        }
    }

    /// Returns whether this project type is known (not Unknown).
    pub fn is_known(&self) -> bool {
        !matches!(self, ProjectType::Unknown)
    }

    /// Reads the version for this project type from the given directory.
    ///
    /// Returns `Ok("latest")` for project types that don't have version files
    /// or if the version file doesn't exist.
    pub fn get_version(&self, path: &Path) -> std::io::Result<String> {
        match self {
            ProjectType::Buck2 => buck2::get_buck2_version(path),
            ProjectType::Bazel => bazel::get_bazel_version(path),
            ProjectType::Npm | ProjectType::Pnpm | ProjectType::Yarn => npm::get_node_version(path),
            ProjectType::Gradle => gradle::get_gradle_version(path),
            ProjectType::Maven => maven::get_maven_version(path),
            // These tools don't typically have version pinning
            ProjectType::Cargo | ProjectType::Go | ProjectType::Make | ProjectType::Unknown => {
                Ok("latest".to_string())
            }
        }
    }
}

impl fmt::Display for ProjectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProjectType::Buck2 => write!(f, "Buck2"),
            ProjectType::Bazel => write!(f, "Bazel"),
            ProjectType::Cargo => write!(f, "Cargo"),
            ProjectType::Maven => write!(f, "Maven"),
            ProjectType::Gradle => write!(f, "Gradle"),
            ProjectType::Npm => write!(f, "npm"),
            ProjectType::Go => write!(f, "Go"),
            ProjectType::Make => write!(f, "Make"),
            ProjectType::Pnpm => write!(f, "pnpm"),
            ProjectType::Yarn => write!(f, "Yarn"),
            ProjectType::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Detects the build system type for a project at the given path.
///
/// Detection is based on the presence of specific marker files. The order
/// of detection matters - monorepo tools (Buck2, Bazel) are checked first,
/// followed by language-specific build tools.
///
/// # Marker files by project type:
/// - **Buck2**: `.buckconfig` or `BUCK`
/// - **Bazel**: `WORKSPACE`, `WORKSPACE.bazel`, or `MODULE.bazel`
/// - **Cargo**: `Cargo.toml`
/// - **Maven**: `pom.xml`
/// - **Gradle**: `build.gradle` or `build.gradle.kts`
/// - **Go**: `go.mod`
/// - **Make**: `Makefile` or `makefile`
/// - **pnpm**: `pnpm-lock.yaml`
/// - **Yarn**: `yarn.lock`
/// - **npm**: `package.json` (checked last among JS tools)
///
/// # Arguments
/// * `path` - The directory path to check
///
/// # Returns
/// The detected [`ProjectType`], or [`ProjectType::Unknown`] if no build system is detected.
pub fn detect_project_type(path: &Path) -> ProjectType {
    // Monorepo/polyglot build tools first (highest precedence)
    if path.join(".buckconfig").exists() || path.join("BUCK").exists() {
        return ProjectType::Buck2;
    }
    if path.join("WORKSPACE").exists()
        || path.join("WORKSPACE.bazel").exists()
        || path.join("MODULE.bazel").exists()
    {
        return ProjectType::Bazel;
    }

    // Language-specific build tools
    if path.join("Cargo.toml").exists() {
        return ProjectType::Cargo;
    }
    if path.join("go.mod").exists() {
        return ProjectType::Go;
    }
    if path.join("pom.xml").exists() {
        return ProjectType::Maven;
    }
    if path.join("build.gradle").exists() || path.join("build.gradle.kts").exists() {
        return ProjectType::Gradle;
    }

    // JavaScript ecosystem - check specific package managers before generic npm
    if path.join("pnpm-lock.yaml").exists() {
        return ProjectType::Pnpm;
    }
    if path.join("yarn.lock").exists() {
        return ProjectType::Yarn;
    }
    if path.join("package.json").exists() {
        return ProjectType::Npm;
    }

    // Task runners (lower precedence)
    if path.join("Makefile").exists() || path.join("makefile").exists() {
        return ProjectType::Make;
    }

    ProjectType::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_detect_cargo_project() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("Cargo.toml")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Cargo);
    }

    #[test]
    fn test_detect_buck2_project() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join(".buckconfig")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Buck2);
    }

    #[test]
    fn test_detect_buck2_project_alternative() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("BUCK")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Buck2);
    }

    #[test]
    fn test_detect_bazel_workspace() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("WORKSPACE")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Bazel);
    }

    #[test]
    fn test_detect_bazel_module() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("MODULE.bazel")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Bazel);
    }

    #[test]
    fn test_detect_maven_project() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("pom.xml")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Maven);
    }

    #[test]
    fn test_detect_gradle_project() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("build.gradle")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Gradle);
    }

    #[test]
    fn test_detect_gradle_kotlin_project() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("build.gradle.kts")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Gradle);
    }

    #[test]
    fn test_detect_npm_project() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("package.json")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Npm);
    }

    #[test]
    fn test_detect_go_project() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("go.mod")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Go);
    }

    #[test]
    fn test_detect_make_project() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("Makefile")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Make);
    }

    #[test]
    fn test_detect_pnpm_project() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("package.json")).unwrap();
        File::create(dir.path().join("pnpm-lock.yaml")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Pnpm);
    }

    #[test]
    fn test_detect_yarn_project() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("package.json")).unwrap();
        File::create(dir.path().join("yarn.lock")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Yarn);
    }

    #[test]
    fn test_tool_name() {
        assert_eq!(ProjectType::Buck2.tool_name(), "buck2");
        assert_eq!(ProjectType::Cargo.tool_name(), "cargo");
        assert_eq!(ProjectType::Maven.tool_name(), "mvn");
        assert_eq!(ProjectType::Go.tool_name(), "go");
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", ProjectType::Buck2), "Buck2");
        assert_eq!(format!("{}", ProjectType::Npm), "npm");
        assert_eq!(format!("{}", ProjectType::Unknown), "Unknown");
    }

    #[test]
    fn test_is_known() {
        assert!(ProjectType::Cargo.is_known());
        assert!(!ProjectType::Unknown.is_known());
    }
}
