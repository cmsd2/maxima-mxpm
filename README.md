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

Download a binary from the [releases page](https://github.com/cmsd2/maxima-packages/releases)
and place it on your PATH.

Or build from source:

```bash
cargo install --locked --path .
```

## Usage

```
mxpm search <query>           # Search for packages
mxpm info <package>           # Show package details
mxpm install <package>        # Install a package
mxpm install --reinstall <p>  # Reinstall a package
mxpm list                     # List installed packages
mxpm outdated                 # Show packages with updates available
mxpm upgrade                  # Upgrade all outdated packages
mxpm upgrade <package>        # Upgrade a specific package
mxpm remove <package>         # Remove a package
mxpm index update             # Force-refresh the package index
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

## Requirements

- Maxima 5.47+ (for automatic `~/.maxima/` subdirectory scanning)
- No other dependencies — mxpm is a single static binary

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
