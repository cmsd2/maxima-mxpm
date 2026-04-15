# Maxima Package System: Requirements

This document captures requirements for a Maxima package management system,
drawn from mailing list discussions (2016–2025). Each requirement is marked
with its provenance:

- **[DISCUSSED]** — Explicitly raised and broadly agreed upon in the mailing
  list discussions.
- **[PROPOSED]** — Raised by one or more contributors but not broadly
  discussed or agreed upon.
- **[INFERRED]** — Not explicitly discussed. Filled in by the document author
  based on gaps in the discussions and by analogy with comparable systems.

See [mailing-list-discussions.md](mailing-list-discussions.md) for the full
source material.

---

## 1. Goals

### 1.1 Enable third-party package distribution [DISCUSSED]

Package authors must be able to publish and distribute Maxima packages without
requiring intervention from the core Maxima developers or inclusion in the
official `share/` directory.

> *"building an ecosystem around Maxima which allows interested parties to
> develop and release their own packages without requiring intervention from
> the official developers"* — Robert Dodier, 2016

### 1.2 Improve discoverability [DISCUSSED]

Users must be able to find packages relevant to their problem. This is the
self-identified "big question" from the community — even the existing `share/`
packages are hard to discover.

> *"The big question I have about outside repositories is discoverability and
> documentation."* — Stavros Macrakis, 2022

### 1.3 Minimize maintenance burden [DISCUSSED]

The system must be sustainable with minimal ongoing human maintenance. The
Maxima community is small and volunteer-driven; previous efforts (mext,
maxima-asdf) stalled partly because they required sustained effort from their
authors.

> *"I don't think we want a central package repository, because that requires
> someone to maintain it."* — Robert Dodier, 2024

### 1.4 Bootstrap from existing packages [INFERRED]

To break the chicken-and-egg problem (Toy, 2025), the system should launch
with content already in it. This means indexing known GitHub packages and
possibly the existing `share/` directory from day one, rather than starting
from an empty catalog.

---

## 2. Architecture

### 2.1 Decentralized hosting, centralized index [DISCUSSED]

Packages are hosted in their own repositories (primarily GitHub). The system
provides a lightweight index/catalog that maps package names to their source
locations. There is no central artifact store.

This model was consistently preferred across multiple discussions (Dodier
2016/2024, Königsmann 2017, Macrakis 2022).

### 2.2 Index as data, not infrastructure [INFERRED]

The index itself should be a static data file (e.g. a JSON or YAML file in a
Git repository), not a running service. This keeps maintenance costs near zero
and allows the index to be mirrored, forked, or contributed to via pull
requests.

### 2.3 No dependency on Maxima internals [INFERRED]

The package system should work alongside Maxima without requiring patches to
the Maxima source code. While deeper integration (e.g. extending `load()` to
resolve package names) is desirable long-term, it must not be a prerequisite
for v1. This avoids coupling the package system's release cycle to Maxima's.

---

## 3. Package metadata

### 3.1 Short name mapping [DISCUSSED]

Each package has a short, unique name that can be used to refer to it without
knowing its URL.

> *"the package management system has some sort of directory to map short
> package names to URLs"* — Michael Soegtrop, 2017

### 3.2 Documentation [DISCUSSED]

Packages must include or link to documentation. The system should make
documentation accessible — not just list package names.

> *"the package management system includes documentation"*
> — Michael Soegtrop, 2017

### 3.3 Dependency declaration [DISCUSSED]

Packages must be able to declare dependencies on other packages so that the
system can resolve and fetch them.

> *"the package management system handles dependencies"*
> — Michael Soegtrop, 2017

### 3.4 Metadata file format [INFERRED]

Each package repository should contain a metadata file (e.g.
`maxima-package.yaml` or similar) that declares at minimum:

- **name** — the package's short name
- **version** — a version identifier
- **description** — a one-line summary
- **author** — name and optional contact
- **license** — the license under which the package is distributed
- **entry point** — the file to load (e.g. `diophantine.mac`)
- **dependencies** — list of other packages required (may be empty)
- **maxima-compat** — minimum Maxima version required (optional)

The exact format is an implementation decision, but the fields above are the
minimum needed to satisfy the discussed requirements.

### 3.5 Versioning [INFERRED]

The discussions do not address versioning directly, but dependency resolution
requires it. Packages should declare a version, and dependency specifications
should support at least a minimum version constraint (e.g. `>= 1.2`). A full
semver scheme may be overkill for the Maxima ecosystem; simple numeric
versions with a compatibility operator may suffice.

---

## 4. User operations

### 4.1 Search / browse [DISCUSSED]

Users must be able to search or browse the catalog to find packages. This
addresses the discoverability goal (§1.2).

At minimum: search by name, keyword, and description text.

### 4.2 Install a package by name [DISCUSSED]

