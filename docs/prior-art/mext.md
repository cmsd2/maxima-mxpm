# mext

**Repository:** https://github.com/jlapeyre/mext
**Author:** John Lapeyre
**Status:** Abandoned (705 commits; last activity September 2018)
**License:** GPL
**Size:** ~8.7 MB of Common Lisp (including bundled libraries); 25 packages

---

## What it is

The most ambitious attempt at Maxima package management — both a package
manager *and* a collection of 25 repackaged libraries. It provides
installation, loading, metadata, testing, documentation, and even symbol
protection for Maxima packages.

---

## How it works

### Architecture

```
Maxima ←→ mext runtime (require, mext_info, etc.)
              ↓
         MK:DEFSYSTEM (build/load orchestration)
              ↓
         Per-package .system + .mxt definitions
              ↓
         Installation to ~/.maxima/mext-VERSION-LISP-LISPVER/
```

### Build system: MK:DEFSYSTEM

mext uses MK:DEFSYSTEM (Mark Kantrowitz's portable DEFSYSTEM, v3.4i3),
a pre-ASDF Common Lisp system definition facility from 1989. The full
implementation (211 KB) is bundled in the repository. mext extends it with
custom operations and a custom language (`:mext-maxima`) for handling
`.mac` files.

The choice of MK:DEFSYSTEM over ASDF was deliberate: GCL was incompatible
with ASDF at the time, and GCL was the most common Maxima Lisp on Windows.

### Package metadata: `.mxt` files

Each package has a `.mxt` descriptor:

```lisp
(in-package :mext)
(distribution-description
   :name   "nelder_mead"
   :author "Mario S. Mommer"
   :maintainer "John Lapeyre"
   :version ""
   :license "in_dist"
   :description "Nelder-Mead optimization algorithms")
```

Fields: `:name`, `:author`, `:maintainer`, `:version`, `:license`,
`:description`, `:long-description`.

### System definitions: `.system` files

Each package has a `.system` file (MK:DEFSYSTEM format) declaring its
components and dependencies.

### Installation

- `install_mext.sh` bootstraps the build: loads `bin/build_essential.mac`,
  builds the core `mext_system` first, then progressively builds other
  packages.
- Each package's `ibuild.mac` file orchestrates its own build.
- Files are compiled to native Lisp binaries and installed to
  `~/.maxima/mext-maxima-VERSION-LISP-LISPVERSION/`.
- The version-qualified directory structure allows multiple Maxima/Lisp
  combinations to coexist.

### Runtime loading

1. `load(mext)` loads `mext_load-user.lisp`
2. Detects Lisp implementation, constructs correct binary extension,
   finds installation directory
3. `require('pkg)` checks a hash table of loaded packages; loads only
   if not already loaded
4. Tiered loading: `load(mext1)` through `load(mext3)` load progressively
   more packages automatically

### User-facing operations

```maxima
load(mext);                /* bootstrap */
require('aex);             /* idempotent load */
mext_list();               /* list all installed packages */
mext_list_loaded();        /* list currently loaded packages */
mext_info('aex);           /* show package metadata */
mext_test('aex);           /* run package's regression tests */
mext_list_package('aex);   /* list functions in package */
```

### Symbol protection

`dont_kill('sym)` / `allow_kill('sym)` prevents `kill(all)` from
destroying package state — an important concern for interactive use.

### The `defmfun1` macro

A substantial subsystem (38 KB) for defining Maxima-callable functions
from Lisp with automatic argument type checking, option handling,
documentation generation, and function attributes. Powerful but complex.

---

## What state it's in

- **705 commits** over ~6 years of active development (2012–2018)
- **Last tested** with Maxima 5.41.0 + SBCL 1.4.6 (current Maxima is
  5.47+)
- **18% test failure rate** (230 failures out of 1,302 tests) even on
  the "supported" configuration
- **6 open issues**, all from 2018, none resolved
- **GCL support broken** by GCL changes
- **No CI/CD**
- Version numbers never advanced beyond 0.0.1/0.0.3 for any component

### Included packages (25)

Core system, directory functions (chdir/pwd), AEX (array-based
expressions), discrete mathematics, Nelder-Mead optimization, numerical
methods, LAPACK/BLAS wrappers, a store/serialization system, and others.

---

## FFI / native code

mext takes the **same f2cl approach as Maxima itself** for its LAPACK/BLAS
package — Fortran source is translated to Common Lisp via f2cl, not
compiled to native binaries. The `fortran/` directories contain reference
sources but are never compiled as part of normal operation.

The consolidated files `blas_all.lisp` (1.1 MB) and `lapack_all.lisp`
(2.8 MB) require significant memory for compilation (~2 GB dynamic space
on SBCL).

mext does not use CFFI or any other FFI mechanism. All native code goes
through f2cl translation.

---

## Documentation generation / installation

mext includes a custom documentation system called **maxdoc**:

- **Embedded documentation**: package authors annotate functions using
  maxdoc macros in the source code
- **Help integration**: registered documentation is accessible through
  Maxima's `?` and `??` help commands at runtime
- **HTML and PDF generation**: maxdoc can produce HTML and PDF output from
  the embedded annotations
- **Example system**: documentation examples can include Maxima code with
  transparent variable localization (examples don't pollute the user's
  session)

The maxdoc format is a **custom format**, not Texinfo. This means it does
not integrate with Maxima's standard documentation pipeline and cannot be
included in the Maxima reference manual. It is a parallel documentation
system.

---

## Strengths

- **Most complete UX.** `require()`, `mext_info()`, `mext_test()`,
  `mext_list()` — the user-facing API is well-designed and covers the
  essential operations.
- **Well-designed metadata format.** The `.mxt` fields (name, author,
  maintainer, version, license, description) are simple and sufficient.
- **Idempotent loading.** `require()` tracks loaded state and prevents
  redundant loading. This is the right semantics.
- **Version-qualified installation.** Separate directories per
  Maxima-version + Lisp-implementation avoids binary compatibility issues.
- **Symbol protection.** `dont_kill()` / `allow_kill()` addresses a real
  problem in interactive Maxima use.
- **Proven at scale.** 25 packages, 705 commits, real-world testing —
  demonstrates the approach works.

## Weaknesses

- **Abandoned.** No activity since 2018. Likely broken on current Maxima.
- **Monorepo architecture.** All 25 packages in one repo. No mechanism
  for independent distribution, no remote registry, no way for others to
  publish packages.
- **Modifies Maxima internals.** Patches `alike1`, `msize`, `msize-atom`,
  and `descr1.lisp`. This creates fragility with Maxima upgrades and makes
  mext hard to maintain independently.
- **Obsolete build system.** MK:DEFSYSTEM is from 1989 and has been
  superseded by ASDF. The choice was pragmatic (GCL compatibility) but is
  now a liability.
- **Over-engineered.** The `defmfun1` macro system (38 KB), the custom
  documentation format, the tiered loading system — significant complexity
  that raises the barrier to contribution and maintenance.
- **No remote download/install.** Users must clone the repo and run the
  build script. There's no `install("pkgname")` from the Maxima prompt.
- **Custom doc format.** maxdoc is not Texinfo, so it doesn't integrate
  with Maxima's standard documentation and can't be included in the
  reference manual.
