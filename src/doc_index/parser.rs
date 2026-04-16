//! Markdown section splitting and content extraction.

use regex::Regex;

use super::ExampleEntry;

// ---------------------------------------------------------------------------
// Intermediate parse types
// ---------------------------------------------------------------------------

pub(crate) enum ParsedBlock {
    Section { title: String, body: String },
    Symbol { kind: SymbolKind, body: String },
}

pub(crate) struct SymbolKind {
    pub name: String,
    pub symbol_type: String,
    pub signature: String,
}

// ---------------------------------------------------------------------------
// Section splitter
// ---------------------------------------------------------------------------

pub(crate) fn split_into_blocks(content: &str) -> Vec<ParsedBlock> {
    let func_re = Regex::new(r"^### Function:\s+(\S+)\s*\((.*?)\)\s*$").unwrap();
    let var_re = Regex::new(r"^### Variable:\s+(\S+)\s*$").unwrap();
    let section_re = Regex::new(r"^## (.+)$").unwrap();

    let mut blocks = Vec::new();
    let mut current: Option<ParsedBlock> = None;
    let mut body = String::new();

    for line in content.lines() {
        // Check for ## section heading
        if let Some(caps) = section_re.captures(line) {
            flush_block(&mut current, &mut body, &mut blocks);
            current = Some(ParsedBlock::Section {
                title: caps[1].trim().to_string(),
                body: String::new(),
            });
            continue;
        }

        // Check for ### Function: heading
        if let Some(caps) = func_re.captures(line) {
            flush_block(&mut current, &mut body, &mut blocks);
            let name = caps[1].to_string();
            let args = caps[2].to_string();
            let signature = if args.is_empty() {
                format!("{name}()")
            } else {
                format!("{name}({args})")
            };
            current = Some(ParsedBlock::Symbol {
                kind: SymbolKind {
                    name,
                    symbol_type: "Function".to_string(),
                    signature,
                },
                body: String::new(),
            });
            continue;
        }

        // Check for ### Variable: heading
        if let Some(caps) = var_re.captures(line) {
            flush_block(&mut current, &mut body, &mut blocks);
            let name = caps[1].to_string();
            current = Some(ParsedBlock::Symbol {
                kind: SymbolKind {
                    name: name.clone(),
                    symbol_type: "Variable".to_string(),
                    signature: name,
                },
                body: String::new(),
            });
            continue;
        }

        // Skip the top-level # heading
        if line.starts_with("# ") && current.is_none() {
            continue;
        }

        // Accumulate body lines
        if current.is_some() {
            body.push_str(line);
            body.push('\n');
        }
    }

    flush_block(&mut current, &mut body, &mut blocks);
    blocks
}

fn flush_block(
    current: &mut Option<ParsedBlock>,
    body: &mut String,
    blocks: &mut Vec<ParsedBlock>,
) {
    if let Some(mut block) = current.take() {
        let b = std::mem::take(body);
        match &mut block {
            ParsedBlock::Section { body: bb, .. } => *bb = b,
            ParsedBlock::Symbol { body: bb, .. } => *bb = b,
        }
        blocks.push(block);
    }
    body.clear();
}

// ---------------------------------------------------------------------------
// Summary extraction
// ---------------------------------------------------------------------------

/// Extract the first paragraph as a plain-text summary.
pub(crate) fn extract_summary(body_md: &str) -> String {
    let mut lines = Vec::new();
    let mut started = false;

    for line in body_md.lines() {
        let trimmed = line.trim();
        if !started {
            if trimmed.is_empty() {
                continue;
            }
            // Skip fenced code blocks at the very start
            if trimmed.starts_with("```") {
                break;
            }
            started = true;
        }

        if started {
            if trimmed.is_empty() {
                break;
            }
            lines.push(trimmed);
        }
    }

    let raw = lines.join(" ");
    strip_inline_markdown(&raw)
}

/// Remove inline markdown formatting for plain text.
fn strip_inline_markdown(s: &str) -> String {
    let re_code = Regex::new(r"`([^`]+)`").unwrap();
    let s = re_code.replace_all(s, "$1");
    let re_bold = Regex::new(r"\*\*([^*]+)\*\*").unwrap();
    let s = re_bold.replace_all(&s, "$1");
    let re_italic = Regex::new(r"\*([^*]+)\*").unwrap();
    let s = re_italic.replace_all(&s, "$1");
    s.to_string()
}

// ---------------------------------------------------------------------------
// Example extraction
// ---------------------------------------------------------------------------

