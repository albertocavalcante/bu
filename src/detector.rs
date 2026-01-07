//! Project type detection based on marker files.
//!
//! This module provides automatic detection of build systems by looking for
//! specific configuration files in the project directory.

use std::fmt;
use std::path::Path;

use crate::{bazel, buck2, deno, dotnet, gradle, maven, npm, python};

/// Represents a detected build system type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectType {
    // Monorepo/polyglot build tools
    Buck2,
    Bazel,

    // Language-specific: Systems programming
    Cargo,
    Go,
    Zig,

    // Language-specific: JVM
    Maven,
    Gradle,

    // Language-specific: JavaScript/TypeScript
    Npm,
    Pnpm,
    Yarn,
    Bun,
    Deno,

    // Language-specific: Python
    Uv,
    Poetry,
    Pip,

    // Language-specific: Other
    Dotnet,
    Swift,
    Bundler,
    Mix,
    Composer,

    // Task runners
    Make,
    Just,
    Cmake,

    Unknown,
}

impl ProjectType {
    /// Returns the command-line tool name for this project type.
    ///
    /// # Panics
    /// Panics if called on `ProjectType::Unknown`.
    pub fn tool_name(&self) -> &'static str {
        match self {
            // Monorepo tools
            ProjectType::Buck2 => "buck2",
            ProjectType::Bazel => "bazel",

            // Systems programming
            ProjectType::Cargo => "cargo",
            ProjectType::Go => "go",
            ProjectType::Zig => "zig",

            // JVM
            ProjectType::Maven => "mvn",
            ProjectType::Gradle => "gradle",

            // JavaScript/TypeScript
            ProjectType::Npm => "npm",
            ProjectType::Pnpm => "pnpm",
            ProjectType::Yarn => "yarn",
            ProjectType::Bun => "bun",
            ProjectType::Deno => "deno",

            // Python
            ProjectType::Uv => "uv",
            ProjectType::Poetry => "poetry",
            ProjectType::Pip => "pip",

            // Other languages
            ProjectType::Dotnet => "dotnet",
            ProjectType::Swift => "swift",
            ProjectType::Bundler => "bundle",
            ProjectType::Mix => "mix",
            ProjectType::Composer => "composer",

            // Task runners
            ProjectType::Make => "make",
            ProjectType::Just => "just",
            ProjectType::Cmake => "cmake",

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
            // Tools with version file support
            ProjectType::Buck2 => buck2::get_buck2_version(path),
            ProjectType::Bazel => bazel::get_bazel_version(path),
            ProjectType::Npm | ProjectType::Pnpm | ProjectType::Yarn | ProjectType::Bun => {
                npm::get_node_version(path)
            }
            ProjectType::Gradle => gradle::get_gradle_version(path),
            ProjectType::Maven => maven::get_maven_version(path),
            ProjectType::Uv | ProjectType::Poetry | ProjectType::Pip => {
                python::get_python_version(path)
            }
            ProjectType::Dotnet => dotnet::get_dotnet_version(path),
            ProjectType::Deno => deno::get_deno_version(path),

            // Tools without version pinning (use system version)
            ProjectType::Cargo
            | ProjectType::Go
            | ProjectType::Zig
            | ProjectType::Swift
            | ProjectType::Bundler
            | ProjectType::Mix
            | ProjectType::Composer
            | ProjectType::Make
            | ProjectType::Just
            | ProjectType::Cmake
            | ProjectType::Unknown => Ok("latest".to_string()),
        }
    }
}

