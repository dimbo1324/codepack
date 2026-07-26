//! Technology-stack detection, ported from legacy `services/stack_detector.py`:
//! marker files in the project root drive which extra directories get pruned during
//! the walk (e.g. `node_modules` for Node.js). A mono-repo can match several stacks;
//! all matches are kept, sorted by marker count for display purposes only.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

struct StackRule {
    name: &'static str,
    markers: &'static [&'static str],
    marker_extensions: &'static [&'static str],
    extra_ignored_dirs: &'static [&'static str],
    /// Markers that must **all** be present before the rule can match at all.
    ///
    /// `markers` is an any-of list, which is right for an ecosystem identified by one
    /// unambiguous file (`Cargo.toml`, `go.mod`). It is wrong for a tree identified by a
    /// *combination*: `COPYING` alone would have made every GPL project a Linux kernel,
    /// and that verdict then prunes directories. Empty for every legacy rule, so their
    /// behaviour is unchanged.
    required_markers: &'static [&'static str],
}

/// One matched technology stack, with the markers that matched and the extra
/// directories this stack recommends pruning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackInfo {
    pub name: String,
    pub markers_found: Vec<String>,
    pub extra_ignored_dirs: Vec<String>,
}

const STACK_RULES: &[StackRule] = &[
    StackRule {
        name: "Node.js",
        markers: &["package.json"],
        marker_extensions: &[],
        extra_ignored_dirs: &[
            "node_modules",
            ".pnp",
            ".npm",
            ".yarn",
            ".next",
            ".nuxt",
            ".expo",
            ".turbo",
            ".parcel-cache",
            "out",
        ],
        required_markers: &[],
    },
    StackRule {
        name: "Python",
        markers: &[
            "requirements.txt",
            "pyproject.toml",
            "setup.py",
            "setup.cfg",
            "Pipfile",
        ],
        marker_extensions: &[],
        extra_ignored_dirs: &[
            ".venv",
            "venv",
            "env",
            "__pycache__",
            ".mypy_cache",
            ".ruff_cache",
            ".pytest_cache",
            ".tox",
            "htmlcov",
            "*.egg-info",
        ],
        required_markers: &[],
    },
    StackRule {
        name: "Go",
        markers: &["go.mod"],
        marker_extensions: &[],
        extra_ignored_dirs: &["vendor"],
        required_markers: &[],
    },
    StackRule {
        name: "Rust",
        markers: &["Cargo.toml"],
        marker_extensions: &[],
        extra_ignored_dirs: &["target"],
        required_markers: &[],
    },
    StackRule {
        name: "Java / Maven",
        markers: &["pom.xml"],
        marker_extensions: &[],
        extra_ignored_dirs: &["target", ".mvn"],
        required_markers: &[],
    },
    StackRule {
        name: "Java / Gradle",
        markers: &["build.gradle", "settings.gradle", "build.gradle.kts"],
        marker_extensions: &[],
        extra_ignored_dirs: &["build", ".gradle", ".idea"],
        required_markers: &[],
    },
    StackRule {
        name: ".NET / C#",
        markers: &[],
        marker_extensions: &[".csproj", ".sln"],
        extra_ignored_dirs: &["bin", "obj", ".vs"],
        required_markers: &[],
    },
    StackRule {
        name: "Flutter / Dart",
        markers: &["pubspec.yaml"],
        marker_extensions: &[],
        extra_ignored_dirs: &[".dart_tool", "build", ".flutter-plugins"],
        required_markers: &[],
    },
    StackRule {
        name: "PHP / Composer",
        markers: &["composer.json"],
        marker_extensions: &[],
        extra_ignored_dirs: &["vendor"],
        required_markers: &[],
    },
    StackRule {
        name: "Ruby",
        markers: &["Gemfile"],
        marker_extensions: &[],
        extra_ignored_dirs: &[".bundle", "vendor/bundle"],
        required_markers: &[],
    },
    StackRule {
        name: "iOS / Swift",
        markers: &["Package.swift"],
        marker_extensions: &[".xcodeproj", ".xcworkspace"],
        extra_ignored_dirs: &[".build", "DerivedData"],
        required_markers: &[],
    },
    StackRule {
        name: "Android",
        markers: &["AndroidManifest.xml"],
        marker_extensions: &[],
        extra_ignored_dirs: &["build", ".gradle", ".idea"],
        required_markers: &[],
    },
    // --- Systems and OS trees (owner decision 2026-07-26) ------------------------
    //
    // Legacy stopped at application ecosystems, so a kernel checkout matched nothing at
    // all and got no directory pruning. These rules are additive: none of their markers
    // appears in any golden fixture, so legacy parity cannot move.
    StackRule {
        name: "Linux kernel",
        // `Kconfig` is required and one of these must join it. `Kconfig` alone appears in
        // buildroot, Zephyr and plenty of embedded projects, and `COPYING` alone appears
        // in a large share of GPL repositories — either on its own would have declared
        // them kernels *and then pruned directories on the strength of that verdict*.
        // This mirrors `codepack_reports::context::stack`, which already required the
        // pair; the two must agree or the export plan and PROJECT_PROFILE.json describe
        // different projects.
        markers: &["Kbuild", "MAINTAINERS"],
        required_markers: &["Kconfig"],
        marker_extensions: &[],
        // Kbuild output. The path-shaped entries prune exactly one place each, which is
        // the point: a bare `generated` would also remove any directory a project happens
        // to call that. `debian/` is deliberately absent — it is real packaging source,
        // not build output. `.o`/`.ko` scattered beside their sources are kept out by the
        // binary-extension set, not by directory rules.
        extra_ignored_dirs: &[
            ".tmp_versions",
            "include/generated",
            "include/config",
            "arch/x86/include/generated",
            "arch/arm/include/generated",
            "arch/arm64/include/generated",
            "tools/testing/selftests/output",
            "Documentation/output",
            "rust/target",
        ],
    },
    StackRule {
        name: "C / CMake",
        markers: &["CMakeLists.txt"],
        marker_extensions: &[],
        extra_ignored_dirs: &[
            "build",
            "cmake-build-debug",
            "cmake-build-release",
            "_build",
        ],
        required_markers: &[],
    },
    StackRule {
        name: "C / Meson",
        markers: &["meson.build"],
        marker_extensions: &[],
        extra_ignored_dirs: &["builddir", "_build"],
        required_markers: &[],
    },
    StackRule {
        name: "C / Autotools",
        markers: &["configure.ac", "configure.in", "Makefile.am"],
        marker_extensions: &[],
        extra_ignored_dirs: &["autom4te.cache", ".deps", ".libs"],
        required_markers: &[],
    },
    StackRule {
        // Last of the C family on purpose: a bare Makefile is the weakest signal of the
        // four, and plenty of non-C projects ship one. It still earns a rule, because
        // "Makefile" is one of the languages the owner named.
        name: "C / Make",
        markers: &["Makefile", "makefile", "GNUmakefile"],
        marker_extensions: &[],
        // Deliberately empty. `obj` and `bin` were here and had to go: the matcher prunes
        // any directory with that *name* at any depth, so one root Makefile — which a
        // huge number of projects have — silently removed `src/bin/` from a Rust crate,
        // a Go repo's `bin/` of scripts, and so on. Unlike `.NET / C#`, where `.csproj`
        // unambiguously implies MSBuild's layout, a bare Makefile implies nothing about
        // directory names. `build` needs no entry: it is already in `IGNORED_DIR_NAMES`.
        extra_ignored_dirs: &[],
        required_markers: &[],
    },
];

