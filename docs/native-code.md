# Native Code and Build Steps

Research into how Maxima handles native code, whether third-party packages
need build steps, and what mxpm should (or shouldn't) do about it.

## How Maxima handles native code today

### f2cl: Fortran-to-Common-Lisp translation

Maxima does **not** use a traditional FFI for its numerical packages.
Instead, it uses **f2cl**, a source-to-source translator that converts
Fortran 77 into Common Lisp:

```
Fortran 77 source --> f2cl --> Common Lisp source --> CL compiler --> FASL
```

The translated Lisp files are pre-generated and shipped with Maxima. At
load time, the Lisp compiler produces platform-appropriate FASL (Fast
Load) files. No C or Fortran compiler is needed on the user's machine.

Packages using this approach (all bundled in `share/`):

| Package | What it wraps |
|---------|---------------|
| LAPACK | Linear algebra (eigenvalues, SVD, etc.) |
| COLNEW | Boundary value problems for ODEs |
| COBYLA | Constrained optimization |
| MINPACK | Nonlinear least-squares fitting |
| ODEPACK | Stiff/non-stiff ODE integration |
| HOMPACK | Polynomial systems via homotopy continuation |
| LBFGS | Limited-memory BFGS optimization |
| FFTPACK5 | Fast Fourier transforms |

### CFFI: discussed but not adopted

CFFI (Common Foreign Function Interface) is the standard CL library for
calling C shared libraries. It has been discussed on the Maxima mailing
list since 2018 but never adopted. Key blockers:

- **GCL incompatibility** — GCL's CFFI backend is broken/unmaintained
- **No existing infrastructure** in Maxima for loading shared libraries
- **Cross-implementation complexity** — each Lisp handles memory and GC
  differently when interfacing with C
- **f2cl already works** for the packages that need numerical Fortran

Raymond Toy (core developer, 2022): "CFFI is the 'standard' library for
lisps to interface to foreign functions. I think it works for all the
lisps we support except, perhaps, gcl."

### External program calls

Some packages call external programs via pipes. The most notable is
`draw`, which communicates with gnuplot. This approach has per-Lisp
branches (SBCL, CCL, CMUCL, CLISP, LispWorks, GCL, ECL) for the pipe
implementation.

### Lisp implementations Maxima supports

SBCL (most common), GCL, CCL (Clozure), ECL, CLISP, CMUCL, ABCL
(JVM-based), LispWorks (commercial). Each has different FFI capabilities.
CFFI abstracts over all except GCL.

## Current demand: zero

- **0 of 12** indexed packages require native compilation
- **0 third-party** packages in the wild require it
- All 8 f2cl packages ship with Maxima itself, not as third-party packages
- No third-party CFFI-based Maxima package has ever been published
- The only external-program dependency in the ecosystem is **gnuplot**

The Maxima third-party ecosystem is tiny. Even in ecosystems with hundreds
of thousands of packages (PyPI, npm), the fraction needing native
compilation is 5-10%. In a <100 package ecosystem, the expected count is
effectively zero.

## Options

### Option A: Document only

No build support in mxpm. A `[build]` section in `manifest.toml` triggers
a warning: "This package requires a build step. See README."

- **Enables:** Packages can exist in the index even if they need compilation
- **Doesn't cover:** Automation, error handling, platform detection
- **Complexity:** ~20 lines
- **Who benefits:** Nobody directly, but doesn't block native packages

### Option B: Simple build command

The manifest declares a command to run after extraction. mxpm checks that
required tools are on PATH, runs the command, reports success/failure.

```toml
[build]
command = "make"
requires = ["make", "cc"]
```

- **Enables:** One-command install for packages with Makefiles
- **Doesn't cover:** Platform detection, prebuilt binaries, Windows
- **Complexity:** ~130 lines (manifest parsing + PATH check + exec)
- **Who benefits:** Package authors who ship build scripts

### Option C: Prebuilt binaries + native dependency warnings

Option B plus platform detection and prebuilt binary support:

```toml
[build]
command = "make"
requires = ["make", "cc"]

[build.prebuilt]
platforms = ["x86_64-linux", "x86_64-darwin", "aarch64-darwin"]

[native]
external-programs = ["gnuplot"]
shared-libraries = ["liblapack.so"]
```

- **Enables:** Skip build on supported platforms; runtime dependency warnings
- **Doesn't cover:** Binary hosting, Windows native builds
- **Complexity:** ~450 lines total
- **Who benefits:** Users on platforms with prebuilts; packages with gnuplot deps

### Option D: Full build support with hosted binaries

Option C plus CI recipes for building platform-specific artifacts,
downloading prebuilts from GitHub Releases, and index schema extensions.

- **Enables:** Fully automated cross-platform native packages
- **Doesn't cover:** Build environment isolation, toolchain installation
- **Complexity:** ~820 lines total + CI templates
- **Who benefits:** A hypothetical future with many native-code packages

## Cross-platform challenges

| Platform | Compiler | Fortran | Feasibility |
|----------|----------|---------|-------------|
| Linux | gcc/g++ (usually installed) | gfortran (often not) | Most straightforward |
| macOS | clang (Xcode CLI Tools) | gfortran (Homebrew) | Feasible with setup |
| Windows | No default compiler | No default compiler | Not feasible without significant user setup |

Windows is the hard case. Maxima's own Windows installer uses
cross-compilation from Linux. Running `make` on Windows requires MSYS2,
MinGW, or WSL — tools most Maxima users don't have. Pre-built binaries
are the only practical path for Windows.

The Lisp implementation adds another variable: CFFI-based packages produce
platform-specific `.so`/`.dylib`/`.dll` files that must match the user's
OS, architecture, *and* Lisp implementation. f2cl sidesteps all of this
because translated Lisp is platform-independent.

## How other package managers do it

| Manager | Approach | Key lesson |
|---------|----------|------------|
| **Cargo** (Rust) | `build.rs` compiled and run before the package | Convention over configuration |
| **pip** (Python) | `pyproject.toml` build backend; **wheels** (prebuilt binaries) dominate | Pre-built binaries are the primary path |
| **npm** (Node) | `scripts.install` runs any command; `node-pre-gyp` downloads prebuilts | Simple command hook + prebuilt fallback |
| **Homebrew** | Ruby formulas with `depends_on`; **bottles** (prebuilt) are default | Build/runtime dependency distinction |

Common pattern: pre-built binaries are the primary distribution path.
Build-from-source is a fallback. In all four ecosystems, the fraction of
packages needing compilation is small and shrinking.

## Recommendation

**Defer Phase 6** until a concrete package needs it. There is no current
demand, and all higher-priority work (Phase 3 `mxpm check`, Phase 4
dependency resolution, Phase 8 catalog website, Phase 9 Maxima-side
integration) serves the existing and near-future package ecosystem.

### If implementing incrementally

1. **Now (standalone, useful today):** Parse `[native]` section for
   `external-programs` warnings at install time. Packages depending on
   gnuplot benefit immediately. ~50 lines.

2. **When first needed:** Option B — `[build]` with `command` and
   `requires`. ~130 lines of straightforward code on top of existing
   install logic.

3. **When packages ship binaries:** `[build.prebuilt]` platform
   detection. Only implement when a real package needs it.

4. **Defer indefinitely:** Hosted prebuilt binaries, shared library
   detection, CI scaffolding. Solutions looking for a problem that does
   not exist.

The manifest schema in the technical requirements already accounts for
`[build]`, `[build.prebuilt]`, and `[native]`. The design is done;
implementation can wait for demand.
