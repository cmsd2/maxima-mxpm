# mxpm — Maxima Package Manager

A command-line tool for installing, managing, and discovering packages for
the [Maxima](https://maxima.sourceforge.io/) computer algebra system.

```
$ mxpm install diophantine
Found diophantine in registry 'community'
Cloning https://github.com/sdemarre/maxima-diophantine.git (4adf29185a49)...
Installing to /home/user/.maxima/diophantine/
Commit:  4adf29185a49
Done.
Use: load("diophantine");
```

Packages are installed to `~/.maxima/<name>/`, which Maxima 5.47+
auto-scans on startup. No path configuration needed — just
`load("pkgname")`.

## Install

Download a binary from the [releases page](https://github.com/cmsd2/maxima-mxpm/releases)
and place it on your PATH.

Or build from source:

```bash
cargo install --locked --path .
```

## Usage

```
mxpm new <name>               # Scaffold a new package
mxpm search <query>           # Search for packages
mxpm info <package>           # Show package details
mxpm install <package>        # Install a package from the registry
mxpm install --path <dir>     # Install from a local directory
mxpm install --path . -e      # Install as editable (symlink)
mxpm install --reinstall <p>  # Reinstall a package
mxpm list                     # List installed packages
mxpm outdated                 # Show packages with updates available
mxpm upgrade                  # Upgrade all outdated packages
mxpm upgrade <package>        # Upgrade a specific package
mxpm remove <package>         # Remove a package
mxpm test <package>           # Run package tests via Maxima
mxpm test                     # Test all installed packages
mxpm publish                  # Publish current HEAD to the index
mxpm publish --tag v1.0.0     # Publish a specific tag
mxpm index update             # Force-refresh the package index

# Documentation
mxpm doc build                # Build .info and help index from manifest
mxpm doc build file.md        # Build from a specific source file
mxpm doc build --mdbook       # Also generate mdBook HTML
mxpm doc watch                # Watch source and rebuild on changes
mxpm doc serve                # Live preview with mdbook serve
mxpm doc index file.info      # Generate help index from .info file
```

### Global flags

| Flag | Description |
|------|-------------|
| `--json` | Machine-readable JSON output |
| `--yes`, `-y` | Skip confirmation prompts |

## Configuration

Optional config file at `~/.config/mxpm/config.toml` (Linux/macOS) or
`%APPDATA%\mxpm\config.toml` (Windows):

```toml
maxima_userdir = "/custom/path/.maxima"
cache_ttl = 600  # seconds (default: 3600)

[[registries]]
name = "private"
url = "https://example.com/index.json"
```

The default community registry is always appended unless you explicitly
include an entry named `community`.

### Environment variables

| Variable | Description |
|----------|-------------|
| `MAXIMA_USERDIR` | Override the Maxima user directory |
| `MXPM_REGISTRY_URL` | Override the default registry URL |

## Creating packages

Scaffold a new package:

```bash
mxpm new my-package
cd my-package
git init
```

This creates a `manifest.toml`, entry point `.mac` file, test file, doc source,
CI workflows, README, and `.gitignore`. Install it locally for development:

```bash
mxpm install --path . --editable   # symlink — edits are live
mxpm install --path .              # copy — snapshot install
```

Or start from the [maxima-package-template](https://github.com/cmsd2/maxima-package-template)
on GitHub.

### manifest.toml

Every package has a `manifest.toml` at its root:

```toml
[package]
name = "my-package"          # Required. 2-64 chars, lowercase + hyphens
version = "0.1.0"            # Required. Semver
description = "..."          # Required
license = "MIT"              # Required. SPDX identifier
entry = "my-package.mac"     # Required. Main Maxima file
doc = "doc/my-package.md"    # Doc source (.md or .texi)

# Optional fields
homepage = "https://..."
repository = "https://..."
keywords = ["math", "algebra"]
maxima = ">= 5.47"

[package.authors]
names = ["Author Name"]

[test]
files = ["rtest_my-package.mac"]
```

| Field | Required | Description |
|-------|----------|-------------|
| `name` | yes | Package name (lowercase letters, digits, hyphens; cannot start with `maxima-`) |
| `version` | yes | Package version |
| `description` | yes | Short description |
| `license` | yes | SPDX license identifier |
| `entry` | yes | Main `.mac` file loaded by `load("name")` |
| `doc` | no | Path to documentation source (`.md` or `.texi`), relative to package root |
| `homepage` | no | Project homepage URL |
| `repository` | no | Source code repository URL |
| `keywords` | no | List of keywords for search |
| `maxima` | no | Maxima version requirement |
| `authors.names` | no | List of author names |
| `test.files` | no | List of test files for `batch()` |

## Documentation

`mxpm doc build` generates documentation artifacts from `.texi` or `.md` source files.
If a `doc` field is set in `manifest.toml`, no arguments are needed.

From Markdown input, headings like `### Function: name (args)` are converted to
Texinfo `@deffn` definitions, making them available via Maxima's `?` and `??` help.

Use `mxpm doc serve` for a live preview with hot reload while writing docs.

### External tools

| Tool | Required for |
|------|-------------|
| `makeinfo` (GNU Texinfo) | All doc builds |
| `pandoc` | Markdown input |
| `mdbook` | `--mdbook` flag / `doc serve` |

## Requirements

- Maxima 5.47+ (for automatic `~/.maxima/` subdirectory scanning)
- `makeinfo` and `pandoc` for documentation builds (optional)

## Package index

The community package index lives at
[cmsd2/maxima-package-index](https://github.com/cmsd2/maxima-package-index).
To add a package, submit a pull request adding an entry to `index.json`.
See [CONTRIBUTING.md](https://github.com/cmsd2/maxima-package-index/blob/master/CONTRIBUTING.md)
for details.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.
