# Maxima Package System: Phased Roadmap

A delivery plan organized around working increments. Each phase produces
something usable that builds on the previous phase. Later phases can be
re-prioritized or dropped based on what we learn.

Cross-reference: [technical requirements](technical-requirements.md),
[functional requirements](requirements.md),
[prior art](prior-art.md).

---

## Phase 0: Seed the index — DONE

**Goal:** A machine-readable catalog of known Maxima packages exists.
**Delivers:** The package index repository.
**Status:** Complete. Index at https://github.com/cmsd2/maxima-package-index

### What was delivered

- `maxima-package-index` repository with `index.json` (12 packages)
- `schema.json` (JSON Schema draft 2020-12) with two source types:
  `git` (any clonable repo + pinned commit hash) and `tarball` (HTTP
  URL + optional SHA-256 hash)
- `CONTRIBUTING.md` with PR submission process
- CI workflow validating schema conformance, commit hash refs, and URL
  reachability
- CC0-1.0 license

### Design decisions made during implementation

- **Two source types instead of four.** The original design had
  github, gitlab, tarball, and git source types. Simplified to just
  `git` and `tarball` — git clone (via libgit2) works with any
  hosting provider, and tarball covers release assets from anywhere.
- **Commit hash pinning.** All git refs must be full 40-character
  commit SHAs, not branch names or tags. This ensures reproducible
  installs at the cost of slightly more friction when adding packages.
- **Tarball integrity.** Optional `hash` and `hash_algorithm` fields
  on tarball sources for SHA-256 verification.

---

## Phase 1: CLI scaffold and install — DONE

**Goal:** Users can install a package by name from the command line.
**Delivers:** The `mxpm` binary with `install`, `list`, and `remove`.
**Status:** Complete.

### What was delivered

- Rust project with lib+bin split (`src/lib.rs` + `src/bin/mxpm/`)
- Async HTTP via `reqwest` + `tokio`, git clone via `git2` (libgit2)
- `mxpm install <package>` with `--reinstall` flag
- `mxpm list` with tabular output
- `mxpm remove <package>` with confirmation prompt
- `mxpm index update` to force-refresh the cache
- `--json` global flag for machine-readable output
- `--yes` / `-y` global flag for non-interactive use
- Interactive progress bars (indicatif) for downloads and clones
- Atomic installs via staging directory + rename
- `.mxpm.json` metadata with resolved commit hash and tarball hash
- Config file support (`config.toml`) with environment variable
  overrides (`MAXIMA_USERDIR`, `MXPM_REGISTRY_URL`)
- Multi-registry support with config-ordered resolution
- Index caching with configurable TTL (default 1 hour)
- CI workflow (fmt, clippy, test on Linux/macOS/Windows)
- Release workflow (cross-compile 5 targets, GitHub Releases)
- Release binary ~3.6 MB

### Design decisions made during implementation

- **lib+bin split.** The library can be used by other tools (e.g. a
  future Maxima-side integration). The CLI is a thin wrapper.
- **Async.** Using tokio + async reqwest rather than blocking HTTP.
  Git clone via libgit2 is synchronous but wrapped in async commands.
- **Install metadata.** `.mxpm.json` stores the resolved source (with
  actual commit hash, not the index ref) so the `outdated` command
  can compare what's installed against the registry.

---

## Phase 2: Search and discovery — DONE

**Goal:** Users can find packages from the command line.
**Delivers:** `mxpm search`, `mxpm info`, `mxpm outdated`, `mxpm upgrade`.
**Status:** Complete.

### What was delivered

- `mxpm search <query>` with ranked full-text matching across name,
  keywords, and description
- `mxpm info <package>` showing all metadata + install status
- `mxpm outdated` comparing installed sources against the registry
- `mxpm upgrade [package]` to reinstall outdated packages

### Design decisions made during implementation

- **Outdated comparison.** Compares the identifying fields of the
  source (git ref or tarball URL) rather than naive equality, to avoid
  false positives from computed metadata like tarball hashes.
- **Upgrade = remove + install.** Simple and correct. No incremental
  update mechanism needed at this scale.

