# Maxima Package Repository: Mailing List Discussions

A summary of discussions on the maxima-discuss mailing list about package
management, third-party package distribution, and discoverability. The
conversation spans 2016-2025 with no resolution — Maxima still has no
standard package manager or registry.

## Timeline

### 2016: Robert Dodier proposes third-party packages

Robert Dodier opens the discussion, arguing that Maxima should move away
from requiring all packages to be merged into the official `share/`
directory.

> Without considering this package in particular, the direction that I'd
> like to try to go with packages created for Maxima is to use some kind
> of tool which allows one to download the package containing its source
> code and documentation, download any dependencies, set up load paths,
> and maybe run tests.

— [Robert Dodier, 2016-07-15][third-party-2016]

He follows up with a progress report on loading from unofficial sources:

> I've made progress on loading Maxima projects from sources other than
> the official Maxima repository. I think this is important for building
> an ecosystem around Maxima which allows interested parties to develop
> and release their own packages without requiring intervention from the
> official developers. Other projects such as R have a well-developed
> third-party development system which has been very important for their
> growth.

— [Robert Dodier, 2016-08-21][progress-2016]

Dimiter Prodanov also raises the package manager question, linking to
snapcraft.io as inspiration. Robert responds that a Maxima package manager
needs to work on Windows too and operate at a different level than OS-level
package managers.

— [Dimiter Prodanov, 2016-08-29][snapcraft-2016];
[Robert Dodier, 2016-08-31][snapcraft-reply-2016]

### 2017: Concrete proposals and requirements

Michael Soegtrop lays out specific requirements for a package management
system:

> - the package management system works with all typical Maxima
>   distributions
> - the package management system has some sort of directory to map short
>   package names to URLs
> - the package management system handles dependencies
> - the package management system includes documentation

— [Michael Soegtrop, 2017-09-12][soegtrop-2017]

