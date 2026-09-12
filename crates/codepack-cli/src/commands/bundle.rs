//! Making an exported bundle available as a directory, for the commands that work on
//! one after the fact.
//!
//! Two commands need this and need it to mean the same thing: `handoff`, which points a
//! local agent at a folder, and `ask`, which sends a folder's AI context to a provider.
//! It lived inside `handoff` until `ask` arrived; a second copy would have been a second
//! answer to "what counts as a bundle", and the first symptom of drift would have been
//! one command accepting an archive set the other refused.

use std::path::{Path, PathBuf};

use crate::error::{CliError, Result};

#[derive(Debug)]
pub(crate) struct OpenedBundle {
    pub directory: PathBuf,
    pub extracted: bool,
}

/// Makes the bundle's content available as a directory that outlives this process.
///
/// Deliberately unlike `verify`, which unpacks into a temporary directory and throws it
/// away: there, the answer is the report; here, the directory outlives the command —
/// `handoff`'s agent has to open it afterwards, and `ask`'s answer is written into it.
pub(crate) fn open_bundle(bundle: &Path) -> Result<OpenedBundle> {
    if bundle.is_dir() {
        if bundle.join("ARCHIVE_SET_MANIFEST.json").is_file() {
            let destination = bundle.join("_extracted");
            codepack_archive::restore_archive_set(bundle, &destination)
                .map_err(|error| CliError::message(error.to_string()))?;
            return Ok(OpenedBundle {
                directory: destination,
                extracted: true,
            });
        }
        return Ok(OpenedBundle {
            directory: bundle.to_path_buf(),
            extracted: false,
        });
    }

    if bundle.is_file() {
        codepack_archive::ArchiveFormat::ensure_reopenable(bundle)
            .map_err(|error| CliError::message(error.to_string()))?;
        let parent = bundle.parent().ok_or_else(|| {
            CliError::message(format!("{} has no parent directory", bundle.display()))
        })?;
        let stem = bundle
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_else(|| "bundle".to_string());
        let destination = parent.join(format!("{stem}_extracted"));
        // Traversal-checked extraction (invariant I7). A bundle can have come from
        // somewhere else, and every file in it is about to be read — by a coding agent,
        // or by the request assembler.
        codepack_archive::extract_zip_safely(bundle, &destination)
            .map_err(|error| CliError::message(error.to_string()))?;
        return Ok(OpenedBundle {
            directory: destination,
            extracted: true,
        });
    }

    Err(CliError::message(format!(
        "{} is not a file or a directory",
        bundle.display()
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_extracted_bundle_directory_is_used_as_it_is() {
        let dir = tempfile::tempdir().unwrap();
        let opened = open_bundle(dir.path()).unwrap();
        assert_eq!(opened.directory, dir.path());
        assert!(!opened.extracted);
    }

    #[test]
    fn a_missing_bundle_is_named_in_the_error() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("nope.zip");
        let error = open_bundle(&missing).unwrap_err().to_string();
        assert!(error.contains("nope.zip"), "{error}");
    }

    #[test]
    fn an_archive_is_unpacked_beside_itself_so_it_outlives_this_process() {
        // The whole reason this does not reuse `verify`'s temporary directory.
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("bundle.zip");
        let file = std::fs::File::create(&archive).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        writer
            .start_file::<_, ()>("AI_CONTEXT/00.md", zip::write::SimpleFileOptions::default())
            .unwrap();
        std::io::Write::write_all(&mut writer, b"overview\n").unwrap();
        writer.finish().unwrap();

        let opened = open_bundle(&archive).unwrap();
        assert!(opened.extracted);
        assert_eq!(opened.directory, dir.path().join("bundle_extracted"));
        assert!(opened.directory.join("AI_CONTEXT").join("00.md").is_file());
    }
}
