use std::path::Path;

#[derive(Debug, PartialEq, Eq)]
pub enum ProjectType {
    Buck2,
    Bazel,
    Cargo,
    Maven,
    Gradle,
    Npm,
    Unknown,
}

pub fn detect_project_type(path: &Path) -> ProjectType {
    if path.join(".buckconfig").exists() || path.join("BUCK").exists() {
        return ProjectType::Buck2;
    }
    if path.join("WORKSPACE").exists()
        || path.join("WORKSPACE.bazel").exists()
        || path.join("MODULE.bazel").exists()
    {
        return ProjectType::Bazel;
    }
    if path.join("Cargo.toml").exists() {
        return ProjectType::Cargo;
    }
    if path.join("pom.xml").exists() {
        return ProjectType::Maven;
    }
    if path.join("build.gradle").exists() || path.join("build.gradle.kts").exists() {
        return ProjectType::Gradle;
    }
    if path.join("package.json").exists() {
        return ProjectType::Npm;
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
}
