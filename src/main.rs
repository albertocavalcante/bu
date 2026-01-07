mod bazel;
mod buck2;
mod config;
mod detector;
mod gradle;
mod maven;
mod npm;
mod tool_cache;
mod toolchain;

use anyhow::{Context, Result};
use clap::Parser;
use std::process::Command;
use tracing::{debug, error, info};

#[derive(Parser, Debug)]
#[command(name = "bu")]
#[command(about = "A smart build tool wrapper", long_about = None)]
struct Args {
    #[arg(long, help = "Run in offline mode")]
    offline: bool,

    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    let cwd = std::env::current_dir()?;

    // 1. Detect Project Type
    let project_type = detector::detect_project_type(&cwd);
    info!("Detected project type: {:?}", project_type);

    let tool_name = match project_type {
        detector::ProjectType::Buck2 => "buck2",
        detector::ProjectType::Bazel => "bazel",
        detector::ProjectType::Cargo => "cargo",
        detector::ProjectType::Maven => "mvn",
        detector::ProjectType::Gradle => "gradle",
        detector::ProjectType::Npm => "npm",
        detector::ProjectType::Unknown => {
            error!("Could not detect project type in {:?}", cwd);
            std::process::exit(1);
        }
    };

    // 2. Load Configuration
    // Check for bu.star, otherwise use default config
    let config_path = cwd.join("bu.star");
    let config = if config_path.exists() {
        info!("Loading configuration from {:?}", config_path);
        let content = std::fs::read_to_string(&config_path)?;
        config::load_config(&content).context("Failed to load bu.star")?
    } else {
        debug!("No bu.star found, using defaults");
        config::Config::default()
    };

    // 3. Determine Version
    // Read tool-specific version files, fallback to "latest"
    let version = match tool_name {
        "buck2" => buck2::get_buck2_version(&cwd).unwrap_or_else(|_| "latest".to_string()),
        "bazel" => bazel::get_bazel_version(&cwd).unwrap_or_else(|_| "latest".to_string()),
        "npm" => npm::get_node_version(&cwd).unwrap_or_else(|_| "latest".to_string()),
        "gradle" => gradle::get_gradle_version(&cwd).unwrap_or_else(|_| "latest".to_string()),
        "mvn" => maven::get_maven_version(&cwd).unwrap_or_else(|_| "latest".to_string()),
        _ => "latest".to_string(),
    };

    // 4. Resolve Tool
    // If user config defines the tool, use that. Otherwise fallback to HostProvider.
    let provider = if let Some(p) = config.get_tool_provider(tool_name) {
        p
    } else {
        // Default provider chain for unknown tools in config
        Box::new(toolchain::ChainProvider::new(vec![
            Box::new(toolchain::HostProvider),
            // We could add a default UrlProvider here if we had a registry
        ]))
    };

    let tool_context = toolchain::ToolContext {
        offline: args.offline,
        cache: &tool_cache::ToolCache::new().expect("Failed to initialize cache"),
    };

    let tool_path = provider
        .provide(tool_name, &version, &tool_context)
        .context(format!(
            "Failed to provide tool '{}' version '{}'",
            tool_name, version
        ))?;

    info!("Using tool at: {:?}", tool_path);

    // 5. Execute Tool
    let status = Command::new(tool_path)
        .args(&args.args)
        .status()
        .context("Failed to execute tool")?;

    std::process::exit(status.code().unwrap_or(1));
}