Dimiter Prodanov proposes a minimalistic approach using quicklisp (Common
Lisp's package manager) to download shared packages from update sites:

> My idea is rather minimalistic. Just to distribute a mechanism (i.e.
> quicklisp) to download shared packages from number of update sites.

— [Dimiter Prodanov, 2017-09-13][quicklisp-2017]

Gunter Königsmann suggests using GitHub repos directly, citing KiCad's
footprint library approach:

> They save things in a github repo and seem to download the uncompressed
> files directly from github's homepage using a curl library. As Maxima
> files typically are a few 100k at most this might work.

— [Gunter Königsmann, 2017-10-16][kicad-2017]

Martin Schorer raises practical concerns:

> * whatever the chosen system will be, it would be good if the (local)
>   name of it were easy to find by web searches;
> * Maxima mail-list gets amounts of tidbits that are lesser than packages,
>   but still possibly usable: could these be hosted by the system too?

— [Martin Schorer, 2017-11-03][schorer-2017]

A thread on naming the project considers options like "maxima-lib" and
"maxima-userlib".

— [Jaime Villate, 2018-02-24][naming-2018]

### 2021: Renewed calls for a package manager

Dimiter Prodanov renews his request:

> I have asked for this before. Can you include a simple package manager
> with Maxima distribution? The way how packages are distributed (or not
> distributed) with Maxima really hampers the adoption of the platform.
> This was outdated already 10 years ago.

— [Dimiter Prodanov, 2021-01-02][prodanov-2021]

Roy Smith asks a basic usability question that illustrates the problem:

> Could anyone help me discover how to manage the packages in
> maxima/wxmaxima. By manage I mean, list installed packages, check their
> content, and add new ones.

— [Roy Smith, 2021-08-21][roy-2021]

### 2022: share package contribution policy

A thread on the Carleman matrix contribution triggers a broad discussion
about the share package contribution policy and third-party distribution.

Stavros Macrakis proposes extending `load` to accept GitHub URLs:

> We could, for example, extend load so that it accepts addresses on
> github, e.g.,
> `load("github://maxima-packages/sdemarre/diophantine_system/diophantine_system.mac")`.
> Discovering the existence of such a package is the next hurdle. It would
> be nice to have an organized catalog of packages.

— [Stavros Macrakis, 2022-09-14][macrakis-load-2022]

He later raises what becomes the central concern — discoverability:

> The big question I have about outside repositories is discoverability
> and documentation. We are already doing a pretty poor job of helping
> users understand which share packages might be useful for their
> problems — we just list them.

— [Stavros Macrakis, 2022-11-02][macrakis-discover-2022]

He also draws a useful distinction between packages that belong in core
versus third-party:

> Yes, we should support loading from outside repositories. But broadly
> useful functionality such as raddenest should be part of Maxima, as
> opposed to packages addressing verticals like transient analysis of
> analog circuits or like administering math quizzes.

— [Stavros Macrakis, 2022-11-02][macrakis-discover-2022]

Robert Dodier responds, agreeing on the discoverability problem:

> [We need] a way to handle stuff that people create without coordination.

— [Robert Dodier, 2022-11-03][dodier-discover-2022]

### 2024: Distributing third party packages

Raymond Toy (Maxima maintainer) and Robert Dodier discuss the
chicken-and-egg problem and the question of a central vs decentralized
model.

Robert argues against a central repository:

> To be honest, I don't think we want a central package repository,
> because that requires someone to maintain it. What we need is a way to
> handle stuff that people create without coordination.

— [Robert Dodier, 2024-01-20][dodier-2024]

The thread also references **maxima-asdf**, Robert's project using Common
Lisp's ASDF (Another System Definition Facility) as a foundation for
Maxima package management.

### 2025: Still unresolved

The most recent discussion (September 2025) shows the problem remains open.

Raymond Toy articulates the chicken-and-egg problem:

> A chicken-and-egg problem. No one wants to make the effort to contribute
> to a package manager if there aren't many packages in it. But no one
> wants to use a package manager if there aren't any useful packages.

— [Raymond Toy, 2025-09-22][toy-2025]

ehm mentions **mext**, a package manager written by jlapeyre on GitHub,
abandoned for 7+ years:

> jlapeyre on github has written a package manager called "mext" but it
> has been untouched for 7 years.

— [ehm, 2025-09-21][ehm-2025]

ehm also mentions a package template that he and Ray wrote that generates
documentation from source, but adoption has been minimal.

## Existing efforts

| Project | Author | Status | Approach |
|---------|--------|--------|----------|
| maxima-asdf | Robert Dodier | Stalled | Uses Common Lisp ASDF for dependency management |
| mext | jlapeyre | Abandoned (7+ years) | Full package manager with install/update |
| Package template | ehm + Ray Toy | Available | Template for creating well-documented packages |
| share/ directory | Maxima project | Active | Official curated packages bundled with Maxima |

## Known third-party packages on GitHub

Packages mentioned in the mailing list discussions:

- **diophantine** (Serge de Marre) — Diophantine equation solver
  https://github.com/sdemarre/maxima-diophantine
- **padics** (José A. Vallejo) — p-adic number arithmetic
  https://github.com/josanvallejo/padics
- **raddenest** — Radical denesting
- **clifford** — Clifford algebra
- **numerical** (Ramani) — Numerical methods collection
  https://github.com/ramaniji/numericalMethods

## Key themes and open questions

### 1. Central vs decentralized

The community leans decentralized (GitHub repos) rather than a central
registry like PyPI or CRAN. Robert Dodier has consistently argued that a
central repository requires maintenance that nobody wants to provide. The
alternative is a catalog/index that points to repos hosted elsewhere.

### 2. Discoverability is the main unsolved problem

Even the existing `share/` packages are poorly documented and hard to find.
A third-party ecosystem would make this worse without a discovery mechanism.
Stavros Macrakis identified this as the "big question" — how do users find
packages that solve their problem?

### 3. The chicken-and-egg problem

No package manager gains traction without packages, and nobody packages
their code without a manager. Breaking this cycle likely requires:
- Making packaging trivially easy (minimal metadata, no special tooling)
- Providing immediate value (discoverability, documentation rendering)
- Bootstrapping with existing packages (share/ + known GitHub repos)

### 4. Cross-platform requirements

Any solution must work on Windows, macOS, and Linux, across different
Maxima distributions (source build, MacPorts, Windows installer, etc.).

### 5. Documentation and metadata

Michael Soegtrop's requirements (2017) remain valid: short name mapping,
dependency handling, and integrated documentation are all needed.

### 6. Package vs snippet

Martin Schorer raised the question of whether sub-package-level code
(useful snippets, one-off functions shared on the mailing list) should also
be discoverable. This is a different granularity than full packages.

## References

[third-party-2016]: https://sourceforge.net/p/maxima/mailman/message/35221804/
[progress-2016]: https://sourceforge.net/p/maxima/mailman/message/35291831/
[snapcraft-2016]: https://sourceforge.net/p/maxima/mailman/message/35311600/
[snapcraft-reply-2016]: https://sourceforge.net/p/maxima/mailman/message/35320033/
[soegtrop-2017]: https://sourceforge.net/p/maxima/mailman/message/36034913/
[quicklisp-2017]: https://sourceforge.net/p/maxima/mailman/message/36036951/
[kicad-2017]: https://sourceforge.net/p/maxima/mailman/message/36077811/
[schorer-2017]: https://sourceforge.net/p/maxima/mailman/message/36102165/
[naming-2018]: https://sourceforge.net/p/maxima/mailman/message/36237170/
[prodanov-2021]: https://sourceforge.net/p/maxima/mailman/message/37188706/
[roy-2021]: https://sourceforge.net/p/maxima/mailman/message/37338904/
[macrakis-load-2022]: https://sourceforge.net/p/maxima/mailman/message/37707309/
[macrakis-discover-2022]: https://sourceforge.net/p/maxima/mailman/message/37729637/
[dodier-discover-2022]: https://sourceforge.net/p/maxima/mailman/message/37730085/
[dodier-2024]: https://sourceforge.net/p/maxima/mailman/message/58726725/
[toy-2025]: https://sourceforge.net/p/maxima/mailman/message/59237105/
[ehm-2025]: https://sourceforge.net/p/maxima/mailman/message/59236493/
