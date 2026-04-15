# maxima-packages repository

**Repository:** https://github.com/maxima-project-on-github/maxima-packages
**Author:** Robert Dodier (maintainer), community contributions
**Status:** Experimental. Last updated August 2025.
**License:** Per-package

---

## What it is

A GitHub repository serving as a community package registry. It holds
third-party Maxima packages organized by contributor, with a pull-request
contribution model. It is the closest thing to a centralized package index
that exists, but it stores the actual package code (monorepo) rather than
pointing to external repositories.

The README explicitly states: "there is not yet an automatic mechanism to
download and install these packages."

---

## How it works

### Structure

```
maxima-packages/
  robert-dodier/
    package_a/
      package_a.mac
      package_a.asd       (some packages)
      rtest_package_a.mac  (some packages)
      README.md            (some packages)
    package_b/
      ...
  sdemarre/
    diophantine/
      diophantine.mac
      diophantine.asd
      diophantine.texi
      diophantine.info
      diophantine-index.lisp
      rtest_diophantine.mac
    diophantine_system/
      ...
  yitzchak/
    texify/
      texify.asd
      CHANGELOG.md
      LICENSE.md
```

Packages are grouped by author username, then by package name.

### Contribution model

Contributors fork the repo, add their package in a directory under their
username, and submit a pull request. Per the README: "Project
administrators will accept almost any contribution made in good faith."
Packages are not reviewed for correctness or security.

### Package contents

The repo also includes a `MYPACKAGE` template showing the recommended
structure:

- `MYPACKAGE.mac` — main source file
- `MYPACKAGE.asd` — ASDF system definition (optional but recommended)
- `MYPACKAGE.texi` — texinfo documentation (optional)
- `rtest_MYPACKAGE.mac` — regression tests (optional)

### Current contents (~23 packages from 3 contributors)

**robert-dodier** (~20 packages): mostly small utilities. A few have
`.asd` files and `rtest_*.mac` tests. Examples: `low_discrepancy`,
`superq`, `with_gensyms`, `operpart`.

**sdemarre** (2 packages): diophantine, diophantine_system. These are the
best-packaged examples — each has `.asd`, `.texi`, `.info`,
`-index.lisp`, and tests.

**yitzchak** (1 package): texify. Has `.asd`, `CHANGELOG.md`, `LICENSE.md`.

---

## What state it's in

- Active but sparsely populated — 3 contributors over ~7 years
- No CI, no automated testing of contributed packages
- No download/install mechanism
- The repo itself predates and is separate from the individual packages'
  own GitHub repositories (where they exist)

---

## FFI / native code

Not addressed. All packages in the repository are pure Maxima (`.mac`)
and/or Common Lisp (`.lisp`). There is no support for or examples of
packages with native code dependencies.

---

## Documentation generation / installation

The repository includes a `MYPACKAGE.texi` template that demonstrates
the Texinfo documentation format. Two of sdemarre's packages include
pre-built documentation artifacts:

- `diophantine.texi` — texinfo source
- `diophantine.info` — pre-built info file
- `diophantine-index.lisp` — pre-built index for `?`/`??` integration

These are the same artifacts that the PKG-maxima template generates, but
here they were produced manually (or with an earlier version of similar
tooling). When loaded alongside the package, they integrate with Maxima's
help system.

There is no documentation generation tooling in the repository itself —
authors are expected to produce these files with their own toolchain.

---

## Strengths

- **Exists and is maintained.** It's the only community package collection
  that is still active.
- **Low barrier to contribution.** Fork, add files, submit PR. No special
  tooling required.
- **sdemarre's packages as exemplars.** The diophantine packages
  demonstrate what a well-structured Maxima package looks like: `.asd`,
  `.texi`, `.info`, `-index.lisp`, tests.
- **Pull-request model works.** Demonstrates that community contribution
  via GitHub PRs is viable for this community.

## Weaknesses

- **Monorepo, not an index.** Stores actual package code rather than
  pointing to external repos. Users must clone the entire repository to
  get one package.
- **No install mechanism.** "There is not yet an automatic mechanism to
  download and install these packages."
- **Sparse.** ~23 packages from 3 contributors in ~7 years.
- **Inconsistent structure.** Packages range from a single `.mac` file to
  fully documented packages with `.asd`, `.texi`, and tests. No
  enforcement of minimum standards.
- **No metadata index.** There's no machine-readable catalog of what's in
  the repo — you have to browse the directory structure.
- **No dependency tracking.** Packages can't declare dependencies on each
  other.
