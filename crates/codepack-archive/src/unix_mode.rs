//! Unix permission bits on the way into and out of a ZIP.
//!
//! A ZIP member carries the mode of the file it was made from, in the "external
//! attributes" field. Without it every extracted file lands at the `zip` crate's default
//! 0644, so a project's `gradlew`, `./configure` and shell scripts come back unable to
//! run — invisible on Windows, immediate on Linux, and exactly the kind of difference
//! that makes a bundle worth less than the folder it came from.
//!
//! Both directions live here rather than beside their call sites because there are two
//! writers (`build::write_zip` and `pack::zip_writer::write`) and one reader, and a mode
//! that is recorded one way and read back another is worse than no mode at all.

use std::path::Path;

/// The permission bits a member should record, from the file being packed.
///
/// `None` where the platform has no such thing, which leaves the `zip` crate's own
/// default in place. A bundle built on Windows therefore still extracts as 0644
/// everywhere: the information was never there to record.
#[cfg(unix)]
pub(crate) fn for_member(file: &std::fs::File) -> Option<u32> {
    use std::os::unix::fs::PermissionsExt;

    // Masked to the permission bits: `st_mode` also carries the file type, and the
    // set-user-ID, set-group-ID and sticky bits, none of which belong in an archive
    // somebody else will unpack.
    Some(file.metadata().ok()?.permissions().mode() & 0o777)
}

#[cfg(not(unix))]
pub(crate) fn for_member(_file: &std::fs::File) -> Option<u32> {
    None
}

/// Applies a member's recorded mode to the file just extracted from it.
///
/// **The mask is the security part.** The mode comes out of an archive, which is
/// untrusted input like every other field in it: `0o777` drops set-user-ID,
/// set-group-ID and sticky, so a crafted archive cannot ask for a setuid file to be
/// written. `!0o022` additionally drops group- and world-write (audit 2026-09-07, S-5)
/// — `std` has no thread-safe way to read the *real* umask (`libc::umask` is the only
/// way, and it is not thread-safe, so reading it would mean a `libc` dependency and a
/// process-wide lock this project has no other reason to take on), so this hardcodes
/// `0o022`, the default on the overwhelming majority of systems. `tar` and `unzip` both
/// apply the caller's actual umask on top of an archive's recorded mode; without any
/// mask at all, an archive member recorded as `0o777` extracted verbatim into a
/// world-writable file — on a shared machine (`/tmp`, a CI runner with several jobs on
/// it), any local user could then rewrite a script like `gradlew` or `configure`
/// before the next person who runs it does, a local privilege escalation through an
/// artifact this product itself calls safe to hand out. `| 0o600` guarantees the owner
/// can still read and write what was just created, so an archive claiming mode 0 cannot
/// produce a file its extractor cannot open.
///
/// Best effort: a filesystem that does not carry Unix modes at all (a mounted FAT
/// volume, say) refuses `chmod`, and failing an extraction that has already written
/// every byte correctly would be the wrong answer to that.
#[cfg(unix)]
pub(crate) fn apply_extracted(path: &Path, mode: Option<u32>) {
    use std::os::unix::fs::PermissionsExt;

    let Some(mode) = mode else {
        return;
    };
    let _ = std::fs::set_permissions(
        path,
        std::fs::Permissions::from_mode((mode & 0o777 & !0o022) | 0o600),
    );
}

#[cfg(not(unix))]
pub(crate) fn apply_extracted(_path: &Path, _mode: Option<u32>) {}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn mode_of(path: &Path) -> u32 {
        std::fs::metadata(path).unwrap().permissions().mode() & 0o7777
    }

    #[test]
    fn the_executable_bit_survives_the_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("gradlew");
        std::fs::write(&script, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();

        let recorded = for_member(&std::fs::File::open(&script).unwrap());
        assert_eq!(recorded, Some(0o755));

        let extracted = dir.path().join("copy");
        std::fs::write(&extracted, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&extracted, std::fs::Permissions::from_mode(0o644)).unwrap();
        apply_extracted(&extracted, recorded);
        assert_eq!(mode_of(&extracted), 0o755);
    }

    /// The one that matters for safety: an archive asking for setuid gets the permission
    /// bits it asked for and none of the privilege.
    #[test]
    fn a_setuid_member_is_extracted_without_the_setuid_bit() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("payload");
        std::fs::write(&target, "x").unwrap();

        apply_extracted(&target, Some(0o4755));
        let mode = mode_of(&target);
        assert_eq!(mode & 0o4000, 0, "set-user-ID survived: {mode:o}");
        assert_eq!(mode & 0o2000, 0, "set-group-ID survived: {mode:o}");
        assert_eq!(mode, 0o755);
    }

    /// Audit 2026-09-07, S-5: an archive member recorded as world- and group-writable
    /// must not extract as one — the exact scenario a shared `/tmp` or a multi-job CI
    /// runner turns into a local privilege escalation the next time someone runs the
    /// extracted `gradlew`/`configure`.
    #[test]
    fn a_world_and_group_writable_member_loses_both_bits_on_extraction() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("shared_script");
        std::fs::write(&target, "#!/bin/sh\n").unwrap();

        apply_extracted(&target, Some(0o777));
        let mode = mode_of(&target);
        assert_eq!(mode & 0o022, 0, "group/world write survived: {mode:o}");
        // The executable bit — the whole reason this module exists — must survive
        // alongside the mask, not be a casualty of it.
        assert_eq!(mode, 0o755);
    }

    /// And an archive asking for nothing still yields a file its extractor can read.
    #[test]
    fn a_member_claiming_mode_zero_is_still_readable_afterwards() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("unreadable");
        std::fs::write(&target, "x").unwrap();

        apply_extracted(&target, Some(0));
        assert_eq!(mode_of(&target), 0o600);
        assert!(std::fs::read(&target).is_ok());
    }

    /// A member with no recorded mode is left exactly as the extractor created it.
    #[test]
    fn a_member_without_a_mode_changes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("plain");
        std::fs::write(&target, "x").unwrap();
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o640)).unwrap();

        apply_extracted(&target, None);
        assert_eq!(mode_of(&target), 0o640);
    }
}
