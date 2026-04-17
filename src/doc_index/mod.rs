//! Re-exports from the `mxpm-doc-index` crate.
//!
//! The doc-index types and parser live in a standalone crate so they can be
//! used by downstream consumers (e.g. the Maxima LSP) without pulling in
//! mxpm's heavy dependencies.

pub use mxpm_doc_index::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_testpkg_fixture() {
        let content = include_str!("../../tests/fixtures/doc/testpkg.md");
        let idx = parse_markdown(content, "testpkg", "doc/testpkg.md");

        assert_eq!(idx.package, "testpkg");
        assert_eq!(idx.version, 1);

        // Sections: "Introduction to testpkg" and "Definitions for testpkg"
        assert_eq!(idx.sections.len(), 2);
        assert_eq!(idx.sections[0].title, "Introduction to testpkg");
        assert_eq!(idx.sections[1].title, "Definitions for testpkg");
        // No subsections for simple packages
        assert!(idx.sections[0].subsections.is_empty());
        assert!(idx.sections[1].subsections.is_empty());

        // Symbols
        assert_eq!(idx.symbols.len(), 2);

        let hello = &idx.symbols["hello"];
        assert_eq!(hello.symbol_type, "Function");
        assert_eq!(hello.signature, "hello(name)");
        assert_eq!(hello.summary, "Returns a greeting for name.");
        assert_eq!(hello.section.as_deref(), Some("Definitions for testpkg"));

        let greeting = &idx.symbols["greeting"];
        assert_eq!(greeting.symbol_type, "Variable");
        assert_eq!(greeting.signature, "greeting");
        assert_eq!(greeting.section.as_deref(), Some("Definitions for testpkg"));
    }

    #[test]
    fn parse_richpkg_fixture() {
        let content = include_str!("../../tests/fixtures/doc/richpkg.md");
        let idx = parse_markdown(content, "richpkg", "doc/richpkg.md");

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
        assert_eq!(solve.section.as_deref(), Some("Definitions for richpkg"));

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