---

## Phase 3: Package manifest and metadata — MOSTLY DONE

**Goal:** Package authors can describe their packages with structured
metadata.
**Delivers:** The `manifest.toml` spec, `mxpm check`, and manifest-aware
install.
**Depends on:** Phase 1.
**Status:** Mostly complete. Manifest spec defined, parsed, and documented.
`mxpm new` scaffolds it. Missing: `mxpm check` validation command.

### What was delivered

- `manifest.toml` format defined and parsed (`src/manifest.rs`)
  - Required fields: name, version, description, license, entry
  - Optional fields: homepage, repository, keywords, maxima compat,
    doc, test files, authors
- Manifest-aware install: `install_package` reads version from manifest
- `mxpm list` shows version numbers for manifested packages
- `mxpm new <name>` scaffolds a complete package with manifest, entry
  point, test file, doc source, CI workflows, README, and `.gitignore`
- Package name validation (2–64 chars, lowercase + hyphens, no `maxima-`
  prefix)
- `mxpm install --path <dir>` for local installs (copy mode)
- `mxpm install --path <dir> --editable` for symlinked development
- Author documentation: full `manifest.toml` reference in README with
  field table
- Example package: [maxima-example-package](https://github.com/cmsd2/maxima-example-package)

### Remaining work

1. **`mxpm check <path>`** (tech req §4.8)
   - Validate `manifest.toml` against the spec
   - Check that referenced files exist (entry point, tests, docs)
   - Useful error messages for authors

---

## Phase 4: Dependency resolution

**Goal:** Packages can declare and resolve dependencies on other packages.
**Delivers:** `[dependencies]` in `manifest.toml` and automatic
transitive install.
**Depends on:** Phase 3 (manifests exist).

### Work

1. **Dependency declaration** (tech req §8)
   - `[dependencies]` section in `manifest.toml`
   - Version constraint syntax: `"^1.0"`, `">= 1.2"`, etc.

2. **Resolver** (tech req §8.2)
   - Depth-first resolution with cycle detection
   - Version constraint checking
   - Clear error messages for conflicts

3. **Install with deps**
   - `mxpm install foo` automatically installs foo's dependencies
   - Show what will be installed before proceeding
   - `mxpm remove` warns about reverse dependencies

### Notes

With < 100 packages and likely < 5 dependency edges in the entire graph,
a simple resolver is fine. Defer this until there's actual demand — no
existing Maxima package declares dependencies on other third-party
packages.

### Acceptance criteria

- Transitive dependencies are resolved and installed
- Circular dependencies are detected and rejected
- Version conflicts produce clear error messages
- `mxpm remove` warns about dependent packages

---

## Phase 5: Testing integration — DONE

**Goal:** Package tests are runnable from the CLI.
**Delivers:** `mxpm test`.
**Depends on:** Phase 1.
**Status:** Complete.

### What was delivered

- **`mxpm test [package]`** — runs package tests through Maxima batch mode
  - Discovers test files from `[test]` section in `manifest.toml`, or
    falls back to `rtest_*.mac` convention
  - Invokes `maxima --batch-string='load("pkg"); batch("rtest.mac", test);'`
  - Parses both modern (`M/N tests passed`) and legacy
    (`N problems attempted; M correct.`) Maxima output formats
  - Human-readable and `--json` output
  - Exit code 1 on any test failure
- **`mxpm test`** (no argument) — tests all installed packages
- **Maxima binary detection** via `paths::maxima_bin(config)`:
  `config.maxima_bin` / `$MAXIMA_BIN` → `"maxima"` (OS PATH lookup)
- `[test]` section parsed from `manifest.toml` (`test.files` field)

### Design decisions made during implementation

- **Two output parsers.** Maxima 5.47+ uses `M/N tests passed` format;
  older versions use `N problems attempted; M correct.`. Both are handled.
- **No `--all` flag.** Omitting the package name already tests all
  installed packages, keeping the CLI simpler.
- **Sync execution.** Test runs are local (no network), so no async needed.

---

## Phase 6: Native code and build steps

**Goal:** Packages that need compilation can declare and execute build
steps.
**Delivers:** `[build]` manifest section support in `mxpm install`.
**Depends on:** Phase 3.

### Work

1. **`[build]` manifest support** (tech req §9.1)
   - Parse `system`, `command`, `requires` fields
   - Check `requires` tools are on `$PATH`
   - Execute build command after extraction

2. **`[build.prebuilt]` support**
   - Check user's platform against `platforms` list
   - Skip build step when pre-built artifacts are available

3. **External program / shared library checks**
   - Parse `[native]` section
   - Check availability at install time
   - Warn but don't block installation

### Acceptance criteria

- `mxpm install` runs build commands when declared
- Missing build tools produce clear error messages
- Pre-built platforms skip the build step
- `[native]` warnings are informative

---

## Phase 7: Documentation integration — DONE

**Goal:** Package documentation integrates with Maxima's help system.
**Delivers:** `mxpm doc` toolchain for building, previewing, and serving docs.
**Depends on:** Phase 3.
**Status:** Complete. Full doc toolchain delivered.

### What was delivered

- **`mxpm doc build [file]`** — builds all doc artifacts from `.texi` or
  `.md` source:
  - `.info` file (GNU Info format, via `makeinfo`)
  - `*-index.lisp` (keyword → byte-offset lookup for `?`/`??` help)
  - `--xml` flag for Texinfo XML output
  - `--mdbook` flag for mdBook HTML generation
  - `-o <dir>` for custom output directory
  - Manifest-driven: reads `doc` field from `manifest.toml` when no file
    argument given; walks parent directories to find manifest when
    explicit file path is provided
  - Staleness detection: warns when outputs are older than source
- **`mxpm doc watch [file]`** — watches source file and rebuilds on changes
- **`mxpm doc serve [file]`** — live preview with `mdbook serve` and
  automatic source regeneration on changes
  - `-p <port>`, `-n <hostname>`, `--open` flags
- **`mxpm doc index <file>`** — low-level index generator from `.info`
  or `.texi` files
  - `-o <path>` for output file (`-` for stdout)
  - `--install-path <dir>` to hardcode info file location
- Markdown-to-Texinfo conversion via Pandoc with post-processing for
  `@deffn`/`@defvr` blocks from heading conventions (`### Function:`,
  `### Variable:`)
- mdBook source generation with section splitting and styled definition
  headings
- Reimplemented `build_index.pl` in Rust (`src/info_index.rs`) — no Perl
  dependency needed
- CI workflows scaffolded by `mxpm new`: docs build and commit artifacts,
  GitHub Pages deployment
- `load("pkgname")` followed by `? func` returns package documentation
  for packages with `-index.lisp`

### Design decisions made during implementation

- **Rust reimplementation of `build_index.pl`.** The original Perl script
  is a build-time tool not distributed with Maxima. Reimplementing it in
  Rust means package authors don't need Perl or the Maxima source tree.
- **CI as canonical doc builder.** Different `makeinfo` versions produce
  slightly different byte offsets. Rather than checking for staleness in
  CI (which fails across environments), the docs workflow builds artifacts
  and commits them back to the repo.
- **Parent directory walk for manifest.** When an explicit file path is
  given (e.g. `mxpm doc build doc/pkg.md`), the builder walks parent
  directories to find `manifest.toml` and uses that directory as the
  output root. This matches Cargo's behavior of finding `Cargo.toml`.

---

## Phase 8: Static catalog website

**Goal:** A browsable web catalog of all indexed packages.
**Delivers:** A static site generated from `index.json`.
**Depends on:** Phase 0 (index), Phase 3 (manifests provide richer data).

### Work

1. **Static site generator**
   - Read `index.json` and generate HTML pages
   - Package listing with search
   - Per-package detail pages
   - Build via GitHub Actions, host on GitHub Pages

2. **Content**
   - Package name, description, authors, license, keywords
   - Link to repository
   - Installation command (`mxpm install foo`)
   - README rendering (fetched from repo)

### Acceptance criteria

- Site builds automatically from `index.json` on push
- Search works client-side (no server needed)
- Mobile-friendly
- Each package has a permanent URL

---

## Phase 9: Maxima-side integration

**Goal:** Package operations accessible from within a Maxima session.
**Delivers:** A `mxpm.mac` package.
**Depends on:** Phases 1–5 (CLI is mature).

### Work

1. **`mxpm.mac` package**
   - `mxpm_search("query")` — shells out to `mxpm search --json`,
     displays results
   - `mxpm_install("pkg")` — shells out to `mxpm install --yes`
   - `mxpm_list()` — reads `.mxpm.json` files directly (no CLI needed)
   - `mxpm_info("pkg")` — reads manifest directly
   - `mxpm_test("pkg")` — runs tests via Maxima's own `batch(..., test)`

2. **Distribution**
   - Ship `mxpm.mac` as a package in the index (self-hosting)
   - Or bundle with the CLI installer

### Acceptance criteria

- All operations work from the Maxima prompt
- Graceful fallback if CLI binary not found
- Works on SBCL, GCL, CCL (portable `system()` calls)

---

## Phase 10: Core Maxima docs as a doc-index package

**Goal:** Move Maxima's built-in function documentation out of the compiled
aximar-core binary and into a standalone mxpm package, unifying the data path
for core and third-party docs.
**Delivers:** `maxima-core-docs` package with all ~2500 built-in function docs
and ~244 figures, consumable through the same `~/.maxima/` doc-index
infrastructure used by third-party packages.
**Depends on:** Phase 7 (doc-index format exists), LSP doc-index consumption
(already implemented).

See [ide-integration.md § Proposal: Core Maxima docs as a doc-index package](ide-integration.md#proposal-core-maxima-docs-as-a-doc-index-package)
for full details.

### Why

- Fixes broken images in the VS Code docs webview (97 entries reference PNGs
  that aren't accessible at runtime)
- Eliminates dual overlapping data sources in aximar-core (`catalog.json` +
  `docs.json`)
- Makes docs independently updatable via `mxpm upgrade`
- Unifies the code path — all docs (core + packages) flow through `DocIndexStore`

### Work

1. Generate `maxima-core-docs-doc-index.json` from existing `docs.json` or
   Maxima `.texi` source
2. Package the 244 figures alongside the doc-index
3. Add image resolution support to the LSP/webview (resolve relative paths
   from the package directory via `webview.asWebviewUri()`)
4. Remove `docs.json` from aximar-core (keep a minimal catalog for fallback)
5. Auto-install or prompt for `maxima-core-docs` from the extension

---

## Summary

| Phase | Deliverable | Status |
|-------|------------|--------|
| 0 | Package index repo | **Done** |
| 1 | CLI: install, list, remove | **Done** |
| 2 | CLI: search, info, outdated, upgrade | **Done** |
| 3 | manifest.toml spec + `mxpm new` | **Mostly done** (missing `mxpm check`) |
| 4 | Dependency resolution | Not started |
| 5 | Test integration | **Done** |
| 6 | Native code / build steps | Not started |
| 7 | Documentation integration | **Done** |
| 8 | Static catalog website | Not started |
| 9 | Maxima-side integration | Not started |
| 10 | Core Maxima docs as doc-index package | Not started |

### What to do next

Phases 0–2 (MVP), 3 (manifest), and 7 (documentation) are complete or
nearly complete. Remaining priorities:

1. **`mxpm check`** — finish Phase 3 by adding manifest validation.
2. **Phase 5 (testing)** — the most independently useful next step.
   Doesn't require further manifest work and provides immediate value.
3. **Community engagement.** Reach out to the Maxima mailing list. The
   index and the tool should reflect community needs, not assumptions.
4. **Real-world testing.** Install every indexed package on Linux,
   macOS, and Windows. Verify `load()` works in Maxima 5.47+.

### Critical path

```
Phase 0 → Phase 1 → Phase 3 (mostly done) → Phase 4
                   ↘ Phase 2 (done)
                   ↘ Phase 5
                   ↘ Phase 7 (done)
```

After Phase 3, work fans out and phases 4–9 can be pursued in any
order based on community feedback.
