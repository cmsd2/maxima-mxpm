//! XML parsing and symbol extraction from Maxima's makeinfo XML output.

use std::collections::HashSet;

use crate::errors::MxpmError;

use super::ExtractedSymbol;
use super::emit::map_category;
use super::markdown::{
    collect_raw_text, collect_text, definition_body_to_markdown, replace_texinfo_entities,
};

/// Parse makeinfo XML and extract all symbol definitions.
pub(super) fn parse_xml(xml: &str) -> Result<Vec<ExtractedSymbol>, MxpmError> {
    let xml = replace_texinfo_entities(xml);
    let doc = roxmltree::Document::parse_with_options(
        &xml,
        roxmltree::ParsingOptions {
            allow_dtd: true,
            ..Default::default()
        },
    )
    .map_err(|e| MxpmError::MakeinfoFailed {
        message: format!("XML parse error: {e}"),
    })?;

    let mut symbols = Vec::new();
    let mut seen = HashSet::new();
    collect_definitions(doc.root_element(), &mut symbols, &mut seen, "", "");
    symbols.sort_by_key(|s| s.name.to_lowercase());
    Ok(symbols)
}

fn collect_definitions(
    node: roxmltree::Node,
    symbols: &mut Vec<ExtractedSymbol>,
    seen: &mut HashSet<String>,
    chapter_title: &str,
    section_title: &str,
) {
    let mut current_chapter = chapter_title.to_string();
    let mut current_section = section_title.to_string();

    for child in node.children() {
        if !child.is_element() {
            continue;
        }
        match child.tag_name().name() {
            "chapter" | "unnumbered" | "appendix" => {
                // Chapter-level element: update chapter title, reset section
                if let Some(title) = child
                    .children()
                    .find(|c| c.is_element() && c.tag_name().name() == "sectiontitle")
                {
                    current_chapter = collect_text(&title);
                    current_section = String::new();
                }
                collect_definitions(child, symbols, seen, &current_chapter, "");
            }
            "section" => {
                // Section-level element: update section title, keep chapter
                if let Some(title) = child
                    .children()
                    .find(|c| c.is_element() && c.tag_name().name() == "sectiontitle")
                {
                    current_section = collect_text(&title);
                }
                collect_definitions(child, symbols, seen, &current_chapter, &current_section);
            }
            "deffn" | "defvr" => {
                if let Some(sym) = parse_definition(&child, &current_chapter, &current_section)
                    && seen.insert(sym.name.clone())
                {
                    symbols.push(sym);
                }
            }
            _ => {
                collect_definitions(child, symbols, seen, &current_chapter, &current_section);
            }
        }
    }
}

fn parse_definition(
    node: &roxmltree::Node,
    chapter_title: &str,
    section_title: &str,
) -> Option<ExtractedSymbol> {
    let is_variable = node.tag_name().name() == "defvr";
    let symbol_type = if is_variable { "Variable" } else { "Function" };

    let name = extract_name(node)?;
    let signatures = extract_signatures(node, &name);
    let category = extract_category(node, chapter_title, section_title);
    let chapter = clean_chapter_name(chapter_title);

    let keywords = extract_keywords(node);

    // Find <definitionitem> for body content
    let def_item = node
        .children()
        .find(|c| c.is_element() && c.tag_name().name() == "definitionitem");

    let (body_md, examples, see_also) = if let Some(item) = def_item {
        let md = definition_body_to_markdown(&item);
        let examples = extract_examples(&item);
        let see_also = extract_see_also(&item);
        (md, examples, see_also)
    } else {
        (String::new(), Vec::new(), Vec::new())
    };

    Some(ExtractedSymbol {
        name,
        symbol_type: symbol_type.to_string(),
        signatures,
        body_md,
        _examples: examples,
        see_also,
        category,
        chapter,
        keywords,
    })
}

