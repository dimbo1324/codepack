//! [`DetectedStack`]/[`detect_stack`], ported from legacy
//! `utils/inventory.py::detect_stack`/`detect_package_managers`. Reads exactly one
//! file from `staging_root` (`package.json`, for its `dependencies` keys) plus a
//! bounded set of root-level existence checks against the already-collected
//! [`super::Inventory`] — no directory walk (this crate's scope boundary, `lib.rs`).

use std::collections::BTreeSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::inventory::Inventory;
use crate::text::safe_read_json;

/// Field order matches legacy `detect_stack`'s returned dict (invariant I5's spirit,
/// applied to this new artifact from day one): `frontend`, `backend`, `tools`,
/// `testing`, `styling`, `infrastructure`, `package_managers`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetectedStack {
    pub frontend: Vec<String>,
    pub backend: Vec<String>,
    pub tools: Vec<String>,
    pub testing: Vec<String>,
    pub styling: Vec<String>,
    pub infrastructure: Vec<String>,
    pub package_managers: Vec<String>,
}

impl DetectedStack {
    /// Every detected label across all groups, deduplicated and sorted
    /// case-insensitively (legacy `_flatten_stack`).
    pub fn flattened(&self) -> Vec<String> {
        let mut values: BTreeSet<String> = BTreeSet::new();
        for group in [
            &self.frontend,
            &self.backend,
            &self.tools,
            &self.testing,
            &self.styling,
            &self.infrastructure,
        ] {
            for value in group {
                values.insert(value.clone());
            }
        }
        let mut flattened: Vec<String> = values.into_iter().collect();
        flattened.sort_by_key(|value| value.to_lowercase());
        flattened
    }
}

fn root_file_exists(inventory: &Inventory, name: &str) -> bool {
    inventory
        .files
        .iter()
        .any(|file| file.relative_path == name)
}

/// Legacy `detect_package_managers`: lockfile/manifest presence at the project root.
pub fn detect_package_managers(inventory: &Inventory) -> Vec<String> {
    let mut managers = Vec::new();
    if root_file_exists(inventory, "pnpm-lock.yaml") {
        managers.push("pnpm".to_string());
    }
    if root_file_exists(inventory, "package-lock.json") {
        managers.push("npm".to_string());
    }
    if root_file_exists(inventory, "yarn.lock") {
        managers.push("Yarn".to_string());
    }
    if root_file_exists(inventory, "bun.lockb") || root_file_exists(inventory, "bun.lock") {
        managers.push("Bun".to_string());
    }
    if root_file_exists(inventory, "poetry.lock") {
        managers.push("Poetry".to_string());
    }
    if root_file_exists(inventory, "Pipfile") {
        managers.push("Pipenv".to_string());
    }
    if root_file_exists(inventory, "requirements.txt") {
        managers.push("pip/requirements.txt".to_string());
    }
    if root_file_exists(inventory, "go.mod") {
        managers.push("Go modules".to_string());
    }
    if root_file_exists(inventory, "Cargo.toml") {
        managers.push("Cargo".to_string());
    }
    managers.sort();
    managers
}

fn package_json_dependency_names(staging_root: &Path) -> BTreeSet<String> {
    let value = safe_read_json(&staging_root.join("package.json"));
    let mut names = BTreeSet::new();
    for section in [
        "dependencies",
        "devDependencies",
        "peerDependencies",
        "optionalDependencies",
    ] {
        if let Some(object) = value.get(section).and_then(|v| v.as_object()) {
            names.extend(object.keys().cloned());
        }
    }
    names
}

/// Legacy `detect_stack`'s `checks` table: `(package_name, label, target_group)`.
const DEPENDENCY_CHECKS: &[(&str, &str, StackGroup)] = &[
    ("react", "React", StackGroup::Frontend),
    (
        "@vitejs/plugin-react",
        "Vite React plugin",
        StackGroup::Frontend,
    ),
    ("vite", "Vite", StackGroup::Tools),
    ("next", "Next.js", StackGroup::Frontend),
    ("vue", "Vue", StackGroup::Frontend),
    ("nuxt", "Nuxt", StackGroup::Frontend),
    ("svelte", "Svelte", StackGroup::Frontend),
    ("@sveltejs/kit", "SvelteKit", StackGroup::Frontend),
    ("astro", "Astro", StackGroup::Frontend),
    ("typescript", "TypeScript", StackGroup::Tools),
    ("tailwindcss", "Tailwind CSS", StackGroup::Styling),
    (
        "@tailwindcss/vite",
        "Tailwind CSS Vite plugin",
        StackGroup::Styling,
    ),
    ("sass", "Sass", StackGroup::Styling),
    ("less", "Less", StackGroup::Styling),
    ("zustand", "Zustand", StackGroup::Frontend),
    ("zod", "Zod", StackGroup::Frontend),
    (
        "@tanstack/react-query",
        "TanStack Query",
        StackGroup::Frontend,
    ),
    (
        "@tanstack/react-router",
        "TanStack Router",
        StackGroup::Frontend,
    ),
    ("react-router", "React Router", StackGroup::Frontend),
    ("react-hook-form", "React Hook Form", StackGroup::Frontend),
    ("framer-motion", "Framer Motion", StackGroup::Frontend),
    ("lucide-react", "Lucide React", StackGroup::Frontend),
    ("recharts", "Recharts", StackGroup::Frontend),
    ("echarts", "ECharts", StackGroup::Frontend),
    ("vitest", "Vitest", StackGroup::Testing),
    ("jest", "Jest", StackGroup::Testing),
    ("@playwright/test", "Playwright", StackGroup::Testing),
    ("cypress", "Cypress", StackGroup::Testing),
    ("storybook", "Storybook", StackGroup::Testing),
    ("@storybook/react", "Storybook React", StackGroup::Testing),
    ("eslint", "ESLint", StackGroup::Tools),
    ("prettier", "Prettier", StackGroup::Tools),
    ("express", "Express", StackGroup::Backend),
    ("fastify", "Fastify", StackGroup::Backend),
    ("nestjs", "NestJS", StackGroup::Backend),
];

