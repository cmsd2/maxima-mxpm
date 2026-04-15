# Maxima Package System: Phased Roadmap

A delivery plan organized around working increments. Each phase produces
something usable that builds on the previous phase. Later phases can be
re-prioritized or dropped based on what we learn.

Cross-reference: [technical requirements](technical-requirements.md),
[functional requirements](requirements.md),
[prior art](prior-art.md).

---

## Phase 0: Seed the index

**Goal:** A machine-readable catalog of known Maxima packages exists.
**Delivers:** The package index repository.
**Depends on:** Nothing.

### Work

1. Create the `maxima-package-index` repository
2. Define `schema.json` for the index format (tech req §3.2)
3. Populate `index.json` with every known third-party package:
   - diophantine (sdemarre)
   - padics (josanvallejo)
   - clifford (Prodanov)
   - numericalMethods (ramaniji)
   - raddenest
   - Packages from maxima-project-on-github/maxima-packages (~20)
   - qm-maxima (QMeqGR)
4. Write `CONTRIBUTING.md` with the PR submission process
5. Set up CI to validate `index.json` against the schema on PRs

### What's usable after this phase

The index is browsable on GitHub. Anyone can submit a package via PR.
Nothing is installable yet, but the catalog exists and the community
can start contributing to it immediately.

### Acceptance criteria

- `index.json` passes schema validation
- At least 10 packages indexed
- CI runs on PRs and validates schema + URL reachability

---

## Phase 1: CLI scaffold and install

**Goal:** Users can install a package by name from the command line.
**Delivers:** The `mxpm` binary with `install`, `list`, and `remove`.
**Depends on:** Phase 0 (index exists).

### Work

1. **Project setup**
   - Rust project with `clap` for CLI argument parsing
   - CI pipeline: build + test for all 5 target platforms (tech req §10.1)
   - GitHub releases with pre-built binaries

2. **Index fetching and caching** (tech req §5.4)
   - Fetch `index.json` over HTTPS (`reqwest` crate)
   - Cache in platform-appropriate location (XDG / Library / AppData)
   - `mxpm index update` to force refresh
   - 1-hour TTL, auto-refresh on stale cache

3. **Maxima user directory detection** (tech req §5.5)
   - `$MAXIMA_USERDIR` → `~/.maxima/` → `%USERPROFILE%/maxima/`
   - Create if missing (with prompt)

4. **`mxpm install <package>`** (tech req §4.3)
   - Look up package in index
   - Download tarball from GitHub/GitLab/URL
   - Extract to `~/.maxima/<pkgname>/`
   - Write `.mxpm.json` with install metadata
   - Handle the "no manifest.toml" fallback (convention-based install)

5. **`mxpm list`** (tech req §4.6)
   - Scan `~/.maxima/*/` for `.mxpm.json` files
   - Display name, version, install date

6. **`mxpm remove <package>`** (tech req §4.4)
   - Delete `~/.maxima/<pkgname>/`
   - Confirm before deleting

7. **Configuration** (tech req §12)
   - Read `config.toml` from platform-appropriate location
   - Support `$MAXIMA_USERDIR` and `$MAXIMA_BIN` overrides

### What's usable after this phase

```
$ mxpm install diophantine
Downloading diophantine from github:sdemarre/maxima-diophantine...
Installing to ~/.maxima/diophantine/
Done. Use: load("diophantine");

$ mxpm list
NAME          VERSION  INSTALLED
diophantine   -        2026-05-01

$ mxpm remove diophantine
Remove diophantine from ~/.maxima/diophantine/? [y/N] y
Removed.
```

The core value proposition works: search GitHub → `mxpm install foo` →
`load("foo")` in Maxima. No path configuration, no git clone, no
manual file placement.

Version column shows `-` for packages without `manifest.toml` — this
is expected for most existing packages during bootstrap.

### Acceptance criteria

- `mxpm install` works on Linux, macOS, and Windows
- Installed package is loadable via `load("pkgname")` in Maxima 5.47+
- `mxpm list` shows installed packages
- `mxpm remove` cleanly removes a package
- CI builds and tests pass on all platforms
- Binary size < 10 MB