fn extract_name(node: &roxmltree::Node) -> Option<String> {
    // Look in <definitionterm> for <deffunction> or <defvariable>
    for child in node.children() {
        if child.is_element() && child.tag_name().name() == "definitionterm" {
            for term_child in child.children() {
                if term_child.is_element() {
                    match term_child.tag_name().name() {
                        "deffunction" | "defvariable" => {
                            let name = collect_text(&term_child).trim().to_string();
                            if !name.is_empty() {
                                return Some(name);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    // Fallback: look in <indexterm>
    for child in node.descendants() {
        if child.is_element() && child.tag_name().name() == "indexterm" {
            let name = collect_text(&child).trim().to_string();
            if !name.is_empty() {
                return Some(name);
            }
        }
    }
    None
}

fn extract_signatures(node: &roxmltree::Node, name: &str) -> Vec<String> {
    let mut signatures = Vec::new();

    // Collect from <definitionterm>
    for child in node.children() {
        if child.is_element() && child.tag_name().name() == "definitionterm" {
            signatures.extend(build_signatures_from_term(&child));
        }
    }

    // Also check <deffnx>/<defvrx> for alternative signatures
    for child in node.children() {
        if child.is_element() {
            match child.tag_name().name() {
                "deffnx" | "defvrx" => {
                    for term in child.children() {
                        if term.is_element() && term.tag_name().name() == "definitionterm" {
                            signatures.extend(build_signatures_from_term(&term));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Remove bare name if there are more specific signatures with arguments
    if signatures.len() > 1 {
        signatures.retain(|s| s != name);
    }

    // If no signatures found, use the bare name
    if signatures.is_empty() {
        signatures.push(name.to_string());
    }

    signatures
}

/// Build one or more signatures from a `<definitionterm>` element.
///
/// Maxima uses `@fname{}` to list multiple signature variants inside a single
/// `@deffn` block. In the XML this produces a single `<definitionterm>` where
/// each variant appears as a `<defparam>` containing the function name (from
/// `@fname`), preceded by a linebreak `<defparam>`, with its own `<defdelimiter>`
/// parens and `<defparam>` args.
///
/// We detect variant boundaries by looking for a `<defparam>` whose text
/// matches the function name — this indicates the start of a new `@fname`
/// variant. The initial `<deffunction>` element defines the primary signature.
///
/// Signatures are built incrementally by tracking `<defdelimiter>` characters
/// (`(`, `)`, `[`, `]`, `,`) so that bracket subscripts and list arguments
/// are faithfully preserved. Infix `<defparam>` tokens like `=` are joined
/// with spaces rather than commas.
/// Build one or more signatures from a `<definitionterm>` element.
///
/// Collects all `<deffunction>`/`<defvariable>`, `<defdelimiter>`, and
/// `<defparam>`/`<var>` children as text parts, concatenated in document
/// order. Empty `<defparam>` elements (linebreak separators from `@fname`
/// macros) split the parts into separate signatures.
///
/// This mirrors the approach used by catalog-gen: no special handling of
/// brackets, commas, or infix operators — the XML already contains the
/// correct delimiters and they are joined literally.
fn build_signatures_from_term(term: &roxmltree::Node) -> Vec<String> {
    // None = linebreak separator between signature variants
    let mut parts: Vec<Option<String>> = Vec::new();
    let mut found_name = false;

    let func_tag = if term
        .parent()
        .is_some_and(|p| matches!(p.tag_name().name(), "deffn" | "deffnx"))
    {
        "deffunction"
    } else {
        "defvariable"
    };

    for child in term.children() {
        if child.is_element() {
            match child.tag_name().name() {
                t if t == func_tag => {
                    parts.push(Some(collect_text(&child).trim().to_string()));
                    found_name = true;
                }
                "defdelimiter" => {
                    let delim = collect_text(&child).trim().to_string();
                    if delim == "," {
                        parts.push(Some(", ".to_string()));
                    } else {
                        parts.push(Some(delim));
                    }
                }
                "defparam" | "var" => {
                    let text = collect_text(&child);
                    let trimmed = text.trim();
                    if trimmed.is_empty() {
                        parts.push(None); // linebreak separator
                    } else {
                        parts.push(Some(trimmed.to_string()));
                    }
                }
                "indexterm" | "defcategory" => {
                    // Skip metadata elements
                }
                _ => {}
            }
        }
    }

    if !found_name && parts.iter().all(|p| p.is_none()) {
        return Vec::new();
    }

    // Split parts on None separators into individual signatures
    let mut sigs = Vec::new();
    let mut current = Vec::new();

    for part in parts {
        match part {
            Some(s) => current.push(s),
            None => {
                if !current.is_empty() {
                    sigs.push(current.join(""));
                    current = Vec::new();
                }
            }
        }
    }
    if !current.is_empty() {
        sigs.push(current.join(""));
    }

    sigs.retain(|s| !s.trim().is_empty());
    sigs
}

fn extract_category(_node: &roxmltree::Node, chapter_title: &str, section_title: &str) -> String {
    // <defcategory> in Maxima's XML contains the definition type ("Function",
    // "Variable", etc.) — not a topical category. So we always fall through
    // to the chapter/section title for topical classification.
    //
    // Prefer the section title (more specific), falling back to the chapter title.
    if !section_title.is_empty() {
        let mapped = map_category(section_title);
        if mapped != "Other" {
            return mapped;
        }
    }
    map_category(chapter_title)
}

/// Extract keyword index entries from `<indexterm>`/`<cindex>` descendants within a definition node.
fn extract_keywords(node: &roxmltree::Node) -> Vec<String> {
    let mut keywords = Vec::new();
    let mut seen = HashSet::new();
    for child in node.descendants() {
        if child.is_element() {
            match child.tag_name().name() {
                "indexterm" | "cindex" => {
                    let text = collect_text(&child).trim().to_string();
                    if !text.is_empty() && seen.insert(text.clone()) {
                        keywords.push(text);
                    }
                }
                _ => {}
            }
        }
    }
    keywords
}

/// Strip common prefixes from a Maxima chapter title to get a clean subcategory name.
fn clean_chapter_name(raw: &str) -> String {
    raw.strip_prefix("Functions and Variables for ")
        .or_else(|| raw.strip_prefix("Introduction to "))
        .or_else(|| raw.strip_prefix("Package "))
        .or_else(|| raw.strip_prefix("Definitions for "))
        .unwrap_or(raw)
        .to_string()
}

fn extract_examples(node: &roxmltree::Node) -> Vec<(String, String)> {
    let mut examples = Vec::new();
    for child in node.descendants() {
        if child.is_element() && child.tag_name().name() == "example" {
            let text = collect_raw_text(&child);
            let parsed = parse_repl_examples(&text);
            for (input, output) in parsed {
                examples.push((input, output));
                if examples.len() >= 5 {
                    return examples;
                }
            }
        }
    }
    examples
}

fn extract_see_also(node: &roxmltree::Node) -> Vec<String> {
    let mut refs = Vec::new();
    for child in node.descendants() {
        if child.is_element() {
            match child.tag_name().name() {
                "ref" | "xref" | "pxref" => {
                    if let Some(label) = child.attribute("label") {
                        // Decode URL-encoded label (e.g. _005f -> _)
                        let name = decode_texinfo_label(label.trim());
                        if !name.is_empty() && !refs.contains(&name) {
                            refs.push(name);
                        }
                    }
                }
                _ => {}
            }
        }
    }
    refs
}

/// Parse REPL-style examples from text: `(%i1)` input; `(%o1)` output.
pub(super) fn parse_repl_examples(text: &str) -> Vec<(String, String)> {
    let mut examples = Vec::new();
    let mut current_input = String::new();
    let mut current_output = String::new();
    let mut in_output = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = strip_repl_marker(trimmed, "%i") {
            // Flush previous
            if !current_input.is_empty() {
                examples.push((
                    current_input.trim().to_string(),
                    current_output.trim().to_string(),
                ));
            }
            current_input = rest.to_string();
            current_output.clear();
            in_output = false;
        } else if let Some(rest) = strip_repl_marker(trimmed, "%o") {
            current_output = rest.to_string();
            in_output = true;
        } else if in_output && !trimmed.is_empty() {
            current_output.push('\n');
            current_output.push_str(trimmed);
        } else if !in_output && !current_input.is_empty() && !trimmed.is_empty() {
            // Continuation of input
            current_input.push('\n');
            current_input.push_str(trimmed);
        }
    }
    // Flush last
    if !current_input.is_empty() {
        examples.push((
            current_input.trim().to_string(),
            current_output.trim().to_string(),
        ));
    }
    examples
}

fn strip_repl_marker<'a>(line: &'a str, marker: &str) -> Option<&'a str> {
    // Match patterns like (%i1), (%i2), (%o1), etc.
    if let Some(pos) = line.find(&format!("({marker}"))
        && let Some(end) = line[pos..].find(')')
    {
        let rest = line[pos + end + 1..].trim();
        return Some(rest);
    }
    None
}

/// Decode Texinfo URL-encoded labels (e.g. `_005f` -> `_`, `_0025` -> `%`).
fn decode_texinfo_label(label: &str) -> String {
    let mut result = String::with_capacity(label.len());
    let mut chars = label.chars();
    while let Some(c) = chars.next() {
        if c == '_' {
            // Try to read 4 hex digits
            let hex: String = chars.by_ref().take(4).collect();
            if hex.len() == 4
                && let Ok(code) = u32::from_str_radix(&hex, 16)
                && let Some(decoded) = char::from_u32(code)
            {
                result.push(decoded);
                continue;
            }
            // Not a valid encoding — keep as-is
            result.push('_');
            result.push_str(&hex);
        } else {
            result.push(c);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_repl_examples() {
        let text = "(%i1) diff(x^3, x);\n(%o1)                           3 x^2\n(%i2) integrate(x, x);\n(%o2)                           x^2/2\n";
        let examples = parse_repl_examples(text);
        assert_eq!(examples.len(), 2);
        assert_eq!(examples[0].0, "diff(x^3, x);");
        assert_eq!(examples[0].1, "3 x^2");
        assert_eq!(examples[1].0, "integrate(x, x);");
        assert_eq!(examples[1].1, "x^2/2");
    }

    #[test]
    fn test_strip_repl_marker() {
        assert_eq!(strip_repl_marker("(%i1) foo;", "%i"), Some("foo;"));
        assert_eq!(strip_repl_marker("(%o1)  bar", "%o"), Some("bar"));
        assert_eq!(strip_repl_marker("no marker", "%i"), None);
    }

    #[test]
    fn test_decode_texinfo_label() {
        assert_eq!(decode_texinfo_label("zn_005fprimroot"), "zn_primroot");
        assert_eq!(decode_texinfo_label("_0025_0025"), "%%");
        assert_eq!(decode_texinfo_label("simple"), "simple");
        assert_eq!(decode_texinfo_label("_003d"), "=");
    }
}
