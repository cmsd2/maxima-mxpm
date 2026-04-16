# mxpm — Maxima Package Manager

## Project overview

A Rust CLI tool (`mxpm`) for installing, managing, and discovering packages for the Maxima computer algebra system. It leverages Maxima 5.47+'s auto-scanning of `~/.maxima/` subdirectories so `load("pkgname")` just works after install.

Licensed under MIT OR Apache-2.0. The companion package index repo (`maxima-package-index`) is CC0-1.0.

## Architecture

- **lib+bin split**: `src/lib.rs` exposes the library; `src/bin/mxpm/` is a thin CLI wrapper.
- **Async**: Uses `tokio` runtime + async `reqwest` for HTTP. Git clone via `git2` (libgit2) is sync but wrapped in async commands.
- **Two source types**: `git` (clone + checkout ref) and `tarball` (HTTP download). No GitHub/GitLab-specific types.
- **Package index**: Static JSON hosted in a separate Git repo, fetched over HTTPS and cached locally with a TTL.
- **Multi-registry**: Config supports multiple registries; packages resolve in config order (first match wins).
- **Atomic installs**: Download to `.mxpm_staging/`, then rename to final location. `.mxpm.json` metadata written per package.
- **Version pinning**: Index refs must be full 40-character commit hashes for reproducibility.

## Project structure

```
src/
  lib.rs              # Public module declarations
  bin/mxpm/
    main.rs           # Entry point (#[tokio::main])
    cli.rs            # clap derive CLI definition + dispatch
  commands/           # One file per subcommand
    install.rs, list.rs, remove.rs, search.rs, info.rs,
    outdated.rs, upgrade.rs, index.rs, new.rs, test.rs, doc.rs
  config.rs           # config.toml loading, env var overrides
  index.rs            # PackageIndex, PackageEntry, Source enum
  registry.rs         # Fetch/cache/resolve across registries
  source.rs           # Download: tarball extraction + git clone
  install.rs          # Install/remove/list/search logic
  manifest.rs         # manifest.toml parsing
  paths.rs            # Platform path detection (dirs crate)
  output.rs           # OutputFormat enum (Human/Json), print_json helper
  info_index.rs       # .info parser + Lisp index generator
  errors.rs           # MxpmError (thiserror)
  types.rs            # InstallMetadata (.mxpm.json schema)
```

## Build and test

```bash
cargo build           # Dev build
cargo build --release # Release build (LTO + strip, targets <10MB)
cargo test            # Run tests
cargo clippy          # Lint
cargo fmt --check     # Check formatting
```

## Key conventions

- All commands accept `--json` for machine-readable output and `--yes`/`-y` to skip confirmation prompts.
- Human output uses `eprintln!` for status messages and `println!` for primary output (tables, load instructions).
- JSON output uses `serde::Serialize` structs per command, printed via `output::print_json`.
- Errors use `thiserror` in the library (`MxpmError`) and `anyhow` at the binary boundary.
- The default registry URL points to `https://raw.githubusercontent.com/cmsd2/maxima-package-index/master/index.json`.
- Environment variable overrides: `MAXIMA_USERDIR`, `MAXIMA_BIN`, `MXPM_REGISTRY_URL`.

## Documentation tools

`mxpm doc` builds package documentation from `.texi` or `.md` source files.

- **`mxpm doc build [file]`** — builds all doc artifacts from a `.texi` or `.md` source:
  - Always: `.info` + `*-index.lisp` (for Maxima's `?`/`??` help system)
  - `--xml`: also generates Texinfo XML via `makeinfo --xml`
  - `--mdbook`: also generates mdBook source (`.md` input only; `.texi` not yet supported)
  - `-o <dir>`: output directory (default: alongside source file)
  - If `file` is omitted, reads the `doc` field from `manifest.toml` and places outputs in the package root
- **`mxpm doc watch [file]`** — watches source file and rebuilds on changes (same flags as `build`)
- **`mxpm doc serve [file]`** — live preview: builds mdBook, spawns `mdbook serve` with livereload, and watches the source `.md` for changes
  - `-p <port>`: HTTP port (default: 3000)
  - `-n <hostname>`: bind address (default: localhost)
  - `--open`: open browser after starting
- **`mxpm doc index <file>`** — low-level: generates just the `*-index.lisp` from a `.info` or `.texi` file
  - `-o <path>`: output file (`-` for stdout)
  - `--install-path <dir>`: hardcode info file location (default: dynamic `maxima-load-pathname-directory`)

### Markdown conventions

When using `.md` input, Pandoc converts to Texinfo. A post-processor recognizes heading conventions for Maxima help index entries:

- `### Function: name (arg1, arg2)` → `@deffn {Function} name (@var{arg1}, @var{arg2})`
- `### Variable: name` → `@defvr {Variable} name`

### External tool requirements

- `makeinfo` (GNU Texinfo) — required for all doc builds
- `pandoc` — required for `.md` input
- `mdbook` — optional; if installed, `--mdbook` builds HTML automatically

## Companion repository

The package index lives at https://github.com/cmsd2/maxima-package-index — `index.json` with a JSON Schema (`schema.json`). All git source refs must be full commit hashes.
