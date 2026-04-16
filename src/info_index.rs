//! Parse GNU Info files and generate Maxima help system indexes.
//!
//! This is a clean-room reimplementation of Maxima's `build_index.pl`.
//! It reads `.info` files (produced by `makeinfo`) and generates
//! `*-index.lisp` files that Maxima's `cl-info::load-info-hashtables`
//! uses to power the `?` and `??` help commands.

use std::collections::BTreeMap;
use std::path::Path;

use regex::Regex;

use crate::errors::MxpmError;

/// A single function/variable documentation entry.
#[derive(Debug, Clone)]
pub struct DeffnEntry {
    pub topic: String,
    pub filename: String,
    pub byte_offset: usize,
    pub char_length: usize,
    pub node_name: String,
}

/// A single section heading entry.
#[derive(Debug, Clone)]
pub struct SectionEntry {
    pub title: String,
    pub filename: String,
    pub byte_offset: usize,
    pub char_length: usize,
}

/// A complete info index, ready to render as Lisp.
#[derive(Debug)]
pub struct InfoIndex {
    pub deffn_defvr_entries: Vec<DeffnEntry>,
    pub section_entries: Vec<SectionEntry>,
}

/// Location of a node within an info file.
#[derive(Debug, Clone)]
struct NodeLocation {
    filename: String,
    byte_offset: usize,
}

/// A topic from the index menu, before resolution.
#[derive(Debug)]
struct TopicRef {
    node_name: String,
    line_offset: usize,
}

/// Detected makeinfo version.
#[derive(Debug, Clone, Copy)]
struct MakeinfoVersion {
    major: u32,
    #[allow(dead_code)]
    minor: u32,
}

const UNIT_SEPARATOR: u8 = 0x1F;

/// Parse a `.info` file (and its split parts) into an [`InfoIndex`].
pub fn build_index(main_info_path: &Path) -> Result<InfoIndex, MxpmError> {
    let main_info_path =
        main_info_path
            .canonicalize()
            .map_err(|_| MxpmError::InfoFileNotFound {
                path: main_info_path.display().to_string(),
            })?;
    let dir = main_info_path.parent().unwrap_or(Path::new("."));
    let main_filename = main_info_path
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();

    let main_data = std::fs::read(&main_info_path)?;

    // Detect makeinfo version
    let version = detect_makeinfo_version(&main_data);

    // Part 1: Build node offset table
    let mut node_offsets: BTreeMap<String, NodeLocation> = BTreeMap::new();
    let mut last_node_name = String::new();
    let mut info_filenames: Vec<String> = vec![main_filename.clone()];

    // Scan main file for nodes
    scan_nodes(
        &main_data,
        &main_filename,
        &mut node_offsets,
        &mut last_node_name,
    );

    // Find split files from indirect table
    let split_files = find_split_files(&main_data, &main_filename);
    for split_filename in &split_files {
        info_filenames.push(split_filename.clone());
        let split_path = dir.join(split_filename);
        let split_data = std::fs::read(&split_path).map_err(|_| MxpmError::InfoFileNotFound {
            path: split_path.display().to_string(),
        })?;
        scan_nodes(
            &split_data,
            split_filename,
            &mut node_offsets,
            &mut last_node_name,
        );
    }

    // Part 2: Build deffn/defvr index
    let index_node_name = last_node_name;
    let deffn_defvr_entries = if let Some(index_loc) = node_offsets.get(&index_node_name) {
        let index_filename = &index_loc.filename;
        let index_path = dir.join(index_filename);
        let index_data = std::fs::read(&index_path)?;

        // Parse the index menu entries
        let topic_refs = parse_index_menu(&index_data, &index_node_name);

        // Resolve each topic to a byte offset and character length
        let mut entries = Vec::new();
        for (topic, tref) in &topic_refs {
            if let Some(node_loc) = node_offsets.get(&tref.node_name) {
                let file_path = dir.join(&node_loc.filename);
                let file_data = std::fs::read(&file_path)?;

                let byte_offset =
                    seek_lines(&file_data, node_loc.byte_offset, tref.line_offset, version);

                let char_length = measure_deffn_length(&file_data, byte_offset);

                entries.push(DeffnEntry {
                    topic: topic.clone(),
                    filename: node_loc.filename.clone(),
                    byte_offset,
                    char_length,
                    node_name: tref.node_name.clone(),
                });
            }
        }
        entries
    } else {
        Vec::new()
    };

    // Part 3: Build section index
    let mut section_entries = Vec::new();
    for filename in &info_filenames {
        let file_path = dir.join(filename);
        let file_data = std::fs::read(&file_path)?;
        scan_sections(&file_data, filename, &mut section_entries);
    }

    Ok(InfoIndex {
        deffn_defvr_entries,
        section_entries,
    })
}

