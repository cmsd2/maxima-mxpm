# Markdown Authoring Guide

How to structure package documentation so it works across all output formats: Maxima's help system (`.info` + `-index.lisp`), mdBook HTML, and the structured doc index JSON for GUI applications.

## Document structure

A documentation file uses three heading levels:

```markdown
# Package mypackage

## Introduction to mypackage

Introductory prose, tutorials, etc.

## Definitions for mypackage

### Function: my_func (x, y)

Description of my_func...

### Variable: my_option

Description of my_option...
```

- **`#`** — Package title (one per file). Ignored by the parser; used as the book title in mdBook.
- **`##`** — Section headings. Become separate chapters in mdBook. Become `@section` in Texinfo. Appear in the `sections` array in the doc index.
- **`### Function: name (args)`** — Function definition. Becomes `@deffn {Function}` in Texinfo, a styled definition heading in mdBook, and a symbol entry in the doc index.
- **`### Variable: name`** — Variable definition. Same treatment as functions but as `@defvr {Variable}`.

Use `####` for subsections within a symbol (e.g. Examples, Options). These are not parsed specially — they appear in the body text of the enclosing symbol.

## Summary rule

The **first paragraph** after a `###` heading is extracted as the plain-text summary for hover docs and search results. It must be a prose description, not code or signature lines.

**Good:**
```markdown
### Function: ax_bar (categories, values)

Bar chart for labeled or numeric data. Use inside `ax_draw2d`.
```

**Bad** (summary will be the signature text, not a description):
```markdown
### Function: ax_bar (categories, values)

`ax_bar(categories, values)`
`ax_bar(values)`

Bar chart. Use inside `ax_draw2d`.
```

The summary is stripped of inline markdown (`*bold*`, `` `code` ``) for plain-text contexts like tooltips.

## Overload signatures

The `### Function:` heading carries the primary signature. If a function has multiple calling forms, document them **after** the summary paragraph:

```markdown
### Function: ax_bar (categories, values)

Bar chart for labeled or numeric data. Use inside `ax_draw2d`.

Calling forms:

- `ax_bar(categories, values)` — string category list + numeric values
- `ax_bar(values)` — auto-numbered 1, 2, 3, ...
```

If the function has only one signature, do not repeat it after the heading — the heading already carries it.

## Examples

Use fenced code blocks tagged `` ```maxima ``. Two styles are supported:

**Plain style** — the entire block is treated as input:
```markdown
#### Examples

```maxima
ax_draw2d(
  ax_bar(["Q1","Q2","Q3"], [100,150,120]),
  title="Quarterly Sales"
)$
`` `
```

**I/O style** — structured input/output pairs using `(%i`/`(%o` markers:
```markdown
```maxima
(%i1) solve(x^2 - 1, x);
(%o1)                      [x = -1, x = 1]
(%i2) expand((x+1)^3);
(%o2)                    x^3 + 3*x^2 + 3*x + 1
`` `
```

Both styles work in all output formats. The doc index extracts examples into structured `input`/`output` pairs for programmatic use.

## Cross-references

Add a `See also:` line as the **last non-blank line** in a symbol section. Reference other symbols with backtick quoting:

```markdown
See also: `ax_draw2d`, `ax_histogram`
```

For cross-package references, use the `package:symbol` convention:

```markdown
See also: `diophantine:dio_solve`
```

The doc build will warn if a `See also:` reference points to a symbol not found in the current document.

## Images

Use standard markdown image syntax with relative paths:

```markdown
![Phase portrait](phase-portrait.png)
```

Place image files alongside the markdown source. In the doc index JSON, images are automatically inlined as data URLs so the output is self-contained. In mdBook, relative paths work as-is. For Texinfo, Pandoc handles the conversion.

Images larger than 500 KB will produce a warning but are still inlined.

## Multi-file documentation

For larger packages, split documentation across multiple files and use include directives:

```markdown
# Package mypackage

## Introduction to mypackage

Introductory text here.

## Definitions for mypackage

<!-- include: my_func.md -->
<!-- include: my_option.md -->
```

Each included file should contain a single `### Function:` or `### Variable:` definition. The include system:

- Expands all includes into a single file for Pandoc/Texinfo processing
- Creates nested mdBook chapters (each include becomes a sub-chapter)
- Watches all included files in `doc watch` and `doc serve`

## Options and tables

Markdown tables render well in all formats. Use them for documenting options:

```markdown
#### Options

| Option | Default | Description |
|--------|---------|-------------|
| `color` | auto | Line color |
| `line_width` | 2 | Width in pixels |
```

## Complete template

```markdown
### Function: solve_system (equations, variables)

Solve a system of equations for the given variables.

Returns a list of solutions. Each solution is a list of equations
binding variables to values.

Calling forms:

- `solve_system(eqns, vars)` — solve with default options
- `solve_system(eqns, vars, method)` — specify solver method

#### Examples

```maxima
(%i1) solve_system([x + y = 3, x - y = 1], [x, y]);
(%o1)                     [[x = 2, y = 1]]
`` `

#### Options

| Option | Default | Description |
|--------|---------|-------------|
| `method` | `"auto"` | Solver: `"auto"`, `"linear"`, `"newton"` |
| `tolerance` | 1e-10 | Convergence tolerance |

See also: `solve`, `find_root`
```
