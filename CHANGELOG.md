# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/).

## [Unreleased]

### Added

### Changed

### Fixed

## [0.1.0]

### Added

- `mxpm install <package>` with `--reinstall` flag
- `mxpm list` to show installed packages
- `mxpm remove <package>` with confirmation prompt
- `mxpm search <query>` with ranked full-text matching
- `mxpm info <package>` with install status
- `mxpm outdated` to show packages with updates available
- `mxpm upgrade [package]` to reinstall outdated packages
- `mxpm index update` to force-refresh the cached index
- `--json` global flag for machine-readable output
- `--yes` global flag to skip confirmation prompts
- Git clone (libgit2) and tarball source types
- SHA-256 integrity verification for tarball downloads
- Commit hash pinning for reproducible installs
- Interactive progress bars (indicatif) for downloads
- Multi-registry support with config-ordered resolution
- Index caching with configurable TTL
- Atomic installs via staging directory
- Platform path detection (Linux, macOS, Windows)
- Config file support (`~/.config/mxpm/config.toml`)
- Environment variable overrides (`MAXIMA_USERDIR`, `MXPM_REGISTRY_URL`)
