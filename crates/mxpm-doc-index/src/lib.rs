//! Parse package documentation markdown into a structured per-symbol JSON index.
//!
//! Generates `<pkg>-doc-index.json` for GUI applications (VS Code extension,
//! web UIs, notebook frontends) to provide hover docs, help panels, and search.
//!
//! The CLI help system (`.info` + `-index.lisp`) is unaffected — this is an
//! additional artifact generated alongside existing outputs.

pub mod lint;
pub mod loader;
mod parser;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// Top-level structured documentation index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocIndex {
    pub version: u32,
    pub package: String,
    /// The language of the index's text, a BCP 47 tag such as `es` or
    /// `pt-BR`.  English when absent, and left out of English indexes, so
    /// they read as they did before languages.
    #[serde(default = "english", skip_serializing_if = "is_english")]
    pub language: String,
    pub source: String,
    pub symbols: BTreeMap<String, SymbolEntry>,
    pub sections: Vec<SectionEntry>,
}

/// Documentation for a single function or variable.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub section: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub keywords: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub signatures: Vec<String>,
}

/// A structured example extracted from documentation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleEntry {
    pub input: String,
    pub output: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub description: String,
}

/// A non-symbol documentation section (introduction, tutorials, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionEntry {
    pub title: String,
    pub body_md: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub subsections: Vec<SectionEntry>,
}

/// The language of an index without one
pub const ENGLISH: &str = "en";

fn english() -> String {
    ENGLISH.to_string()
}

fn is_english(language: &str) -> bool {
    language.eq_ignore_ascii_case(ENGLISH)
}

/// The file name of a doc index for `stem` in `language`:
/// `<stem>-doc-index.json` for English, `<stem>-doc-index.<lang>.json` for
/// any other language
pub fn doc_index_file_name(stem: &str, language: &str) -> String {
    if is_english(language) {
        format!("{stem}-doc-index.json")
    } else {
        format!("{stem}-doc-index.{language}.json")
    }
}

/// The file name of a slim doc index, as [`doc_index_file_name`]
pub fn slim_doc_index_file_name(stem: &str, language: &str) -> String {
    doc_index_file_name(&format!("{stem}-slim"), language)
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse expanded markdown content into a [`DocIndex`].
///
/// `content` should have all `<!-- include: ... -->` directives already
/// expanded.
pub fn parse_markdown(content: &str, package_name: &str, source_path: &str) -> DocIndex {
    let blocks = parser::split_into_blocks(content);

    let mut symbols = BTreeMap::new();
    let mut current_section: Option<String> = None;

    // Collect symbols with their section associations
    for block in &blocks {
        match block {
            parser::ParsedBlock::Section { title, .. } => {
                current_section = Some(title.clone());
            }
            parser::ParsedBlock::Symbol { kind, body } => {
                let body_md = body.trim().to_string();
                let summary = parser::extract_summary(&body_md);
                let examples = parser::extract_examples(&body_md);
                let see_also = parser::extract_see_also(&body_md);
                symbols.insert(
                    kind.name.clone(),
                    SymbolEntry {
                        symbol_type: kind.symbol_type.clone(),
                        signature: kind.signature.clone(),
                        summary,
                        body_md,
                        examples,
                        see_also,
                        category: kind.category.clone(),
                        section: current_section.clone(),
                        keywords: kind.keywords.clone(),
                        signatures: kind.signatures.clone(),
                    },
                );
            }
        }
    }

    // Build hierarchical section tree
    let sections = build_section_hierarchy(&blocks);

    DocIndex {
        version: 1,
        package: package_name.to_string(),
        language: english(),
        source: source_path.to_string(),
        symbols,
        sections,
    }
}

impl DocIndex {
    /// Strip full docs, keeping only type + signature + summary per symbol.
    /// Useful for embedding a lightweight default in applications.
    pub fn slim(&self) -> DocIndex {
        DocIndex {
            version: self.version,
            package: self.package.clone(),
            language: self.language.clone(),
            source: self.source.clone(),
            symbols: self
                .symbols
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        SymbolEntry {
                            symbol_type: v.symbol_type.clone(),
                            signature: v.signature.clone(),
                            summary: v.summary.clone(),
                            body_md: String::new(),
                            examples: Vec::new(),
                            see_also: Vec::new(),
                            category: v.category.clone(),
                            section: v.section.clone(),
                            keywords: v.keywords.clone(),
                            signatures: v.signatures.clone(),
                        },
                    )
                })
                .collect(),
            sections: Vec::new(),
        }
    }
}