/// Render an [`InfoIndex`] as Lisp code compatible with Maxima's `load-info-hashtables`.
pub fn render_lisp(index: &InfoIndex, install_path: Option<&str>) -> String {
    let mut out = String::new();

    out.push_str("(in-package :cl-info)\n");
    out.push_str("(let (\n");

    // deffn-defvr-pairs
    out.push_str("(deffn-defvr-pairs '(\n");
    out.push_str(
        "; CONTENT: (<INDEX TOPIC> . (<FILENAME> <BYTE OFFSET> <LENGTH IN CHARACTERS> <NODE NAME>))\n",
    );

    let mut sorted_deffn: Vec<&DeffnEntry> = index.deffn_defvr_entries.iter().collect();
    sorted_deffn.sort_by_key(|e| &e.topic);

    for entry in &sorted_deffn {
        out.push_str(&format!(
            "(\"{}\" . (\"{}\" {} {} \"{}\"))\n",
            escape_lisp_string(&entry.topic),
            escape_lisp_string(&entry.filename),
            entry.byte_offset,
            entry.char_length,
            escape_lisp_string(&entry.node_name),
        ));
    }
    out.push_str("))\n");

    // section-pairs
    out.push_str("(section-pairs '(\n");
    out.push_str("; CONTENT: (<NODE NAME> . (<FILENAME> <BYTE OFFSET> <LENGTH IN CHARACTERS>))\n");

    let mut sorted_sections: Vec<&SectionEntry> = index.section_entries.iter().collect();
    sorted_sections.sort_by_key(|e| &e.title);

    for entry in &sorted_sections {
        out.push_str(&format!(
            "(\"{}\" . (\"{}\" {} {}))\n",
            escape_lisp_string(&entry.title),
            escape_lisp_string(&entry.filename),
            entry.byte_offset,
            entry.char_length,
        ));
    }
    out.push_str(")))\n");

    // load-info-hashtables call
    let dir_expr = match install_path {
        Some(path) => format!("#p\"{}\"", path),
        None => "(maxima::maxima-load-pathname-directory)".to_string(),
    };
    out.push_str(&format!(
        "(load-info-hashtables {} deffn-defvr-pairs section-pairs))\n",
        dir_expr
    ));

    out
}

/// Detect makeinfo version from the file header.
fn detect_makeinfo_version(data: &[u8]) -> Option<MakeinfoVersion> {
    let header = String::from_utf8_lossy(&data[..std::cmp::min(data.len(), 200)]);
    let re = Regex::new(r"makeinfo version (\d+)\.(\d+)").unwrap();
    re.captures(&header).map(|caps| MakeinfoVersion {
        major: caps[1].parse().unwrap(),
        minor: caps[2].parse().unwrap(),
    })
}

/// Scan a file's bytes for nodes delimited by unit separator characters.
/// Populates `node_offsets` with `{node_name → (filename, byte_offset)}`.
fn scan_nodes(
    data: &[u8],
    filename: &str,
    node_offsets: &mut BTreeMap<String, NodeLocation>,
    last_node_name: &mut String,
) {
    let node_re = Regex::new(r"^File:.*?Node:\s*(.*?),").unwrap();

    // Find all unit separator positions.
    // The pattern is \n\x1F\n (newline, unit separator, newline) followed by "File:..."
    // but the first one may be \n\n\x1F\n at the start of the file.
    let mut pos = 0;
    while pos < data.len() {
        // Find next unit separator
        let sep_pos = match memchr(UNIT_SEPARATOR, &data[pos..]) {
            Some(offset) => pos + offset,
            None => break,
        };

        // The byte offset we record is the position of the \n before the unit separator.
        // In the Perl script, `pos $stuff` after matching `\G.*?(?=\n$unit_separator)`
        // gives the offset of the \n before the separator.
        // But what Maxima actually uses is the position of the \n before 0x1F.
        let record_offset = if sep_pos > 0 && data[sep_pos - 1] == b'\n' {
            sep_pos - 1
        } else {
            sep_pos
        };

        // After the unit separator, expect \nFile:...
        let after_sep = sep_pos + 1;
        if after_sep >= data.len() {
            break;
        }

        // Skip the newline after separator
        let line_start = if data[after_sep] == b'\n' {
            after_sep + 1
        } else {
            after_sep
        };

        // Read until end of line to get the node header
        let line_end = find_newline(data, line_start);
        if let Ok(line) = std::str::from_utf8(&data[line_start..line_end])
            && let Some(caps) = node_re.captures(line)
        {
            let node_name = caps[1].to_string();
            *last_node_name = node_name.clone();
            node_offsets.insert(
                node_name,
                NodeLocation {
                    filename: filename.to_string(),
                    byte_offset: record_offset,
                },
            );
        }

        pos = line_end;
    }
}