/// Non-recursive: only looks at `root`'s immediate entries, matching legacy's
/// `root.iterdir()` two-pass scan (one pass for exact names, one for file extensions).
/// Extension markers only ever match plain *files*, never directories — a legacy quirk
/// preserved verbatim: bundle-style markers such as `.xcodeproj`/`.xcworkspace` are
/// directories on disk, so that marker path is effectively unreachable in practice,
/// exactly as in the Python original.
pub fn detect_stacks(root: &Path) -> Vec<StackInfo> {
    if !root.is_dir() {
        return Vec::new();
    }

    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };

    let mut names: HashSet<String> = HashSet::new();
    let mut suffixes: HashSet<String> = HashSet::new();
    for entry in entries.flatten() {
        let file_name = entry.file_name().to_string_lossy().to_string();
        if entry.path().is_file()
            && let Some(extension) = Path::new(&file_name).extension()
        {
            suffixes.insert(format!(".{}", extension.to_string_lossy()));
        }
        names.insert(file_name);
    }

    let mut results: Vec<StackInfo> = Vec::new();
    for rule in STACK_RULES {
        // Every required marker must be present before the rule is even considered. No
        // legacy rule sets any, so this is a no-op for all twelve of them.
        if !rule
            .required_markers
            .iter()
            .all(|marker| names.contains(*marker))
        {
            continue;
        }

        // Required markers count as found: they are the strongest evidence the rule has,
        // and leaving them out would make a kernel look less certain than a lone
        // `package.json` when the results are ranked.
        let mut found: Vec<String> = rule
            .required_markers
            .iter()
            .map(|marker| (*marker).to_string())
            .collect();
        for marker in rule.markers {
            if names.contains(*marker) {
                found.push((*marker).to_string());
            }
        }
        for extension in rule.marker_extensions {
            if suffixes.contains(*extension) {
                found.push(format!("*{extension}"));
            }
        }

        // A rule with required markers and no any-of hit has only proved half its case.
        if !rule.markers.is_empty()
            && !rule.required_markers.is_empty()
            && found.len() == rule.required_markers.len()
        {
            continue;
        }

        if !found.is_empty() {
            results.push(StackInfo {
                name: rule.name.to_string(),
                markers_found: found,
                extra_ignored_dirs: rule
                    .extra_ignored_dirs
                    .iter()
                    .map(|value| (*value).to_string())
                    .collect(),
            });
        }
    }

    // Stable sort: for an equal marker count, original rule-table order is preserved,
    // matching Python's guaranteed-stable `list.sort(reverse=True)`.
    results.sort_by_key(|info| std::cmp::Reverse(info.markers_found.len()));
    results
}

