//! Comment stripping via real tree-sitter parsers — never regex (owner decision,
//! `docs/__arch__/open-questions.md`, 2026-07-28). One generic algorithm plus a
//! per-language grammar/comment-node-kind table, rather than one hand-written stripper
//! per language: every Batch 1 grammar exposes comments as one or two named node kinds
//! (`comment`, or `line_comment`/`block_comment`), and removing named byte ranges is
//! language-agnostic once the parser has told us where they are.
//!
//! ## Fail-safe on parse errors (Q24, open)
//!
//! If the parsed tree contains any `ERROR` node — tree-sitter's signal that it could not
//! make full sense of the input — [`strip_comments`] returns `None` rather than guessing.
//! The caller must copy the file through unmodified and report `FileOutcome::Error`: the
//! owner has not signed off on a specific behavior for this case (Q24 lists it as open),
//! and stripping under uncertain parse conditions risks corrupting code, which this
//! feature must never do. This is a deliberately conservative default, not a shortcut.

use tree_sitter::{Node, Parser};

use crate::language::Language;

fn ts_language(language: Language) -> tree_sitter::Language {
    match language {
        Language::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
        Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        Language::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
        Language::Python => tree_sitter_python::LANGUAGE.into(),
        Language::Go => tree_sitter_go::LANGUAGE.into(),
        Language::Rust => tree_sitter_rust::LANGUAGE.into(),
        Language::Java => tree_sitter_java::LANGUAGE.into(),
        Language::CSharp => tree_sitter_c_sharp::LANGUAGE.into(),
        Language::Php => tree_sitter_php::LANGUAGE_PHP.into(),
        Language::Ruby => tree_sitter_ruby::LANGUAGE.into(),
        Language::C => tree_sitter_c::LANGUAGE.into(),
        Language::Cpp => tree_sitter_cpp::LANGUAGE.into(),
        Language::Shell => tree_sitter_bash::LANGUAGE.into(),
        Language::Makefile => tree_sitter_make::LANGUAGE.into(),
        Language::Kotlin => tree_sitter_kotlin_ng::LANGUAGE.into(),
    }
}

/// Node kinds that represent a whole comment in each grammar, probed directly against
/// each Batch 1 grammar (see the crate's task notes) rather than assumed: most expose a
/// single `comment` kind, Rust and Java split line/block comments into two kinds, and
/// JS/TS/TSX additionally have `html_comment` for embedded-HTML contexts.
fn comment_kinds(language: Language) -> &'static [&'static str] {
    match language {
        Language::JavaScript | Language::TypeScript | Language::Tsx => &["comment", "html_comment"],
        Language::Rust | Language::Java | Language::Kotlin => &["line_comment", "block_comment"],
        Language::Python
        | Language::Go
        | Language::CSharp
        | Language::Php
        | Language::Ruby
        | Language::C
        | Language::Cpp
        | Language::Shell
        | Language::Makefile => &["comment"],
    }
}

/// Strips every comment node from `source`, returning `None` (fail-safe, see module
/// doc) when the parse contains any `ERROR` node.
pub(crate) fn strip_comments(language: Language, source: &str) -> Option<String> {
    let mut parser = Parser::new();
    // Every Batch 1 grammar version pinned in `Cargo.toml` was probed against this exact
    // `tree-sitter` core version before being added (Q24 names this compatibility check
    // as open, so it is closed here rather than assumed): `set_language` failing at run
    // time would mean a version pin drifted, which is a build-time-detectable defect,
    // not a per-file failure this function should have to recover from.
    parser
        .set_language(&ts_language(language))
        .expect("Batch 1 grammar versions are pinned to match this tree-sitter core version");
    let tree = parser.parse(source, None)?;
    let root = tree.root_node();
    if root.has_error() {
        return None;
    }

    let kinds = comment_kinds(language);
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    collect_comment_ranges(root, kinds, &mut ranges);
    if ranges.is_empty() {
        return Some(source.to_string());
    }
    ranges.sort_unstable();

    let mut out = String::with_capacity(source.len());
    let mut cursor = 0usize;
    for (start, end) in ranges {
        if start < cursor {
            // A nested named node sharing a comment kind with its parent (none of the
            // Batch 1 grammars do this today, but the guard costs nothing and keeps the
            // byte-splice below from panicking if one ever does).
            continue;
        }
        out.push_str(&source[cursor..start]);
        cursor = end;
    }
    out.push_str(&source[cursor..]);
    Some(out)
}