/// Extract structured examples from fenced `maxima` code blocks.
pub(crate) fn extract_examples(body_md: &str) -> Vec<ExampleEntry> {
    let mut examples = Vec::new();
    let mut in_block = false;
    let mut is_maxima = false;
    let mut block_lines: Vec<String> = Vec::new();

    for line in body_md.lines() {
        if !in_block {
            if line.trim().starts_with("```") {
                let lang = line.trim().trim_start_matches('`').trim();
                is_maxima = lang.eq_ignore_ascii_case("maxima") || lang.is_empty();
                in_block = true;
                block_lines.clear();
            }
        } else if line.trim() == "```" {
            in_block = false;
            if is_maxima && !block_lines.is_empty() {
                let block_content = block_lines.join("\n");
                if block_content.contains("(%i") {
                    examples.extend(parse_io_examples(&block_content));
                } else if !block_content.trim().is_empty() {
                    examples.push(ExampleEntry {
                        input: block_content.trim().to_string(),
                        output: String::new(),
                        description: String::new(),
                    });
                }
            }
        } else if in_block {
            block_lines.push(line.to_string());
        }
    }

    examples
}

/// Parse `(%i1)` / `(%o1)` style examples into input/output pairs.
fn parse_io_examples(content: &str) -> Vec<ExampleEntry> {
    let mut examples = Vec::new();
    let input_re = Regex::new(r"^\(%i\d+\)\s*(.*)$").unwrap();
    let output_re = Regex::new(r"^\(%o\d+\)\s*(.*)$").unwrap();

    let mut current_input = String::new();
    let mut current_output = String::new();
    let mut in_input = false;
    let mut in_output = false;

    for line in content.lines() {
        if let Some(caps) = input_re.captures(line) {
            if !current_input.is_empty() {
                examples.push(ExampleEntry {
                    input: current_input.trim().to_string(),
                    output: current_output.trim().to_string(),
                    description: String::new(),
                });
            }
            current_input = caps[1].to_string();
            current_output.clear();
            in_input = true;
            in_output = false;
        } else if let Some(caps) = output_re.captures(line) {
            current_output = caps[1].to_string();
            in_input = false;
            in_output = true;
        } else if in_output {
            current_output.push('\n');
            current_output.push_str(line);
        } else if in_input {
            current_input.push('\n');
            current_input.push_str(line);
        }
    }

    if !current_input.is_empty() {
        examples.push(ExampleEntry {
            input: current_input.trim().to_string(),
            output: current_output.trim().to_string(),
            description: String::new(),
        });
    }

    examples
}

// ---------------------------------------------------------------------------
// See-also extraction
// ---------------------------------------------------------------------------

/// Extract cross-references from a trailing "See also:" line.
pub(crate) fn extract_see_also(body_md: &str) -> Vec<String> {
    let see_also_re = Regex::new(r"(?i)^See also:\s*(.+)$").unwrap();
    let name_re = Regex::new(r"`([^`]+)`").unwrap();

    for line in body_md.lines().rev() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(caps) = see_also_re.captures(trimmed) {
            return name_re
                .captures_iter(&caps[1])
                .map(|c| c[1].to_string())
                .collect();
        }
        break;
    }

    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_first_paragraph() {
        let body = "First line of summary.\nStill first paragraph.\n\nSecond paragraph.\n";
        let summary = extract_summary(body);
        assert_eq!(summary, "First line of summary. Still first paragraph.");
    }

    #[test]
    fn examples_with_io_markers() {
        let body = "Some text.\n\n```maxima\n(%i1) solve(x^2 - 1, x);\n(%o1)                      [x = -1, x = 1]\n```\n";
        let examples = extract_examples(body);
        assert_eq!(examples.len(), 1);
        assert_eq!(examples[0].input, "solve(x^2 - 1, x);");
        assert_eq!(examples[0].output, "[x = -1, x = 1]");
    }

    #[test]
    fn examples_without_markers() {
        let body = "Some text.\n\n```maxima\nmy_func(x, y)$\n```\n";
        let examples = extract_examples(body);
        assert_eq!(examples.len(), 1);
        assert_eq!(examples[0].input, "my_func(x, y)$");
        assert!(examples[0].output.is_empty());
    }

    #[test]
    fn examples_multiple_io_pairs() {
        let body = "```maxima\n(%i1) 2+2;\n(%o1) 4\n(%i2) 3*3;\n(%o2) 9\n```\n";
        let examples = extract_examples(body);
        assert_eq!(examples.len(), 2);
        assert_eq!(examples[0].input, "2+2;");
        assert_eq!(examples[0].output, "4");
        assert_eq!(examples[1].input, "3*3;");
        assert_eq!(examples[1].output, "9");
    }

    #[test]
    fn see_also_basic() {
        let body = "Some text.\n\nSee also: `foo`, `bar`.\n";
        let refs = extract_see_also(body);
        assert_eq!(refs, vec!["foo", "bar"]);
    }

    #[test]
    fn see_also_cross_package() {
        let body = "Some text.\n\nSee also: `diophantine:dio_solve`.\n";
        let refs = extract_see_also(body);
        assert_eq!(refs, vec!["diophantine:dio_solve"]);
    }

    #[test]
    fn see_also_none() {
        let body = "Just some text with no see-also line.\n";
        let refs = extract_see_also(body);
        assert!(refs.is_empty());
    }
}