pub fn primary_stack(root: &Path) -> Option<StackInfo> {
    detect_stacks(root).into_iter().next()
}

/// Union of every matched stack's extra ignored directories — not just the primary
/// one's, since a mono-repo's directories must all be pruned.
pub fn merged_extra_ignored_dirs(stacks: &[StackInfo]) -> Vec<String> {
    let mut merged: HashSet<String> = HashSet::new();
    for stack in stacks {
        merged.extend(stack.extra_ignored_dirs.iter().cloned());
    }
    let mut merged: Vec<String> = merged.into_iter().collect();
    merged.sort();
    merged
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    fn touch(dir: &Path, names: &[&str]) {
        for name in names {
            File::create(dir.join(name)).unwrap();
        }
    }

    #[test]
    fn detects_nodejs() {
        let dir = tempfile::tempdir().unwrap();
        touch(dir.path(), &["package.json"]);
        let stacks = detect_stacks(dir.path());
        let node = stacks.iter().find(|s| s.name == "Node.js").unwrap();
        assert!(node.markers_found.contains(&"package.json".to_string()));
        assert!(
            node.extra_ignored_dirs
                .contains(&"node_modules".to_string())
        );
    }

    #[test]
    fn detects_python_via_any_marker() {
        for marker in [
            "requirements.txt",
            "pyproject.toml",
            "setup.py",
            "setup.cfg",
            "Pipfile",
        ] {
            let dir = tempfile::tempdir().unwrap();
            touch(dir.path(), &[marker]);
            let stacks = detect_stacks(dir.path());
            assert!(stacks.iter().any(|s| s.name == "Python"), "marker {marker}");
        }
    }

    #[test]
    fn detects_dotnet_via_extension_marker() {
        let dir = tempfile::tempdir().unwrap();
        touch(dir.path(), &["App.csproj"]);
        let stacks = detect_stacks(dir.path());
        let dotnet = stacks.iter().find(|s| s.name == ".NET / C#").unwrap();
        assert_eq!(dotnet.markers_found, vec!["*.csproj".to_string()]);
    }

    #[test]
    fn unknown_stack_returns_empty() {
        // Loose sources are still not a "stack": detection is marker-file based, and a
        // stray main.c says nothing about how the project is built. The C rules key on
        // build files (Makefile, CMakeLists.txt, configure.ac), never on `.c` itself.
        let dir = tempfile::tempdir().unwrap();
        touch(dir.path(), &["README.md", "main.c"]);
        assert!(detect_stacks(dir.path()).is_empty());
    }

    #[test]
    fn detects_a_linux_kernel_tree() {
        let dir = tempfile::tempdir().unwrap();
        touch(
            dir.path(),
            &["Kconfig", "Kbuild", "MAINTAINERS", "COPYING", "Makefile"],
        );

        let stacks = detect_stacks(dir.path());
        let kernel = stacks
            .iter()
            .find(|s| s.name == "Linux kernel")
            .expect("a kernel tree must be recognised as one");

        // Kconfig (required) + Kbuild + MAINTAINERS. COPYING is deliberately not a
        // marker at all — see the rule's comment.
        assert_eq!(kernel.markers_found.len(), 3);
        assert!(kernel.markers_found.contains(&"Kconfig".to_string()));
        // Those three outweigh the lone Makefile, so the primary answer is the specific
        // one rather than the generic one.
        assert_eq!(primary_stack(dir.path()).unwrap().name, "Linux kernel");
    }

    #[test]
    fn a_kernel_marker_on_its_own_is_not_a_kernel() {
        // Each of these appears in projects that are emphatically not kernels: Kconfig in
        // buildroot and Zephyr, COPYING and MAINTAINERS across GPL software generally.
        // Getting this wrong is not merely a wrong label — the verdict prunes
        // directories, so a false positive removes real files from the export.
        for marker in ["Kconfig", "COPYING", "MAINTAINERS", "Kbuild"] {
            let dir = tempfile::tempdir().unwrap();
            touch(dir.path(), &[marker]);
            assert!(
                !detect_stacks(dir.path())
                    .iter()
                    .any(|s| s.name == "Linux kernel"),
                "{marker} alone must not be read as a kernel"
            );
        }
    }

    #[test]
    fn every_extra_ignored_dir_is_something_the_matcher_can_match() {
        // Both kinds of entry now fire — a bare name at any depth, a `/`-path in exactly
        // one place. What must never appear is an entry the matcher cannot represent:
        // a backslash (it normalises to `/`, so a literal one is a typo), a leading or
        // trailing slash, or an empty segment, each of which would silently never match
        // while `manifest.json` advertised the directory as ignored.
        for rule in STACK_RULES {
            for entry in rule.extra_ignored_dirs {
                assert!(
                    !entry.contains('\\'),
                    "{}: {entry:?} uses a backslash; write paths with '/'",
                    rule.name
                );
                assert!(
                    !entry.starts_with('/') && !entry.ends_with('/'),
                    "{}: {entry:?} has a leading or trailing separator",
                    rule.name
                );
                assert!(
                    !entry.contains("//"),
                    "{}: {entry:?} has an empty path segment",
                    rule.name
                );
            }
        }
    }

    #[test]
    fn the_kernels_generated_output_is_actually_prunable() {
        // These used to be dead: entries with a separator landed in the name table and
        // could never fire, yet were reported as ignored. Pinning that they are real
        // paths the matcher understands, in the rule that owns them.
        let dir = tempfile::tempdir().unwrap();
        touch(dir.path(), &["Kconfig", "Kbuild"]);

        let kernel = detect_stacks(dir.path())
            .into_iter()
            .find(|s| s.name == "Linux kernel")
            .unwrap();

        let matcher =
            crate::walk::IgnoredDirMatcher::new(kernel.extra_ignored_dirs.iter().cloned());
        assert!(matcher.is_ignored(Path::new("include/generated")));
        assert!(matcher.is_ignored(Path::new("rust/target")));
        // and does not over-reach: a project's own `generated` elsewhere survives
        assert!(!matcher.is_ignored(Path::new("src/generated")));
    }

    #[test]
    fn a_bare_makefile_never_prunes_a_source_directory() {
        // A root Makefile is extremely common. When this rule carried `bin`/`obj`, one
        // such file silently removed `src/bin/` from any Rust crate and `bin/` from any
        // repository keeping scripts there.
        let dir = tempfile::tempdir().unwrap();
        touch(dir.path(), &["Makefile"]);

        let make = detect_stacks(dir.path())
            .into_iter()
            .find(|s| s.name == "C / Make")
            .expect("a Makefile should still be detected");

        assert!(
            make.extra_ignored_dirs.is_empty(),
            "a bare Makefile must not prune anything: {:?}",
            make.extra_ignored_dirs
        );
    }

    #[test]
    fn detects_ordinary_c_build_systems() {
        for (marker, expected) in [
            ("CMakeLists.txt", "C / CMake"),
            ("meson.build", "C / Meson"),
            ("configure.ac", "C / Autotools"),
            ("Makefile", "C / Make"),
        ] {
            let dir = tempfile::tempdir().unwrap();
            touch(dir.path(), &[marker]);
            let stacks = detect_stacks(dir.path());
            assert!(
                stacks.iter().any(|s| s.name == expected),
                "{marker} should detect {expected}"
            );
        }
    }

    #[test]
    fn a_rust_in_kernel_tree_reports_both() {
        // `torvalds/linux` carries rust/ with its own Cargo.toml. A mono-repo keeps every
        // match, so the export prunes both `target` and the kernel's generated output.
        let dir = tempfile::tempdir().unwrap();
        touch(dir.path(), &["Kconfig", "Kbuild", "Cargo.toml"]);

        let names: HashSet<String> = detect_stacks(dir.path())
            .into_iter()
            .map(|s| s.name)
            .collect();
        assert!(names.contains("Linux kernel"));
        assert!(names.contains("Rust"));

        let merged = merged_extra_ignored_dirs(&detect_stacks(dir.path()));
        assert!(merged.contains(&"target".to_string()));
        assert!(merged.contains(&".tmp_versions".to_string()));
    }

    #[test]
    fn monorepo_detects_all_matched_stacks() {
        let dir = tempfile::tempdir().unwrap();
        touch(dir.path(), &["package.json", "requirements.txt"]);
        let names: HashSet<String> = detect_stacks(dir.path())
            .into_iter()
            .map(|s| s.name)
            .collect();
        assert!(names.contains("Node.js"));
        assert!(names.contains("Python"));
    }

    #[test]
    fn primary_stack_is_most_confident_match() {
        let dir = tempfile::tempdir().unwrap();
        touch(dir.path(), &["package.json"]);
        assert_eq!(primary_stack(dir.path()).unwrap().name, "Node.js");
    }

    #[test]
    fn primary_stack_none_for_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        assert!(primary_stack(dir.path()).is_none());
    }

    #[test]
    fn merged_extra_ignored_dirs_unions_all_matched_stacks() {
        let dir = tempfile::tempdir().unwrap();
        touch(dir.path(), &["package.json", "requirements.txt"]);
        let merged = merged_extra_ignored_dirs(&detect_stacks(dir.path()));
        assert!(merged.contains(&"node_modules".to_string()));
        assert!(merged.contains(&".venv".to_string()));
    }

    #[test]
    fn merged_extra_ignored_dirs_empty_when_no_stack_detected() {
        let dir = tempfile::tempdir().unwrap();
        assert!(merged_extra_ignored_dirs(&detect_stacks(dir.path())).is_empty());
    }

    #[test]
    fn detect_stacks_nonexistent_dir_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(detect_stacks(&dir.path().join("does-not-exist")).is_empty());
    }

    #[test]
    fn python_extra_dirs_includes_the_literal_egg_info_glob_string() {
        let dir = tempfile::tempdir().unwrap();
        touch(dir.path(), &["pyproject.toml"]);
        let python = detect_stacks(dir.path())
            .into_iter()
            .find(|s| s.name == "Python")
            .unwrap();
        assert!(
            python
                .extra_ignored_dirs
                .contains(&"*.egg-info".to_string())
        );
    }
}
