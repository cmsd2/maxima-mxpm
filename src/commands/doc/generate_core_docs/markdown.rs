//! XML element to Markdown conversion and text helpers.

/// Convert a `<definitionitem>` node's children to markdown.
pub(super) fn definition_body_to_markdown(node: &roxmltree::Node) -> String {
    let mut out = String::new();
    convert_children_to_md(node, &mut out, 0);
    out.trim().to_string()
}

pub(super) fn convert_children_to_md(node: &roxmltree::Node, out: &mut String, list_depth: usize) {
    for child in node.children() {
        if child.is_text() {
            if let Some(text) = child.text() {
                out.push_str(text);
            }
        } else if child.is_element() {
            convert_element_to_md(&child, out, list_depth);
        }
    }
}

fn convert_element_to_md(node: &roxmltree::Node, out: &mut String, list_depth: usize) {
    match node.tag_name().name() {
        "para" => {
            if !out.is_empty() && !out.ends_with('\n') {
                out.push('\n');
            }
            if !out.is_empty() && !out.ends_with("\n\n") {
                out.push('\n');
            }
            convert_children_to_md(node, out, list_depth);
            out.push('\n');
        }
        "example" | "smallexample" => {
            if !out.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("\n```maxima\n");
            let text = collect_raw_text(node);
            out.push_str(text.trim());
            out.push_str("\n```\n");
        }
        "code" | "kbd" | "samp" | "command" | "env" | "file" | "option" => {
            out.push('`');
            let text = collect_text(node);
            out.push_str(text.trim());
            out.push('`');
        }
        "var" | "emph" | "i" | "dfn" => {
            out.push('*');
            convert_children_to_md(node, out, list_depth);
            out.push('*');
        }
        "b" | "strong" => {
            out.push_str("**");
            convert_children_to_md(node, out, list_depth);
            out.push_str("**");
        }
        "math" => {
            out.push('$');
            let text = collect_text(node);
            out.push_str(text.trim());
            out.push('$');
        }
        "sc" => {
            let text = collect_text(node);
            out.push_str(&text.to_uppercase());
        }
        "ref" | "xref" | "pxref" => {
            if let Some(label) = node.attribute("label") {
                out.push('`');
                out.push_str(label);
                out.push('`');
            } else {
                let text = collect_text(node);
                out.push('`');
                out.push_str(text.trim());
                out.push('`');
            }
        }
        "uref" | "url" => {
            let url = node.attribute("url").unwrap_or("");
            let desc = collect_text(node);
            let desc = desc.trim();
            if desc.is_empty() || desc == url {
                out.push_str(url);
            } else {
                out.push('[');
                out.push_str(desc);
                out.push_str("](");
                out.push_str(url);
                out.push(')');
            }
        }
        "itemize" => {
            if !out.ends_with('\n') {
                out.push('\n');
            }
            out.push('\n');
            for item in node.children() {
                if item.is_element() && item.tag_name().name() == "listitem" {
                    let indent = "  ".repeat(list_depth);
                    out.push_str(&indent);
                    out.push_str("- ");
                    let mut item_text = String::new();
                    convert_children_to_md(&item, &mut item_text, list_depth + 1);
                    let trimmed = item_text.trim();
                    // Remove leading paragraph breaks within list items
                    out.push_str(&trimmed.replace("\n\n", "\n"));
                    out.push('\n');
                }
            }
        }
        "enumerate" => {
            if !out.ends_with('\n') {
                out.push('\n');
            }
            out.push('\n');
            let mut n = 1;
            for item in node.children() {
                if item.is_element() && item.tag_name().name() == "listitem" {
                    let indent = "  ".repeat(list_depth);
                    out.push_str(&format!("{indent}{n}. "));
                    let mut item_text = String::new();
                    convert_children_to_md(&item, &mut item_text, list_depth + 1);
                    out.push_str(item_text.trim());
                    out.push('\n');
                    n += 1;
                }
            }
        }
        "multitable" => {
            convert_multitable(node, out);
        }
        "table" => {
            // Texinfo @table — render as definition list
            if !out.ends_with('\n') {
                out.push('\n');
            }
            out.push('\n');
            for entry in node.children() {
                if entry.is_element() && entry.tag_name().name() == "tableentry" {
                    let mut term = String::new();
                    let mut desc = String::new();
                    for part in entry.children() {
                        if part.is_element() {
                            match part.tag_name().name() {
                                "tableterm" => {
                                    let mut t = String::new();
                                    convert_children_to_md(&part, &mut t, list_depth);
                                    term = t.trim().to_string();
                                }
                                "tableitem" => {
                                    convert_children_to_md(&part, &mut desc, list_depth);
                                }
                                _ => {}
                            }
                        }
                    }
                    out.push_str(&format!("**{}**", term));
                    let desc_trimmed = desc.trim();
                    if !desc_trimmed.is_empty() {
                        out.push_str(" — ");
                        out.push_str(&desc_trimmed.replace("\n\n", " "));
                    }
                    out.push('\n');
                }
            }
        }
        "quotation" => {
            if !out.ends_with('\n') {
                out.push('\n');
            }
            let mut inner = String::new();
            convert_children_to_md(node, &mut inner, list_depth);
            for line in inner.trim().lines() {
                out.push_str("> ");
                out.push_str(line);
                out.push('\n');
            }
        }
        "html" => {
            // Raw Texinfo in XML — handle figures and math
            let text = collect_text(node);
            convert_raw_texinfo(&text, out);
        }
        "pre" => {
            let text = collect_raw_text(node);
            out.push_str(text.trim());
        }
        "group" => {
            convert_children_to_md(node, out, list_depth);
        }
        "image" => {
            // <image><imagefile>figures/name</imagefile><imagewidth>8cm</imagewidth></image>
            let file = node
                .children()
                .find(|c| c.is_element() && c.tag_name().name() == "imagefile")
                .map(|c| collect_text(&c))
                .unwrap_or_default();
            if !file.is_empty() {
                out.push_str(&format!("![{file}]({file}.png)"));
            }
        }
        "anchor" | "indexterm" | "cindex" | "findex" | "vindex" | "tindex" | "pindex"
        | "kindex" => {
            // Skip index entries
        }
        _ => {
            // Unknown element — recurse into children
            convert_children_to_md(node, out, list_depth);
        }
    }
}

