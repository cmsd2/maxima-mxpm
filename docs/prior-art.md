# Prior Art: Comparison and Assessment

A comparative analysis of every significant attempt at Maxima package
management: maxima-asdf, mext, PKG-maxima template, the maxima-packages
repository, and Maxima's built-in share/ directory. Individual deep-dives
are in the [prior-art/](prior-art/) subfolder.

---

## Technology comparison matrix

### Core capabilities

| Capability | maxima-asdf | mext | PKG-maxima | maxima-packages | share/ |
|-----------|:-----------:|:----:|:----------:|:---------------:|:------:|
| Install packages | Partial | Yes | No | No | Bundled |
| Remove packages | No | No | No | No | No |
| Update packages | No | No | No | No | Bundled |
| Load by name | Yes | Yes | Manual | Manual | Yes |
| Search/discover | No | No | No | No | No |
| Dependency resolution | CL-level only | Basic | No | No | No |
| Version management | No | No | No | No | No |
| Package metadata | `.asd` | `.mxt` | None | `.asd` (some) | None |
| List installed | No | Yes | No | No | No |
| Run tests | No | Yes | Generated | No | Via testsuite |

### Documentation

| Capability | maxima-asdf | mext | PKG-maxima | maxima-packages | share/ |
|-----------|:-----------:|:----:|:----------:|:---------------:|:------:|
| Doc format | None | Custom (maxdoc) | Texinfo | Texinfo (some) | Texinfo |
| `?`/`??` help integration | Path only | Yes (custom) | Yes (native) | Pre-built (some) | Yes |
| Doc generation tooling | No | Yes (custom) | Yes (scripts) | No | Build-time |
| Auto-generate examples | No | No | Yes | No | No |
| Auto-generate tests from docs | No | No | Yes | No | No |
| HTML docs | No | Yes | Yes | No | Yes |
| PDF docs | No | Yes | Yes | No | Yes |

### Native code / FFI

| Capability | maxima-asdf | mext | PKG-maxima | maxima-packages | share/ |
|-----------|:-----------:|:----:|:----------:|:---------------:|:------:|
| f2cl (Fortran→Lisp) | No | Yes (bundled) | No | No | Yes |
| CFFI support | Possible via ASDF | No | No | No | No |
| External program calls | No | No | No | No | Yes (gnuplot) |
| Build system for native | ASDF (CL) | MK:DEFSYSTEM | None | None | MK:DEFSYSTEM |
| Native code examples | None | LAPACK/BLAS | None | None | 8 packages |

### Platform and compatibility

| Requirement | maxima-asdf | mext | PKG-maxima | maxima-packages | share/ |
|------------|:-----------:|:----:|:----------:|:---------------:|:------:|
| Linux | Yes | Yes | Yes | Yes | Yes |
| macOS | Yes | Untested | Unfinished | Yes | Yes |
| Windows | Unlikely | Partial | Unfinished | Yes | Yes |
| Works with GCL | **No** | Broken | N/A | N/A | Yes |
| Works with SBCL | Yes | Yes | Yes | N/A | Yes |
| Works with standalone binary | **No** | Yes | N/A | N/A | Yes |
| Minimum Maxima version | Any | 5.41 | 5.48 | Any | N/A |
| External tools required | Quicklisp, ASDF, drakma | Bash | Perl, TeX, Bash, AWK, Maxima src | Git | None |

### Adoption and maturity

| Metric | maxima-asdf | mext | PKG-maxima | maxima-packages | share/ |
|--------|:-----------:|:----:|:----------:|:---------------:|:------:|
| First commit | Jan 2016 | Nov 2012 | Aug 2024 | ~2018 | Decades |
| Last activity | Jan 2024 | Sep 2018 | Dec 2025 | Aug 2025 | Ongoing |
| Total commits | 30 | 705 | 26 | Moderate | N/A |
| Contributors | 3 | 1 | 1 | 3 | Many |
| Packages using it | ~4 | 25 (bundled) | 1 | ~23 | ~99 |
| GitHub stars | 12 | 3 | 1 | — | N/A |
| Tests | None | Yes (18% fail) | Generated | Per-package | Yes |
| CI/CD | No | No | No | No | Yes |
| Status | Stalled | Abandoned | Active | Experimental | Active |

