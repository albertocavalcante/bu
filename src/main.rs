//! bu - A smart build tool wrapper
//!
//! Automatically detects your project type and runs the appropriate build tool
//! with zero configuration.

mod bazel;
mod buck2;
mod config;
mod deno;
mod detector;
mod dotnet;
mod gradle;
mod maven;
mod npm;
mod python;
mod tool_cache;
mod toolchain;

use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use tracing::{debug, info, warn};

use detector::ProjectType;

// ============================================================================
// CLI Definition
// ============================================================================

#[derive(Parser, Debug)]
#[command(name = "bu")]
#[command(version)]
#[command(about = "A smart build tool wrapper")]
#[command(long_about = "A universal build tool wrapper that automatically detects your project type \
and runs the appropriate build tool with zero configuration.

Examples:
  bu build                    Run the detected tool's build command
  bu test                     Run tests using the detected tool
  bu which                    Show which tool would be executed
  bu config                   Show effective configuration
  bu cache list               List cached tools
  bu cache clean              Clear all cached tools
  bu completions bash         Generate bash completions")]
struct Cli {
    /// Run in offline mode (don't download tools)
    #[arg(long)]
    offline: bool,

    /// Enable verbose output for debugging
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Option<Commands>,

    /// Arguments to pass to the detected build tool
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Show the resolved tool path that would be executed
    Which,

    /// Show effective configuration (detected tool, version, providers)
    Config,

    /// Cache management commands
    Cache {
        #[command(subcommand)]
        command: CacheCommands,
    },

    /// Generate shell completions
    Completions {
        /// The shell to generate completions for
        shell: Shell,
    },
}

#[derive(Subcommand, Debug)]
enum CacheCommands {
    /// List cached tools
    List,

    /// Remove all cached tools
    Clean,
}

// ============================================================================
// Tool Resolution (shared logic)
// ============================================================================

/// Resolved tool information ready for execution or display.
struct ToolResolution {
    project_type: ProjectType,
    tool_name: &'static str,
    version: String,
    tool_path: PathBuf,
    #[allow(dead_code)] // Reserved for future use (e.g., displaying config details)
    config: config::Config,
    cwd: PathBuf,
}

/// Resolves the tool for the current directory.
///
/// This is the shared logic used by both `run_tool` and `get_tool_info`.
fn resolve_tool(offline: bool) -> Result<ToolResolution> {
    let cwd = std::env::current_dir().context("Failed to get current directory")?;

    // 1. Detect project type
    let project_type = detector::detect_project_type(&cwd);
    if !project_type.is_known() {
        anyhow::bail!(
            "Could not detect project type in {:?}.\n\n\
            Supported build tools:\n  \
            Monorepo: Buck2, Bazel\n  \
            Systems:  Cargo, Go, Zig\n  \
            JVM:      Maven, Gradle\n  \
            JS/TS:    npm, pnpm, Yarn, Bun, Deno\n  \
            Python:   uv, Poetry, pip\n  \
            Other:    .NET, Swift, Bundler, Mix, Composer\n  \
            Tasks:    Make, Just, CMake",
            cwd
        );
    }

    let tool_name = project_type.tool_name();
    info!("Detected project type: {}", project_type);

    // 2. Load configuration
    let config_path = cwd.join("bu.star");
    let config = load_config(&config_path)?;

    // 3. Determine version (with warning on error instead of silent failure)
    let version = get_version_with_warning(project_type, &cwd);
    debug!("Using version: {}", version);

    // 4. Resolve tool path via provider chain
    let provider = get_provider(&config, tool_name);
    let cache = tool_cache::ToolCache::new()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory for cache"))?;

    let tool_context = toolchain::ToolContext {
        offline,
        cache: &cache,
    };

    let tool_path = provider
        .provide(tool_name, &version, &tool_context)
        .with_context(|| format!("Failed to provide tool '{}' version '{}'", tool_name, version))?;

    info!("Resolved tool path: {:?}", tool_path);

    Ok(ToolResolution {
        project_type,
        tool_name,
        version,
        tool_path,
        config,
        cwd,
    })
}

