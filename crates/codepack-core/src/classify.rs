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

/// Systems-programming extensions legacy never had, kept **separate** from the verbatim
/// legacy set above so its provenance and its "133 entries" claim stay literally true.
/// Both sets are unioned at lookup time.
///
/// Owner decision 2026-07-26 (`docs/decisions/open-questions.md`): support OS and kernel
/// trees such as `torvalds/linux`, whose languages are C, Assembly, Rust, Shell, Python
/// and Make.
///
/// Assembly is the reason this is not cosmetic. Without `s`/`asm` here,
/// [`should_consider_text_file`] answers "no" for every `.S` file in a kernel tree, and
/// the export drops them from the text dump **silently** — mislabelled content is
/// annoying, missing content is a wrong answer to "what is in this project".
pub const SYSTEMS_TEXT_EXTENSIONS: &[&str] = &[
    // Assembly. `.S` is preprocessed assembly and is the common form in the kernel;
    // lookups lowercase the extension, so one entry covers `.s` and `.S` both.
    "s",
    "asm",
    // Device trees: sources and includes are text, `.dtb` is compiled output and stays
    // out deliberately.
    "dts",
    "dtsi",
    "dtso",
    // Linker scripts, including the `.lds.S` form that ends in `.s` above.
    "lds",
    "ld",
    // Build inputs and generated-file templates. `config` is already in the legacy
    // set, so it is not repeated here.
    "ac",
    "am",
    "m4",
    "in",
    "mak",
    "make",
    "kconfig",
    "defconfig",
    // Kernel tooling: Coccinelle semantic patches, awk and sed scripts.
    "cocci",
    "awk",
    "sed",
];
// Deliberately NOT here, though an earlier draft had them: `map` (a `.js.map` source map
// is routinely multi-megabyte and embeds the whole original source, so admitting it would
// bloat the text dump and the token budget of every front-end project — none of which
// follows from "support kernel trees"), plus `spec`, `service`, `rules`, `syms`, `ver`,
// `texi` and `overlay`, which no part of the six requested languages needs. This set
// changes what gets exported for *existing* users, so it stays as narrow as the task.

/// Extensionless filenames a systems or kernel tree is full of, kept separate from the
/// verbatim legacy 15 for the same provenance reason as [`SYSTEMS_TEXT_EXTENSIONS`].
///
/// `kconfig` and `kbuild` matter most: a kernel checkout holds thousands of each, they
/// carry no extension at all, and legacy's list stops at `makefile`.
pub const SYSTEMS_TEXT_FILENAMES: &[&str] = &[
    "kconfig",
    "kbuild",
    "gnumakefile",
    // The extensionless files that sit in a kernel root beside `makefile`, which legacy
    // already covered. `vagrantfile`, `justfile`, `procfile`, `todo`, `version` and
    // friends were dropped from an earlier draft: they are not kernel files, and this
    // list changes what gets exported for everyone.
    "maintainers",
    "copying",
    "authors",
    "credits",
];

// Both halves are unioned once, here, so every caller sees one set and no lookup has to
// remember there are two lists.
static TEXT_EXTENSIONS_SET: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    TEXT_EXTENSIONS
        .iter()
        .chain(SYSTEMS_TEXT_EXTENSIONS.iter())
        .copied()
        .collect()
});
static BINARY_EXTENSIONS_SET: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| BINARY_EXTENSIONS.iter().copied().collect());
static TEXT_FILENAMES_SET: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    TEXT_FILENAMES_WITHOUT_EXTENSION
        .iter()
        .chain(SYSTEMS_TEXT_FILENAMES.iter())
        .copied()
        .collect()
});

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
        // The lookup sets are the union of the legacy list and the systems list, so the
        // expected size is the sum. Comparing against the sum still catches a repeat
        // within either list — which is the whole point of this test — while
        // `the_systems_additions_never_overlap_the_legacy_sets` covers repeats across
        // the two.
        assert_eq!(
            TEXT_EXTENSIONS_SET.len(),
            TEXT_EXTENSIONS.len() + SYSTEMS_TEXT_EXTENSIONS.len()
        );
        assert_eq!(BINARY_EXTENSIONS_SET.len(), BINARY_EXTENSIONS.len());
        assert_eq!(
            TEXT_FILENAMES_SET.len(),
            TEXT_FILENAMES_WITHOUT_EXTENSION.len() + SYSTEMS_TEXT_FILENAMES.len()
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

    #[test]
    fn kernel_assembly_counts_as_text() {
        // The gap that motivated SYSTEMS_TEXT_EXTENSIONS: before it, every `.S` in a
        // kernel tree answered `false` here and vanished from the text dump without a
        // word. Both cases are asserted because `.S` (preprocessed) is the common form
        // and only lowercasing makes one entry cover both.
        assert!(should_consider_text_file(Path::new("arch/x86/boot/head.S")));
        assert!(should_consider_text_file(Path::new(
            "arch/arm/lib/memcpy.s"
        )));
        assert!(should_consider_text_file(Path::new("boot/entry.asm")));
    }

    #[test]
    fn kernel_build_files_without_an_extension_count_as_text() {
        for path in [
            "drivers/net/Kconfig",
            "drivers/net/Kbuild",
            "MAINTAINERS",
            "GNUmakefile",
        ] {
            assert!(should_consider_text_file(Path::new(path)), "{path}");
        }
    }

    #[test]
    fn device_trees_and_linker_scripts_count_as_text_but_compiled_output_does_not() {
        assert!(should_consider_text_file(Path::new(
            "arch/arm/boot/dts/a.dts"
        )));
        assert!(should_consider_text_file(Path::new("include/soc.dtsi")));
        assert!(should_consider_text_file(Path::new("kernel/vmlinux.lds")));
        // `.dtb` is the compiled blob. Left out on purpose: it is real binary content,
        // and admitting it would put megabytes of it into a text dump.
        assert!(!should_consider_text_file(Path::new("boot/board.dtb")));
    }

    #[test]
    fn the_systems_additions_never_overlap_the_legacy_sets() {
        // An overlap would be harmless at lookup time but would mean the separation is
        // decorative rather than real, and the next reader could not trust either list.
        for extension in SYSTEMS_TEXT_EXTENSIONS {
            assert!(
                !TEXT_EXTENSIONS.contains(extension),
                "{extension} is already in the legacy set"
            );
            assert!(
                !BINARY_EXTENSIONS.contains(extension),
                "{extension} is claimed by both text and binary"
            );
        }
        for name in SYSTEMS_TEXT_FILENAMES {
            assert!(
                !TEXT_FILENAMES_WITHOUT_EXTENSION.contains(name),
                "{name} is already in the legacy set"
            );
        }
    }
}