---

## Maturity assessment

### maxima-asdf — Proof of concept

A clean, minimal proof of concept (172 lines) that demonstrates ASDF
integration. The core abstraction (`:maxima-file` component type) is
sound, but the system is unusable by typical Maxima users because it
requires a full Common Lisp + Quicklisp environment that most
installations don't provide. Effectively dormant since 2018.

**Maturity: 2/5** — Working prototype with a fatal adoption barrier.

### mext — Abandoned prototype

The most feature-complete attempt, with a well-designed user experience
(`require()`, `mext_info()`, `mext_test()`). However, it's a monorepo
that modifies Maxima internals, uses an obsolete build system, and hasn't
been maintained since 2018. Even on its last tested configuration, 18% of
tests fail. The scope grew far beyond package management into a parallel
Maxima distribution.

**Maturity: 3/5** — Substantial engineering, but abandoned and now
incompatible with current Maxima.

### PKG-maxima — Early-stage template

The newest effort, focused specifically on documentation integration
rather than package management. Its approach to `?`/`??` help integration
is the best of any existing effort, and auto-generating tests from
documentation examples is a genuinely good idea. But the heavy tooling
requirements (Maxima source tree, Perl, TeX) and lack of any distribution
mechanism limit its value. Only one package uses it.

**Maturity: 1/5** — Good ideas, heavy dependencies, near-zero adoption.

### maxima-packages repository — Experimental index

A GitHub repo containing community-contributed packages. Demonstrates
that the pull-request contribution model works, but it's a monorepo of
package code (not an index pointing to external repos) and has no install
mechanism. sdemarre's packages serve as useful exemplars of good package
structure.

**Maturity: 2/5** — Useful as a proof of concept for community
contribution, not as a distribution mechanism.

### share/ directory — Production baseline

The only system in production use, shipping with every Maxima install.
Mature and battle-tested with ~99 packages. The Maxima 5.47+ auto-scanning
of `~/.maxima/` subdirectories was a major quality-of-life improvement
for third-party packages. However, it has no metadata, no dependency
management, and inclusion is gatekept by core developers.

**Maturity: 4/5** — Production-grade for bundled packages, but doesn't
solve the third-party distribution problem.

---

## Requirements coverage

Cross-referencing against the [requirements document](requirements.md):

| Requirement | maxima-asdf | mext | PKG-maxima | maxima-packages | share/ |
|------------|:-----------:|:----:|:----------:|:---------------:|:------:|
| §1.1 Third-party distribution | Partial | No (monorepo) | No | Partial | No (gatekept) |
| §1.2 Discoverability | No | No | No | No | No |
| §1.3 Low maintenance burden | Yes (minimal) | No (complex) | Low | Low | N/A |
| §1.4 Bootstrap with content | No | Bundled 25 | No | 23 packages | 99 packages |
| §2.1 Decentralized + index | No index | No | No | Monorepo | No |
| §3.1 Short name mapping | Via ASDF | Via `.mxt` | No | No | Via `load()` |
| §3.2 Documentation | Path only | Custom format | Texinfo+index | Some | Texinfo |
| §3.3 Dependencies | CL-level | Basic | No | No | No |
| §3.4 Metadata file | `.asd` | `.mxt` | No | `.asd` (some) | No |
| §4.1 Search/browse | No | No | No | No | No |
| §4.2 Install by name | Partial | Yes | No | No | Yes (bundled) |
| §4.3 Load installed | Yes | Yes | Manual | Manual | Yes |
| §4.4 List installed | No | Yes | No | No | No |
| §4.5 Update/remove | No | No | No | No | No |
| §4.6 Run tests | No | Yes | Generated | No | Yes |
| §5.1 Minimal overhead | `.asd` needed | `.mxt`+`.system` | Heavy tooling | Low | High (build sys) |
| §6.1 Cross-platform | Partial | Partial | Linux only | Yes | Yes |
| §6.2 All distributions | No (needs CL) | Partial | Needs src | Yes | Yes |
| §6.3 No external tools | No (Quicklisp) | Bash | Many tools | Git | None |