/// Loads configuration from bu.star if it exists.
fn load_config(config_path: &Path) -> Result<config::Config> {
    if config_path.exists() {
        info!("Loading configuration from {:?}", config_path);
        let content = std::fs::read_to_string(config_path)
            .with_context(|| format!("Failed to read config file: {:?}", config_path))?;
        config::load_config(&content).context("Failed to parse bu.star")
    } else {
        debug!("No bu.star found, using defaults");
        Ok(config::Config::default())
    }
}

/// Gets version for the tool, logging a warning on error instead of silently failing.
fn get_version_with_warning(project_type: ProjectType, cwd: &Path) -> String {
    match project_type.get_version(cwd) {
        Ok(version) => version,
        Err(e) => {
            warn!(
                "Failed to read version file for {}: {}. Using 'latest'",
                project_type, e
            );
            "latest".to_string()
        }
    }
}

/// Gets the appropriate provider for the tool.
fn get_provider(config: &config::Config, tool_name: &str) -> Box<dyn toolchain::ToolProvider> {
    config.get_tool_provider(tool_name).unwrap_or_else(|| {
        Box::new(toolchain::ChainProvider::new(vec![Box::new(
            toolchain::HostProvider,
        )]))
    })
}

// ============================================================================
// Main Entry Point
// ============================================================================

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging based on verbose flag
    let log_level = if cli.verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::WARN
    };
    tracing_subscriber::fmt().with_max_level(log_level).init();

    // Dispatch to subcommands or default tool execution
    match cli.command {
        Some(Commands::Which) => cmd_which(cli.offline),
        Some(Commands::Config) => cmd_config(cli.offline),
        Some(Commands::Cache { command }) => match command {
            CacheCommands::List => cmd_cache_list(),
            CacheCommands::Clean => cmd_cache_clean(),
        },
        Some(Commands::Completions { shell }) => {
            cmd_completions(shell);
            Ok(())
        }
        None => cmd_run(cli.offline, &cli.args),
    }
}

// ============================================================================
// Subcommand Implementations
// ============================================================================

/// Default command: execute the detected build tool.
fn cmd_run(offline: bool, args: &[String]) -> Result<()> {
    let resolution = resolve_tool(offline)?;

    let status = Command::new(&resolution.tool_path)
        .args(args)
        .status()
        .with_context(|| format!("Failed to execute {:?}", resolution.tool_path))?;

    std::process::exit(status.code().unwrap_or(1));
}

/// Show which tool would be executed.
fn cmd_which(offline: bool) -> Result<()> {
    let resolution = resolve_tool(offline)?;
    println!("{}", resolution.tool_path.display());
    Ok(())
}

/// Show effective configuration.
fn cmd_config(offline: bool) -> Result<()> {
    let resolution = resolve_tool(offline)?;

    println!("Tool:         {}", resolution.tool_name);
    println!("Version:      {}", resolution.version);
    println!("Path:         {}", resolution.tool_path.display());
    println!("Project type: {}", resolution.project_type);
    println!(
        "Config file:  {}",
        if resolution.cwd.join("bu.star").exists() {
            "bu.star"
        } else {
            "(none)"
        }
    );
    Ok(())
}

/// List cached tools.
fn cmd_cache_list() -> Result<()> {
    let cache = tool_cache::ToolCache::new()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
    let cache_dir = cache.cache_dir();

    if !cache_dir.exists() {
        println!("Cache is empty");
        return Ok(());
    }

    let mut found = false;
    for entry in std::fs::read_dir(cache_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            found = true;
            let name = entry.file_name();
            let size = dir_size(&entry.path()).unwrap_or(0);
            println!("{:<30} {:>10}", name.to_string_lossy(), format_size(size));
        }
    }

    if !found {
        println!("Cache is empty");
    }

    Ok(())
}

