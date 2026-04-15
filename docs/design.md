# Maxima Package System: Design

## The problem

Maxima has no way to distribute, discover, or install third-party
packages. This has been discussed on the mailing list since at least
2016 with no resolution. The community broadly agrees on what's needed
but not how to build it, and three previous attempts have stalled or
been abandoned.

The consequences are real. New users can't find packages. Authors have
no distribution channel beyond "put it on GitHub and hope someone finds
it." Useful code gets lost on mailing list threads. The `share/`
directory — Maxima's only official distribution mechanism — is gatekept
by core developers and can't grow without their involvement.

This design describes a system that solves these problems while
respecting the Maxima community's values, working with the tools and
conventions that already exist, and remaining maintainable by a small
volunteer team.

### Related documents

- [Mailing list discussions](mailing-list-discussions.md) — source
  material from the community
- [Functional requirements](requirements.md) — what the system must do
- [Prior art analysis](prior-art.md) — assessment of existing efforts
- [Technical requirements](technical-requirements.md) — concrete
  technical decisions
- [Roadmap](roadmap.md) — phased delivery plan

---

## Design principles

These principles are drawn from the community's own discussions and from
the lessons of the prior attempts.

### 1. Work with Maxima, not against it

The system should feel like a natural extension of the Maxima ecosystem,
not a foreign import. It should use the conventions that Maxima users
and package authors already follow, and leverage the mechanisms that
Maxima already provides.

Concretely: packages are loaded with `load()`. Tests use `rtest_*.mac`.
Documentation uses Texinfo and integrates with `?`/`??`. Installation
targets `~/.maxima/`, which Maxima 5.47+ scans automatically.

### 2. The common case must be trivial

For most Maxima packages — a `.mac` file, maybe some tests, maybe a
README — the packaging overhead should be nearly zero. Complex features
(native code builds, documentation generation, dependency resolution)
exist for the packages that need them but don't burden the majority that
don't.

### 3. Complexity belongs on the author side, not the user side

Users should never need to install toolchains, configure paths, or
understand Common Lisp internals. If something is hard, it should be
hard for the package author (once) rather than for every user
(repeatedly). Pre-built documentation artifacts, pre-translated Fortran,
pre-compiled binaries: the theme is "authors build, users consume."

### 4. No infrastructure to maintain

The system must be sustainable with effectively zero ongoing
infrastructure cost. No servers to run, no databases to manage, no
build farm to operate. Static files, Git repositories, and GitHub's
free hosting tier provide everything needed.

This is the lesson of every prior Maxima packaging effort: ambitious
systems that require sustained maintenance stall when their single
maintainer loses interest.

### 5. Start with what exists

The chicken-and-egg problem is real. The system must launch with
packages already in it — indexed from existing GitHub repositories and
the `maxima-packages` collection. Package authors shouldn't need to
change anything for their packages to be discoverable; the system
should accommodate packages as they exist today.

---

## Architecture

Three components, each independently simple:

```
┌──────────────────────────────────────────────────────┐
│                    User's machine                     │
│                                                      │
│  ┌──────────┐   ┌─────────────────────────────────┐  │
│  │   mxpm    │──►│  ~/.maxima/                     │  │
│  │  (CLI)   │   │    diophantine/                  │  │
│  └────┬─────┘   │      diophantine.mac             │  │
│       │         │      manifest.toml               │  │
│       │         │      rtest_diophantine.mac        │  │
│       │         │      .mxpm.json                   │  │
│       │         │    padics/                        │  │
│       │         │      ...                          │  │
│       │         └───────────────┬───────────────────┘  │
│       │                         │                      │
│       │              ┌──────────▼──────────┐           │
│       │              │   Maxima 5.47+      │           │
│       │              │                     │           │
│       │              │   load("diophan..") │           │
│       │              │   ? solve_dioph     │           │
│       │              └─────────────────────┘           │
└───────┼────────────────────────────────────────────────┘
        │ HTTPS
┌───────▼────────────────────────────────────────────────┐
│                      GitHub                            │
│                                                        │
│  ┌────────────────────┐    ┌────────────────────────┐  │
│  │  Package index     │    │  Package repositories  │  │
│  │  (index.json)      │    │                        │  │
│  │                    │    │  sdemarre/diophantine   │  │
│  │  Maps names to     │───►│  josanvallejo/padics   │  │
│  │  source locations  │    │  dprodanov/clifford    │  │
│  │                    │    │  ...                   │  │
│  └────────────────────┘    └────────────────────────┘  │
└────────────────────────────────────────────────────────┘
```

