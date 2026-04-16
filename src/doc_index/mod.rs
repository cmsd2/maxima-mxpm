//! Parse package documentation markdown into a structured per-symbol JSON index.
//!
//! Generates `<pkg>-doc-index.json` for GUI applications (VS Code extension,
//! web UIs, notebook frontends) to provide hover docs, help panels, and search.
//!
//! The CLI help system (`.info` + `-index.lisp`) is unaffected — this is an
//! additional artifact generated alongside existing outputs.

pub(crate) mod lint;
mod parser;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::errors::MxpmError;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// Top-level structured documentation index.
#[derive(Debug, Serialize, Deserialize)]
pub struct DocIndex {
    pub version: u32,
    pub package: String,
    pub source: String,
    pub symbols: BTreeMap<String, SymbolEntry>,
    pub sections: Vec<SectionEntry>,
}

/// Documentation for a single function or variable.
#[derive(Debug, Serialize, Deserialize)]
pub struct SymbolEntry {
    #[serde(rename = "type")]
    pub symbol_type: String,
    pub signature: String,
    pub summary: String,
    pub body_md: String,
    pub examples: Vec<ExampleEntry>,
    pub see_also: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub category: Option<String>,
}

/// A structured example extracted from documentation.
#[derive(Debug, Serialize, Deserialize)]
pub struct ExampleEntry {
    pub input: String,
    pub output: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub description: String,
}