/// Depth-first collection of every comment-kind node's byte range. Deliberately does
/// not descend into a matched node's children — Rust's `line_comment` wraps a doc-marker
/// and a `doc_comment` child for `///`/`//!` forms, and descending would just re-find a
/// range already covered by the parent's.
fn collect_comment_ranges(node: Node<'_>, kinds: &[&str], out: &mut Vec<(usize, usize)>) {
    if kinds.contains(&node.kind()) {
        out.push((node.start_byte(), node.end_byte()));
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_comment_ranges(child, kinds, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_line_and_block_comments_are_removed_but_strings_survive() {
        let source = "fn main() {\n    // a line comment\n    let s = \"// not a comment\";\n    /* a block */\n    let n = 1;\n}\n";
        let stripped = strip_comments(Language::Rust, source).expect("valid rust parses");
        assert!(!stripped.contains("a line comment"));
        assert!(!stripped.contains("a block"));
        assert!(stripped.contains("\"// not a comment\""));
    }

    #[test]
    fn rust_doc_comments_are_removed_too() {
        let source = "/// doc comment\nfn main() {}\n";
        let stripped = strip_comments(Language::Rust, source).unwrap();
        assert!(!stripped.contains("doc comment"));
    }

    #[test]
    fn python_comments_are_removed_but_strings_survive() {
        let source = "# a real comment\nx = \"# not a comment\"\ny = 1  # trailing comment\n";
        let stripped = strip_comments(Language::Python, source).unwrap();
        assert!(!stripped.contains("a real comment"));
        assert!(!stripped.contains("trailing comment"));
        assert!(stripped.contains("\"# not a comment\""));
    }

    #[test]
    fn javascript_block_and_line_comments_leave_regex_literals_untouched() {
        let source = "const s = \"// not a comment\";\nconst re = /\\/\\* not \\*\\//;\n// real comment\n/* block */\n";
        let stripped = strip_comments(Language::JavaScript, source).unwrap();
        assert!(!stripped.contains("real comment"));
        assert!(!stripped.contains("/* block */"));
        assert!(stripped.contains("\"// not a comment\""));
        assert!(stripped.contains("/\\/\\* not \\*\\//"));
    }

    #[test]
    fn typescript_comments_are_removed() {
        let source = "// c\nconst x: number = 1; /* c2 */\n";
        let stripped = strip_comments(Language::TypeScript, source).unwrap();
        assert!(!stripped.contains(" c\n"));
        assert!(!stripped.contains("c2"));
    }

    #[test]
    fn go_comments_are_removed_but_strings_survive() {
        let source = "package main\n// a comment\nvar s = \"// not a comment\"\n/* block */\n";
        let stripped = strip_comments(Language::Go, source).unwrap();
        assert!(!stripped.contains("// a comment"));
        assert!(!stripped.contains("/* block */"));
        assert!(stripped.contains("\"// not a comment\""));
    }

    #[test]
    fn java_comments_are_removed_but_strings_survive() {
        let source =
            "class A {\n  // a comment\n  String s = \"// not a comment\";\n  /* block */\n}\n";
        let stripped = strip_comments(Language::Java, source).unwrap();
        assert!(!stripped.contains("// a comment"));
        assert!(!stripped.contains("/* block */"));
        assert!(stripped.contains("\"// not a comment\""));
    }

    #[test]
    fn csharp_comments_are_removed_but_strings_survive() {
        let source = "class A { void M() {\n  // a comment\n  var s = \"// not a comment\";\n  /* block */\n} }\n";
        let stripped = strip_comments(Language::CSharp, source).unwrap();
        assert!(!stripped.contains("// a comment"));
        assert!(!stripped.contains("/* block */"));
        assert!(stripped.contains("\"// not a comment\""));
    }

    #[test]
    fn php_comments_are_removed_but_strings_survive() {
        let source = "<?php\n// a comment\n$s = \"// not a comment\";\n/* block */\n";
        let stripped = strip_comments(Language::Php, source).unwrap();
        assert!(!stripped.contains("// a comment"));
        assert!(!stripped.contains("/* block */"));
        assert!(stripped.contains("\"// not a comment\""));
    }

    #[test]
    fn ruby_line_and_block_comments_are_removed_but_strings_survive() {
        let source = "=begin\na block comment\n=end\nputs \"# not a comment\"\n# a comment\n";
        let stripped = strip_comments(Language::Ruby, source).unwrap();
        assert!(!stripped.contains("a block comment"));
        assert!(!stripped.contains("# a comment"));
        assert!(stripped.contains("\"# not a comment\""));
    }

    #[test]
    fn c_comments_are_removed_but_strings_survive() {
        let source = "int main() {\n  const char *s = \"/* not a comment */\"; // real\n  /* block */\n  return 0;\n}\n";
        let stripped = strip_comments(Language::C, source).unwrap();
        assert!(!stripped.contains("// real"));
        assert!(!stripped.contains("/* block */"));
        assert!(stripped.contains("\"/* not a comment */\""));
    }

    #[test]
    fn cpp_comments_are_removed_but_strings_survive() {
        let source = "int main() {\n  const char* s = \"/* not a comment */\"; // real\n}\n";
        let stripped = strip_comments(Language::Cpp, source).unwrap();
        assert!(!stripped.contains("// real"));
        assert!(stripped.contains("\"/* not a comment */\""));
    }

    #[test]
    fn shell_comments_are_removed_but_strings_survive() {
        let source = "#!/bin/bash\n# a real comment\necho \"# not a comment\"\n";
        let stripped = strip_comments(Language::Shell, source).unwrap();
        // The shebang is itself a `comment` node in tree-sitter-bash's grammar and is
        // removed along with real comments - it is not executable Rust/Python/etc, so
        // dropping it is consistent with dropping every other comment node.
        assert!(!stripped.contains("a real comment"));
        assert!(!stripped.contains("#!/bin/bash"));
        assert!(stripped.contains("\"# not a comment\""));
    }

    #[test]
    fn kotlin_comments_are_removed_but_strings_survive() {
        let source = "fun main() {\n    // a line comment\n    val s = \"// not a comment\"\n    /* a block */\n    val n = 1\n}\n";
        let stripped = strip_comments(Language::Kotlin, source).expect("valid kotlin parses");
        assert!(!stripped.contains("a line comment"));
        assert!(!stripped.contains("a block"));
        assert!(stripped.contains("\"// not a comment\""));
    }

    #[test]
    fn makefile_comments_are_removed() {
        let source = "# a comment\nall:\n\techo hi\n";
        let stripped = strip_comments(Language::Makefile, source).unwrap();
        assert!(!stripped.contains("a comment"));
        assert!(stripped.contains("echo hi"));
    }

    #[test]
    fn a_syntactically_broken_file_is_left_for_the_caller_to_copy_unmodified() {
        let source = "fn main( {\n";
        assert!(strip_comments(Language::Rust, source).is_none());
    }
}