**No existing system scores above 50% on the requirements.** The biggest
universal gap is discoverability (§1.2) — none of the efforts address it
at all.

---

## What to build on

### Foundations worth adopting

1. **`~/.maxima/` auto-scanning (Maxima 5.47+).** The most impactful
   existing development. "Install" = "place files in `~/.maxima/pkgname/`"
   and `load("pkgname")` just works. Any new system should use this as
   the installation target.

2. **`rtest_*.mac` test convention.** ~60% adoption among existing
   packages, matches Maxima's own convention. Use as the standard.

3. **Texinfo + `-index.lisp` for documentation.** The only approach that
   integrates with `?`/`??`. Worth standardizing, but the tooling needs
   radical simplification.

4. **Pull-request contribution model.** maxima-packages demonstrates
   this works for the Maxima community.

### Ideas worth adopting (from different implementations)

5. **mext's metadata fields.** name, author, version, license, description
   — simple and sufficient. Use a similar schema.

6. **mext's user-facing API.** `require()`, `mext_info()`, `mext_test()`,
   `mext_list()` — well-designed operations.

7. **maxima-asdf's auto path updates.** After loading, search paths are
   updated automatically.

8. **PKG-maxima's auto-generated tests from docs.** Tests extracted from
   documentation examples ensure docs and code stay in sync.

### Anti-patterns to avoid

