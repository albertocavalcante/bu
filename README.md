# bu

A smart build tool wrapper that automatically detects your project type and runs the appropriate build tool with zero configuration.

## Features

- **Automatic Project Detection**: Identifies project type via marker files (`.buckconfig`, `WORKSPACE`, `Cargo.toml`, etc.)
- **Starlark Configuration**: Optional `bu.star` file for advanced tool customization
- **Multi-Strategy Tool Resolution**: Resolves tools via configurable provider chain:
  - Host system PATH lookup
  - URL download with checksum verification
  - Source builds via `cargo install`
- **Version Management**: Reads version files (`.buckversion`, `.bazelversion`, `.nvmrc`, etc.)
- **Smart Caching**: Downloads and builds are cached in `~/.bu/cache/`
- **Offline Mode**: Works offline using cached tools or host binaries
- **Zero-Config by Default**: Works out of the box for standard projects

## Installation

Install via Cargo:

```bash
cargo install --path .
```

Binary releases will be available in the future.

## Quick Start

Simply run `bu` in any supported project directory:

```bash
# In a Buck2 project
cd my-buck2-project
bu build //...

# In a Bazel project
cd my-bazel-project
bu build //...

# In a Rust project
cd my-rust-project
bu build --release
```

The tool automatically detects your project type and forwards all arguments to the appropriate build tool.

## Supported Project Types

| Project Type | Marker Files | Tool |
|-------------|--------------|------|
| **Buck2** | `.buckconfig`, `BUCK` | `buck2` |
| **Bazel** | `WORKSPACE`, `WORKSPACE.bazel`, `MODULE.bazel` | `bazel` |
| **Rust** | `Cargo.toml` | `cargo` |
| **Maven** | `pom.xml` | `mvn` |
| **Gradle** | `build.gradle`, `build.gradle.kts` | `gradle` |
| **NPM** | `package.json` | `npm` |

## Configuration with bu.star

Create a `bu.star` file in your project root for advanced configuration:

### Basic Tool Registration

```starlark
bu.register_tool(
    name = "buck2",
    version = "2024-01-01",
    url_template = "https://github.com/facebook/buck2/releases/download/{version}/buck2-{platform}.zst",
    sha256 = "abc123def456...",
    strategies = ["url", "host"]
)
```

### Configuration Options

- **name**: Tool identifier (string)
- **version**: Tool version (string)
- **url_template**: URL template supporting `{version}` and `{platform}` placeholders (optional)
- **sha256**: SHA-256 checksum for download verification (optional)
- **git_url**: Git repository URL for source builds (optional)
- **strategies**: Resolution strategy order (list of strings)

### Resolution Strategies

1. **"host"**: Look for the tool in system PATH
2. **"url"**: Download from URL (with automatic `.zst` decompression)
3. **"source"**: Build from source using `cargo install --git`

### Platform Placeholders

The `{platform}` placeholder in `url_template` resolves to:
- `aarch64-apple-darwin` (macOS ARM64)
- `x86_64-apple-darwin` (macOS Intel)
- `x86_64-unknown-linux-musl` (Linux)
- `x86_64-pc-windows-msvc` (Windows)

## Version Files

`bu` automatically reads version files to determine which tool version to use:

| Tool | Version File(s) |
|------|-----------------|
| Buck2 | `.buckversion` |
| Bazel | `.bazelversion` |
| NPM/Node | `.nvmrc`, `.node-version` |
| Gradle | `gradle/wrapper/gradle-wrapper.properties` |
| Maven | `.mvn/wrapper/maven-wrapper.properties` |

For tools without version files, `bu` defaults to `"latest"`.

## Cache Location

Tools are cached in `~/.bu/cache/` with the following structure:

```
~/.bu/cache/
├── buck2/
│   ├── 2024-01-01/
│   │   └── buck2
│   └── latest/
│       └── buck2
├── bazel/
│   └── 6.4.0/
│       └── bazel
└── ...
```

## Offline Mode

Use the `--offline` flag to prevent network access:

```bash
bu --offline build //...
```

In offline mode:
- Only `file://` URLs are allowed (no HTTP/HTTPS downloads)
- Source builds use `cargo install --offline`
- Cached tools are used if available
- Host tools are used as fallback

## Command-Line Arguments

All arguments after `bu` are passed directly to the detected build tool:

```bash
bu build //target           # Forwards to: buck2 build //target
bu test --all               # Forwards to: cargo test --all
bu --offline run --release  # Runs cargo with --offline mode on bu
```

The `--offline` flag is the only `bu`-specific flag and must come before tool arguments.

## How It Works

1. **Detection**: Scans current directory for marker files to identify project type
2. **Configuration**: Loads `bu.star` if present, otherwise uses defaults
3. **Version Resolution**: Reads version files (e.g., `.buckversion`)
4. **Tool Resolution**: Runs through provider chain to find/download tool
5. **Execution**: Runs the resolved tool with all pass-through arguments

## License

MIT
