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
    },
    StackRule {
        name: "Go",
        markers: &["go.mod"],
        marker_extensions: &[],
        extra_ignored_dirs: &["vendor"],
    },
    StackRule {
        name: "Rust",
        markers: &["Cargo.toml"],
        marker_extensions: &[],
        extra_ignored_dirs: &["target"],
    },
    StackRule {
        name: "Java / Maven",
        markers: &["pom.xml"],
        marker_extensions: &[],
        extra_ignored_dirs: &["target", ".mvn"],
    },
    StackRule {
        name: "Java / Gradle",
        markers: &["build.gradle", "settings.gradle", "build.gradle.kts"],
        marker_extensions: &[],
        extra_ignored_dirs: &["build", ".gradle", ".idea"],
    },
    StackRule {
        name: ".NET / C#",
        markers: &[],
        marker_extensions: &[".csproj", ".sln"],
        extra_ignored_dirs: &["bin", "obj", ".vs"],
    },
    StackRule {
        name: "Flutter / Dart",
        markers: &["pubspec.yaml"],
        marker_extensions: &[],
        extra_ignored_dirs: &[".dart_tool", "build", ".flutter-plugins"],
    },
    StackRule {
        name: "PHP / Composer",
        markers: &["composer.json"],
        marker_extensions: &[],
        extra_ignored_dirs: &["vendor"],
    },
    StackRule {
        name: "Ruby",
        markers: &["Gemfile"],
        marker_extensions: &[],
        extra_ignored_dirs: &[".bundle", "vendor/bundle"],
    },
    StackRule {
        name: "iOS / Swift",
        markers: &["Package.swift"],
        marker_extensions: &[".xcodeproj", ".xcworkspace"],
        extra_ignored_dirs: &[".build", "DerivedData"],
    },
    StackRule {
        name: "Android",
        markers: &["AndroidManifest.xml"],
        marker_extensions: &[],
        extra_ignored_dirs: &["build", ".gradle", ".idea"],
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
        let mut found: Vec<String> = Vec::new();
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
        let dir = tempfile::tempdir().unwrap();
        touch(dir.path(), &["README.md", "main.c"]);
        assert!(detect_stacks(dir.path()).is_empty());
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
