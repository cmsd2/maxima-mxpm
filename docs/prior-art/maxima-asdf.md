# maxima-asdf

**Repository:** https://github.com/robert-dodier/maxima-asdf
**Author:** Robert Dodier
**Status:** Stalled (30 commits; last substantive work 2018; single bugfix
January 2024)
**License:** GPL v2
**Size:** ~172 lines of Common Lisp across 7 files

---

## What it is

A thin glue layer (~172 lines) that bridges Maxima and the Common Lisp
ecosystem's package management tools (ASDF and Quicklisp). It lets Maxima
load packages defined using ASDF system definitions (`.asd` files) and
download them from GitHub.

---

## How it works

Three components:

### 1. `:maxima-file` ASDF component type (`maxima-file.lisp`, 33 lines)

Teaches ASDF how to handle `.mac` files by subclassing ASDF's `source-file`
class and overriding three generic functions:

- `perform(load-source-op, maxima-file)` — calls Maxima's `$LOAD`
- `perform(load-op, maxima-file)` — calls CL's `LOAD` (for pre-compiled)
- `perform(compile-op, maxima-file)` — uses Maxima's `$TRANSLATE_FILE` to
  convert `.mac` → `.LISP`, then compiles the Lisp

### 2. Maxima-callable glue (`maxima-asdf.lisp`, 40 lines)

Exposes three functions at the Maxima prompt:

- `asdf_load("name")` — compile and load a system
- `asdf_load_source("name")` — load source without compilation
- `asdf_compile("name")` — compile only

After loading, automatically appends the package's directory to all of
Maxima's `$file_search_*` paths so that `load()`, `demo()`, and help can
find the package's files.

### 3. GitHub download (`maxima-quicklisp.lisp`, 39 lines)

`install_github(user, repo, branch)` downloads a tarball from GitHub's
API via the `drakma` HTTP library, decompresses using Quicklisp's bundled
gunzip/untar, and places the result in Quicklisp's `local-projects/`
directory.

### Package author requirements

Authors must add a `.asd` file to their repository:

```lisp
(defsystem foo
  :defsystem-depends-on ("maxima-file")
  :pathname "src"
  :components
    ((:maxima-file "foo1")    ;; loads foo1.mac via Maxima
     (:maxima-file "foo2")
     (:file "bar1")))         ;; loads bar1.lisp via CL
```

### User workflow

```
;; One-time setup: install Quicklisp, clone maxima-asdf into
;; quicklisp/local-projects/, add loading to maxima-init.lisp

;; Install a package (once):
(%i1) install_github("sdemarre", "diophantine", "master");

;; Load per session:
(%i2) asdf_load_source("diophantine");
```

---

## What state it's in

- **12 stars, 1 fork** on GitHub
- **3 contributors** (robert-dodier: 8 commits, sdemarre: 4, yitzchak: 1)
- **No tests, no CI**
- **1 open bug** since 2018 (`*LOAD-PATHNAME*` differs between
  `asdf_load` and `asdf_load_source`)
- Activity: burst of work in 2016, refinements in 2018, then dormant
  until a single bugfix in January 2024

Known consumers of the `:maxima-file` component type:

| Package | Author | Notes |
|---------|--------|-------|
| maxima-foo | robert-dodier | Trivial test package |
| clifford | robert-dodier | Fork of Prodanov's package, `.asd` added |
| maxima-read-wxmx | robert-dodier | wxMaxima file parser |
| diophantine | sdemarre | Real-world consumer |

---

## Dependencies

- **Maxima** (calls internals: `$LOAD`, `$TRANSLATE_FILE`,
  `*AUTOCONF-VERSION*`)
- **ASDF 3+** (modern API: `source-file` class, `perform`, `compile-file*`)
- **Quicklisp** (`ql:quickload`, `ql:where-is-system`,
  `ql-gunzipper`, `ql-minitar`)
- **drakma** (Common Lisp HTTP client, for GitHub downloads)
- **UIOP** (part of ASDF; used for pathname operations)

---

## FFI / native code

maxima-asdf inherits ASDF's existing support for native code:

- ASDF can define `:c-source-file` and `:static-file` components
- In principle, a `.asd` file could declare Fortran or C compilation steps
- In practice, no maxima-asdf package uses this — all known consumers are
  pure Maxima/Lisp

The reliance on ASDF means packages *could* leverage CFFI or other
ASDF-managed native dependencies, but this is theoretical.

---

## Documentation generation / installation

maxima-asdf has **no documentation generation system**. It handles
documentation *discovery* only:

- After loading a package via `asdf_load`, it appends the package's
  directory to `$file_search_usage` and `$file_search_demo`, so Maxima's
  `?`, `??`, and `demo()` commands can find any `.usg`, `.info`, or `.dem`
  files the package includes.
- The `:info-index` component type (`info-index.lisp`, 27 lines) handles
  Maxima `.info` documentation index files as ASDF components, copying them
  alongside compiled output.

This means: if a package ships pre-built `.info` and index files, they'll
be findable after loading. But maxima-asdf provides no tooling to *create*
those files.

---

## Strengths

- **Minimal and focused.** 172 lines that do something real. Easy to
  understand and audit.
- **Leverages existing infrastructure.** ASDF is battle-tested and widely
  used in the CL ecosystem.
- **`.asd` as metadata.** ~10 third-party Maxima packages already have
  `.asd` files — the closest thing to a de facto metadata standard.
- **Automatic path management.** After loading a package, Maxima's search
  paths are updated automatically. Users don't need to configure
  `file_search_maxima` manually.
- **Sound core abstraction.** The `:maxima-file` ASDF component type is a
  well-implemented, clean extension of ASDF's component model.

## Weaknesses

- **Requires a full CL + Quicklisp environment.** Most Maxima users have
  standalone binaries where `(require 'asdf)` doesn't work. This is the
  fatal adoption barrier.
- **GCL incompatible.** GCL (historically the default Lisp for Maxima on
  many Linux distros) doesn't support ASDF.
- **GitHub-only downloads.** No support for other hosting, mirrors, or
  local archives.
- **No registry or discovery.** Users must already know the
  user/repo/branch to install anything.
- **No versioning.** `install_github` takes a branch name, not a version.
  No version constraints, no lockfiles, no reproducible installs.
- **No update or remove.** Once installed, there's no mechanism to update
  or uninstall a package.
- **No dependency resolution between Maxima packages.** ASDF handles
  CL-level dependencies, but there's nothing for Maxima-level deps.
- **Fragile Quicklisp interaction.** Quicklisp bundles an ancient ASDF
  (2.26), which Dodier noted as a reliability problem.
