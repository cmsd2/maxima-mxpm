//! Category mapping, markdown file emission, and package scaffolding.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::errors::MxpmError;

use super::ExtractedSymbol;
use super::preprocess::extract_version;

/// Map a raw Maxima category string to one of 13 top-level categories.
pub(super) fn map_category(raw: &str) -> String {
    // Strip common prefixes from Maxima section titles
    let stripped = raw
        .strip_prefix("Functions and Variables for ")
        .or_else(|| raw.strip_prefix("Introduction to "))
        .or_else(|| raw.strip_prefix("Package "))
        .or_else(|| raw.strip_prefix("Definitions for "))
        .unwrap_or(raw);
    let lower = stripped.to_lowercase();

    // Skip pure definition-type strings (e.g. "Function", "Variable")
    if is_definition_type(&lower) {
        return "Other".to_string();
    }

    // Topical categories — checked in order of specificity
    if contains_any(
        &lower,
        &[
            "linear algebra",
            "matri",
            "vector",
            "eigen",
            "linearalgebra",
        ],
    ) {
        return "LinearAlgebra".to_string();
    }
    if contains_any(&lower, &["trigonometr", "hyperbolic", "trigtools"]) {
        return "Trigonometry".to_string();
    }
    if contains_any(
        &lower,
        &[
            "calcul",
            "integr",
            "differenti",
            "ode",
            "laplace",
            "quadpack",
            "romberg",
            "contrib_ode",
            "differential equation",
        ],
    ) {
        return "Calculus".to_string();
    }
    if contains_any(
        &lower,
        &["simplif", "rational", "rules and pattern", "opsubst"],
    ) {
        return "Simplification".to_string();
    }
    if contains_any(&lower, &["solv", "equation", "to_poly_solve", "algsys"]) {
        return "Solving".to_string();
    }
    if contains_any(
        &lower,
        &[
            "plot", "draw", "graph", "dynamics", "worldmap", "picture", "bode",
        ],
    ) {
        return "Plotting".to_string();
    }
    if contains_any(&lower, &["number theory", "prime", "number", "divis"]) {
        return "NumberTheory".to_string();
    }
    if contains_any(&lower, &["polynom", "grobner", "algebraic", "ratpow"]) {
        return "Polynomials".to_string();
    }
    if contains_any(
        &lower,
        &[
            "series",
            "taylor",
            "fourier",
            "power series",
            "limit",
            "sum",
            "product",
            "zeilberger",
            "solve_rec",
        ],
    ) {
        return "Series".to_string();
    }
    if contains_any(&lower, &["combinat", "set", "permut"]) {
        return "Combinatorics".to_string();
    }
    if contains_any(
        &lower,
        &[
            "elliptic",
            "special function",
            "gamma",
            "bessel",
            "hypergeometric",
            "airy",
            "struve",
            "orthogonal poly",
            "orthopoly",
            "math",
        ],
    ) {
        return "SpecialFunctions".to_string();
    }
    if contains_any(
        &lower,
        &[
            "input",
            "output",
            "file",
            "display",
            "print",
            "read",
            "string",
            "numericalio",
            "format",
            "fortran",
            "tex output",
            "f90",
            "gentran",
            "alt-display",
        ],
    ) {
        return "IO".to_string();
    }
    if contains_any(
        &lower,
        &[
            "tensor", "ctensor", "itensor", "atensor", "cartan", "frame", "affine",
        ],
    ) {
        return "Tensors".to_string();
    }
    if contains_any(
        &lower,
        &[
            "program",
            "flow",
            "function defin",
            "debug",
            "compile",
            "evaluation",
            "macro",
            "operator",
            "expression",
            "data type",
            "structure",
            "array",
            "list",
            "predicate",
            "propert",
            "database",
            "command line",
            "runtime",
            "help",
            "constant",
            "comment",
            "identifier",
            "reserved",
        ],
    ) {
        return "Programming".to_string();
    }
    if contains_any(&lower, &["algebra", "sym"]) && !lower.contains("linear") {
        return "Algebra".to_string();
    }
    if contains_any(
        &lower,
        &[
            "stat",
            "distrib",
            "random",
            "lsquare",
            "descriptive",
            "inference",
        ],
    ) {
        return "Statistics".to_string();
    }
    if contains_any(
        &lower,
        &[
            "numer",
            "float",
            "fft",
            "lapack",
            "minpack",
            "lbfgs",
            "mnewton",
            "simplex",
            "cobyla",
            "hompack",
            "interpol",
            "rk_adaptive",
            "odepack",
            "colnew",
            "levin",
            "pslq",
        ],
    ) {
        return "Numerical".to_string();
    }
    if contains_any(&lower, &["unit", "physical_constant", "ezunit"]) {
        return "Units".to_string();
    }
    if contains_any(&lower, &["crypto", "finance", "misc", "share", "functs"]) {
        return "Other".to_string();
    }

    "Other".to_string()
}