/// A non-symbol documentation section (introduction, tutorials, etc.).
#[derive(Debug, Serialize, Deserialize)]
pub struct SectionEntry {
    pub title: String,
    pub body_md: String,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse expanded markdown content into a [`DocIndex`].
///
/// `content` should have all `<!-- include: ... -->` directives already
/// expanded.
pub fn parse_markdown(
    content: &str,
    package_name: &str,
    source_path: &str,
) -> Result<DocIndex, MxpmError> {
    let blocks = parser::split_into_blocks(content);

    let mut symbols = BTreeMap::new();
    let mut sections = Vec::new();

    for block in blocks {
        match block {
            parser::ParsedBlock::Section { title, body } => {
                let body_md = body.trim().to_string();
                sections.push(SectionEntry { title, body_md });
            }
            parser::ParsedBlock::Symbol { kind, body } => {
                let body_md = body.trim().to_string();
                let summary = parser::extract_summary(&body_md);
                let examples = parser::extract_examples(&body_md);
                let see_also = parser::extract_see_also(&body_md);
                symbols.insert(
                    kind.name,
                    SymbolEntry {
                        symbol_type: kind.symbol_type,
                        signature: kind.signature,
                        summary,
                        body_md,
                        examples,
                        see_also,
                        category: None,
                    },
                );
            }
        }
    }

    Ok(DocIndex {
        version: 1,
        package: package_name.to_string(),
        source: source_path.to_string(),
        symbols,
        sections,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_document() {
        let idx = parse_markdown("", "pkg", "doc/pkg.md").unwrap();
        assert_eq!(idx.version, 1);
        assert_eq!(idx.package, "pkg");
        assert!(idx.symbols.is_empty());
        assert!(idx.sections.is_empty());
    }

    #[test]
    fn parse_section_only() {
        let md = "## Introduction\n\nSome text about the package.\n";
        let idx = parse_markdown(md, "pkg", "doc/pkg.md").unwrap();
        assert_eq!(idx.sections.len(), 1);
        assert_eq!(idx.sections[0].title, "Introduction");
        assert!(idx.sections[0].body_md.contains("Some text"));
        assert!(idx.symbols.is_empty());
    }

    #[test]
    fn parse_function_symbol() {
        let md = "### Function: hello (name)\n\nGreets the user by *name*.\n";
        let idx = parse_markdown(md, "pkg", "doc/pkg.md").unwrap();
        assert_eq!(idx.symbols.len(), 1);
        let sym = &idx.symbols["hello"];
        assert_eq!(sym.symbol_type, "Function");
        assert_eq!(sym.signature, "hello(name)");
        assert_eq!(sym.summary, "Greets the user by name.");
    }

    #[test]
    fn parse_function_no_args() {
        let md = "### Function: version ()\n\nReturns the version.\n";
        let idx = parse_markdown(md, "pkg", "doc/pkg.md").unwrap();
        let sym = &idx.symbols["version"];
        assert_eq!(sym.signature, "version()");
    }

    #[test]
    fn parse_variable_symbol() {
        let md = "### Variable: greeting\n\nDefault value: `\"hi\"`\n";
        let idx = parse_markdown(md, "pkg", "doc/pkg.md").unwrap();
        assert_eq!(idx.symbols.len(), 1);
        let sym = &idx.symbols["greeting"];
        assert_eq!(sym.symbol_type, "Variable");
        assert_eq!(sym.signature, "greeting");
        assert_eq!(sym.summary, "Default value: \"hi\"");
    }

    #[test]
    fn symbols_sorted_alphabetically() {
        let md = "### Function: zebra ()\n\nZ.\n\n### Function: alpha ()\n\nA.\n";
        let idx = parse_markdown(md, "pkg", "doc/pkg.md").unwrap();
        let keys: Vec<&String> = idx.symbols.keys().collect();
        assert_eq!(keys, vec!["alpha", "zebra"]);
    }

    #[test]
    fn json_roundtrip() {
        let md = "## Intro\n\nHello.\n\n### Function: foo (x)\n\nDoes foo.\n\n```maxima\n(%i1) foo(1);\n(%o1) 42\n```\n\nSee also: `bar`.\n";
        let idx = parse_markdown(md, "test", "doc/test.md").unwrap();
        let json = serde_json::to_string_pretty(&idx).unwrap();
        assert!(json.contains("\"version\": 1"));
        assert!(json.contains("\"type\": \"Function\""));
        assert!(json.contains("\"see_also\""));
        assert!(json.contains("\"bar\""));
        assert!(json.contains("\"examples\""));
        // Verify roundtrip via Deserialize
        let _parsed: DocIndex = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn parse_testpkg_fixture() {
        let content = include_str!("../../tests/fixtures/doc/testpkg.md");
        let idx = parse_markdown(content, "testpkg", "doc/testpkg.md").unwrap();

        assert_eq!(idx.package, "testpkg");
        assert_eq!(idx.version, 1);

        // Sections: "Introduction to testpkg" and "Definitions for testpkg"
        assert_eq!(idx.sections.len(), 2);
        assert_eq!(idx.sections[0].title, "Introduction to testpkg");
        assert_eq!(idx.sections[1].title, "Definitions for testpkg");

        // Symbols
        assert_eq!(idx.symbols.len(), 2);

        let hello = &idx.symbols["hello"];
        assert_eq!(hello.symbol_type, "Function");
        assert_eq!(hello.signature, "hello(name)");
        assert_eq!(hello.summary, "Returns a greeting for name.");

        let greeting = &idx.symbols["greeting"];
        assert_eq!(greeting.symbol_type, "Variable");
        assert_eq!(greeting.signature, "greeting");
    }

    #[test]
    fn parse_richpkg_fixture() {
        let content = include_str!("../../tests/fixtures/doc/richpkg.md");
        let idx = parse_markdown(content, "richpkg", "doc/richpkg.md").unwrap();

        // 3 sections: Introduction, Tutorial, Definitions
        assert_eq!(idx.sections.len(), 3);
        assert_eq!(idx.sections[0].title, "Introduction");
        assert_eq!(idx.sections[1].title, "Tutorial");

        // 3 symbols: rich_opts, rich_solve, rich_verbose (BTreeMap order)
        assert_eq!(idx.symbols.len(), 3);
        let keys: Vec<&String> = idx.symbols.keys().collect();
        assert_eq!(keys, vec!["rich_opts", "rich_solve", "rich_verbose"]);

        // rich_solve: function with examples and see_also
        let solve = &idx.symbols["rich_solve"];
        assert_eq!(solve.symbol_type, "Function");
        assert_eq!(solve.signature, "rich_solve(expr, vars)");
        assert_eq!(solve.summary, "Solves expr for vars using the rich method.");
        assert_eq!(solve.examples.len(), 2);
        assert_eq!(solve.examples[0].input, "rich_solve(x^2 - 1, x);");
        assert!(solve.examples[0].output.contains("[x = -1, x = 1]"));
        assert_eq!(solve.see_also, vec!["rich_opts", "rich_verbose"]);

        // rich_opts: function with one example
        let opts = &idx.symbols["rich_opts"];
        assert_eq!(opts.symbol_type, "Function");
        assert_eq!(opts.examples.len(), 1);
        assert!(opts.see_also.is_empty());

        // rich_verbose: variable with see_also
        let verbose = &idx.symbols["rich_verbose"];
        assert_eq!(verbose.symbol_type, "Variable");
        assert_eq!(
            verbose.summary,
            "When true, prints extra diagnostics during solving."
        );
        assert_eq!(verbose.see_also, vec!["rich_solve"]);
    }
}
