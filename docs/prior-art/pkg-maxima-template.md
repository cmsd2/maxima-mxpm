# PKG-maxima template

**Repository:** https://github.com/QMeqGR/PKG-maxima
**Author:** Eric Majzoub (ehm) with Maxima-side support from Raymond Toy
**Status:** Active but minimal adoption (26 commits; August 2024 –
December 2025)
**License:** GPL v3

---

## What it is

A template for structuring Maxima packages with integrated documentation.
Not a package manager — it standardizes package layout and provides
tooling to generate documentation that integrates with Maxima's `?` and
`??` help system. The newest of the existing efforts.

---

## How it works

### Template structure

```
PKG-maxima/
  PKG.mac                   — main source file
  PKG.lisp                  — optional Lisp code (placeholder)
  LICENSE
  docs/
    PKG.texi                — texinfo documentation source
    PKG.info                 — generated info file
    PKG-index.lisp           — generated: maps names → .info byte offsets
    PKG-index-html.lisp      — generated: maps names → HTML anchors
    PKG_html/                — generated HTML documentation
    PKG.pdf                  — generated PDF documentation
    create_docs.sh           — builds all docs from .texi
    regen_examples.sh        — auto-generates example outputs
    rtest_PKG.mac            — auto-generated test file from examples
```

### The key innovation: help system integration

The `PKG.mac` entry point loads pre-built index files:

```maxima
load("PKG-index.lisp")$
load("PKG-index-html.lisp")$
```

These index files contain Lisp code that registers the package's
documentation with Maxima's internal help tables using
`load-info-hashtables` and `load-html-index`. After `load(PKG)`, typing
`? myfunc` at the Maxima prompt returns the package's documentation for
`myfunc` — the same experience as built-in functions.

This required changes to Maxima core (Raymond Toy's
`rtoy-html-support-external-docs` branch), merged into Maxima 5.48.0+.

### Documentation generation pipeline

**Phase 1: Example regeneration (`regen_examples.sh`, 236 lines)**

Package authors write only *input* commands in their `.texi` file. The
script:

1. Parses `.texi` to find `@example`/`@group` blocks with `(%i` prefixes
2. Extracts input commands into batch files
3. Runs them through `maxima -q --batch` to compute outputs
4. Reassembles the `.texi` with computed outputs inserted
5. Produces `regen.texi` for manual review

**Phase 2: Documentation building (`create_docs.sh`, 119 lines)**

Requires the **Maxima source tree** (not just a binary install) for two
utilities:

- `doc/info/build_index.pl` — Perl script that reads `.info` and generates
  Lisp index with byte offsets
- `doc/info/build-html-index.lisp` — generates HTML index mappings

The script runs:

1. `makeinfo PKG.texi` → `.info`
2. `makeinfo --pdf PKG.texi` → PDF
3. `makeinfo --split=chapter --no-node-files --html PKG.texi` → HTML
4. `makeinfo --plaintext PKG.texi` → `README.txt`
5. `build_index.pl PKG.info` → `PKG-index.lisp`
6. Maxima + `build-html-index.lisp` → `PKG-index-html.lisp`
7. Extracts examples from `.texi` → `rtest_example_PKG.mac`

---

## What state it's in

- **26 commits**, all by Eric Majzoub
- **1 star, 0 forks** on GitHub
- **One known user**: the author's own `qm-maxima` quantum mechanics
  package (https://github.com/QMeqGR/qm-maxima)
- Windows and macOS instructions marked **"THIS SECTION IS UNFINISHED"**
- Generated HTML index contains **hardcoded absolute paths** — needs
  regeneration per machine
- No CI, no Makefile, no version management

---

## Dependencies

- **Maxima 5.48.0+** (for HTML doc support)
- **Maxima source tree** (for `build_index.pl` and
  `build-html-index.lisp`)
- **Perl** (for `build_index.pl`)
- **GNU Texinfo / makeinfo**
- **TeX/LaTeX** (for PDF generation)
- **Bash, AWK** (shell scripts)
- Unix/Linux environment (or Cygwin/WSL on Windows)

---

## FFI / native code

Not addressed. The template provides a `PKG.lisp` placeholder file for
optional Lisp code, but there is no support for native code compilation,
FFI bindings, or external library dependencies.

---

## Documentation generation / installation

This is PKG-maxima's **primary focus** and its strongest contribution.

### What it provides

- **Texinfo-based documentation** that follows Maxima's own internal doc
  format (`@deffn`, `@defvr`, `@anchor`, `@example`/`@group` blocks)
- **Auto-generated example outputs**: authors write inputs only; the
  `regen_examples.sh` script computes all outputs by running them through
  Maxima
- **Auto-generated test suites**: test files are extracted from the
  documentation examples, ensuring docs and code stay in sync
- **Multi-format output**: `.info` (for terminal help), HTML (for web/GUI
  help), PDF (for print), plain text (for README)
- **Native help integration**: after `load(PKG)`, Maxima's `?` and `??`
  commands find the package's documentation alongside built-in docs

### What it doesn't provide

- No mechanism to install documentation — the user must manually place
  files where Maxima can find them
- No documentation hosting or web catalog
- The toolchain requires the Maxima source tree, which most users don't
  have
- PDF generation requires a full TeX installation

### Documentation installation for end users

The template assumes the end user receives pre-built documentation files
(`.info`, `-index.lisp`, `-index-html.lisp`, HTML directory) alongside
the package source. When `load(PKG)` is called, the index files are loaded
and documentation becomes available. No special installation step is
needed beyond placing the files on the Maxima search path.

This is a good design — the build complexity is borne by the package
author, not the end user.

---

## Strengths

- **Native `?`/`??` integration.** The generated index files make
  third-party package documentation appear alongside built-in docs. This
  is the best documentation UX of any existing effort.
- **Auto-generated examples and tests.** Writing inputs only and having
  outputs computed automatically is genuinely clever. Tests extracted from
  docs ensure documentation accuracy.
- **Texinfo compatibility.** Uses the same documentation format as Maxima
  itself, enabling natural integration.
- **Author-side complexity, user-side simplicity.** End users just
  `load(PKG)` and get documentation. The build toolchain is only needed
  by package authors.
- **Newest effort.** Actively maintained (as of December 2025), designed
  for current Maxima (5.48+).

## Weaknesses

- **Heavy tooling requirements.** The Maxima source tree, Perl, Texinfo,
  TeX, Bash, AWK — this is a dealbreaker for most potential package
  authors.
- **Not a package manager.** Provides no distribution, installation,
  discovery, or dependency management. Only addresses the "structure and
  document your package" problem.
- **Near-zero adoption.** Only one package (the author's own) uses it.
- **Unix-only in practice.** Windows/macOS support is explicitly
  unfinished.
- **Fragile scripts.** Shell scripts with AWK parsing of Texinfo are
  brittle and hard to extend.
- **Hardcoded paths.** Generated HTML index files contain absolute
  filesystem paths.