/// Find split files referenced in the indirect table of a main .info file.
/// Returns filenames like ["maxima.info-1", "maxima.info-2", ...].
fn find_split_files(data: &[u8], main_filename: &str) -> Vec<String> {
    let mut files = Vec::new();
    let text = String::from_utf8_lossy(data);

    // The pattern in the Perl script: ^($main_info-\d+): (\d+)
    let pattern = format!(r"^({}-\d+): \d+", regex::escape(main_filename));
    let re = Regex::new(&pattern).unwrap();

    for line in text.lines() {
        if let Some(caps) = re.captures(line) {
            files.push(caps[1].to_string());
        }
    }
    files
}

/// Parse the index menu in the last node to extract topic references.
/// Returns a sorted map of `{topic_name → TopicRef}`.
fn parse_index_menu(data: &[u8], index_node_name: &str) -> BTreeMap<String, TopicRef> {
    let text = String::from_utf8_lossy(data);
    let mut topics = BTreeMap::new();

    // Find where the index node starts using "Node: <name>" in a File: header.
    let node_pattern = format!(r"(?mi)^File:.*?Node:\s*{}", regex::escape(index_node_name));
    let node_re = Regex::new(&node_pattern).unwrap();

    let index_start = match node_re.find(&text) {
        Some(m) => m.start(),
        None => return topics,
    };
    let index_text = &text[index_start..];

    // Collect menu entries by joining continuation lines.
    // Menu entries start with "* " at the beginning of a line.
    // Continuation lines start with whitespace.
    let mut entries: Vec<String> = Vec::new();
    let mut in_menu = false;

    for line in index_text.lines() {
        if line.starts_with("* Menu:") {
            in_menu = true;
            continue;
        }
        if !in_menu {
            continue;
        }
        if line.starts_with("* ") {
            entries.push(line.to_string());
        } else if !entries.is_empty() && (line.starts_with(' ') || line.starts_with('\t')) {
            // Continuation line — append to the last entry
            let last = entries.last_mut().unwrap();
            last.push(' ');
            last.push_str(line.trim());
        }
    }

    // Parse each collected entry
    let entry_re = Regex::new(r"^\* (\S+|[^:]+):\s+(.*?)\.\s+\(line\s+(\d+)\)").unwrap();

    for entry in &entries {
        if let Some(caps) = entry_re.captures(entry) {
            let topic_name = caps[1].to_string();
            let node_name = caps[2].trim().to_string();
            let line_offset: usize = caps[3].parse().unwrap_or(0);
            topics.insert(
                topic_name,
                TopicRef {
                    node_name,
                    line_offset,
                },
            );
        }
    }

    topics
}

