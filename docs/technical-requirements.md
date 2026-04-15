# Maxima Package System: Technical Requirements

Concrete technical decisions for implementing the Maxima package system,
based on the [functional requirements](requirements.md) and
[prior art analysis](prior-art.md).

---

## 1. System architecture

The system has three components:

```
┌─────────────┐     fetches      ┌─────────────────┐
│  CLI tool    │ ◄──────────────► │  Package index   │
│  (Rust bin)  │                  │  (Git repo, JSON) │
└──────┬──────┘                  └─────────────────┘
       │ downloads & installs
       ▼
┌──────────────────┐   load()    ┌─────────────────┐
│  ~/.maxima/       │ ◄───────── │  Maxima 5.47+    │
│  pkgname/         │            │                   │
│    pkgname.mac    │            │  load("pkgname") │
│    manifest.toml  │            │  just works       │
└──────────────────┘            └─────────────────┘
```

### 1.1 CLI tool

A self-contained Rust binary (`mxpm` — Maxima Package Manager, or similar)
distributed as a single executable per platform. No runtime dependencies.

Responsibilities:
- Fetch and cache the package index
- Search and browse packages
- Download packages from their source repositories
- Install packages into `~/.maxima/`
- Manage installed packages (list, update, remove)
- Run package tests
- Validate package metadata

Not responsible for:
- Anything that happens inside a Maxima session
- Building or compiling Maxima code
- Documentation generation (that's the author's job)

### 1.2 Package index

A Git repository containing a single JSON file that maps package names to
their metadata and source locations. Contributions via pull request.

The CLI fetches this index (or a cached copy) to resolve package names.
See §3 for the schema.

### 1.3 Maxima-side loader (optional, deferred)

A thin Maxima package (loaded via `load("mxpm")`) that provides
convenience functions like `mxpm_list()`, `mxpm_info("pkg")`,
`mxpm_test("pkg")`. This wraps calls to the CLI or reads the installed
manifest files directly.

This is a nice-to-have. The core system works without it — users run the
CLI to manage packages and use Maxima's standard `load()` to use them.

---

## 2. Package metadata: manifest file

Each package repository contains a `manifest.toml` at the repository root.

### 2.1 Format: TOML

TOML is chosen over alternatives for these reasons:
- Human-readable and easy to edit (unlike JSON)
- Well-specified with no ambiguities (unlike YAML)
- Widely supported in Rust (the `toml` crate)
- Familiar to users of Cargo, pyproject.toml, etc.
- Not tied to Common Lisp (unlike `.asd` or `.mxt`)

### 2.2 Required fields

```toml
[package]
name = "diophantine"
version = "1.0.0"
description = "Solver for Diophantine equations"
license = "GPL-3.0-or-later"
entry = "diophantine.mac"

[package.authors]
names = ["Serge de Marre"]
```

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Unique short name. Lowercase, alphanumeric + hyphens. Must match the index entry. |
| `version` | string | Version string (see §7). |
| `description` | string | One-line summary, max 200 characters. |
| `license` | string | SPDX license identifier. |
| `entry` | string | Relative path to the file that `load()` should find. Typically `<name>.mac`. |
| `authors.names` | array of strings | Author names. |

### 2.3 Optional fields

```toml
[package]
homepage = "https://github.com/sdemarre/maxima-diophantine"
repository = "https://github.com/sdemarre/maxima-diophantine"
keywords = ["number-theory", "diophantine", "equations"]
maxima = ">= 5.47"

[dependencies]
some_other_package = ">= 1.0"

[docs]
info = "docs/diophantine.info"
index = "docs/diophantine-index.lisp"
html-index = "docs/diophantine-index-html.lisp"
texi = "docs/diophantine.texi"

[test]
files = ["rtest_diophantine.mac"]

[native]
external-programs = ["gnuplot"]
shared-libraries = ["liblapack.so"]
```

