//! `PlannedFile.group` classification, ported from legacy `_classify_group`. Checked
//! in this exact if/elif order — the first matching bucket wins.

use std::path::Path;

pub(super) fn classify_group(relative_path: &Path) -> &'static str {
    let name = relative_path
        .file_name()
        .map(|value| value.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let suffix = relative_path
        .extension()
        .map(|value| value.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let is_test_segment = relative_path.components().any(|component| {
        let part = component.as_os_str().to_string_lossy().to_lowercase();
        part == "test" || part == "tests" || part == "__tests__"
    });

    if is_test_segment || name.starts_with("test_") {
        return "tests";
    }
    if matches!(suffix.as_str(), "py" | "pyw" | "pyi") {
        return "python_source";
    }
    if matches!(
        suffix.as_str(),
        "js" | "jsx" | "ts" | "tsx" | "css" | "scss" | "html" | "vue" | "svelte"
    ) {
        return "frontend_source";
    }
    if matches!(
        suffix.as_str(),
        "go" | "rs" | "java" | "kt" | "cs" | "c" | "cpp" | "h" | "hpp"
    ) {
        return "backend_or_system_source";
    }
    if matches!(suffix.as_str(), "md" | "rst" | "adoc" | "txt")
        || name == "readme.md"
        || name == "license"
    {
        return "docs";
    }
    if matches!(
        suffix.as_str(),
        "json" | "yaml" | "yml" | "toml" | "ini" | "cfg" | "conf" | "lock"
    ) || name.starts_with("dockerfile")
    {
        return "config_and_locks";
    }
    if matches!(
        suffix.as_str(),
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "svg" | "ico" | "pdf" | "docx" | "xlsx" | "pptx"
    ) {
        return "assets_and_binary_docs";
    }
    if matches!(
        suffix.as_str(),
        "db" | "sqlite" | "sqlite3" | "csv" | "tsv" | "sql" | "dump" | "bak"
    ) {
        return "data_and_exports";
    }
    "other"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tests_bucket_wins_over_extension_rules() {
        assert_eq!(classify_group(Path::new("tests/test_utils.py")), "tests");
        assert_eq!(
            classify_group(Path::new("src/__tests__/App.test.tsx")),
            "tests"
        );
        assert_eq!(classify_group(Path::new("test_something.py")), "tests");
    }

    #[test]
    fn python_source() {
        assert_eq!(classify_group(Path::new("src/main.py")), "python_source");
        assert_eq!(classify_group(Path::new("stub.pyi")), "python_source");
    }

    #[test]
    fn frontend_source() {
        assert_eq!(classify_group(Path::new("src/App.tsx")), "frontend_source");
        assert_eq!(classify_group(Path::new("style.css")), "frontend_source");
    }

    #[test]
    fn backend_or_system_source() {
        assert_eq!(
            classify_group(Path::new("main.go")),
            "backend_or_system_source"
        );
        assert_eq!(
            classify_group(Path::new("lib.rs")),
            "backend_or_system_source"
        );
    }

    #[test]
    fn docs() {
        assert_eq!(classify_group(Path::new("README.md")), "docs");
        assert_eq!(classify_group(Path::new("LICENSE")), "docs");
        assert_eq!(classify_group(Path::new("notes.txt")), "docs");
    }

    #[test]
    fn config_and_locks() {
        assert_eq!(
            classify_group(Path::new("package.json")),
            "config_and_locks"
        );
        assert_eq!(classify_group(Path::new("Dockerfile")), "config_and_locks");
        assert_eq!(classify_group(Path::new("Cargo.lock")), "config_and_locks");
    }

    #[test]
    fn assets_and_binary_docs() {
        assert_eq!(
            classify_group(Path::new("logo.png")),
            "assets_and_binary_docs"
        );
        assert_eq!(
            classify_group(Path::new("report.pdf")),
            "assets_and_binary_docs"
        );
    }

    #[test]
    fn data_and_exports() {
        assert_eq!(classify_group(Path::new("dump.sql")), "data_and_exports");
        assert_eq!(classify_group(Path::new("app.sqlite3")), "data_and_exports");
    }

    #[test]
    fn other_is_the_fallback() {
        assert_eq!(classify_group(Path::new("binary.exe")), "other");
    }
}