9. **Requiring CL toolchain** (maxima-asdf) — excludes most users.
10. **Requiring Maxima source tree** (PKG-maxima) — excludes most users.
11. **Modifying Maxima internals** (mext) — fragile version coupling.
12. **Monorepo distribution** (mext, maxima-packages) — doesn't scale.
13. **Over-engineering** (mext's 705 commits) — unsustainable.
14. **Custom doc format** (mext's maxdoc) — use Texinfo for compatibility.

---

## Native code and FFI summary

Maxima's ecosystem uses three strategies for native code, each with
different implications for a package system:

### Strategy 1: f2cl translation (dominant)

Fortran 77 → Common Lisp source → CL compiler. Used by 8 share/ packages
(LAPACK, COLNEW, COBYLA, MINPACK, ODEPACK, HOMPACK, LBFGS, FFTPACK5) and
by mext. The translated Lisp is pre-generated and shipped; the Fortran
source is reference-only.

- **Pro:** Pure Lisp output, works on all CL implementations including
  GCL, no native compiler needed at runtime
- **Con:** Fortran 77 only, large memory requirements (SBCL needs 2 GB+
  for LAPACK), f2cl has known limitations

### Strategy 2: CFFI (unused by Maxima, common in CL ecosystem)

Common Foreign Function Interface — portable library for calling C shared
libraries. Supported by SBCL, CCL, ECL, CMUCL, CLISP, ABCL.
**Not supported by GCL** (broken backend).

- **Pro:** Native performance, C/C++ library access, mature ecosystem
- **Con:** Requires shared library installation, broken on GCL,
  platform-specific library paths

### Strategy 3: External programs (used by draw/ and others)

Call external programs via pipes or subprocesses. Maxima's gnuplot
integration has per-Lisp-implementation branches (SBCL, CCL, CMUCL,
CLISP, LispWorks, GCL, ECL). UIOP's `run-program` provides a modern
portable abstraction.

- **Pro:** Language-agnostic, simple isolation
- **Con:** Serialization overhead, per-implementation code needed
  (without UIOP)

### Build systems for native code

| Build system | Used by | Status | Notes |
|-------------|---------|--------|-------|
| MK:DEFSYSTEM | share/ (LAPACK etc.), mext | Active (share/), abandoned (mext) | Pre-ASDF, from 1989. Custom `:f2cl-lisp` language. |
| ASDF | maxima-asdf, some third-party pkgs | Active | Modern CL standard. Can define custom component types. |
| Meson | hep-units (one package) | Niche | Linux distro packaging oriented. |
| None (manual) | Most third-party packages | Common | No build system; just `.mac` files. |

### Implications for a package system

A package system should:
- Support **pure Maxima/Lisp packages** (the common case) with zero build
  complexity
- Allow packages to **declare native dependencies** (shared libraries,
  external programs) in metadata without requiring them
- Support **pre-built artifacts** (f2cl-translated Lisp, pre-compiled
  docs) so end users don't need build tools
- Defer complex build orchestration to the package author's own toolchain
  rather than trying to replicate it

---

## Documentation generation and installation summary

### The documentation pipeline problem

Maxima uses Texinfo for documentation, which integrates with the `?`/`??`
help system via index files. Making third-party package docs appear in
this system requires:

1. **Authoring:** Write `.texi` source with `@deffn`, `@defvr`, `@anchor`
2. **Building:** Run `makeinfo` → `.info`; run `build_index.pl` →
   `-index.lisp`; optionally run `makeinfo --html` → HTML
3. **Shipping:** Include pre-built `.info` and `-index.lisp` in the
   package
4. **Loading:** Package entry point loads `-index.lisp`, registering docs
   with Maxima's help tables

### How each system handles this

| Aspect | maxima-asdf | mext | PKG-maxima | share/ |
|--------|:-----------:|:----:|:----------:|:------:|
| Author format | N/A | Custom macros | Texinfo | Texinfo |
| Build tooling | None | Built-in | Shell scripts | Maxima build system |
| Requires Maxima src | No | No | **Yes** | N/A (is the src) |
| Requires TeX | No | No | Yes (for PDF) | Yes (for PDF) |
| Requires Perl | No | No | Yes | Yes |
| End-user experience | Path discovery only | `?` works (custom) | `?` works (native) | `?` works |
| Ships pre-built docs | N/A | Yes | Yes | Yes |
| Auto-gen examples | No | No | **Yes** | No |
| Auto-gen tests from docs | No | No | **Yes** | No |

### The key insight

PKG-maxima's approach of **shipping pre-built documentation** is correct:
the build complexity is borne by the package author, not the end user.
When `load(PKG)` runs, it loads the pre-built `-index.lisp` and
documentation becomes available instantly.

The problem is the build toolchain: requiring the Maxima source tree is
untenable. The two critical utilities (`build_index.pl` and
`build-html-index.lisp`) should either be:
- Extracted from the Maxima source and distributed independently, or
- Reimplemented as a standalone tool (they parse `.info` files and
  generate Lisp index tables — not inherently complex)

### Documentation gap summary

- **Authoring:** Texinfo is the right format but has a learning curve.
  A simpler authoring format (Markdown?) with Texinfo generation would
  lower the barrier.
- **Building:** The toolchain is too heavy. Needs simplification.
- **Distribution:** Pre-built docs should be the norm. Package authors
  build; users consume.
- **Discovery:** No system provides doc-based search or browsing across
  packages. This is part of the broader discoverability gap.

---

## Individual system deep-dives

- [maxima-asdf](prior-art/maxima-asdf.md)
- [mext](prior-art/mext.md)
- [PKG-maxima template](prior-art/pkg-maxima-template.md)
- [maxima-packages repository](prior-art/maxima-packages-repo.md)
- [Maxima share/ directory](prior-art/share-directory.md)