| Field | Type | Description |
|-------|------|-------------|
| `homepage` | string | URL for the package's homepage. |
| `repository` | string | URL for the source repository. |
| `keywords` | array | Freeform keywords for search. |
| `maxima` | string | Minimum Maxima version constraint. |
| `dependencies` | table | Map of package name → version constraint. |
| `docs.info` | string | Relative path to pre-built `.info` file. |
| `docs.index` | string | Relative path to `-index.lisp` for `?`/`??` integration. |
| `docs.html-index` | string | Relative path to HTML index lisp file. |
| `docs.texi` | string | Relative path to texinfo source. |
| `test.files` | array | Relative paths to `rtest_*.mac` test files. |
| `native.external-programs` | array | External programs the package requires at runtime. |
| `native.shared-libraries` | array | Shared libraries the package requires. |

### 2.4 Name constraints

Package names must:
- Be 2–64 characters
- Contain only lowercase ASCII letters, digits, and hyphens
- Start with a letter
- Not start with `maxima-` (reserved for the system itself)
- Be unique within the index

### 2.5 Compatibility with existing packages

Most existing packages have no metadata file at all. To bootstrap the
ecosystem:
- The CLI should be able to install packages that lack a `manifest.toml`
  by falling back to convention (the repo name as the package name, the
  first `.mac` file as the entry point, no dependencies).
- The index can carry metadata for packages whose authors haven't added a
  manifest yet (see §3.2).

---

## 3. Package index

### 3.1 Structure

A Git repository containing:

```
maxima-package-index/
  index.json          — the canonical package list
  CONTRIBUTING.md     — how to add a package
  schema.json         — JSON Schema for validation
```

### 3.2 Index schema

```json
{
  "version": 1,
  "packages": {
    "diophantine": {
      "description": "Solver for Diophantine equations",
      "repository": "https://github.com/sdemarre/maxima-diophantine",
      "homepage": "https://github.com/sdemarre/maxima-diophantine",
      "keywords": ["number-theory", "diophantine"],
      "license": "GPL-3.0-or-later",
      "authors": ["Serge de Marre"],
      "source": {
        "type": "github",
        "owner": "sdemarre",
        "repo": "maxima-diophantine",
        "ref": "v1.0.0"
      }
    }
  }
}
```

Each package entry contains:

| Field | Required | Description |
|-------|----------|-------------|
| `description` | Yes | One-line summary. |
| `repository` | Yes | Canonical repository URL. |
| `source` | Yes | Download source (see §3.3). |
| `homepage` | No | Separate homepage if different from repo. |
| `keywords` | No | Search keywords. |
| `license` | No | SPDX identifier. |
| `authors` | No | Author names. |

The index carries enough metadata for search and display. Full metadata
(dependencies, entry points, test files) lives in the package's own
`manifest.toml`.

### 3.3 Source types

The `source` field describes how to download the package:

```json
// GitHub repository (download tarball via GitHub API)
{
  "type": "github",
  "owner": "sdemarre",
  "repo": "maxima-diophantine",
  "ref": "v1.0.0"
}

// GitLab repository
{
  "type": "gitlab",
  "owner": "user",
  "repo": "project",
  "ref": "main",
  "instance": "https://gitlab.com"
}

// Generic tarball URL
{
  "type": "tarball",
  "url": "https://example.com/pkg-1.0.tar.gz"
}

// Generic git repository
{
  "type": "git",
  "url": "https://git.example.com/repo.git",
  "ref": "v1.0.0"
}
```

The `ref` field can be a tag, branch, or commit hash. Tags are preferred
for reproducibility.

### 3.4 Index versioning

The top-level `"version"` field is a schema version number. The CLI must
check this and warn if it encounters a version it doesn't understand.

The index itself is versioned by Git history. The CLI fetches the latest
version each time (with caching — see §5.4).

### 3.5 Contribution process