/// Detect parent/child section relationships and build a hierarchical tree.
///
/// A section is a "parent" if it has an empty body and is immediately followed
/// by another section in the block stream. This captures the pattern used by
/// core docs where `## Category` headings contain `<!-- include: -->` directives
/// that expand into `## Subcategory` sections with symbols underneath.
fn build_section_hierarchy(blocks: &[parser::ParsedBlock]) -> Vec<SectionEntry> {
    let mut result: Vec<SectionEntry> = Vec::new();
    let mut current_parent: Option<SectionEntry> = None;

    for (i, block) in blocks.iter().enumerate() {
        let parser::ParsedBlock::Section { title, body } = block else {
            continue;
        };
        let body_md = body.trim().to_string();
        let next_is_section =
            matches!(blocks.get(i + 1), Some(parser::ParsedBlock::Section { .. }));
        let is_parent = body_md.is_empty() && next_is_section;

        if is_parent {
            if let Some(parent) = current_parent.take() {
                result.push(parent);
            }
            current_parent = Some(SectionEntry {
                title: title.clone(),
                body_md,
                subsections: Vec::new(),
            });
        } else if let Some(ref mut parent) = current_parent {
            parent.subsections.push(SectionEntry {
                title: title.clone(),
                body_md,
                subsections: Vec::new(),
            });
        } else {
            result.push(SectionEntry {
                title: title.clone(),
                body_md,
                subsections: Vec::new(),
            });
        }
    }

    if let Some(parent) = current_parent.take() {
        result.push(parent);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_document() {
        let idx = parse_markdown("", "pkg", "doc/pkg.md");
        assert_eq!(idx.version, 1);
        assert_eq!(idx.package, "pkg");
        assert!(idx.symbols.is_empty());
        assert!(idx.sections.is_empty());
    }

    #[test]
    fn parse_section_only() {
        let md = "## Introduction\n\nSome text about the package.\n";
        let idx = parse_markdown(md, "pkg", "doc/pkg.md");
        assert_eq!(idx.sections.len(), 1);
        assert_eq!(idx.sections[0].title, "Introduction");
        assert!(idx.sections[0].body_md.contains("Some text"));
        assert!(idx.symbols.is_empty());
    }

    #[test]
    fn parse_function_symbol() {
        let md = "### Function: hello (name)\n\nGreets the user by *name*.\n";
        let idx = parse_markdown(md, "pkg", "doc/pkg.md");
        assert_eq!(idx.symbols.len(), 1);
        let sym = &idx.symbols["hello"];
        assert_eq!(sym.symbol_type, "Function");
        assert_eq!(sym.signature, "hello(name)");
        assert_eq!(sym.summary, "Greets the user by name.");
    }

    #[test]
    fn parse_function_no_args() {
        let md = "### Function: version ()\n\nReturns the version.\n";
        let idx = parse_markdown(md, "pkg", "doc/pkg.md");
        let sym = &idx.symbols["version"];
        assert_eq!(sym.signature, "version()");
    }

    #[test]
    fn parse_variable_symbol() {
        let md = "### Variable: greeting\n\nDefault value: `\"hi\"`\n";
        let idx = parse_markdown(md, "pkg", "doc/pkg.md");
        assert_eq!(idx.symbols.len(), 1);
        let sym = &idx.symbols["greeting"];
        assert_eq!(sym.symbol_type, "Variable");
        assert_eq!(sym.signature, "greeting");
        assert_eq!(sym.summary, "Default value: \"hi\"");
    }

    #[test]
    fn symbols_sorted_alphabetically() {
        let md = "### Function: zebra ()\n\nZ.\n\n### Function: alpha ()\n\nA.\n";
        let idx = parse_markdown(md, "pkg", "doc/pkg.md");
        let keys: Vec<&String> = idx.symbols.keys().collect();
        assert_eq!(keys, vec!["alpha", "zebra"]);
    }

    #[test]
    fn json_roundtrip() {
        let md = "## Intro\n\nHello.\n\n### Function: foo (x)\n\nDoes foo.\n\n```maxima\n(%i1) foo(1);\n(%o1) 42\n```\n\nSee also: `bar`.\n";
        let idx = parse_markdown(md, "test", "doc/test.md");
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
    fn hierarchical_sections() {
        let md = "\
# Core Docs

## Introduction

Some intro text.

## Calculus

## Differentiation

### Function: diff (expr, var)

Differentiates expr with respect to var.

## Integration

### Function: integrate (expr, var)

Integrates expr with respect to var.

## Programming

## Flow Control

### Function: if (cond, then, else)

Conditional expression.
";
        let idx = parse_markdown(md, "core", "doc/core.md");

        // Top-level sections: Introduction, Calculus (parent), Programming (parent)
        assert_eq!(idx.sections.len(), 3);
        assert_eq!(idx.sections[0].title, "Introduction");
        assert!(idx.sections[0].subsections.is_empty());

        assert_eq!(idx.sections[1].title, "Calculus");
        assert_eq!(idx.sections[1].subsections.len(), 2);
        assert_eq!(idx.sections[1].subsections[0].title, "Differentiation");
        assert_eq!(idx.sections[1].subsections[1].title, "Integration");

        assert_eq!(idx.sections[2].title, "Programming");
        assert_eq!(idx.sections[2].subsections.len(), 1);
        assert_eq!(idx.sections[2].subsections[0].title, "Flow Control");

        // Symbols have section associations
        assert_eq!(
            idx.symbols["diff"].section.as_deref(),
            Some("Differentiation")
        );
        assert_eq!(
            idx.symbols["integrate"].section.as_deref(),
            Some("Integration")
        );
        assert_eq!(idx.symbols["if"].section.as_deref(), Some("Flow Control"));
    }

    #[test]
    fn slim_strips_body_and_examples() {
        let md = "## Intro\n\nHello.\n\n### Function: foo (x)\n\nDoes foo in detail.\n\n```maxima\n(%i1) foo(1);\n(%o1) 42\n```\n\nSee also: `bar`.\n";
        let idx = parse_markdown(md, "test", "doc/test.md");
        let slim = idx.slim();

        assert_eq!(slim.symbols.len(), 1);
        let sym = &slim.symbols["foo"];
        assert_eq!(sym.signature, "foo(x)");
        assert_eq!(sym.summary, "Does foo in detail.");
        assert!(sym.body_md.is_empty());
        assert!(sym.examples.is_empty());
        assert!(sym.see_also.is_empty());
        assert!(slim.sections.is_empty());
    }

    #[test]
    fn metadata_comments_parsed() {
        let md = "\
<!-- category: Calculus -->
<!-- keywords: derivative, differentiation -->
<!-- signatures: diff(expr, var), diff(expr, var, n) -->
### Function: diff (expr, var)

Computes the derivative.
";
        let idx = parse_markdown(md, "test", "doc/test.md");
        let sym = &idx.symbols["diff"];
        assert_eq!(sym.category.as_deref(), Some("Calculus"));
        assert_eq!(sym.keywords, vec!["derivative", "differentiation"]);
        assert_eq!(
            sym.signatures,
            vec!["diff(expr, var)", "diff(expr, var, n)"]
        );
    }

    #[test]
    fn metadata_comments_stay_out_of_the_previous_body() {
        let md = "\
<!-- category: Statistics -->
<!-- keywords: pdf_student_t -->
### Function: pdf_student_t (x, n)

Student density.

<!-- category: Statistics -->
<!-- keywords: pdf_weibull -->
<!-- signatures: pdf_weibull(x, a, b) -->
### Function: pdf_weibull (x, a, b)

Weibull density.
";
        let idx = parse_markdown(md, "test", "doc/test.md");
        let student = &idx.symbols["pdf_student_t"];
        assert_eq!(student.body_md, "Student density.");
        let weibull = &idx.symbols["pdf_weibull"];
        assert_eq!(weibull.keywords, vec!["pdf_weibull"]);
        assert_eq!(weibull.signatures, vec!["pdf_weibull(x, a, b)"]);
        assert_eq!(weibull.body_md, "Weibull density.");
    }

    #[test]
    fn slim_preserves_keywords_and_signatures() {
        let md = "\
<!-- keywords: kw1 -->
<!-- signatures: foo(x), foo(x, y) -->
### Function: foo (x)

Does foo.
";
        let idx = parse_markdown(md, "test", "doc/test.md");
        let slim = idx.slim();
        let sym = &slim.symbols["foo"];
        assert_eq!(sym.keywords, vec!["kw1"]);
        assert_eq!(sym.signatures, vec!["foo(x)", "foo(x, y)"]);
    }
}

#[cfg(test)]
mod language_tests {
    use super::*;

    #[test]
    fn an_english_index_has_no_language_field() {
        let index = parse_markdown("### Function: f (x)\n\nDoes f.\n", "p", "p.md");
        let json = serde_json::to_value(&index).unwrap();
        assert!(json.get("language").is_none(), "{json}");
    }

    #[test]
    fn a_translated_index_names_its_language() {
        let mut index = parse_markdown("### Function: f (x)\n\nHace f.\n", "p", "p.md");
        index.language = "es".to_string();
        let json = serde_json::to_value(index.slim()).unwrap();
        assert_eq!(json["language"], "es");
        let read: DocIndex = serde_json::from_value(json).unwrap();
        assert_eq!(read.language, "es");
    }

    #[test]
    fn an_index_without_a_language_reads_as_english() {
        let read: DocIndex = serde_json::from_str(
            r#"{"version": 1, "package": "p", "source": "", "symbols": {}, "sections": []}"#,
        )
        .unwrap();
        assert_eq!(read.language, ENGLISH);
    }

    #[test]
    fn file_names_carry_the_language() {
        assert_eq!(doc_index_file_name("p", "en"), "p-doc-index.json");
        assert_eq!(doc_index_file_name("p", "es"), "p-doc-index.es.json");
        assert_eq!(
            slim_doc_index_file_name("p", "pt-BR"),
            "p-slim-doc-index.pt-BR.json"
        );
    }
}