### The CLI tool: `mxpm`

A single Rust binary, statically linked, distributed for Linux (x86_64,
aarch64), macOS (x86_64, aarch64), and Windows (x86_64). No runtime
dependencies. No installation framework — download the binary and put
it on your PATH.

The CLI is deliberately external to Maxima. This is the most important
architectural decision and the one that distinguishes this design from
maxima-asdf and mext.

**Why external, not embedded in Maxima?**

Every prior attempt that embedded itself in Maxima hit the same wall:
Common Lisp portability. maxima-asdf requires Quicklisp and ASDF, which
don't work with standalone Maxima binaries or GCL. mext patches Maxima
internals, creating version coupling. Both are limited to the subset of
CL implementations they've been tested on.

An external CLI sidesteps all of this. It doesn't care which Lisp runs
Maxima, whether Maxima is a standalone binary or a Lisp image, or which
version of ASDF is available. It manipulates files on disk. Maxima finds
those files through its own standard mechanisms.

The tradeoff is that users must leave the Maxima prompt to manage
packages. This is acceptable because package management is an infrequent
operation (install once, use many times), and a Maxima-side wrapper can
be added later for users who prefer it (see [roadmap](roadmap.md)
Phase 10).

**Why Rust?**

- Single binary, no runtime dependencies — critical for an audience of
  mathematicians who may not have development toolchains installed
- Cross-platform with a single codebase
- Strong HTTP, JSON, TOML, and archive handling in the ecosystem
  (`reqwest`, `serde`, `toml`, `flate2`/`tar`)
- Comparable systems (Nimble, Cargo, KiCad PCM) validate that a compiled
  CLI is the right approach for package management

### The package index

A JSON file in a Git repository. Contributions via pull request.

