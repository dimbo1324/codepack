//! [`Inventory`]: the aggregate file/byte/language summary built once per run,
//! ported from legacy `utils/inventory.py::collect_basic_inventory` — except it is
//! built from [`codepack_scanner::ExportPlan`]'s already-planned `included_files`
//! list, never from a fresh directory walk (this crate's scope boundary, `lib.rs`).
//!
//! `total_dirs` is a documented, narrower stand-in for legacy's `iter_project_dirs`:
//! legacy walks the copied tree and counts every directory, including empty ones.
//! `ExportPlan` carries no directory list at all (only files plus `skipped_dirs`, the
//! dirs pruned by ignore rules), so this counts the distinct directories that contain
//! at least one *included* file. An empty directory therefore never contributes here
//! — a real, accepted parity gap given the architectural boundary (S7 does not
//! re-walk), not an oversight.

use std::collections::BTreeSet;

use codepack_scanner::ExportPlan;

use crate::paths::{extension_key, file_name_of};

/// `LANGUAGE_BY_EXTENSION`, ported verbatim from legacy `constants.py`.
const LANGUAGE_BY_EXTENSION: &[(&str, &str)] = &[
    ("py", "Python"),
    ("pyw", "Python"),
    ("pyi", "Python"),
    ("js", "JavaScript"),
    ("jsx", "JavaScript / React"),
    ("mjs", "JavaScript"),
    ("cjs", "JavaScript"),
    ("ts", "TypeScript"),
    ("tsx", "TypeScript / React"),
    ("css", "CSS"),
    ("scss", "SCSS"),
    ("sass", "Sass"),
    ("less", "Less"),
    ("html", "HTML"),
    ("htm", "HTML"),
    ("vue", "Vue"),
    ("svelte", "Svelte"),
    ("astro", "Astro"),
    ("go", "Go"),
    ("rs", "Rust"),
    ("java", "Java"),
    ("kt", "Kotlin"),
    ("kts", "Kotlin"),
    ("cs", "C#"),
    ("cpp", "C++"),
    ("cxx", "C++"),
    ("cc", "C++"),
    ("c", "C"),
    ("h", "C/C++ Header"),
    ("hpp", "C++ Header"),
    ("rb", "Ruby"),
    ("php", "PHP"),
    ("swift", "Swift"),
    ("dart", "Dart"),
    ("sql", "SQL"),
    ("sh", "Shell"),
    ("bash", "Shell"),
    ("zsh", "Shell"),
    ("fish", "Shell"),
    ("ps1", "PowerShell"),
    ("bat", "Batch"),
    ("cmd", "Batch"),
    ("json", "JSON"),
    ("json5", "JSON5"),
    ("yaml", "YAML"),
    ("yml", "YAML"),
    ("toml", "TOML"),
    ("xml", "XML"),
    ("md", "Markdown"),
    ("markdown", "Markdown"),
    ("rst", "reStructuredText"),
    ("dockerfile", "Dockerfile"),
];

/// Languages legacy's table never covered, kept separate so the list above stays a
/// verbatim port. Owner decision 2026-07-26: support OS and kernel trees, whose
/// breakdown is C, Assembly, Rust, Shell, Python and Make.
const SYSTEMS_LANGUAGE_BY_EXTENSION: &[(&str, &str)] = &[
    ("s", "Assembly"),
    ("asm", "Assembly"),
    ("lds", "Linker Script"),
    ("ld", "Linker Script"),
    ("dts", "Device Tree"),
    ("dtsi", "Device Tree"),
    ("dtso", "Device Tree"),
    ("mk", "Makefile"),
    ("mak", "Makefile"),
    ("make", "Makefile"),
    ("cmake", "CMake"),
    ("ac", "Autoconf"),
    ("am", "Automake"),
    ("m4", "M4"),
    ("pl", "Perl"),
    ("pm", "Perl"),
    ("awk", "Awk"),
    ("cocci", "Coccinelle"),
    ("lua", "Lua"),
    ("zig", "Zig"),
    ("scala", "Scala"),
    ("ex", "Elixir"),
    ("exs", "Elixir"),
    ("erl", "Erlang"),
    ("hs", "Haskell"),
    ("jl", "Julia"),
    ("proto", "Protocol Buffers"),
    ("tf", "Terraform"),
];

/// Extensionless files whose name *is* the language signal.
///
/// A kernel checkout holds thousands of `Makefile` and `Kconfig` files, and
/// `extension_key` reports every one of them as `[no extension]` — so a map keyed on
/// extension can never see them, no matter how many entries it gains.
///
/// Deliberately a **separate lookup** rather than a new special case inside
/// `extension_key`: that function also feeds the `by_extension` statistics, and
/// reshaping those would move report output that legacy parity is measured against.
/// Language detection gets richer; the statistics stay byte-identical.
const LANGUAGE_BY_FILENAME: &[(&str, &str)] = &[
    ("makefile", "Makefile"),
    ("gnumakefile", "Makefile"),
    ("kbuild", "Makefile"),
    ("kconfig", "Kconfig"),
    ("dockerfile", "Dockerfile"),
    ("vagrantfile", "Ruby"),
    ("rakefile", "Ruby"),
    ("gemfile", "Ruby"),
    ("justfile", "Just"),
    ("cmakelists.txt", "CMake"),
    ("meson.build", "Meson"),
];

