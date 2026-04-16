# Package richpkg

## Introduction

Package `richpkg` demonstrates all documentation features:
examples, cross-references, and multiple sections.

## Tutorial

Here is a quick example of how to use richpkg:

```maxima
(%i1) load("richpkg");
(%o1) richpkg.mac
```

## Definitions for richpkg

### Function: rich_solve (expr, vars)

Solves `expr` for `vars` using the **rich** method.

This is a second paragraph with more details about the algorithm.
It supports *multiple* variable systems.

```maxima
(%i1) rich_solve(x^2 - 1, x);
(%o1)                      [x = -1, x = 1]
(%i2) rich_solve(x^2 + y^2 = 1, [x, y]);
(%o2)                      [[x = 0, y = 1]]
```

See also: `rich_opts`, `rich_verbose`.

### Function: rich_opts ()

Returns the current solver options as an association list.

```maxima
(%i1) rich_opts();
(%o1)                 [[method, auto], [verbose, false]]
```

### Variable: rich_verbose

When `true`, prints extra diagnostics during solving.

Default value: `false`

See also: `rich_solve`.