To add a package to the index:
1. Fork the index repository
2. Add an entry to `index.json`
3. Submit a pull request

CI on the index repository validates:
- JSON schema conformance
- The `source` URL is reachable
- No duplicate package names
- Package names conform to naming rules (§2.4)

---

## 4. CLI operations

### 4.1 Command summary

```
mxpm search <query>              Search the index by name/keyword/description
mxpm info <package>              Show detailed package information
mxpm install <package>[@version] Install a package (and dependencies)
mxpm remove <package>            Remove an installed package
mxpm update [package]            Update one or all installed packages
mxpm list                        List installed packages
mxpm test <package>              Run a package's test suite
mxpm check <path>                Validate a manifest.toml (for authors)
mxpm index update                Force-refresh the cached index
```

### 4.2 `mxpm search`

Searches the index by matching `<query>` against package name,
description, and keywords. Returns a table:

```
$ mxpm search diophantine
NAME          VERSION  DESCRIPTION
diophantine   1.0.0    Solver for Diophantine equations
```

Full-text search across all text fields. Rank by relevance (name match
> keyword match > description match).

### 4.3 `mxpm install`

```
$ mxpm install diophantine
Resolving dependencies...
Downloading diophantine v1.0.0 from github:sdemarre/maxima-diophantine
Installing to ~/.maxima/diophantine/
Done. Use: load("diophantine");
```

Steps:
1. Look up the package in the index
2. Download the `manifest.toml` from the source (or use index metadata
   as fallback)
3. Resolve dependencies (§8)
4. Download the package archive (tarball)
5. Extract to `~/.maxima/<pkgname>/`
6. Write installation metadata to `~/.maxima/<pkgname>/.mxpm.json`
   (see §5.3)
7. Print the `load()` command for the user

If `@version` is specified, install that version. Otherwise install the
version specified in the index's `ref` field.

### 4.4 `mxpm remove`

```
$ mxpm remove diophantine
Removing diophantine from ~/.maxima/diophantine/
Done.
```

Deletes the package directory. Warns if other installed packages depend
on it.

### 4.5 `mxpm update`

```
$ mxpm update
Checking for updates...
diophantine: 1.0.0 → 1.1.0 (update available)
padics: up to date
Run 'mxpm install diophantine' to update.
```

Compares the installed version (from `.mxpm.json`) against the current
index. With a package name argument, updates that specific package.

### 4.6 `mxpm list`

```
$ mxpm list
NAME          VERSION  INSTALLED
diophantine   1.0.0    2026-04-10
padics        0.3.1    2026-04-12
```

Reads `.mxpm.json` files from installed packages in `~/.maxima/`.

### 4.7 `mxpm test`

```
$ mxpm test diophantine
Running rtest_diophantine.mac...
All 42 tests passed.
```

Runs the package's `rtest_*.mac` files through Maxima in batch mode:

```
maxima --batch-string='load("diophantine"); batch("rtest_diophantine.mac", test);'
```

The CLI must be able to find the `maxima` binary. It searches:
1. `$MAXIMA_BIN` environment variable
2. `maxima` on `$PATH`

### 4.8 `mxpm check`

```
$ mxpm check .
Validating manifest.toml...
OK: manifest.toml is valid.
Checking entry point: diophantine.mac exists.
Checking test files: rtest_diophantine.mac exists.
Checking docs: docs/diophantine.info exists.
```

For package authors. Validates the `manifest.toml` and checks that
referenced files exist.

---

## 5. Installation mechanics

### 5.1 Installation target

Packages are installed to `~/.maxima/<pkgname>/`.

On Maxima 5.47+, `~/.maxima/` subdirectories are automatically included
in `$file_search_maxima` and `$file_search_lisp`. This means
`load("pkgname")` works without any path configuration.

On Windows, `~/.maxima/` maps to `%USERPROFILE%/maxima/` (Maxima's
standard Windows user directory).

