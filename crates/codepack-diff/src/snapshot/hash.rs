//! Streamed SHA-256 over raw file bytes, ported from legacy `diff_service.py::_hash_file`:
//! `for chunk in iter(lambda: handle.read(1024 * 1024), b""): digest.update(chunk)`.
//! Reading in bounded 1 MiB chunks keeps memory flat regardless of file size — this is
//! `sha2` (pure Rust, RustCrypto), not git2's internal object-hashing (which hashes
//! git-blob-formatted content, not raw bytes, and would silently diverge from legacy).

use std::fs::File;
use std::io::Read;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::error::{DiffError, Result};

/// 1 MiB, matching legacy's `handle.read(1024 * 1024)` chunk size exactly.
pub const HASH_CHUNK_BYTES: usize = 1024 * 1024;

pub fn hash_file(path: &Path) -> Result<String> {
    let mut file = File::open(path).map_err(|source| DiffError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; HASH_CHUNK_BYTES];
    loop {
        let read = file.read(&mut buffer).map_err(|source| DiffError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_a_direct_whole_buffer_hash() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("small.txt");
        std::fs::write(&path, b"hello world").unwrap();

        let mut expected = Sha256::new();
        expected.update(b"hello world");
        let expected_hex = format!("{:x}", expected.finalize());

        assert_eq!(hash_file(&path).unwrap(), expected_hex);
    }

    #[test]
    fn empty_file_hashes_to_the_well_known_empty_sha256() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.txt");
        std::fs::write(&path, b"").unwrap();

        assert_eq!(
            hash_file(&path).unwrap(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    /// Proves the streamed hash is correct across a multi-chunk file (bigger than the
    /// 1 MiB read buffer) without asserting on process memory directly — the buffer
    /// size constant above is the bounded-memory evidence; this test proves
    /// correctness across the chunk boundary that constant creates.
    #[test]
    fn hashes_correctly_across_multiple_chunk_boundaries() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("large.bin");
        let pattern = b"0123456789abcdef";
        let repeats = (HASH_CHUNK_BYTES * 5) / pattern.len() + 7;
        let mut content = Vec::with_capacity(repeats * pattern.len());
        for _ in 0..repeats {
            content.extend_from_slice(pattern);
        }
        std::fs::write(&path, &content).unwrap();

        let mut expected = Sha256::new();
        expected.update(&content);
        let expected_hex = format!("{:x}", expected.finalize());

        assert_eq!(hash_file(&path).unwrap(), expected_hex);
    }

    #[test]
    fn missing_file_is_a_read_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("missing.txt");
        assert!(matches!(hash_file(&path), Err(DiffError::Read { .. })));
    }
}
