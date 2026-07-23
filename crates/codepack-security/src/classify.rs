use std::collections::HashSet;
use std::path::Path;
use std::sync::LazyLock;

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

pub const BINARY_EXTENSIONS: &[&str] = &[
    "7z", "a", "aac", "accdb", "ai", "aiff", "apk", "avi", "bin", "blend", "bmp", "bz2", "cab",
    "class", "db", "dll", "dmg", "doc", "docx", "dwg", "dylib", "ear", "eot", "epub", "exe", "fbx",
    "flac", "flv", "gif", "gz", "heic", "ico", "iso", "jar", "jpeg", "jpg", "lib", "m4a", "m4v",
    "max", "mdb", "mkv", "mov", "mp3", "mp4", "mpeg", "mpg", "o", "obj", "odp", "ods", "odt",
    "ogg", "opus", "otf", "pdf", "png", "ppt", "pptx", "psd", "pyc", "pyd", "pyo", "rar", "raw",
    "so", "sqlite", "sqlite3", "tar", "tif", "tiff", "ttf", "wav", "war", "webm", "webp", "wmv",
    "woff", "woff2", "wma", "xls", "xlsx", "xz", "zip",
];

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

pub const BINARY_SAMPLE_BYTES: usize = 8192;

static TEXT_EXTENSIONS_SET: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| TEXT_EXTENSIONS.iter().copied().collect());

static BINARY_EXTENSIONS_SET: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| BINARY_EXTENSIONS.iter().copied().collect());

static TEXT_FILENAMES_SET: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| TEXT_FILENAMES_WITHOUT_EXTENSION.iter().copied().collect());

pub fn should_consider_text_file(path: &Path) -> bool {
    let name = path
        .file_name()
        .map(|value| value.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let suffix = path
        .extension()
        .map(|value| value.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    if TEXT_FILENAMES_SET.contains(name.as_str()) {
        return true;
    }
    if BINARY_EXTENSIONS_SET.contains(suffix.as_str()) {
        return false;
    }
    if TEXT_EXTENSIONS_SET.contains(suffix.as_str()) {
        return true;
    }
    false
}

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
    (control_bytes as f64) / (sample.len() as f64) > 0.3
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
        let mut sample = vec![b'a'; 6];
        sample.extend(std::iter::repeat_n(0x01u8, 4));
        assert!(looks_binary(&sample));
    }

    #[test]
    fn looks_binary_only_samples_first_8192_bytes() {
        let mut raw = vec![b'a'; BINARY_SAMPLE_BYTES];
        raw.push(0u8);
        assert!(!looks_binary(&raw));
    }
}
