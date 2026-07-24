use std::fs;
use std::path::Path;

use codepack_core::ExportPaths;

pub(crate) fn make_paths(temp_root: &Path, project_name: &str) -> ExportPaths {
    let staging_dir = temp_root.join("staging");
    let bundle_name = format!("{project_name}_export");
    ExportPaths {
        desktop: temp_root.to_path_buf(),
        source_root: temp_root.to_path_buf(),
        project_name: project_name.to_string(),
        bundle_name: bundle_name.clone(),
        staging_dir: staging_dir.clone(),
        final_zip: temp_root.join(format!("{bundle_name}.zip")),
        archive_set_dir: temp_root.join(format!("{bundle_name}_parts")),
        project_dir: staging_dir.join(project_name),
        reports_dir: staging_dir.join("reports"),
        insights_dir: staging_dir.join("reports"),
        manifest_file: staging_dir.join("manifest.json"),
        project_profile_file: staging_dir.join("PROJECT_PROFILE.json"),
        index_file: staging_dir.join("INDEX.md"),
        structure_report: staging_dir.join("structure.md"),
        git_report: staging_dir.join("git.md"),
        text_dump: staging_dir.join("dump.txt"),
    }
}

pub(crate) fn write_file(staging_dir: &Path, relative: &str, content: &[u8]) {
    let full = staging_dir.join(relative);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).expect("create parent dir");
    }
    fs::write(full, content).expect("write fixture file");
}