---

## Phase 2: Search and discovery

**Goal:** Users can find packages from the command line.
**Delivers:** `mxpm search` and `mxpm info`.
**Depends on:** Phase 1.

### Work

1. **`mxpm search <query>`** (tech req §4.2)
   - Full-text search across name, description, keywords
   - Ranked results (name > keyword > description)
   - Tabular output

2. **`mxpm info <package>`** (tech req §4.2)
   - Show full metadata from index (and from `manifest.toml` if installed)
   - Description, authors, license, repository URL, keywords
   - Installation status

### What's usable after this phase

```
$ mxpm search algebra
NAME       DESCRIPTION
clifford   Clifford algebra for Maxima

$ mxpm info clifford
Name:        clifford
Description: Clifford algebra for Maxima
Authors:     Dimiter Prodanov
License:     GPL-3.0
Repository:  https://github.com/dprodanov/clifford
Keywords:    algebra, clifford, geometric-algebra
Status:      not installed
```

Discovery works. Users can find packages without browsing GitHub.

### Acceptance criteria

- `mxpm search` returns relevant results in < 1 second
- `mxpm info` shows all available metadata
- Partial name matches work (e.g. `mxpm search dioph`)

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

### What's usable after this phase

Package authors have a clear spec to follow. `mxpm check` validates their
work before submission. Packages with manifests get richer metadata in
`mxpm list` and `mxpm info`.

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

### What's usable after this phase

```
$ mxpm install my-package
Resolving dependencies...
  my-package 1.0.0
  └─ diophantine ^1.0 (will install 1.0.0)
Install 2 packages? [Y/n] y
Installing diophantine 1.0.0...
Installing my-package 1.0.0...
Done.
```

### Acceptance criteria

- Transitive dependencies are resolved and installed
- Circular dependencies are detected and rejected
- Version conflicts produce clear error messages
- `mxpm remove` warns about dependent packages

---

## Phase 5: Update mechanism

**Goal:** Users can update installed packages.
**Delivers:** `mxpm update`.
**Depends on:** Phase 3 (manifests with versions).

### Work

1. **`mxpm update [package]`** (tech req §4.5)
   - Compare installed version/commit against current index
   - Show available updates
   - `mxpm update` (no args) checks all packages
   - `mxpm update foo` updates a specific package

2. **Update execution**
   - Download new version
   - Replace installed files (atomic: extract to temp, swap directories)
   - Update `.mxpm.json`

### What's usable after this phase

```
$ mxpm update
Checking for updates...
diophantine: 1.0.0 → 1.1.0 (update available)
padics: up to date (0.3.1)

$ mxpm update diophantine
Downloading diophantine 1.1.0...
Updated diophantine: 1.0.0 → 1.1.0
```

### Acceptance criteria

- Version comparison works for semver versions
- Commit-hash comparison works for unversioned packages
- Update is atomic (no half-installed state on failure)
- `mxpm update` with no args lists all available updates

---

## Phase 6: Testing integration

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

### What's usable after this phase

```
$ mxpm test diophantine
Running rtest_diophantine.mac...
42 tests passed, 0 failed.

$ mxpm test --all
diophantine: 42 passed, 0 failed
padics: 18 passed, 2 failed
FAILED: padics
```

### Acceptance criteria

- `mxpm test` runs `rtest_*.mac` through Maxima batch mode
- Pass/fail counts are extracted and reported
- Exit code reflects test results (0 = all pass, 1 = failures)
- Works when Maxima is installed via distro packages

---

## Phase 7: Native code and build steps

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

### What's usable after this phase

Packages with C/Fortran code can be installed. The CLI handles the
orchestration; the package author provides the build system.

### Acceptance criteria

- `mxpm install` runs build commands when declared
- Missing build tools produce clear error messages
- Pre-built platforms skip the build step
- `[native]` warnings are informative

---