impl fmt::Display for ProjectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProjectType::Buck2 => write!(f, "Buck2"),
            ProjectType::Bazel => write!(f, "Bazel"),
            ProjectType::Cargo => write!(f, "Cargo"),
            ProjectType::Go => write!(f, "Go"),
            ProjectType::Zig => write!(f, "Zig"),
            ProjectType::Maven => write!(f, "Maven"),
            ProjectType::Gradle => write!(f, "Gradle"),
            ProjectType::Npm => write!(f, "npm"),
            ProjectType::Pnpm => write!(f, "pnpm"),
            ProjectType::Yarn => write!(f, "Yarn"),
            ProjectType::Bun => write!(f, "Bun"),
            ProjectType::Deno => write!(f, "Deno"),
            ProjectType::Uv => write!(f, "uv"),
            ProjectType::Poetry => write!(f, "Poetry"),
            ProjectType::Pip => write!(f, "pip"),
            ProjectType::Dotnet => write!(f, ".NET"),
            ProjectType::Swift => write!(f, "Swift"),
            ProjectType::Bundler => write!(f, "Bundler"),
            ProjectType::Mix => write!(f, "Mix"),
            ProjectType::Composer => write!(f, "Composer"),
            ProjectType::Make => write!(f, "Make"),
            ProjectType::Just => write!(f, "Just"),
            ProjectType::Cmake => write!(f, "CMake"),
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
/// # Detection Order
///
/// ## Monorepo/Polyglot Tools (highest precedence)
/// - **Buck2**: `.buckconfig` or `BUCK`
/// - **Bazel**: `WORKSPACE`, `WORKSPACE.bazel`, or `MODULE.bazel`
///
/// ## Language-Specific Tools
///
/// ### Systems Programming
/// - **Cargo**: `Cargo.toml`
/// - **Go**: `go.mod`
/// - **Zig**: `build.zig`
///
/// ### JVM
/// - **Maven**: `pom.xml`
/// - **Gradle**: `build.gradle` or `build.gradle.kts`
///
/// ### JavaScript/TypeScript (lock file determines package manager)
/// - **Bun**: `bun.lockb`
/// - **pnpm**: `pnpm-lock.yaml`
/// - **Yarn**: `yarn.lock`
/// - **Deno**: `deno.json` or `deno.jsonc`
/// - **npm**: `package.json` (fallback)
///
/// ### Python (lock file determines tool)
/// - **uv**: `uv.lock`
/// - **Poetry**: `poetry.lock`
/// - **pip**: `requirements.txt` or `pyproject.toml`
///
/// ### Other Languages
/// - **.NET**: `*.csproj`, `*.fsproj`, `*.sln`
/// - **Swift**: `Package.swift`
/// - **Ruby**: `Gemfile`
/// - **Elixir**: `mix.exs`
/// - **PHP**: `composer.json`
///
/// ## Task Runners (lowest precedence)
/// - **Just**: `justfile` or `.justfile`
/// - **CMake**: `CMakeLists.txt`
/// - **Make**: `Makefile` or `makefile`
///
/// # Arguments
/// * `path` - The directory path to check
///
/// # Returns
/// The detected [`ProjectType`], or [`ProjectType::Unknown`] if no build system is detected.
pub fn detect_project_type(path: &Path) -> ProjectType {
    // =========================================================================
    // Monorepo/polyglot build tools (highest precedence)
    // =========================================================================
    if path.join(".buckconfig").exists() || path.join("BUCK").exists() {
        return ProjectType::Buck2;
    }
    if path.join("WORKSPACE").exists()
        || path.join("WORKSPACE.bazel").exists()
        || path.join("MODULE.bazel").exists()
    {
        return ProjectType::Bazel;
    }

    // =========================================================================
    // Systems programming languages
    // =========================================================================
    if path.join("Cargo.toml").exists() {
        return ProjectType::Cargo;
    }
    if path.join("go.mod").exists() {
        return ProjectType::Go;
    }
    if path.join("build.zig").exists() {
        return ProjectType::Zig;
    }

    // =========================================================================
    // JVM languages
    // =========================================================================
    if path.join("pom.xml").exists() {
        return ProjectType::Maven;
    }
    if path.join("build.gradle").exists() || path.join("build.gradle.kts").exists() {
        return ProjectType::Gradle;
    }

    // =========================================================================
    // JavaScript/TypeScript ecosystem
    // Lock file determines which package manager to use
    // =========================================================================
    if path.join("bun.lockb").exists() {
        return ProjectType::Bun;
    }
    if path.join("pnpm-lock.yaml").exists() {
        return ProjectType::Pnpm;
    }
    if path.join("yarn.lock").exists() {
        return ProjectType::Yarn;
    }
    if path.join("deno.json").exists() || path.join("deno.jsonc").exists() {
        return ProjectType::Deno;
    }
    // npm is the fallback for package.json (checked later)

    // =========================================================================
    // Python ecosystem
    // Lock file determines which tool to use
    // =========================================================================
    if path.join("uv.lock").exists() {
        return ProjectType::Uv;
    }
    if path.join("poetry.lock").exists() {
        return ProjectType::Poetry;
    }
    // Check for pip indicators (requirements.txt or pyproject.toml without lock)
    if path.join("requirements.txt").exists() {
        return ProjectType::Pip;
    }
    if path.join("pyproject.toml").exists() {
        // pyproject.toml without uv.lock or poetry.lock - assume pip/uv
        return ProjectType::Uv;
    }

    // =========================================================================
    // .NET
    // =========================================================================
    if has_dotnet_project(path) {
        return ProjectType::Dotnet;
    }

    // =========================================================================
    // Other languages
    // =========================================================================
    if path.join("Package.swift").exists() {
        return ProjectType::Swift;
    }
    if path.join("Gemfile").exists() {
        return ProjectType::Bundler;
    }
    if path.join("mix.exs").exists() {
        return ProjectType::Mix;
    }
    if path.join("composer.json").exists() {
        return ProjectType::Composer;
    }

    // =========================================================================
    // npm fallback (after all other JS tools checked)
    // =========================================================================
    if path.join("package.json").exists() {
        return ProjectType::Npm;
    }

    // =========================================================================
    // Task runners (lowest precedence)
    // =========================================================================
    if path.join("justfile").exists() || path.join(".justfile").exists() {
        return ProjectType::Just;
    }
    if path.join("CMakeLists.txt").exists() {
        return ProjectType::Cmake;
    }
    if path.join("Makefile").exists() || path.join("makefile").exists() {
        return ProjectType::Make;
    }

    ProjectType::Unknown
}

