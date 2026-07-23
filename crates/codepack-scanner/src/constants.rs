//! Constant sets ported verbatim from legacy `constants.py`
//! (`.ai/project/12-domain-rules.md`: changing a set is a separate decision, never a
//! refactoring side effect). Values, casing, and membership are copied as-is; only the
//! lookup mechanism (a lazily built `HashSet`) is new.

use std::collections::HashSet;
use std::sync::LazyLock;

/// Directory basenames pruned during the walk, matched case-insensitively. 18 entries.
pub const IGNORED_DIR_NAMES: &[&str] = &[
    ".git",
    "node_modules",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
    ".tox",
    ".venv",
    "venv",
    "env",
    "dist",
    "build",
    "coverage",
    ".coverage",
    ".next",
    ".nuxt",
    ".turbo",
    ".parcel-cache",
];

/// Extensions (no leading dot, lowercase) considered text. 133 entries.
///
/// The legacy archive's own `constants.py` has 133 entries in `TEXT_EXTENSIONS`, not
/// the 135 this stage's planning materials claimed; counted directly from the archive
/// (`docs/__arch__/codepack-main.zip`) rather than trusting the summary, per
/// `.ai/project/14-legacy-reference.md`.
pub const TEXT_EXTENSIONS: &[&str] = &[
    "adoc",
    "ahk",
    "asciidoc",
    "astro",
    "bash",
    "bat",
    "bib",
    "bzl",
    "c",
    "cc",
    "cfg",
    "cjs",
    "clj",
    "cljc",
    "cljs",
    "cls",
    "cmake",
    "cmd",
    "conf",
    "config",
    "cpp",
    "cs",
    "css",
    "csv",
    "cxx",
    "d",
    "dart",
    "desktop",
    "diff",
    "dockerfile",
    "dockerignore",
    "editorconfig",
    "edn",
    "ejs",
    "elm",
    "env",
    "erl",
    "err",
    "ex",
    "exs",
    "feature",
    "fish",
    "fs",
    "fsi",
    "fsx",
    "gemspec",
    "gitattributes",
    "gitignore",
    "go",
    "gql",
    "gradle",
    "graphql",
    "groovy",
    "h",
    "haml",
    "hcl",
    "hh",
    "hpp",
    "hrl",
    "hs",
    "htm",
    "html",
    "hxx",
    "ini",
    "ipynb",
    "java",
    "jl",
    "js",
    "json",
    "json5",
    "jsx",
    "kt",
    "kts",
    "less",
    "lhs",
    "lock",
    "log",
    "lua",
    "makefile",
    "md",
    "markdown",
    "mjs",
    "mk",
    "ml",
    "mli",
    "nim",
    "npmrc",
    "nvmrc",
    "odin",
    "org",
    "out",
    "patch",
    "php",
    "phtml",
    "pl",
    "pm",
    "pod",
    "properties",
    "proto",
    "ps1",
    "psm1",
    "py",
    "pyi",
    "pyw",
    "r",
    "rake",
    "rb",
    "rs",
    "rst",
    "sass",
    "scala",
    "scss",
    "sh",
    "sol",
    "sql",
    "svelte",
    "tex",
    "tf",
    "tfvars",
    "toml",
    "ts",
    "tsv",
    "tsx",
    "txt",
    "v",
    "vb",
    "vbs",
    "vue",
    "xml",
    "yaml",
    "yml",
    "zig",
    "zsh",
];

