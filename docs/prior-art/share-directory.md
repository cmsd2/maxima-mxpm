# Maxima share/ directory

**Location:** `share/` in the Maxima source tree; installed alongside
Maxima binaries
**Status:** Active. Ships with every Maxima release.
**Maintained by:** Maxima core team (Raymond Toy, Robert Dodier, others)

---

## What it is

The official mechanism for distributing packages with Maxima. The `share/`
directory is a curated collection of ~99 package directories (65 in
`share/` proper + ~34 in `share/contrib/`) that ship with every Maxima
installation. It is not a package manager — it's a directory of code
bundled into the Maxima distribution.

---

## How it works

### Loading packages

```maxima
load("draw");           /* loads share/draw/draw.lisp */
load("linearalgebra");  /* loads share/linearalgebra/linearalgebra.mac */
load("absimp");         /* loads share/simplification/absimp.mac */
```

`load(filename)` searches the `$file_search_maxima` and
`$file_search_lisp` path lists. The search uses a `###` wildcard pattern:

- `###` expands to the requested filename (base name without extension)
- `{mac,mc}` expands to multiple extensions
- `**` matches directories recursively

Example path pattern:
`/usr/share/maxima/5.49.0/share/**/###.mac`

### Key Maxima 5.47+ improvement (2023)

Raymond Toy changed `$file_search_maxima` and `$file_search_lisp` to be
**computed at runtime** by scanning the `share/` directory and
`~/.maxima/`. Subdirectories of `~/.maxima/` are now automatically
included. This means `git clone` into `~/.maxima/` often "just works"
without manual path configuration — a significant simplification for
third-party package installation.

### Autoloading

Some packages are registered for autoloading in `src/max_ext.lisp`:

```lisp
($auto_mexpr '$draw "draw")
($auto_mexpr '$draw2d "draw")
($auto_mexpr '$legendre_p "orthopoly")
($auto_mexpr '$cholesky "linearalgebra")
```

When a user calls `draw2d(...)` without first calling `load("draw")`,
the autoload mechanism transparently loads the package.

Users can set up their own autoloads:

```maxima
setup_autoload("mypackage.mac", myfunc1, myfunc2);
```

### Search path variables

| Variable | Purpose |
|----------|---------|
| `$file_search_maxima` | Paths for `.mac` files (searched by `load()`) |
| `$file_search_lisp` | Paths for `.lisp` files (searched by `load()`) |
| `$file_search_demo` | Paths for demo files (searched by `demo()`) |
| `$file_search_usage` | Paths for `.usg` files (searched by `?`/`??`) |
| `$file_search_tests` | Paths for test files |

Default search order:
1. `~/.maxima/**` (user directory, recursive)
2. `<install>/share/**` (share directory, recursive)
3. `<install>/src/` (source directory)

### Testing

Share packages register tests in `share_testsuite_files`. Run via:

```maxima
run_testsuite(share_tests=true);
```

Known failures can be annotated: `["rtest14", 57, 63]` means tests 57
and 63 are expected to fail.

---

## Package structure

There is **no strict standard**. Packages range from a single `.mac` file
to complex multi-file systems. Common file types:

| File type | Purpose |
|-----------|---------|
| `*.mac` | Maxima source (main entry point) |
| `*.lisp` | Common Lisp source |
| `*.dem` | Demo files |
| `*.texi` | Texinfo documentation |
| `*.usg` | Usage/help documentation |
| `rtest_*.mac` | Regression tests |
| `*.system` | MK:DEFSYSTEM build definitions |
| `load-*.lisp` | Loader files for multi-file packages |
| `Makefile.am` | Autotools build integration |

### Examples of varying complexity

**Simple** — `share/raddenest/`: a few `.mac` files, a `.texi`, a test.

**Medium** — `share/linearalgebra/`: main `.mac`, several `.lisp` files,
a loader, usage docs, demos, multiple test files.

**Complex** — `share/lapack/`: Fortran source (reference), f2cl-translated
Lisp, MK:DEFSYSTEM files, interface Lisp, Maxima wrappers, package
definitions, tests.

---

## What state it's in

- ~99 package directories across `share/` and `share/contrib/`
- Actively maintained as part of the Maxima release process
- `share/` packages are supported by the core team
- `share/contrib/` packages are author-maintained with a lower acceptance
  bar: "we'll accept pretty much any package" (Macrakis)

### Metadata

There is **no metadata format**. Share packages lack:
- Version numbers (beyond the Maxima release they ship with)
- Author/maintainer declarations (sometimes in comments or READMEs)
- License declarations (project-wide GPL assumed)
- Dependency declarations
- Structured descriptions

### Contribution process

Adding a package to `share/` requires:
1. Creating a development branch
2. Adding files under `share/`
3. Modifying build system files: `Makefile.am`, `init-cl.lisp`,
   `include-maxima.texi.in`, optionally `max_ext.lisp`
4. Running full build and test suite
5. Discussion on the mailing list and merge by a core developer

This is **gatekept** — it requires core developer review, build system
knowledge, and merge access. This is the motivation for third-party
package distribution.

---

## FFI / native code

The share directory contains several packages that wrap Fortran numerical
libraries. Maxima uses **f2cl** (Fortran-to-Common-Lisp), a
source-to-source translator, rather than FFI:

### The f2cl approach