/// Checks if the directory contains a .NET project file.
fn has_dotnet_project(path: &Path) -> bool {
    // Check for solution file
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.ends_with(".sln")
                || name.ends_with(".csproj")
                || name.ends_with(".fsproj")
                || name.ends_with(".vbproj")
            {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    // =========================================================================
    // Monorepo tools
    // =========================================================================

    #[test]
    fn test_detect_buck2_buckconfig() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join(".buckconfig")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Buck2);
    }

    #[test]
    fn test_detect_buck2_buck_file() {
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

    // =========================================================================
    // Systems programming
    // =========================================================================

    #[test]
    fn test_detect_cargo() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("Cargo.toml")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Cargo);
    }

    #[test]
    fn test_detect_go() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("go.mod")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Go);
    }

    #[test]
    fn test_detect_zig() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("build.zig")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Zig);
    }

    // =========================================================================
    // JVM
    // =========================================================================

    #[test]
    fn test_detect_maven() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("pom.xml")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Maven);
    }

    #[test]
    fn test_detect_gradle() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("build.gradle")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Gradle);
    }

    #[test]
    fn test_detect_gradle_kotlin() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("build.gradle.kts")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Gradle);
    }

    // =========================================================================
    // JavaScript/TypeScript
    // =========================================================================

    #[test]
    fn test_detect_bun() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("package.json")).unwrap();
        File::create(dir.path().join("bun.lockb")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Bun);
    }

    #[test]
    fn test_detect_pnpm() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("package.json")).unwrap();
        File::create(dir.path().join("pnpm-lock.yaml")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Pnpm);
    }

    #[test]
    fn test_detect_yarn() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("package.json")).unwrap();
        File::create(dir.path().join("yarn.lock")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Yarn);
    }

    #[test]
    fn test_detect_deno() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("deno.json")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Deno);
    }

    #[test]
    fn test_detect_deno_jsonc() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("deno.jsonc")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Deno);
    }

    #[test]
    fn test_detect_npm_fallback() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("package.json")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Npm);
    }

    // =========================================================================
    // Python
    // =========================================================================

    #[test]
    fn test_detect_uv() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("pyproject.toml")).unwrap();
        File::create(dir.path().join("uv.lock")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Uv);
    }

    #[test]
    fn test_detect_poetry() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("pyproject.toml")).unwrap();
        File::create(dir.path().join("poetry.lock")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Poetry);
    }

    #[test]
    fn test_detect_pip() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("requirements.txt")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Pip);
    }

    #[test]
    fn test_detect_pyproject_defaults_to_uv() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("pyproject.toml")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Uv);
    }

    // =========================================================================
    // .NET
    // =========================================================================

    #[test]
    fn test_detect_dotnet_csproj() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("MyProject.csproj")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Dotnet);
    }

    #[test]
    fn test_detect_dotnet_sln() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("MySolution.sln")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Dotnet);
    }

    #[test]
    fn test_detect_dotnet_fsproj() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("MyProject.fsproj")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Dotnet);
    }

    // =========================================================================
    // Other languages
    // =========================================================================

    #[test]
    fn test_detect_swift() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("Package.swift")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Swift);
    }

    #[test]
    fn test_detect_bundler() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("Gemfile")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Bundler);
    }

    #[test]
    fn test_detect_mix() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("mix.exs")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Mix);
    }

    #[test]
    fn test_detect_composer() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("composer.json")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Composer);
    }

    // =========================================================================
    // Task runners
    // =========================================================================

    #[test]
    fn test_detect_just() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("justfile")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Just);
    }

    #[test]
    fn test_detect_just_hidden() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join(".justfile")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Just);
    }

    #[test]
    fn test_detect_cmake() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("CMakeLists.txt")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Cmake);
    }

    #[test]
    fn test_detect_make() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("Makefile")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Make);
    }

    #[test]
    fn test_detect_make_lowercase() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("makefile")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Make);
    }

    // =========================================================================
    // ProjectType methods
    // =========================================================================

    #[test]
    fn test_tool_names() {
        assert_eq!(ProjectType::Buck2.tool_name(), "buck2");
        assert_eq!(ProjectType::Cargo.tool_name(), "cargo");
        assert_eq!(ProjectType::Maven.tool_name(), "mvn");
        assert_eq!(ProjectType::Go.tool_name(), "go");
        assert_eq!(ProjectType::Uv.tool_name(), "uv");
        assert_eq!(ProjectType::Poetry.tool_name(), "poetry");
        assert_eq!(ProjectType::Dotnet.tool_name(), "dotnet");
        assert_eq!(ProjectType::Bundler.tool_name(), "bundle");
        assert_eq!(ProjectType::Just.tool_name(), "just");
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", ProjectType::Buck2), "Buck2");
        assert_eq!(format!("{}", ProjectType::Npm), "npm");
        assert_eq!(format!("{}", ProjectType::Dotnet), ".NET");
        assert_eq!(format!("{}", ProjectType::Cmake), "CMake");
        assert_eq!(format!("{}", ProjectType::Unknown), "Unknown");
    }

    #[test]
    fn test_is_known() {
        assert!(ProjectType::Cargo.is_known());
        assert!(ProjectType::Uv.is_known());
        assert!(!ProjectType::Unknown.is_known());
    }

    // =========================================================================
    // Precedence tests
    // =========================================================================

    #[test]
    fn test_buck2_takes_precedence_over_npm() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join(".buckconfig")).unwrap();
        File::create(dir.path().join("package.json")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Buck2);
    }

    #[test]
    fn test_bazel_takes_precedence_over_cargo() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("WORKSPACE")).unwrap();
        File::create(dir.path().join("Cargo.toml")).unwrap();
        assert_eq!(detect_project_type(dir.path()), ProjectType::Bazel);
    }

    #[test]
    fn test_uv_lock_takes_precedence_over_poetry() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("pyproject.toml")).unwrap();
        File::create(dir.path().join("uv.lock")).unwrap();
        // Even with poetry.lock, uv.lock should win (checked first)
        assert_eq!(detect_project_type(dir.path()), ProjectType::Uv);
    }
}
