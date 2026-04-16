//! Lint checks for the parsed doc index.

use super::DocIndex;

/// Severity level for a lint warning.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum LintLevel {
    Warn,
    Info,
}

/// A single lint warning about the documentation.
#[derive(Debug)]
pub(crate) struct LintWarning {
    /// The symbol name, or `None` for document-level warnings.
    pub symbol: Option<String>,
    pub level: LintLevel,
    pub message: String,
}

/// Run lint checks on a parsed doc index.
pub(crate) fn lint_doc_index(index: &DocIndex) -> Vec<LintWarning> {
    let mut warnings = Vec::new();
    let symbol_names: Vec<&String> = index.symbols.keys().collect();

    for (name, entry) in &index.symbols {
        // Empty body
        if entry.body_md.trim().is_empty() {
            warnings.push(LintWarning {
                symbol: Some(name.clone()),
                level: LintLevel::Warn,
                message: "symbol has no documentation body".to_string(),
            });
            continue;
        }

        // Empty summary
        if entry.summary.is_empty() {
            warnings.push(LintWarning {
                symbol: Some(name.clone()),
                level: LintLevel::Warn,
                message: "symbol has no summary (first paragraph is empty or a code block)"
                    .to_string(),
            });
        } else if entry.summary.starts_with('`')
            || entry.summary.starts_with("ax_")
            || looks_like_signature(&entry.summary, name)
        {
            // Summary looks like a signature line
            warnings.push(LintWarning {
                symbol: Some(name.clone()),
                level: LintLevel::Warn,
                message: format!(
                    "summary looks like a signature, not a description: \"{}\"",
                    truncate(&entry.summary, 60)
                ),
            });
        }

        // No examples
        if entry.examples.is_empty() {
            warnings.push(LintWarning {
                symbol: Some(name.clone()),
                level: LintLevel::Info,
                message: "no examples found".to_string(),
            });
        }

        // No see_also
        if entry.see_also.is_empty() {
            warnings.push(LintWarning {
                symbol: Some(name.clone()),
                level: LintLevel::Info,
                message: "no cross-references (See also:)".to_string(),
            });
        }

        // Unknown see_also references
        for ref_name in &entry.see_also {
            // Skip cross-package references (contain ':')
            if ref_name.contains(':') {
                continue;
            }
            if !symbol_names.contains(&ref_name) {
                warnings.push(LintWarning {
                    symbol: Some(name.clone()),
                    level: LintLevel::Warn,
                    message: format!(
                        "See also references unknown symbol '{ref_name}' (may be external)"
                    ),
                });
            }
        }
    }

    warnings
}

/// Heuristic: does this summary text look like a function signature?
fn looks_like_signature(summary: &str, symbol_name: &str) -> bool {
    let s = summary.trim();
    // Starts with the symbol name followed by '('
    s.starts_with(&format!("{symbol_name}("))
        // Or starts with a backtick-quoted name
        || s.starts_with('`')
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::doc_index::{ExampleEntry, SymbolEntry};

    fn make_symbol(summary: &str, examples: usize, see_also: Vec<&str>) -> SymbolEntry {
        SymbolEntry {
            symbol_type: "Function".to_string(),
            signature: "test()".to_string(),
            summary: summary.to_string(),
            body_md: if summary.is_empty() {
                String::new()
            } else {
                summary.to_string()
            },
            examples: (0..examples)
                .map(|_| ExampleEntry {
                    input: "1+1;".to_string(),
                    output: "2".to_string(),
                    description: String::new(),
                })
                .collect(),
            see_also: see_also.into_iter().map(String::from).collect(),
            category: None,
            section: None,
        }
    }

    #[test]
    fn clean_symbol_no_warnings() {
        let mut index = DocIndex {
            version: 1,
            package: "test".to_string(),
            source: "test.md".to_string(),
            symbols: BTreeMap::new(),
            sections: Vec::new(),
        };
        index.symbols.insert(
            "foo".to_string(),
            make_symbol("Does something useful.", 1, vec!["bar"]),
        );
        index.symbols.insert(
            "bar".to_string(),
            make_symbol("Also useful.", 1, vec!["foo"]),
        );

        let warnings: Vec<_> = lint_doc_index(&index)
            .into_iter()
            .filter(|w| w.level == LintLevel::Warn)
            .collect();
        assert!(warnings.is_empty(), "expected no warnings: {warnings:?}");
    }

    #[test]
    fn signature_as_summary() {
        let mut index = DocIndex {
            version: 1,
            package: "test".to_string(),
            source: "test.md".to_string(),
            symbols: BTreeMap::new(),
            sections: Vec::new(),
        };
        index.symbols.insert(
            "foo".to_string(),
            make_symbol("`foo(x, y)` `foo(x)`", 1, vec![]),
        );

        let warnings: Vec<_> = lint_doc_index(&index)
            .into_iter()
            .filter(|w| w.message.contains("signature"))
            .collect();
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn unknown_see_also() {
        let mut index = DocIndex {
            version: 1,
            package: "test".to_string(),
            source: "test.md".to_string(),
            symbols: BTreeMap::new(),
            sections: Vec::new(),
        };
        index.symbols.insert(
            "foo".to_string(),
            make_symbol("Does foo.", 1, vec!["nonexistent"]),
        );

        let warnings: Vec<_> = lint_doc_index(&index)
            .into_iter()
            .filter(|w| w.message.contains("unknown symbol"))
            .collect();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].message.contains("nonexistent"));
    }

    #[test]
    fn cross_package_ref_not_flagged() {
        let mut index = DocIndex {
            version: 1,
            package: "test".to_string(),
            source: "test.md".to_string(),
            symbols: BTreeMap::new(),
            sections: Vec::new(),
        };
        index.symbols.insert(
            "foo".to_string(),
            make_symbol("Does foo.", 1, vec!["other_pkg:bar"]),
        );

        let warnings: Vec<_> = lint_doc_index(&index)
            .into_iter()
            .filter(|w| w.message.contains("unknown symbol"))
            .collect();
        assert!(warnings.is_empty());
    }
}