/// Convert a node offset + line number into a byte offset.
///
/// This corresponds to `seek_lines` in the Perl script:
/// - Seek to the node's byte offset (character_offset in Perl)
/// - Skip `lines_offset` lines
/// - Return the byte position
///
/// For makeinfo v4, applies a bug workaround that searches for ` -- \S`
/// patterns among the skipped lines.
fn seek_lines(
    data: &[u8],
    node_byte_offset: usize,
    lines_offset: usize,
    version: Option<MakeinfoVersion>,
) -> usize {
    // The Perl script opens with :utf8 and reads `character_offset` chars,
    // then uses `tell` to get the byte position. For our purposes, the
    // node_byte_offset is already a byte offset from scan_nodes.
    //
    // However, the Perl script's `read FH, $stuff, $character_offset` reads
    // character_offset *characters* (not bytes) to advance the file handle.
    // Then it skips lines from that position.
    //
    // Since scan_nodes records the byte offset of the \n before the separator,
    // we need to advance past that to the start of the content after the
    // node header. The node header starts after \x1F\n, but the Perl script
    // uses the character offset from pos() which is where \n\x1F is.
    //
    // Actually in the Perl script:
    // - $character_offset = $node_offset{$node_name}[1] which is set to int($offset)
    //   where $offset = pos $stuff after matching \G.*?(?=\n$unit_separator)
    // - Then: open, read FH, $stuff, $character_offset (reads that many chars)
    // - Then: skip lines_offset lines using readline (<FH>)
    //
    // Since the Perl script reads in :utf8 mode, reading $character_offset
    // chars from the file advances to the byte position of char_offset chars.
    // For ASCII this is the same as byte offset.
    //
    // We stored the byte offset of the \n before \x1F. The Perl stored the
    // character offset of that same \n. So they should be equivalent for
    // the purpose of "start reading lines from this position".

    let start = node_byte_offset;

    if let Some(v) = version
        && v.major == 4
    {
        return seek_lines_v4(data, start, lines_offset);
    }

    // Version 5+: simply skip lines_offset lines
    let mut pos = start;
    for _ in 0..lines_offset {
        pos = find_newline(data, pos);
        if pos < data.len() {
            pos += 1; // skip past the \n
        }
    }

    pos
}

/// Makeinfo v4 bug workaround: line offset points to the last line of
/// a multi-line @deffn, not the first. Scan for ` -- \S` pattern.
fn seek_lines_v4(data: &[u8], start: usize, lines_offset: usize) -> usize {
    let deffn_re = Regex::new(r"^ -- \S").unwrap();
    let mut pos = start;
    let mut result: Option<usize> = None;

    for _ in 0..lines_offset + 1 {
        let line_start = pos;
        let line_end = find_newline(data, pos);
        if let Ok(line) = std::str::from_utf8(&data[line_start..line_end])
            && deffn_re.is_match(line)
        {
            result = Some(line_start);
        }
        pos = if line_end < data.len() {
            line_end + 1
        } else {
            line_end
        };
    }

    // If we found a ` -- \S` line, use it; otherwise use the last line position
    result.unwrap_or(if pos > start { pos - 1 } else { start })
}

/// Measure the character length of a deffn/defvr entry starting at `byte_offset`.
///
/// The entry ends at whichever comes first:
/// - `\n\n` followed by ` -- ` (start of next @deffn/@defvr)
/// - `\n` followed by a digit (start of next section heading)
/// - Unit separator byte (0x1F, start of next node)
/// - End of file
fn measure_deffn_length(data: &[u8], byte_offset: usize) -> usize {
    let slice = &data[byte_offset..];
    let text = String::from_utf8_lossy(slice);

    // Match the Perl regex: (.*?)(?:\n\n(?= -- )|\n(?=[0-9])|(?=$unit_separator))
    // We need to find the earliest of these terminators.

    let mut end_char_pos = text.chars().count(); // default: rest of file

    // Scan character by character tracking positions
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();

    for i in 0..len {
        // Check for unit separator
        if chars[i] == '\x1F' {
            end_char_pos = i;
            break;
        }

        // Check for \n\n followed by " -- "
        if chars[i] == '\n'
            && i + 1 < len
            && chars[i + 1] == '\n'
            && i + 5 < len
            && chars[i + 2] == ' '
            && chars[i + 3] == '-'
            && chars[i + 4] == '-'
            && chars[i + 5] == ' '
        {
            end_char_pos = i;
            break;
        }

        // Check for \n followed by a digit
        if chars[i] == '\n' && i + 1 < len && chars[i + 1].is_ascii_digit() {
            end_char_pos = i;
            break;
        }
    }

    end_char_pos
}