## Phase 8: Documentation integration

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
     (extract and ship `build_index.pl` as a standalone tool, or
     reimplement the index-building logic in Rust as part of a future
     `mxpm doc build` command)

### What's usable after this phase

Packages with pre-built documentation get `?`/`??` integration
automatically. Authors get validation that their doc setup is correct.

### Acceptance criteria

- `load("pkgname")` followed by `? func` returns package documentation
  (for packages with `-index.lisp`)
- `mxpm check` warns about documentation issues
- Author guide covers the full doc pipeline

---

## Phase 9: Static catalog website

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

### What's usable after this phase

Users can browse https://maxima-packages.org (or similar) to discover
packages. This addresses the discoverability requirement (func req §1.2)
for users who haven't yet installed the CLI.

### Acceptance criteria

- Site builds automatically from `index.json` on push
- Search works client-side (no server needed)
- Mobile-friendly
- Each package has a permanent URL

---

## Phase 10: Maxima-side integration

**Goal:** Package operations accessible from within a Maxima session.
**Delivers:** A `mxpm.mac` package.
**Depends on:** Phases 1–6 (CLI is mature).

### Work

1. **`mxpm.mac` package**
   - `mxpm_search("query")` — shells out to `mxpm search`, displays results
   - `mxpm_install("pkg")` — shells out to `mxpm install`
   - `mxpm_list()` — reads `.mxpm.json` files directly (no CLI needed)
   - `mxpm_info("pkg")` — reads manifest directly
   - `mxpm_test("pkg")` — runs tests via Maxima's own `batch(..., test)`

2. **Distribution**
   - Ship `mxpm.mac` as a package in the index (self-hosting)
   - Or bundle with the CLI installer

### What's usable after this phase

```
(%i1) load("mxpm");
(%i2) mxpm_search("diophantine");
  NAME          DESCRIPTION
  diophantine   Solver for Diophantine equations
(%i3) mxpm_install("diophantine");
  Installing diophantine...
  Done.
(%i4) load("diophantine");
```

Users who prefer not to leave the Maxima prompt can manage packages
from within their session.

### Acceptance criteria

- All operations work from the Maxima prompt
- Graceful fallback if CLI binary not found
- Works on SBCL, GCL, CCL (portable `system()` calls)

---

## Summary timeline

| Phase | Deliverable | Key dependency | Effort estimate |
|-------|------------|----------------|-----------------|
| 0 | Package index repo | None | Small |
| 1 | CLI: install, list, remove | Phase 0 | Medium |
| 2 | CLI: search, info | Phase 1 | Small |
| 3 | manifest.toml spec + check | Phase 1 | Small–Medium |
| 4 | Dependency resolution | Phase 3 | Medium |
| 5 | Update mechanism | Phase 3 | Small–Medium |
| 6 | Test integration | Phase 1 | Small |
| 7 | Native code / build steps | Phase 3 | Medium |
| 8 | Documentation integration | Phase 3 | Small–Medium |
| 9 | Static catalog website | Phase 0 | Medium |
| 10 | Maxima-side integration | Phases 1–6 | Medium |

### Critical path

```
Phase 0 → Phase 1 → Phase 3 → Phase 4
                  ↘ Phase 2
                  ↘ Phase 6
```

Phases 0–3 form the critical path. After Phase 3, work fans out and
phases 4–10 can be pursued in parallel or in any order based on
community feedback.

### Parallelizable work

After Phase 1 ships, the following can proceed concurrently:

- **Phase 2** (search) — independent of manifests
- **Phase 6** (testing) — only needs install + Maxima binary detection
- **Phase 9** (website) — only needs the index

After Phase 3 ships:

- **Phases 4, 5, 7, 8** can proceed in any order

Phase 10 (Maxima-side integration) should wait until the CLI is stable.

### Minimum viable product

**Phases 0 + 1 + 2** constitute the MVP. With these three phases, a user
can search for packages, install them by name, and load them in Maxima.
This is enough to break the chicken-and-egg cycle: it provides immediate
value (discovery and one-command install) while requiring minimal effort
from package authors (just submit a PR to the index — no manifest needed).