/// Extensions (no leading dot, lowercase) considered binary. 84 entries (see the
/// `TEXT_EXTENSIONS` doc comment above — same recount-from-archive correction).
pub const BINARY_EXTENSIONS: &[&str] = &[
    "7z", "a", "aac", "accdb", "ai", "aiff", "apk", "avi", "bin", "blend", "bmp", "bz2", "cab",
    "class", "db", "dll", "dmg", "doc", "docx", "dwg", "dylib", "ear", "eot", "epub", "exe", "fbx",
    "flac", "flv", "gif", "gz", "heic", "ico", "iso", "jar", "jpeg", "jpg", "lib", "m4a", "m4v",
    "max", "mdb", "mkv", "mov", "mp3", "mp4", "mpeg", "mpg", "o", "obj", "odp", "ods", "odt",
    "ogg", "opus", "otf", "pdf", "png", "ppt", "pptx", "psd", "pyc", "pyd", "pyo", "rar", "raw",
    "so", "sqlite", "sqlite3", "tar", "tif", "tiff", "ttf", "wav", "war", "webm", "webp", "wmv",
    "woff", "woff2", "wma", "xls", "xlsx", "xz", "zip",
];

/// Full lowercased filenames (no extension involved) considered text. 15 entries.
pub const TEXT_FILENAMES_WITHOUT_EXTENSION: &[&str] = &[
    ".env",
    ".env.example",
    ".env.local",
    ".env.development",
    ".env.production",
    ".gitignore",
    ".gitattributes",
    ".dockerignore",
    ".editorconfig",
    ".npmrc",
    ".nvmrc",
    "dockerfile",
    "makefile",
    "readme",
    "license",
];

static IGNORED_DIR_NAMES_SET: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| IGNORED_DIR_NAMES.iter().copied().collect());

static TEXT_EXTENSIONS_SET: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| TEXT_EXTENSIONS.iter().copied().collect());

static BINARY_EXTENSIONS_SET: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| BINARY_EXTENSIONS.iter().copied().collect());

static TEXT_FILENAMES_SET: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| TEXT_FILENAMES_WITHOUT_EXTENSION.iter().copied().collect());

/// `name` must already be lowercased by the caller.
pub(crate) fn is_base_ignored_dir_name(name: &str) -> bool {
    IGNORED_DIR_NAMES_SET.contains(name)
}

/// `extension` must already be lowercased and stripped of its leading dot.
pub(crate) fn is_text_extension(extension: &str) -> bool {
    TEXT_EXTENSIONS_SET.contains(extension)
}

/// `extension` must already be lowercased and stripped of its leading dot.
pub(crate) fn is_binary_extension(extension: &str) -> bool {
    BINARY_EXTENSIONS_SET.contains(extension)
}

/// `name` must already be lowercased by the caller.
pub(crate) fn is_text_filename_without_extension(name: &str) -> bool {
    TEXT_FILENAMES_SET.contains(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_sizes_match_legacy_counts() {
        assert_eq!(IGNORED_DIR_NAMES.len(), 18);
        assert_eq!(TEXT_EXTENSIONS.len(), 133);
        assert_eq!(BINARY_EXTENSIONS.len(), 84);
        assert_eq!(TEXT_FILENAMES_WITHOUT_EXTENSION.len(), 15);
    }

    #[test]
    fn no_duplicate_entries() {
        assert_eq!(IGNORED_DIR_NAMES_SET.len(), IGNORED_DIR_NAMES.len());
        assert_eq!(TEXT_EXTENSIONS_SET.len(), TEXT_EXTENSIONS.len());
        assert_eq!(BINARY_EXTENSIONS_SET.len(), BINARY_EXTENSIONS.len());
        assert_eq!(
            TEXT_FILENAMES_SET.len(),
            TEXT_FILENAMES_WITHOUT_EXTENSION.len()
        );
    }

    #[test]
    fn text_and_binary_extensions_never_overlap() {
        let overlap: Vec<&&str> = TEXT_EXTENSIONS_SET
            .intersection(&BINARY_EXTENSIONS_SET)
            .collect();
        assert!(overlap.is_empty(), "overlap: {overlap:?}");
    }

    #[test]
    fn spot_check_known_members() {
        assert!(is_base_ignored_dir_name("node_modules"));
        assert!(is_base_ignored_dir_name(".venv"));
        assert!(!is_base_ignored_dir_name("src"));
        assert!(is_text_extension("rs"));
        assert!(is_binary_extension("png"));
        assert!(is_text_filename_without_extension("dockerfile"));
        assert!(is_text_filename_without_extension(".env"));
    }
}