#[derive(Debug, Clone, Copy)]
enum StackGroup {
    Frontend,
    Backend,
    Tools,
    Testing,
    Styling,
}

/// Legacy `detect_stack`. Reads `package.json` once (dependency-name matching) plus
/// root-level existence checks against `inventory`; never walks the tree.
pub fn detect_stack(staging_root: &Path, inventory: &Inventory) -> DetectedStack {
    let dependency_names = package_json_dependency_names(staging_root);
    let mut stack = DetectedStack {
        package_managers: detect_package_managers(inventory),
        ..Default::default()
    };

    for (package, label, group) in DEPENDENCY_CHECKS {
        if !dependency_names.contains(*package) {
            continue;
        }
        let target = match group {
            StackGroup::Frontend => &mut stack.frontend,
            StackGroup::Backend => &mut stack.backend,
            StackGroup::Tools => &mut stack.tools,
            StackGroup::Testing => &mut stack.testing,
            StackGroup::Styling => &mut stack.styling,
        };
        if !target.iter().any(|value| value == label) {
            target.push((*label).to_string());
        }
    }

    if root_file_exists(inventory, "components.json")
        && !stack
            .frontend
            .iter()
            .any(|v| v == "shadcn/ui-style component registry")
    {
        stack
            .frontend
            .push("shadcn/ui-style component registry".to_string());
    }
    if root_file_exists(inventory, "Dockerfile")
        || inventory
            .files
            .iter()
            .any(|file| file.relative_path.to_lowercase().starts_with("dockerfile"))
    {
        stack.infrastructure.push("Dockerfile".to_string());
    }
    if root_file_exists(inventory, "docker-compose.yml")
        || root_file_exists(inventory, "docker-compose.yaml")
    {
        stack.infrastructure.push("Docker Compose".to_string());
    }
    if inventory.files.iter().any(|file| {
        file.relative_path
            .replace('\\', "/")
            .starts_with(".github/workflows/")
    }) {
        stack.infrastructure.push("GitHub Actions".to_string());
    }
    if root_file_exists(inventory, "pyproject.toml")
        || root_file_exists(inventory, "requirements.txt")
    {
        stack.backend.push("Python".to_string());
    }
    if root_file_exists(inventory, "go.mod") {
        stack.backend.push("Go".to_string());
    }
    if root_file_exists(inventory, "Cargo.toml") {
        stack.backend.push("Rust".to_string());
    }

    stack.frontend.sort();
    stack.backend.sort();
    stack.tools.sort();
    stack.testing.sort();
    stack.styling.sort();
    stack.infrastructure.sort();
    stack
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::inventory::InventoryFile;

    fn inventory_with(files: Vec<(&str, u64)>) -> Inventory {
        Inventory {
            files: files
                .into_iter()
                .map(|(relative_path, size)| InventoryFile {
                    relative_path: relative_path.to_string(),
                    size,
                    extension: crate::paths::extension_key(relative_path),
                    language: None,
                })
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn detects_react_from_package_json_dependencies() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"dependencies": {"react": "18.0.0"}}"#,
        )
        .unwrap();
        let inventory = inventory_with(vec![("package.json", 40)]);

        let stack = detect_stack(dir.path(), &inventory);

        assert_eq!(stack.frontend, vec!["React".to_string()]);
    }

    #[test]
    fn detects_rust_backend_from_cargo_toml_presence() {
        let dir = tempfile::tempdir().unwrap();
        let inventory = inventory_with(vec![("Cargo.toml", 20)]);

        let stack = detect_stack(dir.path(), &inventory);

        assert_eq!(stack.backend, vec!["Rust".to_string()]);
        assert_eq!(stack.package_managers, vec!["Cargo".to_string()]);
    }

    #[test]
    fn flattened_deduplicates_and_sorts_case_insensitively() {
        let stack = DetectedStack {
            frontend: vec!["react".to_string(), "Vue".to_string()],
            backend: vec!["Rust".to_string()],
            ..Default::default()
        };
        assert_eq!(
            stack.flattened(),
            vec!["react".to_string(), "Rust".to_string(), "Vue".to_string()]
        );
    }
}
