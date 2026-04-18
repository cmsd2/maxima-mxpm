# Quicklisp and Common Lisp Dependencies

How mxpm supports packages that depend on Common Lisp libraries
distributed via Quicklisp. Motivated by the `numerics` package, which
depends on `magicl`.

---

## The problem

A Maxima package that uses CFFI, magicl, or any other Common Lisp library
needs those libraries compiled and loadable in the user's Lisp image. Without
mxpm support, the user must manually:

1. Install SBCL
2. Install Quicklisp
3. Run `(ql:quickload :magicl)` from an SBCL REPL
4. Run `mxpm install numerics`
5. Then `load("numerics")` works

mxpm now handles steps 2-3 automatically.

## Background: what is Quicklisp?

Quicklisp is the de facto package manager for Common Lisp. It provides:

- A curated repository of ~1,500 CL libraries, updated monthly
- Dependency resolution (`(ql:quickload :magicl)` pulls all transitive deps)
- Integration with ASDF (the CL build system)
- Compilation caching (`.fasl` files, compiled once per SBCL version)

It is installed as `~/quicklisp/setup.lisp` and loaded into the Lisp image
at startup. Quicklisp is not part of SBCL itself — it's a separate install.

---

## What was implemented

### Manifest section

One new optional section in `manifest.toml`:

```toml
[lisp]
quicklisp_systems = ["magicl"]
```

### `mxpm setup quicklisp`

A new command that downloads and installs Quicklisp for SBCL:

```
$ mxpm setup quicklisp
Setting up Quicklisp...
  Downloaded quicklisp.lisp
  Install Quicklisp to ~/quicklisp/? [Y/n] y
  Quicklisp installed to ~/quicklisp/
Done.
```

- Detects SBCL in PATH; errors with install instructions if missing
- Skips if Quicklisp is already installed
- Downloads `quicklisp.lisp` from `https://beta.quicklisp.org/quicklisp.lisp`
- Runs `sbcl --load quicklisp.lisp --eval '(quicklisp-quickstart:install)'`
- Respects `--yes` flag

### Install-time Quicklisp integration

When `mxpm install` finds a package with `[lisp].quicklisp_systems`, it
checks for SBCL and Quicklisp and responds accordingly:

**Both SBCL and Quicklisp present** — offers to install CL deps:

```
$ mxpm install numerics
  CL dependencies needed: magicl
  Install via Quicklisp now? [Y/n] y
  Installing CL dependencies (this may take a few minutes on first run)...
  CL dependencies installed.
```

**SBCL present, Quicklisp missing** — suggests `mxpm setup quicklisp`:

```
  This package requires Quicklisp (SBCL).
  To set up Quicklisp:
    mxpm setup quicklisp
```

**SBCL missing** — suggests installing SBCL first:

```
  This package requires SBCL with Quicklisp.
  Install SBCL first:
    macOS:  brew install sbcl
    Linux:  apt install sbcl

  Then run: mxpm setup quicklisp
```

### Interactive reinstall prompt

When installing an already-installed package, mxpm now prompts instead of
erroring:

```
Package 'numerics' is already installed. Reinstall? [y/N]
```

The `--reinstall` flag and `--yes` flag both skip the prompt.

### SBCL heap size configuration

SBCL's default 1GB heap is insufficient for compiling magicl's dependencies.
The heap size is configurable:

- **Config**: `sbcl_dynamic_space_size = 4096` in `config.toml` (MB, default: 4096)
- **Env var**: `MXPM_SBCL_DYNAMIC_SPACE_SIZE=8192`

### Load-time resolution in numerics.mac

The `.mac` entry point also handles Quicklisp at load time as a fallback:

```maxima
/* Bootstrap Quicklisp if available but not loaded */
:lisp (unless (find-package :quicklisp)
        (let ((ql-init (merge-pathnames "quicklisp/setup.lisp"
                                        (user-homedir-pathname))))
          (if (probe-file ql-init)
            (load ql-init)
            (error "Quicklisp not found..."))))

/* Register package's lisp/ dir with ASDF and load via Quicklisp */
:lisp (let ((here (maxima::maxima-load-pathname-directory)))
        (pushnew (merge-pathnames "lisp/" here)
                 asdf:*central-registry* :test #'equal))
:lisp (ql:quickload "numerics/core" :silent t)
```

This means `load("numerics")` works even if the user skipped the install-time
Quicklisp setup — deps are compiled on first load.

---

## Files changed

| File | What |
|------|------|
| `crates/mxpm-core/src/manifest.rs` | `LispInfo` struct, `lisp` field on `Manifest` |
| `src/quicklisp.rs` | `QuicklispSetup::detect()` and `install_systems()` |
| `src/commands/setup.rs` | `mxpm setup quicklisp` implementation |
| `src/commands/install.rs` | Install-time Quicklisp integration, reinstall prompt |
| `src/config.rs` | `sbcl_dynamic_space_size` field and env var |
| `src/errors.rs` | `QuicklispFailed` variant |
| `src/lib.rs` | `quicklisp` module declaration |
| `src/bin/mxpm/cli.rs` | `Setup` command, `yes` flag threading |

## Design principles

1. **mxpm is not a CL package manager.** Quicklisp handles dependency
   resolution, compilation, and caching. mxpm orchestrates it.

2. **Graceful degradation.** Clear messages at each level: no SBCL, no
   Quicklisp, or ready to install. Load-time fallback in the `.mac` file.

3. **SBCL-only is acceptable.** Packages using CFFI/magicl already require
   SBCL.

4. **Shared library deps are out of scope.** BLAS/LAPACK and libduckdb are
   not managed by mxpm. CFFI surfaces clear runtime errors, and BLAS is
   bundled on macOS via Accelerate.framework.

## SBCL reader caveat

SBCL reads all `--eval` arguments before executing any of them. This means
you cannot do:

```
sbcl --eval '(load "setup.lisp")' --eval '(ql:quickload ...)'
```

because `ql:quickload` is read (and the `ql` package looked up) before
`setup.lisp` is loaded. The fix is to use `--load` for setup.lisp:

```
sbcl --load ~/quicklisp/setup.lisp --eval '(ql:quickload ...)'
```

Similarly, `quicklisp-quickstart:install` cannot appear in the same `--eval`
as the `(load "quicklisp.lisp")` that defines it.