/// Remove all cached tools.
fn cmd_cache_clean() -> Result<()> {
    let cache = tool_cache::ToolCache::new()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
    let cache_dir = cache.cache_dir();

    if cache_dir.exists() {
        std::fs::remove_dir_all(cache_dir)?;
        std::fs::create_dir_all(cache_dir)?;
        println!("Cache cleaned");
    } else {
        println!("Cache is already empty");
    }

    Ok(())
}

/// Generate shell completions.
fn cmd_completions(shell: Shell) {
    let mut cmd = Cli::command();
    generate(shell, &mut cmd, "bu", &mut io::stdout());
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Calculate directory size recursively.
fn dir_size(path: &Path) -> Result<u64> {
    let mut size = 0;
    if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let entry_path = entry.path();
            if entry_path.is_dir() {
                size += dir_size(&entry_path)?;
            } else {
                size += entry.metadata()?.len();
            }
        }
    }
    Ok(size)
}

/// Format size in human-readable form.
fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        format!("{:.1} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.1} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.1} KB", size as f64 / KB as f64)
    } else {
        format!("{} B", size)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing_no_args() {
        let cli = Cli::try_parse_from(["bu"]).unwrap();
        assert!(!cli.offline);
        assert!(!cli.verbose);
        assert!(cli.command.is_none());
        assert!(cli.args.is_empty());
    }

    #[test]
    fn test_cli_parsing_with_tool_args() {
        let cli = Cli::try_parse_from(["bu", "build", "--release"]).unwrap();
        assert!(cli.command.is_none());
        assert_eq!(cli.args, vec!["build", "--release"]);
    }

    #[test]
    fn test_cli_parsing_verbose() {
        let cli = Cli::try_parse_from(["bu", "-v"]).unwrap();
        assert!(cli.verbose);
    }

    #[test]
    fn test_cli_parsing_verbose_long() {
        let cli = Cli::try_parse_from(["bu", "--verbose"]).unwrap();
        assert!(cli.verbose);
    }

    #[test]
    fn test_cli_parsing_offline() {
        let cli = Cli::try_parse_from(["bu", "--offline"]).unwrap();
        assert!(cli.offline);
    }

    #[test]
    fn test_cli_parsing_which_subcommand() {
        let cli = Cli::try_parse_from(["bu", "which"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Which)));
    }

    #[test]
    fn test_cli_parsing_config_subcommand() {
        let cli = Cli::try_parse_from(["bu", "config"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Config)));
    }

    #[test]
    fn test_cli_parsing_cache_list() {
        let cli = Cli::try_parse_from(["bu", "cache", "list"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Cache {
                command: CacheCommands::List
            })
        ));
    }

    #[test]
    fn test_cli_parsing_cache_clean() {
        let cli = Cli::try_parse_from(["bu", "cache", "clean"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Cache {
                command: CacheCommands::Clean
            })
        ));
    }

    #[test]
    fn test_cli_parsing_completions_bash() {
        let cli = Cli::try_parse_from(["bu", "completions", "bash"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Completions { shell: Shell::Bash })
        ));
    }

    #[test]
    fn test_cli_parsing_completions_zsh() {
        let cli = Cli::try_parse_from(["bu", "completions", "zsh"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Completions { shell: Shell::Zsh })
        ));
    }

    #[test]
    fn test_cli_parsing_completions_fish() {
        let cli = Cli::try_parse_from(["bu", "completions", "fish"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Completions { shell: Shell::Fish })
        ));
    }

    #[test]
    fn test_format_size_bytes() {
        assert_eq!(format_size(500), "500 B");
    }

    #[test]
    fn test_format_size_kb() {
        assert_eq!(format_size(2048), "2.0 KB");
    }

    #[test]
    fn test_format_size_mb() {
        assert_eq!(format_size(5 * 1024 * 1024), "5.0 MB");
    }

    #[test]
    fn test_format_size_gb() {
        assert_eq!(format_size(2 * 1024 * 1024 * 1024), "2.0 GB");
    }
}
