use std::path::PathBuf;
use std::io::{self};
use std::fs::{self, File};
use thiserror::Error;
use which::which;
use tracing::{debug, info, instrument};
use sha2::{Sha256, Digest};
use crate::tool_cache::ToolCache;

#[derive(Error, Debug)]
pub enum ToolError {
    #[error("Tool '{0}' not found")]
    NotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Strategy '{0}' failed: {1}")]
    StrategyFailure(String, String),
}

#[derive(Debug)]
pub struct ToolContext<'a> {
    pub offline: bool,
    pub cache: &'a ToolCache,
}

pub trait ToolProvider: std::fmt::Debug {
    fn provide(&self, tool: &str, version: &str, context: &ToolContext) -> Result<PathBuf, ToolError>;
}

#[derive(Debug)]
pub struct HostProvider;

impl ToolProvider for HostProvider {
    #[instrument(skip(self, _context))]
    fn provide(&self, tool: &str, _version: &str, _context: &ToolContext) -> Result<PathBuf, ToolError> {
        debug!("Looking for tool '{}' on host system...", tool);
        match which(tool) {
            Ok(path) => {
                info!("Found host tool at: {:?}", path);
                Ok(path)
            }
            Err(_) => Err(ToolError::NotFound(tool.to_string())),
        }
    }
}

#[derive(Debug)]
pub struct UrlProvider {
    pub url_template: String,
    pub sha256: Option<String>, 
}

impl ToolProvider for UrlProvider {
    #[instrument(skip(self, context))]
    fn provide(&self, tool: &str, version: &str, context: &ToolContext) -> Result<PathBuf, ToolError> {
        if context.cache.is_installed(tool, version) {
            return Ok(context.cache.get_tool_path(tool, version));
        }

        let url = self.resolve_url(version);

        if context.offline {
            // Only allow file:// URLs in offline mode
            if !url.starts_with("file://") {
                return Err(ToolError::StrategyFailure("UrlProvider".into(), "Offline mode: cannot download from network".into()));
            }
        }

        info!("Downloading tool from {}", url);
        
        context.cache.install(tool, version, |dest_path| {
            if url.starts_with("file://") {
                let src_path = url.trim_start_matches("file://");
                fs::copy(src_path, dest_path)?;
            } else {
                let mut response = reqwest::blocking::get(&url).map_err(io::Error::other)?;
                if !response.status().is_success() {
                    return Err(io::Error::other(format!("Download failed: {}", response.status())));
                }

                // Handle decompression if needed
                if url.ends_with(".zst") {
                    let mut decoder = zstd::stream::read::Decoder::new(response)?;
                    let mut dest_file = File::create(dest_path)?;
                    io::copy(&mut decoder, &mut dest_file)?;
                } else {
                    let mut dest_file = File::create(dest_path)?;
                    io::copy(&mut response, &mut dest_file)?;
                }
            }

            // Verify Checksum
            if let Some(expected_hash) = &self.sha256 {
                let mut file = File::open(dest_path)?;
                let mut hasher = Sha256::new();
                io::copy(&mut file, &mut hasher)?;
                let hash = hex::encode(hasher.finalize());
                
                if &hash != expected_hash {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, format!("Checksum mismatch: expected {}, got {}", expected_hash, hash)));
                }
            }
            
            Ok(())
        }).map_err(|e| {
            // Try to recover specific errors if possible, or wrap
            if e.to_string().contains("Checksum mismatch") {
               return ToolError::StrategyFailure("UrlProvider".into(), e.to_string());
            }
            ToolError::StrategyFailure("UrlProvider".into(), e.to_string())
        })
    }
}

impl UrlProvider {
    fn resolve_url(&self, version: &str) -> String {
        let platform = if cfg!(target_os = "macos") {
            if cfg!(target_arch = "aarch64") { "aarch64-apple-darwin" } else { "x86_64-apple-darwin" }
        } else if cfg!(target_os = "windows") {
            "x86_64-pc-windows-msvc"
        } else {
            "x86_64-unknown-linux-musl"
        };
        
        self.url_template
            .replace("{version}", version)
            .replace("{platform}", platform)
    }
}