fn convert_multitable(node: &roxmltree::Node, out: &mut String) {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut has_header = false;

    for child in node.children() {
        if !child.is_element() {
            continue;
        }
        match child.tag_name().name() {
            "thead" => {
                has_header = true;
                for row_node in child.children() {
                    if row_node.is_element() && row_node.tag_name().name() == "row" {
                        rows.push(extract_table_row(&row_node));
                    }
                }
            }
            "tbody" => {
                for row_node in child.children() {
                    if row_node.is_element() && row_node.tag_name().name() == "row" {
                        rows.push(extract_table_row(&row_node));
                    }
                }
            }
            "row" => {
                rows.push(extract_table_row(&child));
            }
            _ => {}
        }
    }

    if rows.is_empty() {
        return;
    }

    // Normalize column count
    let max_cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    for row in &mut rows {
        while row.len() < max_cols {
            row.push(String::new());
        }
    }

    if !out.ends_with('\n') {
        out.push('\n');
    }
    out.push('\n');

    // Write header
    if has_header && !rows.is_empty() {
        let header = &rows[0];
        out.push_str("| ");
        out.push_str(&header.join(" | "));
        out.push_str(" |\n");
        out.push_str("| ");
        out.push_str(
            &header
                .iter()
                .map(|_| "---".to_string())
                .collect::<Vec<_>>()
                .join(" | "),
        );
        out.push_str(" |\n");
        for row in &rows[1..] {
            out.push_str("| ");
            out.push_str(&row.join(" | "));
            out.push_str(" |\n");
        }
    } else {
        // No header — use empty header row
        if let Some(first) = rows.first() {
            out.push_str("| ");
            out.push_str(
                &(0..max_cols)
                    .map(|_| " ".to_string())
                    .collect::<Vec<_>>()
                    .join(" | "),
            );
            out.push_str(" |\n| ");
            out.push_str(
                &(0..max_cols)
                    .map(|_| "---".to_string())
                    .collect::<Vec<_>>()
                    .join(" | "),
            );
            out.push_str(" |\n");
            for row in &rows {
                out.push_str("| ");
                out.push_str(&row.join(" | "));
                out.push_str(" |\n");
            }
            let _ = first; // suppress unused warning
        }
    }
}

fn extract_table_row(node: &roxmltree::Node) -> Vec<String> {
    let mut cells = Vec::new();
    for child in node.children() {
        if child.is_element() && child.tag_name().name() == "entry" {
            let mut cell = String::new();
            convert_children_to_md(&child, &mut cell, 0);
            cells.push(cell.trim().replace('\n', " "));
        }
    }
    cells
}