This follows the model proven by Nimble (Nim's package manager), which
uses a single `packages.json` in a Git repo. Homebrew originally used
a Git repo of Ruby formula files but migrated to a JSON API in v4.0
because the Git-based approach was too slow at scale. For Maxima's
scale (~50–100 packages), a single JSON file fetched over HTTPS is
more than sufficient.

The index contains just enough to search and locate packages:

```json
{
  "schema_version": 1,
  "packages": {
    "diophantine": {
      "description": "Solver for Diophantine equations",
      "keywords": ["number-theory", "diophantine"],
      "authors": ["Serge de Marre"],
      "license": "GPL-3.0-or-later",
      "repository": "https://github.com/sdemarre/maxima-diophantine",
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

Authoritative metadata — dependencies, entry points, test files,
documentation paths — lives in the package's own `manifest.toml`.
The index is a pointer, not a copy. This is the same separation of
concerns used by Nimble (index points to repos; `.nimble` files carry
the details) and MELPA (recipes point to repos; `.el` headers carry
the metadata).

**Why JSON and not TOML/YAML for the index?**

The index is machine-generated and machine-consumed. JSON is the
universal interchange format, has a well-defined schema language (JSON
Schema), and is trivially parseable in every language. TOML is better
for files humans edit directly — which is why `manifest.toml` uses it.

**Why a schema version number?**

KiCad's PCM includes schema versioning from day one, which allows their
CLI to support both old and new index formats during transitions. Adding
`schema_version` now costs nothing and avoids a painful migration later.

### The installation target: `~/.maxima/`

Packages are installed to `~/.maxima/<pkgname>/`. This is the single
most important decision informed by the prior art analysis: Maxima 5.47+
(released 2023) automatically scans subdirectories of `~/.maxima/` and
adds them to `$file_search_maxima` and `$file_search_lisp`. This means:

```
$ mxpm install diophantine
# Extracts files to ~/.maxima/diophantine/

(%i1) load("diophantine");
# Just works. No path configuration needed.
```

No modifications to Maxima. No init file changes. No `file_search`
manipulation. The package system leverages a mechanism Maxima already
provides.

This is the foundation that none of the prior efforts had. maxima-asdf
(2016) and mext (2012) predate the 5.47 auto-scanning feature and had
to solve the path problem themselves — maxima-asdf by updating search
paths from Lisp, mext by maintaining its own installation directories.
We can build on the solution that Maxima itself now provides.

### Registries

A **registry** is a URL that serves a package index. The CLI supports
multiple registries with a well-defined resolution order.

#### Default registry

The CLI ships with a hardcoded default registry URL pointing to the
official community index:

```
https://raw.githubusercontent.com/<org>/maxima-package-index/main/index.json
```

This URL is baked into the binary so that `mxpm install foo` works out
of the box with zero configuration. The default can be overridden but
not removed.

The default registry is a Git repository. Anyone can submit a package
via pull request. CI validates entries against the JSON schema and
checks that source URLs are reachable. Merge rights are held by a
small group of maintainers from the Maxima community.

#### Multiple registries

Users can configure additional registries for proprietary packages,
institutional collections, or mirrors:

```toml
# ~/.config/mxpm/config.toml

[[registries]]
name = "community"
url = "https://raw.githubusercontent.com/<org>/maxima-package-index/main/index.json"
# This is the built-in default; listing it here is optional but allows
# changing its priority.

[[registries]]
name = "company-internal"
url = "https://artifactory.example.com/maxima-packages/index.json"

[[registries]]
name = "research-group"
url = "https://gitlab.university.edu/mathlab/maxima-pkgs/-/raw/main/index.json"
```

This supports the common pattern of layering public and private
packages: a research group or company maintains an internal registry
of proprietary packages alongside the public community registry.

#### Registry resolution order

When looking up a package, the CLI searches registries in the order
they appear in the configuration:

1. First match wins — if a package name appears in multiple registries,
   the first registry in the list provides it.
2. The default community registry is always present as the last entry
   unless explicitly reordered.
3. `mxpm search` searches all registries and labels results with the
   source registry name.
4. `mxpm install` reports which registry a package is being installed
   from.

```
$ mxpm search eigensolve
NAME          REGISTRY          DESCRIPTION
eigensolve    company-internal  Proprietary eigenvalue solver extensions
eigensolve    community         Community eigenvalue solver

$ mxpm install eigensolve
Installing eigensolve from registry 'company-internal'...
```

To install from a specific registry when names collide:

```
$ mxpm install eigensolve --registry community
```

#### Registry proxies and mirrors

The registry protocol is a single JSON file served over HTTPS. This
makes it trivially compatible with artifact proxies like Artifactory,
Nexus, or Verdaccio. A corporate environment can:

1. **Mirror** the community registry — periodically fetch the upstream
   `index.json` and serve it from an internal URL. Provides air-gapped
   access and caching.
2. **Proxy** the community registry — Artifactory-style transparent
   proxying with local cache. The internal URL serves the upstream index
   with corporate packages merged in.
3. **Serve a private registry** — host a separate `index.json` with
   proprietary packages that never touch the public internet.

The CLI does not distinguish between these cases. A registry is a URL
that returns a valid `index.json`. How that JSON gets there — static
file, Git raw content, artifact proxy, CDN — is outside the CLI's
concern.

#### Package source proxying

The index maps package names to source URLs (GitHub tarballs, GitLab
archives, etc.). In environments where direct GitHub access is
restricted, a private registry can override the `source` URLs to point
to internal mirrors:

```json
{
  "schema_version": 1,
  "packages": {
    "diophantine": {
      "description": "Solver for Diophantine equations",
      "source": {
        "type": "tarball",
        "url": "https://artifactory.example.com/maxima-pkg/diophantine-1.0.0.tar.gz"
      }
    }
  }
}
```

This is identical to the standard index format — private registries
simply point `source.url` at internal storage rather than GitHub.

#### Registry configuration via environment

For CI and containerised environments, the registry list can be
overridden without a config file:

```bash
export MXPM_REGISTRY_URL="https://artifactory.example.com/maxima-packages/index.json"
```

When set, this replaces the default registry. Additional registries
still require the config file.

---

## How it meets requirements

The following traces each functional requirement (from
[requirements.md](requirements.md)) to a specific design decision.

### Third-party distribution (§1.1)

Package authors host their code in their own repositories. They don't
need permission from anyone. To make their package discoverable, they
submit a pull request to the index — a process that requires adding
~10 lines of JSON and no special tooling.

This is the decentralized model the community has consistently
preferred (Dodier 2016, 2024; Königsmann 2017; Macrakis 2022). The
index is a directory, not a gatekeeper. It doesn't store code, review
code, or control code.

### Discoverability (§1.2)

`mxpm search` provides command-line search across package names,
descriptions, and keywords. `mxpm info` shows detailed metadata for
any indexed package. A static catalog website (roadmap Phase 9)
provides web-based browsing.

This is the "big question" that Macrakis identified in 2022. No prior
Maxima packaging effort addresses it at all.

### Low maintenance burden (§1.3)

The system has no running infrastructure:
- The index is a static JSON file in a Git repo
- The CLI is a compiled binary with no runtime dependencies
- Packages are hosted by their authors on GitHub/GitLab
- CI validates index contributions automatically

This directly addresses Dodier's concern (2024): "I don't think we
want a central package repository, because that requires someone to
maintain it." The index is a directory maintained by the community via
pull requests, not a service maintained by an individual.

Quicklisp's single-maintainer bottleneck (Zach Beane manually adds
every package) is the anti-model. MELPA's build server is closer but
still requires infrastructure. The Nimble model — a JSON file in a Git
repo with PR-based contributions and CI validation — requires the least
ongoing effort and is what we adopt.

### Bootstrap from existing packages (§1.4)

The index launches pre-populated with every known Maxima package on
GitHub: diophantine, padics, clifford, numericalMethods, raddenest,
qm-maxima, and the ~20 packages in the maxima-packages repository.

Crucially, **packages don't need a `manifest.toml` to be indexed.**
The index carries the metadata. The CLI falls back to conventions (repo
name as package name, first `.mac` file as entry point) for packages
without manifests. This means existing packages work without their
authors changing anything.

### Cross-platform (§6.1) and all distributions (§6.2)

The CLI is distributed as a statically-linked binary for all major
platforms. It doesn't interact with Common Lisp, so it works regardless
of which Lisp implementation Maxima uses, whether Maxima is a standalone
binary or a Lisp image, or how Maxima was installed.

This directly solves the adoption barriers of maxima-asdf (requires
full CL + Quicklisp, doesn't work with standalone binaries, doesn't
work with GCL) and PKG-maxima (requires the Maxima source tree, Perl,
TeX, and Unix).

### No external tooling (§6.3)

The CLI is a single binary. Users don't need git, curl, Python, Perl,
or any other tool. This is the advantage of Rust: statically-linked
binaries with built-in HTTP, TLS, JSON, TOML, and archive handling.

Contrast with: maxima-asdf (requires Quicklisp + drakma), mext
(requires bash), PKG-maxima (requires Perl + TeX + makeinfo + AWK +
the Maxima source tree), Nimble (requires git).

---

## Working with existing conventions

The design deliberately adopts existing Maxima conventions rather than
inventing new ones. Each choice is informed by what's already in use.

### Test files: `rtest_*.mac`

The `rtest_*.mac` convention is used by Maxima's own test suite, by
~60% of existing third-party packages, and by all three prior packaging
efforts. `mxpm test` discovers and runs these files through Maxima's
standard `batch(..., test)` mechanism.

No new test format. No test framework to learn. Authors write tests the
same way they always have.

### Documentation: Texinfo with `-index.lisp`

Maxima uses Texinfo for its internal documentation. The `?` and `??`
help commands search an index that maps function names to byte offsets
in `.info` files. PKG-maxima demonstrated that third-party packages can
integrate with this system by shipping pre-built `-index.lisp` files
that register with the same help tables.

The design adopts this approach: package authors write `.texi` source,
build it to `.info` and `-index.lisp`, and ship the artifacts. When
the user `load()`s the package, the index file is loaded and the
package's documentation appears alongside built-in documentation.

This is the only approach that gives third-party packages the same
documentation UX as built-in packages.

### Package loading: `load("pkgname")`

Maxima's `load()` function is the standard way to use a package. The
design preserves this: after `mxpm install foo`, the user types
`load("foo")` exactly as they would for a `share/` package. There is
no special loading function, no `require()` variant, no URL syntax.

This is possible because Maxima 5.47+ auto-scans `~/.maxima/`
subdirectories. The system adds no new loading mechanism.

### Metadata: informed by `.mxt` and `.asd`

The `manifest.toml` schema borrows its fields from mext's `.mxt` format
(name, author, version, license, description) and ASDF's `.asd` format
(components, dependencies). These are the two metadata formats that
Maxima package authors have actually used. The field names are familiar;
only the file format (TOML instead of s-expressions) is new — and TOML
is chosen precisely because it doesn't require any Lisp knowledge.

### Package naming

Package names use the same conventions as Maxima identifiers: lowercase,
underscores or hyphens, descriptive. Names like `diophantine`, `padics`,
`clifford`, `raddenest` are natural to the community. The system doesn't
impose a namespace hierarchy (no `org.maxima.contrib.foo`), reverse-DNS
naming, or other enterprise conventions.

---

## How it avoids reinventing the wheel

### Architecture: the Nimble model

The overall architecture — a JSON index in a Git repo, fetched over
HTTPS, pointing to packages hosted on GitHub — is the model used by
Nimble (Nim's package manager, ~2000 packages) and validated by
Homebrew's migration from Git-based formulae to a JSON API in v4.0.
These systems demonstrate that a static file served over HTTPS is
sufficient for ecosystems much larger than Maxima's.

We adopt this proven architecture rather than designing a new one.

### Index caching: the KiCad pattern

KiCad's Plugin and Content Manager uses a two-level JSON structure:
a small `repository.json` that includes a SHA256 hash of the full
`packages.json`. The client downloads the tiny descriptor, checks
whether the hash has changed, and only re-downloads the full index
when needed.

We adopt this pattern for efficient cache validation without requiring
complex ETags or conditional HTTP requests.

### Package download: tarballs over HTTPS

Quicklisp, Homebrew, KiCad PCM, and MELPA (for end users) all download
packages as archives over HTTPS rather than requiring git. This is the
right approach for Maxima's audience: mathematicians who may not have
git installed. GitHub, GitLab, and SourceForge all provide tarball
download APIs.

The CLI uses `reqwest` for HTTP and `flate2`/`tar` for archive handling
— well-maintained Rust crates used by Cargo itself.

### Contribution model: pull requests

Every successful package index (Nimble, Homebrew, MELPA, KiCad PCM)
uses pull/merge requests for contributions. The process is well
understood by developers and requires no custom tooling. CI validates
contributions automatically.

We adopt this directly. No custom submission tool, no issue-based
workflow (Quicklisp's bottleneck), no special access requirements.

### Dependency resolution: keep it simple

With < 100 packages and likely < 5 dependency edges in the entire graph,
a SAT solver (Cargo, pip) is unnecessary. A simple depth-first resolver
with cycle detection is sufficient and trivial to implement. If the
ecosystem grows to a point where this breaks, it can be upgraded without
changing the manifest format.

### Test execution: Maxima's own batch mode

`mxpm test` invokes `maxima --batch-string='...'` to run tests. This
uses Maxima's own test infrastructure — no test runner to build, no
output format to parse (beyond Maxima's standard test summary).

---

## Package metadata: `manifest.toml`

A package's authoritative metadata lives in a `manifest.toml` file at
the repository root. This is the file that package authors write.

### Minimal example

```toml
[package]
name = "diophantine"
version = "1.0.0"
description = "Solver for Diophantine equations"
license = "GPL-3.0-or-later"
entry = "diophantine.mac"
authors = ["Serge de Marre"]
```

Six fields. This is all that's needed for a simple Maxima package with
no dependencies, no documentation artifacts, and no native code. The
time to write this is under two minutes.

### Full example

```toml
[package]
name = "diophantine"
version = "1.2.0"
description = "Solver for Diophantine equations"
license = "GPL-3.0-or-later"
entry = "diophantine.mac"
authors = ["Serge de Marre"]
homepage = "https://github.com/sdemarre/maxima-diophantine"
repository = "https://github.com/sdemarre/maxima-diophantine"
keywords = ["number-theory", "diophantine", "equations"]
maxima = ">= 5.47"

[dependencies]
# Other Maxima packages this package requires
some-helper = "^1.0"

[test]
files = ["rtest_diophantine.mac"]

[docs]
info = "docs/diophantine.info"
index = "docs/diophantine-index.lisp"
html-index = "docs/diophantine-index-html.lisp"
texi = "docs/diophantine.texi"

[native]
external-programs = ["gnuplot"]
shared-libraries = ["liblapack.so"]

[build]
system = "make"
command = "make"
requires = ["make", "cc"]

[build.prebuilt]
platforms = ["x86_64-linux", "aarch64-darwin"]
```

Every field beyond the `[package]` section is optional. Packages grow
into more metadata as they need it.

### Why TOML?

- **Human-friendly.** Unlike JSON, TOML supports comments, doesn't
  require quoting keys, and handles multiline strings naturally. Package
  authors will edit this file by hand.
- **Unambiguous.** Unlike YAML, TOML has no implicit type coercion, no
  significant whitespace gotchas, no "Norway problem" (`NO` parsed as
  `false`).
- **Not Lisp.** Unlike `.asd` or `.mxt`, TOML doesn't require knowledge
  of s-expression syntax. This matters for a community where many users
  are mathematicians, not programmers.
- **Rust-native.** The `toml` crate provides derive-based deserialization,
  making validation straightforward.
- **Familiar.** Users of Cargo (`Cargo.toml`), Python (`pyproject.toml`),
  or Hugo (`config.toml`) will recognize the format.

### Fallback for packages without manifests

Most existing Maxima packages have no metadata file. The system handles
this gracefully:

1. The index carries enough metadata (name, description, source URL)
   for search and download.
2. The CLI installs manifest-less packages by convention: the repo name
   is the package name, the first `.mac` file is the entry point.
3. `mxpm list` shows `-` for version when no manifest exists.

This means every known Maxima package on GitHub can be indexed and
installed today, without their authors changing a single file. Authors
add manifests when they're ready, at their own pace.

---

## Documentation pipeline

Documentation integration is a first-class concern, not an afterthought.
Macrakis (2022) identified discoverability and documentation as the "big
question." PKG-maxima demonstrated that third-party packages can
integrate with Maxima's help system. This design adopts and simplifies
that approach.

### How it works

```
Author's machine                          User's machine
─────────────────                         ──────────────

 package.texi                              mxpm install foo
     │                                          │
     ▼                                          ▼
 makeinfo ──► package.info                 ~/.maxima/foo/
     │                                       foo.mac
     ▼                                       foo.info
 build_index.pl ──► package-index.lisp       foo-index.lisp
                                               │
                    ┌──────────────────────────┘
                    ▼
              load("foo")
                    │
                    ▼
              loads foo-index.lisp
                    │
                    ▼
              ? my_function ──► shows help from foo.info
```

**Authors** write Texinfo source, build it with `makeinfo` and
`build_index.pl`, and commit the artifacts to their repository. This
is a one-time setup cost.

**Users** run `mxpm install foo` and then `load("foo")`. The package's
`.mac` entry point loads the `-index.lisp` file, which registers the
package's documentation with Maxima's help system. `? my_function`
then works exactly as it does for built-in functions.

**The CLI** doesn't generate documentation. It doesn't need makeinfo,
Perl, or the Maxima source tree. It just places the pre-built artifacts
in the right directory. All the build complexity is on the author side.

### Reducing the authoring burden

The current doc-building toolchain (PKG-maxima) requires the Maxima
source tree for `build_index.pl` and `build-html-index.lisp`. This is
the primary barrier to documentation adoption.

Mitigation, in order of feasibility:

1. **Extract and distribute** `build_index.pl` as a standalone tool,
   independent of the Maxima source tree. It's a single Perl script
   that parses `.info` files.
2. **Reimplement the index builder** as part of the CLI (`mxpm doc
   build`). The logic is straightforward: read an `.info` file, find
   `@anchor` markers and their byte offsets, emit a Lisp file. A Rust
   implementation would eliminate the Perl and Maxima-source
   dependencies entirely.
3. **Provide a GitHub Action** that authors can add to their CI pipeline
   to build documentation automatically on push.

These are roadmap items (Phase 8), not launch requirements. Packages
launch without documentation and add it when the tooling is ready.

### Documentation is optional

Packages without documentation are fully functional. The `description`
field in the manifest and the project README serve as minimal
documentation. As the ecosystem matures and the tooling improves,
authors can add Texinfo documentation incrementally.

---

## Native code and build steps

Most Maxima packages are pure `.mac` code. But the ecosystem includes
packages with Fortran (via f2cl), C (via CFFI), or external program
dependencies (gnuplot, Python). The design handles each case without
requiring users to install build tools for the common case.

### Strategy by package type

| Package type | Author does | User sees |
|-------------|------------|-----------|
| Pure Maxima (`.mac`/`.lisp`) | Nothing special | `mxpm install`, done |
| Pre-translated Fortran (f2cl) | Ship translated `.lisp` files | Same as pure Maxima |
| External programs | Declare in `[native]` | Warning if program missing |
| Shared libraries | Declare in `[native]` | Warning if library missing |
| Needs build step | Declare in `[build]` | Build runs if tools present |

### The `[build]` section

For packages that need compilation:

```toml
[build]
system = "make"           # or "meson", "custom"
command = "make"
requires = ["make", "cc"]

[build.prebuilt]
platforms = ["x86_64-linux", "x86_64-darwin", "aarch64-darwin"]
```

The CLI checks whether pre-built artifacts exist for the user's platform.
If so, it skips the build. If not, it checks that the required tools are
available and runs the build command. If the tools are missing, it
reports what's needed and aborts.

This mirrors how Homebrew handles bottles: pre-built binaries are the
default; source builds are the fallback when a bottle isn't available
for the user's platform.

### What the system doesn't do

The system does not compile Fortran, install shared libraries, download
build tools, or manage CFFI bindings. These are the package author's
responsibility. The system's role is to declare dependencies
transparently and orchestrate builds that the author has already set up.

---

## Security model

### Trust model

The system operates on the same trust model as installing software from
GitHub: the user trusts the package author. There is no code signing,
no sandboxing, no code review. Maxima code runs with full system access
(it's Common Lisp), which is the same security posture as every existing
`share/` package.

The index provides a lightweight curation layer — packages must be
submitted via PR — but index maintainers do not audit code.

### Transport security

All downloads use HTTPS with certificate verification. The index URL is
hardcoded in the CLI binary (configurable via `config.toml`), preventing
trivial redirection attacks.

### No code execution during install

The CLI never executes code from packages during installation (except
for declared `[build]` commands, which require explicit build tool
declarations). It downloads, extracts, and places files. Code runs only
when the user explicitly calls `load()` or `mxpm test`.

### Future considerations

Index signing (e.g. with minisign) and package checksums in the index
are natural next steps when the ecosystem matures. They can be added
without changing the architecture.

---

## Meeting future needs

The design is intentionally minimal for v1, but the architecture
supports growth in several directions.

### More packages

The JSON index scales comfortably to thousands of packages (Nimble has
~2000). If it ever outgrows a single file, the KiCad PCM's approach —
a small descriptor pointing to a larger data file — provides a
migration path without changing the CLI's interface.

### Richer discovery

The index schema supports keywords, descriptions, and author
information. A static catalog website (roadmap Phase 9) can be generated
from the index and hosted on GitHub Pages at zero cost. Search can be
enhanced with categories, tags, and popularity metrics without changing
the core architecture.

### Maxima-side integration

A thin Maxima package (`mxpm.mac`) can provide in-session operations:

```maxima
load("mxpm");
mxpm_search("diophantine");
mxpm_install("diophantine");
mxpm_test("diophantine");
```

This wraps CLI calls via `system()` or reads `.mxpm.json` files directly.
It's a convenience layer that doesn't change the architecture.

### Dependency graph growth

The simple depth-first resolver is adequate for a small ecosystem. If
dependency graphs become complex, the resolver can be upgraded to a more
sophisticated algorithm without changing the manifest format or the
index schema. The `[dependencies]` table with version constraints is
future-proof.

### Quality and trust

When the ecosystem has enough packages to warrant it, the index can add
fields for quality tiers ("official", "community", "experimental"),
CI status badges, download counts, and trust levels. These are
additive changes to the index schema, covered by the schema version
number.

### Integration with Maxima core

If the Maxima project decides to adopt the package system, deeper
integration is possible:
- `load()` could resolve package names via the index
- `share/` packages could be distributed as packages with manifests
- The CLI could be bundled with the Maxima installer

None of these require architectural changes. The system is designed to
work alongside Maxima today and integrate with it tomorrow.

---

## What this design does not do

Being explicit about scope:

- **Does not replace `share/`.** The share directory continues to serve
  its purpose for core packages. This system is for everything else.
- **Does not build documentation.** Authors build docs; the system
  installs them. (A `mxpm doc build` command is a future possibility.)
- **Does not run a package registry.** There is no server, no database,
  no account system. The index is a file in a Git repo.
- **Does not sandbox packages.** Maxima code has full system access.
  This is a property of Maxima, not something the package system can
  change.
- **Does not handle Common Lisp dependencies.** CL-level dependencies
  (ASDF systems, Quicklisp libraries) are outside scope. Packages that
  need them can document the requirement.
- **Does not enforce code quality.** The index is a directory, not a
  curator. Quality tiers can be added later, but v1 accepts any
  good-faith contribution.

---

## Summary

The system is a package index (JSON file in a Git repo) + a CLI tool
(Rust binary) + a metadata format (`manifest.toml`). Packages are
hosted by their authors on GitHub. Users install them with `mxpm install
foo` and load them with `load("foo")`.

It works because of Maxima 5.47+'s auto-scanning of `~/.maxima/` — the
one recent development that makes everything else possible. It's
maintainable because it has no running infrastructure. It's adoptable
because it requires no changes from existing package authors. And it
leaves room to grow as the ecosystem matures.

The architecture is proven by Nimble, Homebrew, MELPA, and KiCad PCM.
The conventions are native to Maxima. The tradeoffs are informed by the
specific failures of maxima-asdf, mext, and PKG-maxima.

What's new is not the individual pieces but their combination: the
right architecture for this community, at this scale, building on the
specific things that Maxima already provides.