Users should be able to install a package using its short name. The system
resolves the name to a URL via the index, downloads the package, and places
it where Maxima can load it.

> *Implied by Dodier (2016): "download the package containing its source code
> and documentation, download any dependencies, set up load paths"*

### 4.3 Load an installed package [DISCUSSED]

After installation, the package should be loadable via Maxima's standard
`load()` mechanism, ideally using the short name.

> *"extend load so that it accepts addresses on github"*
> — Stavros Macrakis, 2022

### 4.4 List installed packages [INFERRED]

Users should be able to see what packages are currently installed. Roy Smith's
2021 question ("how to list installed packages, check their content, and add
new ones") demonstrates that even this basic operation is not obvious today.

### 4.5 Update / remove a package [INFERRED]

Standard lifecycle operations. Not discussed on the list, but any usable
package manager needs them.

### 4.6 Run package tests [PROPOSED]

Dodier (2016) mentions "maybe run tests" as part of the install flow. Packages
should be able to include a test suite, and the system should provide a way
to run it.

---

## 5. Author operations

### 5.1 Minimal packaging overhead [DISCUSSED]

Creating a package should require minimal effort — ideally just adding a
metadata file to an existing repository. The chicken-and-egg problem will not
be broken if packaging is burdensome.

### 5.2 Register a package in the index [INFERRED]

There must be a process for adding a new package to the central index. Given
the decentralized model (§2.1), this is likely a pull request to the index
repository. The process should be documented and low-friction.

### 5.3 Publish updates [INFERRED]

When a package author pushes changes to their repository, users should be able
to get the update. The simplest model: the index points to a repository (not a
specific archive), and "update" means pulling the latest from that repository.

Whether the index tracks specific versions/tags or always points to a default
branch is an implementation decision.

---

## 6. Platform and compatibility

### 6.1 Cross-platform [DISCUSSED]

Must work on Windows, macOS, and Linux.

> *"a Maxima package manager needs to work on Windows too"*
> — Robert Dodier, 2016

### 6.2 Works with all Maxima distributions [DISCUSSED]

Must work regardless of how Maxima was installed — source build, MacPorts,
Homebrew, Windows installer, Linux distro packages, etc.

> *"the package management system works with all typical Maxima
> distributions"* — Michael Soegtrop, 2017

### 6.3 No external tooling requirement [INFERRED]

Users should not need to install additional tools (git, curl, Python, etc.)
beyond Maxima itself to use the package system. Maxima runs on Common Lisp,
which has HTTP and filesystem capabilities; the system should leverage those
where possible.

This is a pragmatic constraint: many Maxima users are mathematicians, not
software developers, and may not have a development toolchain installed.

---

## 7. Non-goals and deferred scope

These items were raised in discussions but are explicitly deferred from v1.

### 7.1 Snippet-level granularity [PROPOSED, DEFERRED]

Martin Schorer (2017) asked whether sub-package-level code snippets should
be discoverable. This is a different problem with different UX requirements
and is deferred.

### 7.2 Deep Maxima integration [PROPOSED, DEFERRED]

Extending Maxima's `load()` to natively resolve package names or GitHub URLs
(Macrakis 2022) requires changes to Maxima core. This is desirable but not
required for an initial version.

### 7.3 Curated quality tiers [INFERRED, DEFERRED]

Macrakis (2022) distinguished between broadly useful packages that belong in
core versus niche domain-specific ones. A quality/maturity rating system is
useful but adds complexity; defer until there are enough packages to warrant
it.

### 7.4 Security and trust model [INFERRED, DEFERRED]

The discussions do not address code signing, review, or trust. For v1, the
system operates on the same trust model as downloading code from GitHub
directly — the user trusts the package author. A more formal trust model can
be layered on later.

---

## 8. Success criteria [INFERRED]

The following would indicate the system is working:

1. A new user can search for and install a package in under 2 minutes.
2. An existing package author can register their package in under 10 minutes.
3. The system requires less than 1 hour/month of maintainer time.
4. At least 10 packages are indexed within 3 months of launch.
5. The system works out of the box on a fresh Maxima install on all three
   major platforms.

---

## Appendix: Relationship to prior efforts

| Effort | Relationship to these requirements |
|--------|-----------------------------------|
| **maxima-asdf** | Addressed §3.3 (dependencies) via Common Lisp's ASDF. Did not address §1.2 (discoverability) or §2.1 (index). Coupled to Lisp tooling, tension with §6.3. |
| **mext** | Most complete attempt at §4 (user operations). Abandoned; demonstrated the §1.3 (maintenance burden) risk. |
| **Package template** | Addresses §5.1 (author experience) and §3.2 (documentation). Complementary to — not competitive with — a package system. |
| **share/ directory** | Serves as the current §2.1 equivalent but requires core developer gatekeeping, violating §1.1. Useful as bootstrap content (§1.4). |