pub(super) fn convert_raw_texinfo(text: &str, out: &mut String) {
    let text = text.trim();

    // Handle @math{...}
    if let Some(rest) = text.strip_prefix("@math{")
        && let Some(content) = rest.strip_suffix('}')
    {
        out.push('$');
        out.push_str(content);
        out.push('$');
        return;
    }

    // Handle @displaymath
    if text.starts_with("@displaymath") {
        let inner = text
            .strip_prefix("@displaymath")
            .unwrap_or("")
            .strip_suffix("@end displaymath")
            .unwrap_or(text)
            .trim();
        out.push_str("\n$$");
        out.push_str(inner);
        out.push_str("$$\n");
        return;
    }

    // Handle figure references: (Figure name) or (Figure name: desc)
    if let Some(fig) = text.strip_prefix("(Figure ")
        && let Some(name) = fig.strip_suffix(')')
    {
        let (file, desc) = if let Some((f, d)) = name.split_once(':') {
            (f.trim(), d.trim())
        } else {
            (name.trim(), name.trim())
        };
        out.push_str(&format!("![{desc}](figures/{file}.png)"));
        return;
    }

    // Otherwise strip remaining @command{content} patterns
    let cleaned = clean_texinfo_markup(text);
    out.push_str(&cleaned);
}

pub(super) fn clean_texinfo_markup(text: &str) -> String {
    let mut result = text.to_string();
    // @command{content} -> content
    let re = regex::Regex::new(r"@\w+\{([^}]*)\}").unwrap();
    loop {
        let new = re.replace_all(&result, "$1").to_string();
        if new == result {
            break;
        }
        result = new;
    }
    result.replace("@@", "@")
}

// ---------------------------------------------------------------------------
// Text collection helpers
// ---------------------------------------------------------------------------

/// Collect all text from a node, normalizing whitespace.
pub(super) fn collect_text(node: &roxmltree::Node) -> String {
    let mut text = String::new();
    for desc in node.descendants() {
        if desc.is_text()
            && let Some(t) = desc.text()
        {
            text.push_str(t);
        }
    }
    // Normalize whitespace
    let parts: Vec<&str> = text.split_whitespace().collect();
    parts.join(" ")
}

/// Collect all text from a node, preserving whitespace.
pub(super) fn collect_raw_text(node: &roxmltree::Node) -> String {
    let mut text = String::new();
    for desc in node.descendants() {
        if desc.is_text()
            && let Some(t) = desc.text()
        {
            text.push_str(t);
        }
    }
    text
}

// ---------------------------------------------------------------------------
// Entity replacement
// ---------------------------------------------------------------------------

/// Replace Texinfo-specific XML entities that roxmltree doesn't handle.
pub(super) fn replace_texinfo_entities(xml: &str) -> String {
    xml.replace("&arobase;", "@")
        .replace("&lbrace;", "{")
        .replace("&rbrace;", "}")
        .replace("&lbracechar;", "{")
        .replace("&rbracechar;", "}")
        .replace("&atchar;", "@")
        .replace("&bsol;", "\\")
        .replace("&dots;", "...")
        .replace("&enddots;", "...")
        .replace("&comma;", ",")
        .replace("&linebreak;", "\n")
        .replace("&hyphenbreak;", "\u{00AD}")
        .replace("&szlig;", "\u{00DF}")
        .replace("&tex;", "TeX")
        .replace("&textldquo;", "\u{201c}")
        .replace("&textrdquo;", "\u{201d}")
        .replace("&textlsquo;", "\u{2018}")
        .replace("&textrsquo;", "\u{2019}")
        .replace("&textmdash;", "\u{2014}")
        .replace("&textndash;", "\u{2013}")
        .replace("&rarr;", "->")
        .replace("&rArr;", "=>")
        .replace("&euro;", "EUR")
        .replace("&pound;", "GBP")
        .replace("&bullet;", "*")
        .replace("&minus;", "-")
        .replace("&nbsp;", " ")
        .replace("&period;", ".")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_replacement() {
        let xml = "&arobase;foo &lbrace;bar&rbrace; &dots; &rarr;";
        let result = replace_texinfo_entities(xml);
        assert_eq!(result, "@foo {bar} ... ->");
    }

    #[test]
    fn test_clean_texinfo_markup() {
        assert_eq!(clean_texinfo_markup("@code{foo}"), "foo");
        assert_eq!(clean_texinfo_markup("@var{x}"), "x");
        assert_eq!(clean_texinfo_markup("@@"), "@");
        assert_eq!(
            clean_texinfo_markup("@math{x^2} and @code{bar}"),
            "x^2 and bar"
        );
    }

    #[test]
    fn test_convert_raw_texinfo_figure() {
        let mut out = String::new();
        convert_raw_texinfo("(Figure plotting1: A nice plot)", &mut out);
        assert_eq!(out, "![A nice plot](figures/plotting1.png)");
    }

    #[test]
    fn test_convert_raw_texinfo_math() {
        let mut out = String::new();
        convert_raw_texinfo("@math{x^2 + y^2}", &mut out);
        assert_eq!(out, "$x^2 + y^2$");
    }
}
