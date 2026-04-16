//! Category mapping, markdown file emission, and package scaffolding.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::errors::MxpmError;

use super::preprocess::extract_version;
use super::ExtractedSymbol;

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
        &["linear algebra", "matri", "vector", "eigen", "linearalgebra"],
    ) {
        return "LinearAlgebra".to_string();
    }
    if contains_any(&lower, &["trigonometr", "hyperbolic", "trigtools"]) {
        return "Trigonometry".to_string();
    }
    if contains_any(
        &lower,
        &[
            "calcul", "integr", "differenti", "ode", "laplace", "quadpack",
            "romberg", "contrib_ode", "differential equation",
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
        &["plot", "draw", "graph", "dynamics", "worldmap", "picture", "bode"],
    ) {
        return "Plotting".to_string();
    }
    if contains_any(
        &lower,
        &["number theory", "prime", "number", "divis"],
    ) {
        return "NumberTheory".to_string();
    }
    if contains_any(&lower, &["polynom", "grobner", "algebraic", "ratpow"]) {
        return "Polynomials".to_string();
    }
    if contains_any(
        &lower,
        &[
            "series", "taylor", "fourier", "power series", "limit",
            "sum", "product", "zeilberger", "solve_rec",
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
            "elliptic", "special function", "gamma", "bessel",
            "hypergeometric", "airy", "struve", "orthogonal poly",
            "orthopoly", "math",
        ],
    ) {
        return "SpecialFunctions".to_string();
    }
    if contains_any(
        &lower,
        &[
            "input", "output", "file", "display", "print", "read",
            "string", "numericalio", "format", "fortran", "tex output",
            "f90", "gentran", "alt-display",
        ],
    ) {
        return "IO".to_string();
    }
    if contains_any(
        &lower,
        &[
            "tensor", "ctensor", "itensor", "atensor", "cartan",
            "frame", "affine",
        ],
    ) {
        return "Tensors".to_string();
    }
    if contains_any(
        &lower,
        &["program", "flow", "function defin", "debug", "compile", "evaluation",
          "macro", "operator", "expression", "data type", "structure",
          "array", "list", "predicate", "propert", "database",
          "command line", "runtime", "help", "constant",
          "comment", "identifier", "reserved",
        ],
    ) {
        return "Programming".to_string();
    }
    if contains_any(&lower, &["algebra", "sym"]) && !lower.contains("linear") {
        return "Algebra".to_string();
    }
    if contains_any(
        &lower,
        &["stat", "distrib", "random", "lsquare", "descriptive", "inference"],
    ) {
        return "Statistics".to_string();
    }
    if contains_any(
        &lower,
        &["numer", "float", "fft", "lapack", "minpack", "lbfgs",
          "mnewton", "simplex", "cobyla", "hompack", "interpol",
          "rk_adaptive", "odepack", "colnew", "levin", "pslq",
        ],
    ) {
        return "Numerical".to_string();
    }
    if contains_any(&lower, &["unit", "physical_constant", "ezunit"]) {
        return "Units".to_string();
    }
    if contains_any(
        &lower,
        &["crypto", "finance", "misc", "share", "functs"],
    ) {
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

/// Group symbols by category and write one markdown file per category.
///
/// Returns the list of generated filenames in sorted order.
pub(super) fn emit_markdown_files(
    symbols: &[ExtractedSymbol],
    doc_dir: &Path,
) -> Result<Vec<String>, MxpmError> {
    // Group by category
    let mut by_category: BTreeMap<String, Vec<&ExtractedSymbol>> = BTreeMap::new();
    for sym in symbols {
        by_category
            .entry(sym.category.clone())
            .or_default()
            .push(sym);
    }

    let mut category_files = Vec::new();

    for (category, syms) in &by_category {
        let slug = category.to_lowercase().replace(' ', "");
        let filename = format!("{slug}.md");
        let mut content = format!("## {category}\n\n");

        for sym in syms {
            // Heading
            let heading_type = &sym.symbol_type;
            let sig = sym
                .signatures
                .first()
                .map(|s| s.as_str())
                .unwrap_or(&sym.name);
            if heading_type == "Function" {
                // Extract args from signature for heading format
                if let Some(paren) = sig.find('(') {
                    let name = &sig[..paren];
                    let args = &sig[paren..];
                    // Remove outer parens for heading
                    let inner = args
                        .strip_prefix('(')
                        .and_then(|s| s.strip_suffix(')'))
                        .unwrap_or(args);
                    content.push_str(&format!("### Function: {name} ({inner})\n\n"));
                } else {
                    content.push_str(&format!("### Function: {sig} ()\n\n"));
                }
            } else {
                content.push_str(&format!("### Variable: {}\n\n", sym.name));
            }

            // Body markdown
            if !sym.body_md.is_empty() {
                content.push_str(&sym.body_md);
                content.push_str("\n\n");
            }

            // See also
            if !sym.see_also.is_empty() {
                let refs: Vec<String> = sym.see_also.iter().map(|r| format!("`{r}`")).collect();
                content.push_str(&format!("See also: {}.\n\n", refs.join(", ")));
            }
        }

        fs::write(doc_dir.join(&filename), &content)?;
        eprintln!("  {}: {} symbols", filename, syms.len());
        category_files.push(filename);
    }

    Ok(category_files)
}

/// Generate the main doc file with `<!-- include: ... -->` directives.
pub(super) fn emit_main_doc(
    category_files: &[String],
    doc_dir: &Path,
) -> Result<(), MxpmError> {
    let mut content = String::from("# Maxima Core Documentation\n\n");
    content.push_str("## Introduction\n\n");
    content.push_str(
        "Reference documentation for Maxima's built-in functions and variables,\n\
         generated from the official Maxima Texinfo source.\n\n",
    );

    for file in category_files {
        content.push_str(&format!("<!-- include: {file} -->\n"));
    }

    fs::write(doc_dir.join("maxima-core-docs.md"), content)?;
    Ok(())
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
        assert_eq!(
            map_category("Matrices and Linear Algebra"),
            "LinearAlgebra"
        );
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