fn language_for_extension(extension: &str) -> Option<&'static str> {
    LANGUAGE_BY_EXTENSION
        .iter()
        .chain(SYSTEMS_LANGUAGE_BY_EXTENSION.iter())
        .find(|(ext, _)| *ext == extension)
        .map(|(_, language)| *language)
}

/// The language of `relative_path`, in three passes: exact filename, then extension,
/// then the filename's stem.
///
/// The order is the whole design. An exact name has to win first, because `Kconfig` and
/// `Makefile` have no extension to fall back on. A real extension wins next, so
/// `makefile.py` reads as Python and `Makefile.am` as Automake — which is what those
/// files actually are. Only then does the stem decide, leaving `Makefile.in` and
/// `Kconfig.debug` correctly labelled.
///
/// Splitting on both separators is not padding: `Inventory` stores `\`-separated paths,
/// so a `/` path would otherwise keep its directories inside the "filename" and match
/// nothing at all.
fn language_for_path(relative_path: &str, extension: &str) -> Option<&'static str> {
    let name = relative_path
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or(relative_path)
        .to_lowercase();

    if let Some((_, language)) = LANGUAGE_BY_FILENAME.iter().find(|(key, _)| *key == name) {
        return Some(language);
    }
    if let Some(language) = language_for_extension(extension) {
        return Some(language);
    }
    for (key, language) in LANGUAGE_BY_FILENAME {
        if name.starts_with(&format!("{key}.")) {
            return Some(language);
        }
    }
    None
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryFile {
    pub relative_path: String,
    pub size: u64,
    pub extension: String,
    pub language: Option<&'static str>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExtensionStat {
    pub extension: String,
    pub count: usize,
    pub total_size: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LanguageStat {
    pub language: String,
    pub count: usize,
    pub total_size: u64,
}

#[derive(Debug, Clone, Default)]
pub struct Inventory {
    pub files: Vec<InventoryFile>,
    pub total_size: u64,
    pub total_dirs: usize,
    /// Descending by file count, ties broken by extension name (deterministic;
    /// legacy's `Counter.most_common()` tie-break is insertion order, which Rust's
    /// `ExportPlan.included_files` order does not guarantee to reproduce identically).
    pub by_extension: Vec<ExtensionStat>,
    pub by_language: Vec<LanguageStat>,
}

impl Inventory {
    pub fn from_plan(plan: &ExportPlan) -> Self {
        let mut files = Vec::with_capacity(plan.included_files.len());
        let mut total_size = 0u64;
        let mut extension_counts: std::collections::HashMap<String, ExtensionStat> =
            std::collections::HashMap::new();
        let mut language_counts: std::collections::HashMap<String, LanguageStat> =
            std::collections::HashMap::new();
        let mut dirs: BTreeSet<String> = BTreeSet::new();

        for planned in &plan.included_files {
            let extension = extension_key(&planned.relative_path);
            let language = language_for_path(&planned.relative_path, &extension);

            total_size += planned.size;

            let stat = extension_counts
                .entry(extension.clone())
                .or_insert_with(|| ExtensionStat {
                    extension: extension.clone(),
                    count: 0,
                    total_size: 0,
                });
            stat.count += 1;
            stat.total_size += planned.size;

            if let Some(language) = language {
                let stat = language_counts
                    .entry(language.to_string())
                    .or_insert_with(|| LanguageStat {
                        language: language.to_string(),
                        count: 0,
                        total_size: 0,
                    });
                stat.count += 1;
                stat.total_size += planned.size;
            }

            if let Some(index) = planned.relative_path.rfind('\\') {
                dirs.insert(planned.relative_path[..index].to_string());
            }

            files.push(InventoryFile {
                relative_path: planned.relative_path.clone(),
                size: planned.size,
                extension,
                language,
            });
        }

        let mut by_extension: Vec<ExtensionStat> = extension_counts.into_values().collect();
        by_extension.sort_by(|a, b| {
            b.count
                .cmp(&a.count)
                .then_with(|| a.extension.cmp(&b.extension))
        });

        let mut by_language: Vec<LanguageStat> = language_counts.into_values().collect();
        by_language.sort_by(|a, b| {
            b.count
                .cmp(&a.count)
                .then_with(|| a.language.cmp(&b.language))
        });

        Self {
            files,
            total_size,
            total_dirs: dirs.len(),
            by_extension,
            by_language,
        }
    }
}

/// True when any included file's final path segment matches `predicate` (case
/// already lowered by the caller where relevant).
pub fn any_file_name(inventory: &Inventory, mut predicate: impl FnMut(&str) -> bool) -> bool {
    inventory
        .files
        .iter()
        .any(|file| predicate(&file_name_of(&file.relative_path).to_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use codepack_core::CancellationToken;
    use codepack_scanner::{ExportIgnoreRules, ScanOptions, build_export_plan};

    fn build_plan(dir: &std::path::Path) -> ExportPlan {
        build_export_plan(
            dir,
            &ScanOptions::default(),
            &ExportIgnoreRules::default(),
            &codepack_scanner::no_safety_classification,
            &CancellationToken::new(),
        )
        .unwrap()
    }

    #[test]
    fn kernel_shaped_paths_report_their_language() {
        // The languages `torvalds/linux` is actually made of, plus the two build files
        // that carry no extension at all. Paths use `\` because that is what `Inventory`
        // stores.
        let cases = [
            (r"kernel\sched\core.c", "C"),
            (r"include\linux\sched.h", "C/C++ Header"),
            (r"arch\x86\boot\head.S", "Assembly"),
            (r"arch\arm\lib\memcpy.s", "Assembly"),
            (r"rust\kernel\lib.rs", "Rust"),
            (r"scripts\checkpatch.pl", "Perl"),
            (r"scripts\gen.sh", "Shell"),
            (r"tools\perf\util\setup.py", "Python"),
            (r"drivers\net\Makefile", "Makefile"),
            (r"drivers\net\Kconfig", "Kconfig"),
            (r"drivers\gpu\Kbuild", "Makefile"),
            (r"arch\arm\boot\dts\board.dts", "Device Tree"),
            (r"arch\x86\kernel\vmlinux.lds", "Linker Script"),
        ];
        for (path, expected) in cases {
            let extension = extension_key(path);
            assert_eq!(
                language_for_path(path, &extension),
                Some(expected),
                "{path}"
            );
        }
    }

    #[test]
    fn a_suffixed_build_file_keeps_a_sensible_language() {
        // The stem only decides when the suffix means nothing on its own.
        assert_eq!(
            language_for_path("Makefile.in", &extension_key("Makefile.in")),
            Some("Makefile")
        );
        assert_eq!(
            language_for_path("Kconfig.debug", &extension_key("Kconfig.debug")),
            Some("Kconfig")
        );
        // `.am` is a language of its own, so it wins over the stem — correctly:
        // Makefile.am is an Automake input, not a makefile.
        assert_eq!(
            language_for_path("Makefile.am", &extension_key("Makefile.am")),
            Some("Automake")
        );
    }

    #[test]
    fn a_real_extension_wins_over_a_coincidental_stem() {
        // `makefile.py` is a Python script named after what it generates. The extension
        // pass runs before the stem pass precisely so the stem cannot hijack it.
        assert_eq!(
            language_for_path(r"tools\makefile.py", &extension_key(r"tools\makefile.py")),
            Some("Python")
        );
    }

    #[test]
    fn both_path_separators_are_understood() {
        assert_eq!(
            language_for_path(r"drivers\net\Kconfig", "[no extension]"),
            Some("Kconfig")
        );
        assert_eq!(
            language_for_path("drivers/net/Kconfig", "[no extension]"),
            Some("Kconfig")
        );
    }

    #[test]
    fn language_detection_does_not_disturb_extension_statistics() {
        // Why filename detection lives beside `extension_key` rather than inside it:
        // `by_extension` feeds report output that legacy parity is measured on.
        assert_eq!(extension_key(r"drivers\net\Makefile"), "[no extension]");
        assert_eq!(extension_key(r"drivers\net\Kconfig"), "[no extension]");
        assert_eq!(extension_key(r"arch\x86\boot\head.S"), "s");
    }

    #[test]
    fn aggregates_extension_and_language_stats() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("main.py"), "print('hi')\n").unwrap();
        std::fs::write(dir.path().join("second.py"), "x = 1\n").unwrap();
        std::fs::write(dir.path().join("README.md"), "# hi\n").unwrap();

        let plan = build_plan(dir.path());
        let inventory = Inventory::from_plan(&plan);

        assert_eq!(inventory.files.len(), 3);
        assert_eq!(inventory.total_size, plan.summary.estimated_included_bytes);

        let python_ext = inventory
            .by_extension
            .iter()
            .find(|stat| stat.extension == "py")
            .unwrap();
        assert_eq!(python_ext.count, 2);

        let python_lang = inventory
            .by_language
            .iter()
            .find(|stat| stat.language == "Python")
            .unwrap();
        assert_eq!(python_lang.count, 2);
    }

    #[test]
    fn counts_directories_containing_included_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src/utils")).unwrap();
        std::fs::write(dir.path().join("src/main.py"), "x = 1\n").unwrap();
        std::fs::write(dir.path().join("src/utils/helper.py"), "y = 2\n").unwrap();

        let plan = build_plan(dir.path());
        let inventory = Inventory::from_plan(&plan);

        assert_eq!(inventory.total_dirs, 2);
    }
}