/// Scan a file for section headings (lines matching `^\d+\.\d+ <title>`).
fn scan_sections(data: &[u8], filename: &str, entries: &mut Vec<SectionEntry>) {
    let text = String::from_utf8_lossy(data);
    let section_re = Regex::new(r"^(\d+\.\d+) (.*?)$").unwrap();

    // We need character offsets for finding sections, then convert to byte offsets.
    let mut char_offset = 0;
    let mut sections: Vec<(String, usize, usize)> = Vec::new(); // (title, char_offset, ...)

    for line in text.split('\n') {
        if let Some(caps) = section_re.captures(line) {
            let title = caps[2].to_string();
            sections.push((title, char_offset, 0));
        }
        char_offset += line.len() + 1; // +1 for \n
    }

    // Measure length of each section: from its start until the next unit separator or EOF
    let chars: Vec<char> = text.chars().collect();
    let total_chars = chars.len();

    for section in &mut sections {
        let start = section.1;
        // Find the end: unit separator or EOF
        let mut end = total_chars;
        for (j, &ch) in chars.iter().enumerate().skip(start) {
            if ch == '\x1F' {
                end = j;
                break;
            }
        }
        section.2 = end - start;
    }

    // Convert character offsets to byte offsets
    // Build a char→byte offset map
    let byte_offsets = char_to_byte_offsets(data);

    for (title, char_off, char_len) in sections {
        let byte_off = if char_off < byte_offsets.len() {
            byte_offsets[char_off]
        } else {
            data.len()
        };
        entries.push(SectionEntry {
            title,
            filename: filename.to_string(),
            byte_offset: byte_off,
            char_length: char_len,
        });
    }
}

/// Build a mapping from character index to byte offset for a UTF-8 buffer.
fn char_to_byte_offsets(data: &[u8]) -> Vec<usize> {
    let text = String::from_utf8_lossy(data);
    let mut offsets = Vec::with_capacity(text.len());
    let mut byte_pos = 0;
    for ch in text.chars() {
        offsets.push(byte_pos);
        byte_pos += ch.len_utf8();
    }
    offsets.push(byte_pos); // sentinel for "one past the end"
    offsets
}

/// Find the next newline byte starting from `pos`. Returns the position of the `\n`,
/// or `data.len()` if no newline is found.
fn find_newline(data: &[u8], pos: usize) -> usize {
    memchr(b'\n', &data[pos..])
        .map(|offset| pos + offset)
        .unwrap_or(data.len())
}

/// Simple memchr implementation.
fn memchr(needle: u8, haystack: &[u8]) -> Option<usize> {
    haystack.iter().position(|&b| b == needle)
}