```
Fortran 77 source → f2cl translator → Common Lisp source → CL compiler
```

Packages using f2cl: **LAPACK**, **COLNEW**, **COBYLA**, **MINPACK**,
**ODEPACK**, **HOMPACK**, **LBFGS**, **FFTPACK5**.

Each follows a consistent structure:

```
share/<package>/
  fortran/              — original Fortran source (reference)
  lisp/                 — f2cl-translated Lisp (pre-generated, shipped)
  <package>.system      — mk:defsystem for loading the Lisp
  <package>-lisp.system — mk:defsystem for running f2cl (developer tool)
  <package>-interface.lisp — Maxima-facing API
  <package>.mac         — user-level Maxima functions
  load-<package>.lisp   — entry point
  <package>-package.lisp — CL package definition
```

The Fortran source is **never compiled to machine code** in normal
operation. The pre-translated Lisp files in `lisp/` are shipped with
Maxima. The `fortran/` directory and `*-lisp.system` files are developer
tools for regenerating translations.

### Build system for native code: MK:DEFSYSTEM

The `.system` files use `mk:defsystem` (not ASDF):

```lisp
(mk:defsystem "cobyla"
  :source-pathname (maxima::maxima-load-pathname-directory)
  :binary-pathname (maxima::maxima-objdir "share" "cobyla")
  :source-extension "lisp"
  :depends-on ("cobyla-package")
  :components
  ((:module "lisp"
    :components
    ((:file "cobyla" :depends-on ("cobylb"))
     (:file "cobylb" :depends-on ("trstlp"))
     (:file "trstlp")))))
```

For f2cl translation, a custom `:f2cl-lisp` language is defined that
invokes f2cl as a compiler on `.f` files producing `.lisp` files.

### External programs (alternative to FFI)

Some share packages call external programs instead of using FFI:

- **draw** — communicates with gnuplot via a persistent pipe (`*gnuplot-stream*`)
- **draw (VTK mode)** — calls Python with VTK bindings
- gnuplot pipe implementation has **per-Lisp branches** for SBCL, CCL,
  CMUCL, CLISP, LispWorks, GCL, and ECL

### Why not CFFI?

Maxima avoids CFFI (Common Foreign Function Interface) because:
1. GCL's CFFI backend is broken and unmaintained
2. f2cl produces pure Lisp that works on every CL implementation
3. No C compiler or shared library infrastructure needed at runtime

### Implications for a package system

A package system would need to handle three native code strategies:
1. **f2cl-translated packages** — ship pre-translated Lisp (current model)
2. **CFFI-based packages** — require shared library installation (works on
   SBCL, CCL, ECL, etc. but not GCL)
3. **External program packages** — require the external program to be
   installed (gnuplot, Python, etc.)

---

## Documentation generation / installation

### How share/ packages document themselves

Share packages use **Texinfo** (`.texi`) as the primary documentation
format. Texinfo files are integrated into the Maxima reference manual at
build time:

1. Package author writes `package.texi` using `@deffn`, `@defvr`,
   `@anchor`, `@example` blocks
2. The file is registered in `doc/info/Makefile.am` and
   `doc/info/include-maxima.texi.in`
3. At Maxima build time, `makeinfo` processes all `.texi` files into a
   combined `.info` file
4. Perl script `build_index.pl` generates index mappings (function name →
   byte offset in `.info`)
5. The result is installed alongside Maxima

At runtime, Maxima's `?` and `??` commands look up function names in the
index and display the relevant section of the `.info` file.

### Documentation formats in use

| Format | Purpose | How accessed |
|--------|---------|-------------|
| `.texi` (Texinfo) | Primary structured docs | Built into reference manual |
| `.usg` (usage files) | Inline help | `describe()` / `?` / `??` |
| `.dem` (demo files) | Interactive examples | `demo("package")` |
| `README` | Informal docs | Manual reading |

### The documentation gap

- Texinfo is **not automatically included** — a developer must manually
  add it to the build system
- Many share packages have **no `.texi` file** — their only documentation
  is inline comments or a README
- The `?`/`??` integration requires build-time index generation, which is
  tightly coupled to the Maxima build process
- Third-party packages cannot easily integrate with this system without
  the tooling from PKG-maxima or similar

---

## Strengths

- **Ships with Maxima.** Zero installation effort for users. Every Maxima
  user has access to ~99 packages immediately.
- **Mature and battle-tested.** Decades of use. Tests run as part of
  Maxima's release process.
- **`load()` just works.** The search path mechanism is flexible and
  well-understood.
- **Autoloading.** Frequently-used packages load transparently on first
  use.
- **Maxima 5.47+ auto-scanning.** `~/.maxima/` subdirectories are now
  automatically included in search paths — this is the most impactful
  recent development for third-party packages.

## Weaknesses

- **Gatekept.** Inclusion requires core developer review and build system
  modifications. This is the fundamental motivation for a third-party
  package system.
- **No metadata.** No version numbers, no dependency declarations, no
  structured author/license info.
- **Monolithic distribution.** All packages ship together. No way to
  install, update, or remove individual packages.
- **Build-system coupled.** Adding documentation requires modifying
  multiple build files.
- **Poor discoverability.** No search, no categorization, no descriptions
  beyond what's in the manual.
- **No dependency management.** Packages can't declare dependencies on
  other packages.
