//! Text/binary classification, ported from legacy `utils/text_utils.py`:
//! `should_consider_text_file` (extension/filename only, no content read) and
//! `looks_binary` (content sniffing on a bounded sample), together with the three
//! constant sets they consult.
//!
//! This lived in duplicate in `codepack-scanner` (S2) and `codepack-security` (S3),
//! because ROADMAP's dependency table let S3 depend only on S1 and the sets were needed
//! on both sides. Both copies were verified identical entry for entry before the move
//! (133/84/15), so this is a relocation, not a merge of two drifted sets. Open question
//! Q7, closed by owner decision 2026-07-25: one definition, in the crate every other
//! crate already depends on.

use std::collections::HashSet;
use std::path::Path;
use std::sync::LazyLock;

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

static TEXT_EXTENSIONS_SET: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| TEXT_EXTENSIONS.iter().copied().collect());
static BINARY_EXTENSIONS_SET: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| BINARY_EXTENSIONS.iter().copied().collect());
static TEXT_FILENAMES_SET: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| TEXT_FILENAMES_WITHOUT_EXTENSION.iter().copied().collect());

fn is_text_extension(extension: &str) -> bool {
    TEXT_EXTENSIONS_SET.contains(extension)
}

fn is_binary_extension(extension: &str) -> bool {
    BINARY_EXTENSIONS_SET.contains(extension)
}

fn is_text_filename_without_extension(name: &str) -> bool {
    TEXT_FILENAMES_SET.contains(name)
}

/// Legacy's `_BINARY_SAMPLE_BYTES`: only the first 8192 bytes of a file are sniffed.
pub const BINARY_SAMPLE_BYTES: usize = 8192;

/// Extension/filename-only classification — never reads file content.
///
/// Order matters and is load-bearing (ported verbatim from
/// `should_consider_text_file`): a name match short-circuits before the extension is
/// even consulted, and a binary-extension match is checked before a text-extension
/// match.
pub fn should_consider_text_file(path: &Path) -> bool {
    let name = path
        .file_name()
        .map(|value| value.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let suffix = path
        .extension()
        .map(|value| value.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    if is_text_filename_without_extension(&name) {
        return true;
    }
    if is_binary_extension(&suffix) {
        return false;
    }
    if is_text_extension(&suffix) {
        return true;
    }
    false
}

/// Content-sniffing heuristic on the first [`BINARY_SAMPLE_BYTES`] of `raw`: a NUL
/// byte anywhere in the sample means binary; otherwise a file is binary when more than
/// 30% of the sampled bytes are control bytes (`< 9` or strictly between 13 and 32,
/// i.e. excluding tab, newline, and carriage return).
pub fn looks_binary(raw: &[u8]) -> bool {
    let sample_len = raw.len().min(BINARY_SAMPLE_BYTES);
    let sample = &raw[..sample_len];

    if sample.contains(&0u8) {
        return true;
    }
    if sample.is_empty() {
        return false;
    }

    let control_bytes = sample
        .iter()
        .filter(|&&byte| byte < 9 || (13 < byte && byte < 32))
        .count();
    (control_bytes as f64 / sample.len() as f64) > 0.30
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_sizes_match_legacy_counts() {
        assert_eq!(TEXT_EXTENSIONS.len(), 133);
        assert_eq!(BINARY_EXTENSIONS.len(), 84);
        assert_eq!(TEXT_FILENAMES_WITHOUT_EXTENSION.len(), 15);
    }

    #[test]
    fn no_duplicate_entries() {
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
        assert!(is_text_extension("rs"));
        assert!(is_binary_extension("png"));
        assert!(is_text_filename_without_extension("dockerfile"));
        assert!(is_text_filename_without_extension(".env"));
    }

    #[test]
    fn text_filename_without_extension_wins_first() {
        assert!(should_consider_text_file(Path::new("Dockerfile")));
        assert!(should_consider_text_file(Path::new(".env")));
        assert!(should_consider_text_file(Path::new("README")));
        assert!(should_consider_text_file(Path::new("LICENSE")));
    }

    #[test]
    fn binary_extension_beats_no_match() {
        assert!(!should_consider_text_file(Path::new("photo.PNG")));
        assert!(!should_consider_text_file(Path::new("archive.zip")));
    }

    #[test]
    fn text_extension_matches() {
        assert!(should_consider_text_file(Path::new("main.rs")));
        assert!(should_consider_text_file(Path::new("index.TS")));
    }

    #[test]
    fn unknown_extension_defaults_to_binary() {
        assert!(!should_consider_text_file(Path::new("mystery.xyz123")));
        assert!(!should_consider_text_file(Path::new("no_extension_at_all")));
    }

    #[test]
    fn readme_with_extension_uses_extension_rule_not_filename_rule() {
        // "readme.md" as a whole filename is not in TEXT_FILENAMES_WITHOUT_EXTENSION,
        // but "md" is in TEXT_EXTENSIONS, so it is still text via the third branch.
        assert!(should_consider_text_file(Path::new("README.md")));
    }

    #[test]
    fn looks_binary_detects_nul_byte() {
        assert!(looks_binary(b"hello\x00world"));
    }

    #[test]
    fn looks_binary_empty_sample_is_not_binary() {
        assert!(!looks_binary(b""));
    }

    #[test]
    fn looks_binary_plain_text_is_not_binary() {
        assert!(!looks_binary(
            b"the quick brown fox\njumps over\r\n\tthe lazy dog"
        ));
    }

    #[test]
    fn looks_binary_above_threshold_control_bytes() {
        // 4/10 = 0.40 > 0.30 -> binary.
        let mut sample = vec![b'a'; 6];
        sample.extend(std::iter::repeat_n(0x01u8, 4));
        assert!(looks_binary(&sample));
    }

    #[test]
    fn looks_binary_at_or_below_threshold_control_bytes() {
        // 3/10 = 0.30, not strictly greater than 0.30 -> not binary.
        let mut sample = vec![b'a'; 7];
        sample.extend(std::iter::repeat_n(0x01u8, 3));
        assert!(!looks_binary(&sample));
    }

    #[test]
    fn looks_binary_only_samples_first_8192_bytes() {
        let mut raw = vec![b'a'; BINARY_SAMPLE_BYTES];
        raw.push(0u8);
        assert!(!looks_binary(&raw));
    }
}
