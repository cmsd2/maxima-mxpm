# IDE Integration for Package Authors and Users

How the VS Code extension (maxima-extension) and language tools (maxima-lsp,
maxima-dap, aximar-mcp) interact with the mxpm package ecosystem, and where
the gaps are.

## What already works

### For package authors

The core authoring experience is fully supported today:

- **Syntax highlighting** for `.mac` files
- **LSP completions and hover** for all 2500+ built-in Maxima functions
- **Signature help** while typing function arguments
- **Go to Definition / Find References** for user-defined functions across
  open files
- **Step-through debugging** with breakpoints, variable inspection, and
  call stacks via the DAP server
- **Notebook execution** for interactive testing of package functions
- **AI assistance** via Copilot/Claude with Maxima-aware instructions

The `mxpm` CLI handles the rest of the lifecycle:

- `mxpm new <name>` scaffolds manifest, entry point, tests, docs, CI
- `mxpm test` runs `rtest_*.mac` files via Maxima's `batch(..., test)`
- `mxpm doc build` generates `.info` + `-index.lisp` from `.texi` or `.md`
- `mxpm doc watch` / `mxpm doc serve` for live preview during writing
- `mxpm publish` submits the package to the index

### For package users

- `mxpm search` / `mxpm install` / `mxpm upgrade` from the terminal
- `load("pkgname")` just works after install (packages go to `~/.maxima/`)
- `? function_name` in Maxima searches package docs if `-index.lisp` exists

## Gaps

### 1. LSP unaware of mxpm-installed packages

**Impact**: High (affects all package users)

The LSP only indexes:
- The built-in function catalog (~2500 functions)
- The bundled package catalog (~100 share packages)
- User-defined functions in currently open documents

It does not scan `~/.maxima/` for mxpm-installed packages. This means:
- No completions for functions from installed packages
- No hover documentation
- No signature help
- No "package required" hints

This is the single biggest gap. A user who runs `mxpm install diophantine`
and then types `dioph` in the editor gets no completions. They have to know
the exact function names from the docs.

**Possible approaches**:
- Scan `~/.maxima/*/manifest.toml` at LSP startup to discover installed
  packages, then parse their `.mac` entry points for function definitions
- Use `-index.lisp` files (if present) as a pre-built function catalog
- Watch `~/.maxima/` for changes (installs/removals) and re-index
- Expose installed package functions through the MCP `search_functions` tool
  so AI agents can find them too

### 2. No load() resolution or install suggestions

**Impact**: Medium (discoverability)

When the LSP encounters `load("diophantine")` and cannot resolve the file,
it could:
- Check the mxpm index for a matching package name
- Offer a diagnostic: "Package 'diophantine' not installed"
- Provide a Quick Fix code action: "Install via mxpm"
- Or at minimum, suppress the "file not found" warning for known package
  names

This would help both package users (discovering packages) and authors
(testing that their dependencies are declared correctly).

### 3. No manifest.toml support

**Impact**: Low (simple format, infrequent editing)

The `manifest.toml` format has only 6 required fields and is edited
infrequently. However, basic support would reduce friction for new authors:

- JSON Schema (converted to TOML schema) for validation
- Completions for field names and known values (licenses, categories)
- Hover documentation for each field
- Diagnostics: missing required fields, invalid semver, entry file doesn't
  exist

This could be handled by the existing TOML extension ecosystem (e.g.
Even Better TOML with a custom schema) rather than building it into
maxima-lsp.

### 4. No mxpm commands in the editor

**Impact**: Low (CLI works fine, infrequent operations)

Package management operations are infrequent enough that the terminal is
adequate. But VS Code command integration would be a convenience:

- `Maxima: Install Package` — QuickPick search → `mxpm install`
- `Maxima: Run Package Tests` — runs `mxpm test` in the integrated terminal
- `Maxima: Build Package Docs` — runs `mxpm doc build`
- `Maxima: Search Packages` — QuickPick search → show info / install

These could be implemented as simple terminal commands (no special protocol
needed) or as tasks in a `tasks.json` template.

### 5. No test runner integration

**Impact**: Low-medium (convention exists, no IDE support)

The `rtest_*.mac` convention is well established in Maxima. The extension
could:

- Detect `rtest_*.mac` files and show a "Run Tests" CodeLens above them
- Provide a `Maxima: Run Tests` command that finds and runs all rtest files
- Parse test output and show results in the Test Explorer panel
- Integrate with `mxpm test` for packages that have a manifest

### 6. No package browser in the sidebar

**Impact**: Medium (discoverability for users)

A tree view showing available and installed packages would make the
ecosystem more visible:

```
MAXIMA PACKAGES
├── Installed
│   ├── diophantine (1.0.0)
│   └── clifford (0.3.0)
└── Available
    ├── padics — p-adic number arithmetic
    ├── qm-maxima — Quantum mechanics
    └── ...
```

This would require fetching the index (or using mxpm's cached copy) and
scanning `~/.maxima/` for installed packages.

## Recommendations

**Priority 1**: LSP indexing of mxpm-installed packages (gap #1). This has
the highest impact and benefits every package user. Without it, the IDE
experience degrades as soon as someone uses a third-party package.

**Priority 2**: Package browser sidebar (gap #6) and install suggestions
(gap #2). These make the ecosystem visible and discoverable from within the
editor, addressing the "discoverability" concern raised repeatedly on the
mailing list.

**Priority 3**: Test runner integration (gap #5). Useful for package
authors, especially combined with CodeLens on rtest files.

**Low priority**: manifest.toml support (gap #3) and mxpm command wrappers
(gap #4). These are nice-to-haves that can wait — the CLI and existing TOML
tooling cover them adequately.
