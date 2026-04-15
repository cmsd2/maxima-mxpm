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

## Phase 3: Package manifest and metadata

**Goal:** Package authors can describe their packages with structured
metadata.
**Delivers:** The `manifest.toml` spec, `mxpm check`, and manifest-aware
install.
**Depends on:** Phase 1.

### Work

1. **Define `manifest.toml` format** (tech req §2)
   - Required fields: name, version, description, license, entry, authors
   - Optional fields: homepage, repository, keywords, maxima compat,
     docs, test files, native deps
   - Document the spec

2. **`mxpm check <path>`** (tech req §4.8)
   - Validate `manifest.toml` against the spec
   - Check that referenced files exist (entry point, tests, docs)
   - Useful error messages for authors

3. **Manifest-aware install**
   - When a package has `manifest.toml`, use it for version, entry point,
     and display metadata
   - `mxpm list` now shows actual version numbers for manifested packages

4. **Author documentation**
   - How to write a `manifest.toml`
   - How to add a package to the index
   - Example walkthroughs

### Notes

The `manifest.toml` schema is already defined in `src/manifest.rs` and
`install_package` already reads version from it when present. This phase
is about formalizing the spec, adding `mxpm check` validation, and
writing author-facing documentation.

### Acceptance criteria

- `mxpm check` correctly validates well-formed and malformed manifests
- `mxpm install` uses manifest metadata when present
- `mxpm list` shows version numbers from manifests
- Author guide published with working examples

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

## Phase 5: Testing integration

**Goal:** Package tests are runnable from the CLI.
**Delivers:** `mxpm test`.
**Depends on:** Phase 1.

### Work

1. **Maxima binary detection** (tech req §10.4)
   - `$MAXIMA_BIN` → `$PATH` → platform-specific defaults
   - Clear error if Maxima not found

2. **`mxpm test <package>`** (tech req §4.7)
   - Discover test files: `manifest.toml` `[test]` section, or fallback
     to `rtest_*.mac` convention
   - Run via `maxima --batch-string='...'`
   - Parse Maxima test output for pass/fail counts
   - Return appropriate exit code

3. **`mxpm test --all`**
   - Run tests for all installed packages
   - Summary report

### Acceptance criteria

- `mxpm test` runs `rtest_*.mac` through Maxima batch mode
- Pass/fail counts are extracted and reported
- Exit code reflects test results (0 = all pass, 1 = failures)
- Works when Maxima is installed via distro packages

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

## Phase 7: Documentation integration

**Goal:** Package documentation integrates with Maxima's help system.
**Delivers:** Documentation-aware install and `mxpm check` validation.
**Depends on:** Phase 3.

### Work

1. **Documentation-aware install**
   - Recognize `[docs]` section in `manifest.toml`
   - Ensure `.info`, `-index.lisp`, and `-index-html.lisp` files are
     placed where Maxima can find them (within the package directory)

2. **`mxpm check` documentation validation**
   - If `docs.index` is declared, check that the entry point `.mac` file
     contains a `load("...-index.lisp")` call
   - Warn if `.texi` source exists but pre-built `.info` is missing

3. **Author documentation**
   - Guide for producing Texinfo docs with `?`/`??` integration
   - Simplified instructions that don't require the Maxima source tree

### Acceptance criteria

- `load("pkgname")` followed by `? func` returns package documentation
  (for packages with `-index.lisp`)
- `mxpm check` warns about documentation issues
- Author guide covers the full doc pipeline

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

## Summary

| Phase | Deliverable | Status |
|-------|------------|--------|
| 0 | Package index repo | **Done** |
| 1 | CLI: install, list, remove | **Done** |
| 2 | CLI: search, info, outdated, upgrade | **Done** |
| 3 | manifest.toml spec + check | Not started |
| 4 | Dependency resolution | Not started |
| 5 | Test integration | Not started |
| 6 | Native code / build steps | Not started |
| 7 | Documentation integration | Not started |
| 8 | Static catalog website | Not started |
| 9 | Maxima-side integration | Not started |

### What to do next

Phases 0–2 are the MVP and are complete. Before pursuing further phases,
the priority should be:

1. **Community engagement.** Reach out to the maxima-packages
   maintainers and the Maxima mailing list. The index and the tool
   should reflect community needs, not assumptions.
2. **Real-world testing.** Install every indexed package on Linux,
   macOS, and Windows. Verify `load()` works in Maxima 5.47+.
3. **Phase 5 (testing)** is the most independently useful next step —
   it doesn't require manifests and provides immediate value.
4. **Phase 3 (manifests)** unlocks phases 4, 6, and 7 but should wait
   for community input on what fields actually matter.

### Critical path

```
Phase 0 → Phase 1 → Phase 3 → Phase 4
                   ↘ Phase 2 (done)
                   ↘ Phase 5
```

After Phase 3, work fans out and phases 4–9 can be pursued in any
order based on community feedback.