fn is_definition_type(lower: &str) -> bool {
    matches!(
        lower,
        "function"
            | "variable"
            | "option variable"
            | "system variable"
            | "system function"
            | "special operator"
            | "operator"
            | "property"
            | "declaration"
            | "special symbol"
    )
}

fn contains_any(value: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|p| value.contains(p))
}

/// Group symbols by (category, chapter) and write one markdown file per subcategory.
///
/// Returns a list of `(category_name, vec_of_filenames)` in sorted order.
pub(super) fn emit_markdown_files(
    symbols: &[ExtractedSymbol],
    doc_dir: &Path,
) -> Result<Vec<(String, Vec<String>)>, MxpmError> {
    // Remove stale .md files from previous runs (but not subdirectories)
    if doc_dir.exists() {
        for entry in fs::read_dir(doc_dir)?.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "md") && path.is_file() {
                fs::remove_file(&path)?;
            }
        }
    }

    // Group by category, then by chapter within each category
    let mut by_cat_chap: BTreeMap<String, BTreeMap<String, Vec<&ExtractedSymbol>>> =
        BTreeMap::new();
    for sym in symbols {
        by_cat_chap
            .entry(sym.category.clone())
            .or_default()
            .entry(sym.chapter.clone())
            .or_default()
            .push(sym);
    }

    let mut category_groups: Vec<(String, Vec<String>)> = Vec::new();

    for (category, chapters) in &by_cat_chap {
        let cat_slug = slugify(category);
        let mut files = Vec::new();

        // If there's only one chapter and its cleaned name matches the category, emit a single file
        let single_chapter = chapters.len() == 1
            && chapters
                .keys()
                .next()
                .is_some_and(|ch| ch.is_empty() || ch == category);

        for (chapter, syms) in chapters {
            let (filename, heading) = if single_chapter {
                (format!("{cat_slug}.md"), category.clone())
            } else {
                let chap_slug = slugify(chapter);
                let heading = if chapter.is_empty() {
                    category.clone()
                } else {
                    chapter.clone()
                };
                if chap_slug.is_empty() {
                    (format!("{cat_slug}.md"), heading)
                } else {
                    let mut candidate = format!("{cat_slug}-{chap_slug}.md");
                    // Deduplicate: if this filename was already used, append a counter
                    if files.contains(&candidate) {
                        let mut n = 2;
                        loop {
                            candidate = format!("{cat_slug}-{chap_slug}-{n}.md");
                            if !files.contains(&candidate) {
                                break;
                            }
                            n += 1;
                        }
                    }
                    (candidate, heading)
                }
            };

            let mut content = format!("## {heading}\n\n");
            emit_symbols(&mut content, syms);

            fs::write(doc_dir.join(&filename), &content)?;
            eprintln!("  {}: {} symbols", filename, syms.len());
            files.push(filename);
        }

        category_groups.push((category.clone(), files));
    }

    Ok(category_groups)
}

/// Emit symbol headings, bodies, and see-also references into a markdown string.
///
/// Emits metadata comments before each symbol heading:
/// - `<!-- category: ... -->` from the mapped category
/// - `<!-- keywords: ... -->` from extracted index entries
/// - `<!-- signatures: ... -->` when more than one signature exists
fn emit_symbols(content: &mut String, syms: &[&ExtractedSymbol]) {
    for sym in syms {
        // Emit metadata comments before the heading
        content.push_str(&format!("<!-- category: {} -->\n", sym.category));
        if !sym.keywords.is_empty() {
            content.push_str(&format!("<!-- keywords: {} -->\n", sym.keywords.join(", ")));
        }
        if !sym.signatures.is_empty() {
            content.push_str(&format!(
                "<!-- signatures: {} -->\n",
                sym.signatures.join(", ")
            ));
        }

        let heading_type = &sym.symbol_type;
        let sig = sym
            .signatures
            .first()
            .map(|s| s.as_str())
            .unwrap_or(&sym.name);
        if heading_type == "Function" {
            // Extract args from the signature. The name in the heading is always
            // sym.name (bare symbol), and the args come from the signature.
            let args_str = extract_heading_args(sig, &sym.name);
            if let Some(args) = args_str {
                content.push_str(&format!("### Function: {} ({args})\n\n", sym.name));
            } else {
                // No parens — operator or bare function name
                content.push_str(&format!("### Function: {}\n\n", sym.name));
            }
        } else {
            content.push_str(&format!("### Variable: {}\n\n", sym.name));
        }

        if !sym.body_md.is_empty() {
            content.push_str(&sym.body_md);
            content.push_str("\n\n");
        }

        if !sym.see_also.is_empty() {
            let refs: Vec<String> = sym.see_also.iter().map(|r| format!("`{r}`")).collect();
            content.push_str(&format!("See also: {}.\n\n", refs.join(", ")));
        }
    }
}

