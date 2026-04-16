# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/).

## [Unreleased]

### Added

### Changed

### Fixed

## [0.4.0] - 2026-04-16

### Added

- `mxpm publish` command to submit packages to the community index via GitHub PR
- Supports `--tag` to publish a specific git tag and `--ref` for an explicit commit hash
- Forks the index repo, updates `index.json`, and opens a PR automatically using `gh`
- Re-publishing force-pushes and detects existing open PRs instead of creating duplicates
- Index JSON output uses sorted keys at all levels for minimal diffs
- `skip_serializing_if` on optional `PackageEntry` and `Source` fields to avoid `null` values

## [0.3.0] - 2026-04-16

### Added

- `mxpm test [package]` command to run `rtest_*.mac` files through Maxima batch mode
- Maxima binary detection via `$MAXIMA_BIN`, config, or `$PATH` lookup
- Test file discovery from `[test]` section in `manifest.toml` or `rtest_*.mac` convention
- `[test]` section support in `manifest.toml` parser
- `<!-- include: path -->` directive support in doc source files for multi-file documentation
- Include expansion: stitches multiple `.md` files into one before pandoc/makeinfo pipeline
- mdBook nested chapters: included files become sub-chapters under their parent `##` section
- File watcher monitors all included files, not just the main doc source
- `mac_name` template variable (hyphens replaced with underscores) for valid Maxima identifiers

### Changed

- Renamed `mxpm init` to `mxpm new`
- Pandoc invoked with `--wrap=none` to prevent long heading truncation
- Template CI workflows trigger on both `main` and `master` branches
- Template `.gitignore` updated: covers `.ERR` and `.expanded.md`, allows `.info` at root

## [0.2.0] - 2026-04-15

### Added

- `mxpm doc build` command to generate `.info`, `-index.lisp`, XML, and mdBook from `.texi` or `.md` sources
- `mxpm doc index` command to generate Maxima help index (`*-index.lisp`) from `.info` or `.texi` files
- `mxpm doc watch` command to watch doc source files and rebuild on changes
- `mxpm doc serve` command for live preview with `mdbook serve` and automatic source regeneration
- Markdown-to-Texinfo conversion via Pandoc with post-processing for `@deffn`/`@defvr` blocks
- Markdown heading conventions: `### Function: name (args)` and `### Variable: name`
- mdBook source generation from Markdown with section splitting and styled definition headings
- Manifest-driven doc builds: `mxpm doc build` with no arguments reads `doc` field from `manifest.toml`
- `doc` field in `manifest.toml` for specifying the package's documentation source file
- Doc template (`doc/<name>.md`) scaffolded by `mxpm new`, with `load("<name>-index.lisp")` in entry file
- CI validation and GitHub Pages deployment workflows scaffolded by `mxpm new`
- Doc artifact staleness check (warns when outputs are older than source)
- Parent directory walk to find `manifest.toml` when explicit file path is given
- `mxpm new <name>` to scaffold new packages from Tera templates
- `mxpm install --path <dir>` to install from a local directory (copy mode)
- `mxpm install --path <dir> --editable` to symlink a local package for live development
- Package name validation (2-64 chars, lowercase + hyphens, no `maxima-` prefix)
- `Local` source type in `.mxpm.json` metadata for locally installed packages
- Tera templating engine for extensible package scaffolding

### Changed

- `mxpm install` package argument is now optional when `--path` is provided
- `mxpm remove` handles symlinked (editable) packages correctly
- OpenSSL vendoring is now behind an optional `vendored-openssl` feature flag
- Added `repository`, `homepage`, `keywords`, `categories` to Cargo.toml for crates.io

### Fixed

## [0.1.0] - 2026-04-15

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