### 5.2 Installed package layout

After installation, a package directory contains:

```
~/.maxima/diophantine/
  manifest.toml              — package metadata (from the package repo)
  .mxpm.json                  — installation metadata (written by CLI)
  diophantine.mac            — entry point (load target)
  rtest_diophantine.mac      — tests
  docs/
    diophantine.info         — pre-built info docs
    diophantine-index.lisp   — help index for ?/??
    ...
  (other package files)
```

### 5.3 Installation metadata: `.mxpm.json`

Written by the CLI at install time, not part of the package itself:

```json
{
  "name": "diophantine",
  "version": "1.0.0",
  "installed_at": "2026-04-10T14:30:00Z",
  "source": {
    "type": "github",
    "owner": "sdemarre",
    "repo": "maxima-diophantine",
    "ref": "v1.0.0",
    "commit": "abc123def456"
  },
  "index_version": "abc123"
}
```

This allows `mxpm list` and `mxpm update` to work without re-fetching
package metadata.

### 5.4 Index caching

The CLI caches the index locally:

- Cache location: `~/.cache/mxpm/index.json` (XDG on Linux,
  `~/Library/Caches/mxpm/` on macOS, `%LOCALAPPDATA%\mxpm\cache\` on
  Windows)
- Cache TTL: 1 hour by default
- `mxpm index update` forces a refresh
- `mxpm install` and `mxpm search` auto-refresh if cache is stale
- The CLI fetches the raw `index.json` via HTTPS from the index
  repository's hosting (e.g. GitHub raw content URL or a CDN)

### 5.5 Maxima user directory detection

The CLI must determine `~/.maxima/` correctly across platforms:

1. Check `$MAXIMA_USERDIR` environment variable
2. Check `~/.maxima/` (Unix) or `%USERPROFILE%/maxima/` (Windows)
3. Fail with a clear error if neither exists

The CLI should create `~/.maxima/` if it doesn't exist, after confirming
with the user.

---

## 6. Documentation

### 6.1 Strategy: author builds, user consumes

Package authors are responsible for building documentation artifacts.
The CLI installs pre-built docs alongside the package code. Users get
help integration without needing any doc tooling.

This follows the PKG-maxima model (see
[prior-art/pkg-maxima-template.md](prior-art/pkg-maxima-template.md)):
build complexity is on the author side, not the user side.

### 6.2 Documentation artifacts

A package may include these pre-built documentation files:

| File | Purpose | How Maxima uses it |
|------|---------|-------------------|
| `*.info` | Info-format documentation | Displayed by `?` and `??` |
| `*-index.lisp` | Maps function names → byte offsets in `.info` | Loaded at `load()` time to register with help system |
| `*-index-html.lisp` | Maps function names → HTML anchors | Loaded at `load()` time for HTML help |
| `*.texi` | Texinfo source | Not used at runtime; included for reference |
| `rtest_*.mac` | Regression tests | Run by `mxpm test` |

### 6.3 Documentation loading

For `?`/`??` integration to work, the package's entry point (`.mac` file)
must load the index files:

```maxima
/* In diophantine.mac */
load("diophantine-index.lisp")$
load("diophantine-index-html.lisp")$
```

This is the package author's responsibility. The `mxpm check` command
should verify that if `docs.index` is declared in `manifest.toml`, the
entry point loads it.

### 6.4 Author tooling (out of scope for v1)

Building documentation from `.texi` source requires `makeinfo`, Perl
(`build_index.pl`), and ideally the Maxima source tree. This is the
PKG-maxima template's domain.

For v1, the CLI does not provide doc-building tools. Authors use whatever
toolchain they prefer. A future version could include a `mxpm doc build`
command that bundles the necessary tools.

### 6.5 Recommended documentation structure for authors

The `mxpm check` command and documentation should recommend:

```
docs/
  <name>.texi              — texinfo source
  <name>.info              — pre-built (makeinfo output)
  <name>-index.lisp        — pre-built (build_index.pl output)
  <name>-index-html.lisp   — pre-built (build-html-index.lisp output)
```

Packages without documentation are allowed. The `description` field in
`manifest.toml` and the README serve as minimal documentation.

---

## 7. Versioning

### 7.1 Package versions

Packages use [Semantic Versioning 2.0.0](https://semver.org/):
`MAJOR.MINOR.PATCH`.

- MAJOR: breaking changes to the package's public API
- MINOR: new functionality, backwards-compatible
- PATCH: bug fixes

Pre-release versions (e.g. `1.0.0-beta.1`) are allowed but not required.

Rationale: SemVer is well-understood and enables meaningful dependency
constraints. The Maxima ecosystem is small enough that the overhead of
semver is manageable.

### 7.2 Version constraints

Dependency version constraints use Cargo-style syntax:

| Syntax | Meaning |
|--------|---------|
| `"1.2.3"` | Exactly 1.2.3 |
| `">= 1.2"` | 1.2.0 or later |
| `">= 1.2, < 2.0"` | 1.2.0 or later, before 2.0.0 |
| `"^1.2"` | >= 1.2.0 and < 2.0.0 (compatible with 1.2) |

The `^` (caret) operator is the default and recommended constraint.

### 7.3 Version resolution

When the index `ref` is a tag like `v1.0.0`, the version is extracted
from the tag. When it's a branch name, the version comes from the
package's `manifest.toml`.

---

## 8. Dependency resolution

### 8.1 Scope

Dependencies are between Maxima packages managed by the system. The
system does not resolve:
- Common Lisp library dependencies (ASDF/Quicklisp domain)
- System-level dependencies (OS packages, shared libraries)
- External program dependencies (gnuplot, etc.)

These are declared in `manifest.toml` (§2.3) for informational purposes
and checked by `mxpm install` with a warning, but not automatically
resolved.

### 8.2 Algorithm

Dependency resolution uses a simple depth-first approach:

1. Parse the target package's `manifest.toml` for `[dependencies]`
2. For each dependency, check if it's already installed and satisfies
   the version constraint
3. If not, recursively resolve and install the dependency
4. Detect and reject circular dependencies

Given the small size of the Maxima package ecosystem (likely < 100
packages for the foreseeable future), a simple resolver is sufficient.
SAT-based resolution (like Cargo or pip) is unnecessary.

### 8.3 Conflict handling

If two packages require incompatible versions of the same dependency,
the install fails with a clear error message listing the conflicting
constraints. The system does not support multiple simultaneous versions
of the same package.

---

## 9. Native code handling

### 9.1 Strategy

The package system does not compile native code. It handles three
categories of packages with native dependencies:

#### Pure Maxima/Lisp (common case)

No special handling. Package contains only `.mac` and `.lisp` files.

#### Pre-translated Fortran (f2cl)

Packages that include f2cl-translated Lisp files ship the pre-translated
`.lisp` files. No Fortran compiler needed at install time. The package
system treats these as ordinary Lisp files.

#### External program dependencies

Packages declare required external programs in
`native.external-programs`. At install time, the CLI checks if these
programs are on `$PATH` and warns if they're missing:

```
$ mxpm install draw-extras
Warning: This package requires 'gnuplot' but it was not found on PATH.
The package will be installed, but some features may not work.
Install anyway? [Y/n]
```

#### Shared library dependencies

Packages declare required shared libraries in
`native.shared-libraries`. At install time, the CLI warns if they
appear to be missing. Installation proceeds regardless — the user may
install them later.

#### Packages requiring a build step

Some packages may need compilation during installation — e.g. CFFI
bindings that wrap a C library, or f2cl translations that haven't been
pre-generated. These packages declare a build step in `manifest.toml`:

```toml
[build]
system = "make"           # or "meson", "cargo", "custom"
command = "make install"  # executed in the package directory
requires = ["make", "cc"] # tools that must be on PATH
```

The CLI handles this as follows:

1. Check that the declared `requires` tools are available on `$PATH`
2. If missing, report what's needed and abort (don't try to install
   build tools)
3. If present, run the `command` in the package directory after
   extraction but before marking the install as complete
4. Capture build output and display it on failure

Supported build systems (v1):

| `system` | Description | Typical use |
|----------|-------------|-------------|
| `make` | GNU Make | C/Fortran via Makefile |
| `meson` | Meson build system | Cross-platform C/C++ |
| `custom` | Arbitrary command | Anything else |

The CLI does **not** bundle any build tools. It orchestrates builds using
whatever the user has installed. Packages that require a build step
should clearly document what's needed in their README and description.

#### Pre-built vs build-from-source

For maximum accessibility, packages with native code should prefer
shipping pre-built artifacts (f2cl-translated Lisp, pre-compiled shared
libraries per platform) alongside their source. The `[build]` section
is a fallback for when pre-built artifacts aren't available for the
user's platform.

A package can offer both by using platform-specific entries:

```toml
[build]
# Only needed if pre-built binaries don't cover this platform
system = "make"
command = "make"
requires = ["make", "cc"]

[build.prebuilt]
# Pre-built artifacts available for these platforms
platforms = ["x86_64-linux", "x86_64-darwin", "aarch64-darwin"]
```

When pre-built artifacts are available for the user's platform, the
build step is skipped.

### 9.2 Design rationale

The system is intentionally conservative about native code:

- **Pure Maxima/Lisp packages** (the vast majority) require no build
  tools at all.
- **Packages with pre-built native artifacts** (f2cl-translated Lisp)
  install like pure packages.
- **Packages requiring compilation** declare their build requirements
  explicitly. The CLI checks prerequisites and runs the build, but
  doesn't try to install toolchains.
- **System-level dependencies** (shared libraries, external programs)
  are declared for informational purposes and checked with warnings,
  but not automatically installed.

This keeps the common case simple while allowing complex packages to
exist in the ecosystem. The complexity cost is borne by authors of
native-code packages, not by users of pure-Maxima packages.

---

## 10. Platform and compatibility

### 10.1 CLI platforms

The CLI is distributed as a statically-linked Rust binary for:

| Platform | Target triple |
|----------|--------------|
| Linux x86_64 | `x86_64-unknown-linux-musl` |
| Linux aarch64 | `aarch64-unknown-linux-musl` |
| macOS x86_64 | `x86_64-apple-darwin` |
| macOS aarch64 | `aarch64-apple-darwin` |
| Windows x86_64 | `x86_64-pc-windows-msvc` |

Using musl on Linux ensures no glibc dependency.

### 10.2 Maxima compatibility

- **Minimum version:** 5.47 (2023)
  - Rationale: This is the version that introduced automatic scanning of
    `~/.maxima/` subdirectories. Without it, users must manually configure
    `file_search_maxima`, which the system cannot do from outside Maxima.
- **Documentation integration** (`?`/`??`): Requires Maxima 5.48+ for
  full HTML doc support. Packages targeting older versions can omit
  `docs.html-index`.

### 10.3 Lisp implementation compatibility

The CLI is implementation-agnostic — it doesn't interact with Common Lisp
at all. Package compatibility with specific Lisp implementations (SBCL,
GCL, CCL, ECL, etc.) is the package author's concern and can be noted in
the package description or README.

### 10.4 Maxima binary detection

For `mxpm test`, the CLI needs to invoke Maxima. Detection order:

1. `$MAXIMA_BIN` environment variable
2. `maxima` on `$PATH`
3. Platform-specific default locations:
   - macOS: `/Applications/Maxima.app/.../maxima`,
     `/opt/homebrew/bin/maxima`, `/usr/local/bin/maxima`
   - Windows: `C:\maxima-*\bin\maxima.bat`
   - Linux: `/usr/bin/maxima`, `/usr/local/bin/maxima`

---

## 11. Security

### 11.1 Trust model (v1)

The system operates on the same trust model as cloning a GitHub repo:
the user trusts the package author. There is no code signing, review
process, or sandboxing.

The index repository provides a lightweight layer of curation — packages
must be submitted via PR — but index maintainers do not audit package
code.

### 11.2 Transport security

All downloads use HTTPS. The CLI must verify TLS certificates and reject
self-signed or expired certificates.

### 11.3 Index integrity

The CLI fetches the index from a known URL (hardcoded or configured).
The index is a JSON file fetched over HTTPS. A compromised index could
redirect downloads to malicious packages.

Mitigation for v1: the index URL is hardcoded in the CLI binary. Users
can override it via configuration, but the default points to the official
index repository.

Future consideration: signing the index file (e.g. with minisign) so the
CLI can verify it hasn't been tampered with.

### 11.4 Package content safety

The CLI does not execute any code from packages during installation.
It only downloads, extracts, and places files. Code execution happens
only when the user explicitly runs `load()` in Maxima or `mxpm test`.

Maxima code runs with full system access (it's Common Lisp underneath).
There is no sandboxing. This is the same security posture as any
installed share/ package.

---

## 12. Configuration

### 12.1 CLI configuration file

Optional configuration at `~/.config/mxpm/config.toml` (XDG on Linux,
`~/Library/Application Support/mxpm/config.toml` on macOS,
`%APPDATA%\mxpm\config.toml` on Windows):

```toml
# Override Maxima user directory
maxima_userdir = "/custom/path"

# Override Maxima binary path
maxima_bin = "/usr/local/bin/maxima"

# Cache TTL in seconds (default: 3600)
cache_ttl = 7200

# Registries (searched in order; default community registry is always
# appended unless explicitly listed here)
[[registries]]
name = "company-internal"
url = "https://artifactory.example.com/maxima-packages/index.json"

[[registries]]
name = "community"
url = "https://raw.githubusercontent.com/<org>/maxima-package-index/main/index.json"
```

All fields are optional. Environment variables take precedence over the
config file:
- `$MAXIMA_USERDIR` — overrides `maxima_userdir`
- `$MAXIMA_BIN` — overrides `maxima_bin`
- `$MXPM_REGISTRY_URL` — replaces the default registry (for CI/containers)

### 12.2 Multiple registries

The CLI supports multiple package registries for environments that need
to layer public and private package sources.

**Resolution order:** Registries are searched in the order they appear
in the config. First match wins. The default community registry is
always present as the last entry unless explicitly listed.

**Collision handling:** When a package name exists in multiple
registries, the first registry in order provides it. Users can override
this with `--registry <name>`.

**Registry protocol:** A registry is any HTTPS URL that returns a valid
`index.json`. This makes it compatible with artifact proxies
(Artifactory, Nexus), CDNs, static file hosts, and Git raw content
URLs. The CLI does not care how the JSON is served.

**Package source proxying:** Private registries can override `source`
URLs to point at internal mirrors, enabling air-gapped environments
where GitHub is not reachable.

---

## 13. Non-functional requirements

### 13.1 Performance

- `mxpm search` should return results in < 1 second (index is small
  enough to search in memory)
- `mxpm install` latency is dominated by download time; the CLI should
  show progress
- `mxpm list` should return immediately (reads local files only)

### 13.2 Binary size

The CLI binary should be < 10 MB. Rust with static linking and LTO
should achieve this.

### 13.3 Offline operation

- `mxpm list` and `mxpm info <installed-package>` work offline
- `mxpm search` works offline against the cached index
- `mxpm install` requires network access
- `mxpm test` works offline (tests run locally)

### 13.4 Error handling

- All errors must produce human-readable messages
- Network errors should suggest checking connectivity
- Missing Maxima binary should suggest setting `$MAXIMA_BIN`
- Invalid `manifest.toml` should point to the specific problem

---

## 14. Future considerations (out of scope for v1)

These are explicitly deferred but worth noting for architectural
decisions:

- **Maxima-side integration**: A `load("mxpm")` package that provides
  `mxpm_search()`, `mxpm_install()` etc. from the Maxima prompt, shelling
  out to the CLI
- **Lock files**: A `mxpm.lock` file for reproducible installations
- **Registry discovery**: Auto-detecting registries from project-level
  config files
- **Index signing**: Cryptographic verification of the index
- **Package signing**: Cryptographic verification of individual packages
- **Auto-update**: The CLI checking for its own updates
- **Doc generation**: `mxpm doc build` that bundles `makeinfo` and
  `build_index.pl`
- **Snippet support**: Indexing sub-package-level code (Schorer's
  proposal)
- **Quality tiers**: Marking packages as "official", "community", "experimental"
- **CI integration**: A GitHub Action for validating packages on push

---

## Appendix A: Mapping to functional requirements

| Functional requirement | Technical solution |
|-----------------------|-------------------|
| §1.1 Third-party distribution | Decentralized hosting + index (§3) |
| §1.2 Discoverability | `mxpm search` with name/keyword/description (§4.2) |
| §1.3 Low maintenance | Static JSON index + PR model (§3); Rust binary needs no runtime (§1.1) |
| §1.4 Bootstrap | Pre-populate index with known packages (§3.2) |
| §2.1 Decentralized + index | Git repo index pointing to external repos (§3) |
| §2.2 Index as data | JSON file in Git (§3.1) |
| §2.3 No Maxima patches | CLI is external; Maxima's `load()` works via existing path scanning (§5.1) |
| §3.1 Short name mapping | Index maps names to source URLs (§3.2) |
| §3.2 Documentation | Pre-built docs + help integration (§6) |
| §3.3 Dependencies | `[dependencies]` in manifest.toml (§2.3); resolver (§8) |
| §3.4 Metadata format | `manifest.toml` (§2) |
| §4.1 Search/browse | `mxpm search` (§4.2) |
| §4.2 Install by name | `mxpm install` (§4.3) |
| §4.3 Load installed | `load("pkgname")` via 5.47+ auto-scanning (§5.1) |
| §4.4 List installed | `mxpm list` (§4.6) |
| §4.5 Update/remove | `mxpm update`, `mxpm remove` (§4.4, §4.5) |
| §4.6 Run tests | `mxpm test` invokes Maxima batch mode (§4.7) |
| §5.1 Minimal overhead | Add `manifest.toml` to repo, submit index PR (§2, §3.5) |
| §6.1 Cross-platform | Rust binaries for Linux/macOS/Windows (§10.1) |
| §6.2 All distributions | No Maxima patches; works with any 5.47+ install (§10.2) |
| §6.3 No external tools | Self-contained Rust binary (§1.1) |

## Appendix B: Relationship to prior art

| Decision | Informed by |
|----------|------------|
| Install to `~/.maxima/<pkg>/` | share/ auto-scanning (Maxima 5.47+) |
| TOML metadata | mext's `.mxt` fields (good schema, wrong format) |
| `rtest_*.mac` test convention | ~60% existing adoption in the wild |
| Pre-built `-index.lisp` docs | PKG-maxima's approach to `?`/`??` integration |
| Tarball download from GitHub | maxima-asdf's `install_github()` concept |
| No CL toolchain requirement | maxima-asdf's fatal adoption barrier |
| No Maxima source requirement | PKG-maxima template's fatal adoption barrier |
| No Maxima internal modifications | mext's version coupling fragility |
| Simple dep resolver | Ecosystem too small for SAT-based resolution |
| External CLI, not embedded | Avoids all CL-implementation portability issues |
