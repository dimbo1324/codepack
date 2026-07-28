//! Batch 1 language detection (`docs/decisions/open-questions.md`, 2026-07-28): the
//! twelve stacks the owner named for the first version of the sterile copy — JS/TS,
//! Python, Go, Rust, Java, C#, PHP, Ruby, C, C++, Shell, Makefile. Batch 2 (Dart/
//! Flutter, Swift, Kotlin, Assembly, Groovy) is explicitly deferred (Q24) — every file
//! in one of those stacks, or in any stack this crate has no grammar for at all, comes
//! back `None` here and is reported as `FileOutcome::SkippedUnsupportedLanguage`, never
//! silently dropped.
//!
//! This is a separate, narrower map from `codepack-reports`' `LANGUAGE_BY_EXTENSION`
//! (which this crate is not even allowed to depend on — dependencies point strictly
//! downward): that table names ~50 languages for reporting purposes, this one names
//! only the languages a Batch 1 grammar actually exists for.

use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    JavaScript,
    TypeScript,
    Tsx,
    Python,
    Go,
    Rust,
    Java,
    CSharp,
    Php,
    Ruby,
    C,
    Cpp,
    Shell,
    Makefile,
}

impl Language {
    /// The human-readable name that appears in `STERILE_COPY_REPORT.md/.json`.
    pub fn label(self) -> &'static str {
        match self {
            Self::JavaScript => "JavaScript",
            Self::TypeScript => "TypeScript",
            Self::Tsx => "TypeScript (TSX)",
            Self::Python => "Python",
            Self::Go => "Go",
            Self::Rust => "Rust",
            Self::Java => "Java",
            Self::CSharp => "C#",
            Self::Php => "PHP",
            Self::Ruby => "Ruby",
            Self::C => "C",
            Self::Cpp => "C++",
            Self::Shell => "Shell",
            Self::Makefile => "Makefile",
        }
    }
}

/// Detects a Batch 1 language from a relative path, filename first (`Makefile` has no
/// extension to key on), then extension. Returns `None` for anything outside Batch 1 —
/// callers must treat that as "unsupported", not as an error.
pub fn detect_language(relative_path: &Path) -> Option<Language> {
    let file_name = relative_path
        .file_name()
        .map(|value| value.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if file_name == "makefile" || file_name == "gnumakefile" {
        return Some(Language::Makefile);
    }

    let extension = relative_path
        .extension()
        .map(|value| value.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    match extension.as_str() {
        "js" | "jsx" | "mjs" | "cjs" => Some(Language::JavaScript),
        "ts" | "mts" | "cts" => Some(Language::TypeScript),
        "tsx" => Some(Language::Tsx),
        "py" | "pyw" | "pyi" => Some(Language::Python),
        "go" => Some(Language::Go),
        "rs" => Some(Language::Rust),
        "java" => Some(Language::Java),
        "cs" => Some(Language::CSharp),
        "php" | "phtml" => Some(Language::Php),
        "rb" | "rake" | "gemspec" => Some(Language::Ruby),
        "c" | "h" => Some(Language::C),
        "cpp" | "cxx" | "cc" | "hpp" | "hh" | "hxx" => Some(Language::Cpp),
        "sh" | "bash" | "zsh" => Some(Language::Shell),
        "mk" | "mak" => Some(Language::Makefile),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn makefile_is_detected_by_filename_not_extension() {
        assert_eq!(
            detect_language(Path::new("Makefile")),
            Some(Language::Makefile)
        );
        assert_eq!(
            detect_language(Path::new("src/GNUmakefile")),
            Some(Language::Makefile)
        );
    }

    #[test]
    fn every_batch_one_extension_resolves() {
        let cases = [
            ("main.js", Language::JavaScript),
            ("main.jsx", Language::JavaScript),
            ("main.mjs", Language::JavaScript),
            ("main.ts", Language::TypeScript),
            ("main.tsx", Language::Tsx),
            ("main.py", Language::Python),
            ("main.go", Language::Go),
            ("main.rs", Language::Rust),
            ("Main.java", Language::Java),
            ("Program.cs", Language::CSharp),
            ("index.php", Language::Php),
            ("app.rb", Language::Ruby),
            ("main.c", Language::C),
            ("main.cpp", Language::Cpp),
            ("main.h", Language::C),
            ("main.hpp", Language::Cpp),
            ("run.sh", Language::Shell),
            ("build.mk", Language::Makefile),
        ];
        for (path, expected) in cases {
            assert_eq!(detect_language(Path::new(path)), Some(expected), "{path}");
        }
    }

    #[test]
    fn batch_two_and_unknown_stacks_are_unsupported() {
        for path in [
            "main.dart",
            "App.swift",
            "Main.kt",
            "boot.s",
            "Build.gradle",
            "notes.txt",
            "image.png",
        ] {
            assert_eq!(detect_language(Path::new(path)), None, "{path}");
        }
    }

    #[test]
    fn labels_are_distinct_and_non_empty() {
        let all = [
            Language::JavaScript,
            Language::TypeScript,
            Language::Tsx,
            Language::Python,
            Language::Go,
            Language::Rust,
            Language::Java,
            Language::CSharp,
            Language::Php,
            Language::Ruby,
            Language::C,
            Language::Cpp,
            Language::Shell,
            Language::Makefile,
        ];
        let mut labels: Vec<&str> = all.iter().map(|l| l.label()).collect();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), all.len());
        assert!(labels.iter().all(|l| !l.is_empty()));
    }
}