/// Extract the arguments portion from a signature for use in a `### Function:` heading.
///
/// Given a signature like `diff(expr, x)` and name `diff`, returns `Some("expr, x")`.
/// For `absolute_real_time()`, returns `Some("")` to preserve the empty parens.
/// For subscript signatures like `%f[p, q]([a],[b],z)` and name `%f`, returns
/// `Some("[p, q]([a],[b],z)")`.
/// Returns `None` only if the signature has no argument list at all (bare operator name).
fn extract_heading_args(sig: &str, name: &str) -> Option<String> {
    // Strip the function name prefix to get the args portion
    let rest = sig.strip_prefix(name)?;
    let rest = rest.trim();
    if rest.is_empty() {
        return None;
    }
    // Strip outer parens if present, preserving everything inside
    if let Some(inner) = rest.strip_prefix('(').and_then(|s| s.strip_suffix(')')) {
        Some(inner.to_string())
    } else if rest.starts_with('[') {
        // Subscript notation like [p, q](...) — include everything
        Some(rest.to_string())
    } else {
        Some(rest.to_string())
    }
}

/// Generate the main doc file with `<!-- include: ... -->` directives grouped by category.
pub(super) fn emit_main_doc(
    category_groups: &[(String, Vec<String>)],
    doc_dir: &Path,
) -> Result<(), MxpmError> {
    let mut content = String::from("# Maxima Core Documentation\n\n");
    content.push_str("## Introduction\n\n");
    content.push_str(
        "Reference documentation for Maxima's built-in functions and variables,\n\
         generated from the official Maxima Texinfo source.\n\n",
    );

    for (category, files) in category_groups {
        content.push_str(&format!("## {category}\n\n"));
        for file in files {
            content.push_str(&format!("<!-- include: {file} -->\n"));
        }
        content.push('\n');
    }

    fs::write(doc_dir.join("maxima-core-docs.md"), content)?;
    Ok(())
}

/// Slugify a string for use in filenames (lowercase, non-alphanumeric → hyphens).
fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Generate `manifest.toml` and placeholder `.mac` for the package.
pub(super) fn emit_manifest(out_dir: &Path, maxima_src: &Path) -> Result<(), MxpmError> {
    // Try to get version from Maxima source
    let configure_ac = maxima_src.join("configure.ac");
    let version = if configure_ac.exists() {
        extract_version(&configure_ac)?
    } else {
        "5.47.0".to_string()
    };

    let manifest = format!(
        r#"[package]
name = "maxima-core-docs"
version = "{version}"
description = "Reference documentation for Maxima built-in functions and variables"
license = "GPL-2.0-or-later"
entry = "maxima_core_docs.mac"
doc = "doc/maxima-core-docs.md"
keywords = ["documentation", "core", "reference"]
"#
    );
    fs::write(out_dir.join("manifest.toml"), manifest)?;

    let mac = "/* maxima-core-docs: documentation-only package */\n";
    fs::write(out_dir.join("maxima_core_docs.mac"), mac)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_mapping() {
        assert_eq!(map_category("Differential calculus"), "Calculus");
        assert_eq!(map_category("Integration"), "Calculus");
        assert_eq!(map_category("Matrices and Linear Algebra"), "LinearAlgebra");
        assert_eq!(map_category("Simplification"), "Simplification");
        assert_eq!(map_category("Plotting"), "Plotting");
        assert_eq!(map_category("Number Theory"), "NumberTheory");
        assert_eq!(map_category("Function"), "Other"); // definition type
        assert_eq!(map_category("Option variable"), "Other");
        assert_eq!(map_category("Something unknown"), "Other");
    }

    #[test]
    fn test_category_mapping_section_titles() {
        // Common Maxima section title patterns
        assert_eq!(
            map_category("Functions and Variables for Integration"),
            "Calculus"
        );
        assert_eq!(
            map_category("Functions and Variables for Plotting"),
            "Plotting"
        );
        assert_eq!(
            map_category("Functions and Variables for Matrices and Linear Algebra"),
            "LinearAlgebra"
        );
        assert_eq!(
            map_category("Introduction to Simplification"),
            "Simplification"
        );
        assert_eq!(
            map_category("Functions and Variables for Function Definition"),
            "Programming"
        );
        assert_eq!(
            map_category("Functions and Variables for Special Functions"),
            "SpecialFunctions"
        );
        assert_eq!(
            map_category("Functions and Variables for ctensor"),
            "Tensors"
        );
        assert_eq!(
            map_category("Functions and Variables for distrib"),
            "Statistics"
        );
        assert_eq!(
            map_category("Functions and Variables for lapack"),
            "Numerical"
        );
    }

    #[test]
    fn test_is_definition_type() {
        assert!(is_definition_type("function"));
        assert!(is_definition_type("option variable"));
        assert!(!is_definition_type("differential calculus"));
        assert!(!is_definition_type("matrices"));
    }
}