/// Escape a string for Lisp output: double quotes become `\"`.
fn escape_lisp_string(s: &str) -> String {
    s.replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_lisp_string() {
        assert_eq!(escape_lisp_string("hello"), "hello");
        assert_eq!(escape_lisp_string(r#"say "hi""#), r#"say \"hi\""#);
    }

    #[test]
    fn test_detect_makeinfo_version() {
        let data = b"This is foo.info, produced by makeinfo version 5.2 from\nfoo.texi.\n";
        let v = detect_makeinfo_version(data).unwrap();
        assert_eq!(v.major, 5);
        assert_eq!(v.minor, 2);
    }

    #[test]
    fn test_detect_makeinfo_version_7() {
        let data = b"This is bar.info, produced by makeinfo version 7.3 from\nbar.texi.\n";
        let v = detect_makeinfo_version(data).unwrap();
        assert_eq!(v.major, 7);
        assert_eq!(v.minor, 3);
    }

    #[test]
    fn test_detect_makeinfo_version_none() {
        let data = b"no version info here\n";
        assert!(detect_makeinfo_version(data).is_none());
    }

    #[test]
    fn test_find_split_files() {
        let data = b"Indirect:\nmaxima.info-1: 264\nmaxima.info-2: 1030881\n\x1f\n";
        let files = find_split_files(data, "maxima.info");
        assert_eq!(files, vec!["maxima.info-1", "maxima.info-2"]);
    }

    #[test]
    fn test_find_split_files_none() {
        let data = b"This is foo.info, no split files\n\x1f\nFile: foo.info\n";
        let files = find_split_files(data, "foo.info");
        assert!(files.is_empty());
    }

    #[test]
    fn test_scan_nodes() {
        // Simulate a small .info file with two nodes
        let data = b"header stuff\n\n\x1f\nFile: test.info,  Node: Top,  Next: Foo\n\nTop content\n\n\x1f\nFile: test.info,  Node: Foo,  Prev: Top\n\nFoo content\n";
        let mut offsets = BTreeMap::new();
        let mut last = String::new();
        scan_nodes(data, "test.info", &mut offsets, &mut last);

        assert_eq!(offsets.len(), 2);
        assert!(offsets.contains_key("Top"));
        assert!(offsets.contains_key("Foo"));
        assert_eq!(last, "Foo");

        // The byte offset should be the \n before \x1F
        let top_loc = &offsets["Top"];
        assert_eq!(top_loc.filename, "test.info");
        assert_eq!(data[top_loc.byte_offset], b'\n');
    }

    #[test]
    fn test_parse_index_menu() {
        let data = b"\x1f\nFile: test.info,  Node: Function and variable index,  Prev: Defs\n\nAppendix A\n\n [index ]\n* Menu:\n\n* frotz:                                 Definitions for MYTOPIC.\n                                                               (line  11)\n* transmogrify:                          Definitions for MYTOPIC.\n                                                                (line 6)\n\n";
        let topics = parse_index_menu(data, "Function and variable index");
        assert_eq!(topics.len(), 2);

        let frotz = &topics["frotz"];
        assert_eq!(frotz.node_name, "Definitions for MYTOPIC");
        assert_eq!(frotz.line_offset, 11);

        let trans = &topics["transmogrify"];
        assert_eq!(trans.node_name, "Definitions for MYTOPIC");
        assert_eq!(trans.line_offset, 6);
    }

    #[test]
    fn test_measure_deffn_length_double_newline_deffn() {
        let data = b"\n -- Function: foo (x)\n\n     Does foo.\n\n -- Function: bar (y)\n";
        let len = measure_deffn_length(data, 0);
        // Should stop at \n\n before " -- Function: bar"
        let text = std::str::from_utf8(data).unwrap();
        let expected: String = text.chars().take(len).collect();
        assert!(expected.contains("Does foo."));
        assert!(!expected.contains("bar"));
    }

    #[test]
    fn test_measure_deffn_length_unit_separator() {
        let data = b"\n -- Function: foo (x)\n\n     Does foo.\n\n\x1fNext node";
        let len = measure_deffn_length(data, 0);
        let text = std::str::from_utf8(&data[..]).unwrap();
        let result: String = text.chars().take(len).collect();
        assert!(result.contains("Does foo."));
        assert!(!result.contains("Next node"));
    }

    #[test]
    fn test_render_lisp_basic() {
        let index = InfoIndex {
            deffn_defvr_entries: vec![DeffnEntry {
                topic: "frotz".to_string(),
                filename: "test.info".to_string(),
                byte_offset: 100,
                char_length: 50,
                node_name: "Definitions".to_string(),
            }],
            section_entries: vec![SectionEntry {
                title: "Introduction".to_string(),
                filename: "test.info".to_string(),
                byte_offset: 200,
                char_length: 80,
            }],
        };

        let lisp = render_lisp(&index, None);
        assert!(lisp.contains("(in-package :cl-info)"));
        assert!(lisp.contains(r#"("frotz" . ("test.info" 100 50 "Definitions"))"#));
        assert!(lisp.contains(r#"("Introduction" . ("test.info" 200 80))"#));
        assert!(lisp.contains("(maxima::maxima-load-pathname-directory)"));
    }

    #[test]
    fn test_render_lisp_with_install_path() {
        let index = InfoIndex {
            deffn_defvr_entries: vec![],
            section_entries: vec![],
        };
        let lisp = render_lisp(&index, Some("/usr/share/info/"));
        assert!(lisp.contains(r#"#p"/usr/share/info/""#));
    }

    #[test]
    fn test_render_lisp_sorted() {
        let index = InfoIndex {
            deffn_defvr_entries: vec![
                DeffnEntry {
                    topic: "zebra".to_string(),
                    filename: "t.info".to_string(),
                    byte_offset: 1,
                    char_length: 1,
                    node_name: "N".to_string(),
                },
                DeffnEntry {
                    topic: "alpha".to_string(),
                    filename: "t.info".to_string(),
                    byte_offset: 2,
                    char_length: 2,
                    node_name: "N".to_string(),
                },
            ],
            section_entries: vec![],
        };
        let lisp = render_lisp(&index, None);
        let alpha_pos = lisp.find("\"alpha\"").unwrap();
        let zebra_pos = lisp.find("\"zebra\"").unwrap();
        assert!(alpha_pos < zebra_pos);
    }
}