/// Builds the tool from source using `cargo install` (git source).
/// This is robust for Rust-based tools like Buck2.
#[derive(Debug)]
pub struct CargoBuildProvider {
    pub git_url: String,
    pub bin_name: String, // The name of the binary produced (e.g., "buck2")
}

impl ToolProvider for CargoBuildProvider {
    #[instrument(skip(self, context))]
    fn provide(&self, tool: &str, version: &str, context: &ToolContext) -> Result<PathBuf, ToolError> {
        if context.cache.is_installed(tool, version) {
            return Ok(context.cache.get_tool_path(tool, version));
        }
        
        // Ensure cargo is available
        which("cargo").map_err(|_| ToolError::StrategyFailure("CargoBuildProvider".into(), "Cargo not found".into()))?;

        info!("Building {}@{} from source via Cargo...", tool, version);

        context.cache.install(tool, version, |dest_path| {
            let mut cmd = std::process::Command::new("cargo");
            cmd.arg("install");
            cmd.arg("--git").arg(&self.git_url);
            cmd.arg("--rev").arg(version); // Assuming version maps to a git tag/rev
            
            if context.offline {
                cmd.arg("--offline");
            }

            // Install to a temporary root first to extract the binary
            let temp_root = tempfile::tempdir()?;
            cmd.arg("--root").arg(temp_root.path());
            
            // Quiet output
            if !tracing::enabled!(tracing::Level::DEBUG) {
                 cmd.arg("--quiet");
            }
            
            let status = cmd.status()?;
            if !status.success() {
                return Err(io::Error::other("Cargo install failed"));
            }

            let built_bin = temp_root.path().join("bin").join(&self.bin_name).with_extension(std::env::consts::EXE_EXTENSION);
            
            if !built_bin.exists() {
                 return Err(io::Error::new(io::ErrorKind::NotFound, format!("Binary {:?} not found after build", built_bin)));
            }

            fs::copy(&built_bin, dest_path)?;
            Ok(())
        }).map_err(|e| ToolError::StrategyFailure("CargoBuildProvider".into(), e.to_string()))
    }
}

#[derive(Debug)]
pub struct ChainProvider {
    providers: Vec<Box<dyn ToolProvider>>,
}

impl ChainProvider {
    pub fn new(providers: Vec<Box<dyn ToolProvider>>) -> Self {
        Self { providers }
    }
}

impl ToolProvider for ChainProvider {
    fn provide(&self, tool: &str, version: &str, context: &ToolContext) -> Result<PathBuf, ToolError> {
        let mut last_error = ToolError::NotFound(tool.to_string());

        for provider in &self.providers {
            match provider.provide(tool, version, context) {
                Ok(path) => return Ok(path),
                Err(e) => {
                    debug!("Provider {:?} failed: {:?}", provider, e);
                    last_error = e;
                }
            }
        }
        
        Err(last_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_chain_provider_fallback() {
        let dir = tempdir().unwrap();
        let cache = ToolCache::with_dir(dir.path().to_path_buf());
        
        #[derive(Debug)]
        struct MockProvider(bool);
        impl ToolProvider for MockProvider {
            fn provide(&self, _t: &str, _v: &str, _c: &ToolContext) -> Result<PathBuf, ToolError> {
                if self.0 { Ok(PathBuf::from("found")) } else { Err(ToolError::NotFound("".into())) }
            }
        }

        let chain = ChainProvider::new(vec![
            Box::new(MockProvider(false)),
            Box::new(MockProvider(true)),
        ]);
        
        let ctx = ToolContext { offline: false, cache: &cache };
        assert!(chain.provide("t", "v", &ctx).is_ok());
    }

    #[test]
    fn test_url_provider_offline_check() {
        let dir = tempdir().unwrap();
        let cache = ToolCache::with_dir(dir.path().to_path_buf());
        let provider = UrlProvider {
            url_template: "http://example.com/{version}".into(),
            sha256: None,
        };
        let ctx = ToolContext { offline: true, cache: &cache };
        
        let res = provider.provide("foo", "1.0", &ctx);
        assert!(matches!(res, Err(ToolError::StrategyFailure(_, _))));
    }
}