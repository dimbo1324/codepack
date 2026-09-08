//! Byte offsets never land inside a multi-byte character (2026-09-08 polish pass).
//!
//! This crate slices strings by byte offset in roughly forty-five places — redaction
//! walks a line rebuilding it around each match, the code-shape detector reads the span
//! between a qualifier and its parenthesis, the pseudonym labeller strips quotes. Every
//! one of those offsets is derived from `str::find`, a match span, or the length of an
//! ASCII literal, so each is a valid character boundary *by construction*, and reading
//! them one by one is how that was confirmed rather than assumed.
//!
//! "Confirmed by reading" is exactly the guarantee that rots. Rust's answer to a byte
//! offset that lands mid-character is a panic, and this crate's input is other people's
//! source files: a comment in Russian, a Japanese string literal, an emoji in a commit
//! message beside an API key. A panic there takes the whole export down, and the person
//! it happens to has no way to tell it apart from the tool simply being broken.
//!
//! So the property gets a test instead of a promise. Every case below places multi-byte
//! text in a different position relative to a secret — before it, after it, in the key,
//! inside the value, on both sides — and asserts only that the pipeline **completes**
//! and still redacts. What each detector decides is covered by the corpus test; this one
//! exists to fail loudly the day an offset stops being a boundary.

use std::fs;
use std::path::Path;

use codepack_core::CancellationToken;
use codepack_security::{redact_secrets, scan_project};

/// Multi-byte samples with different byte widths, so an off-by-one lands mid-character
/// for at least one of them rather than only for the widest.
const MULTIBYTE: &[(&str, &str)] = &[
    ("cyrillic", "секретный ключ"), // 2 bytes per char
    ("cjk", "秘密の鍵"),            // 3 bytes per char
    ("emoji", "🔑🔒🗝"),             // 4 bytes per char
    ("mixed", "ключ 秘密 🔑 café"), // 2, 3, 4 and combining-friendly Latin-1
];

/// A synthetic, clearly fake credential — format-correct enough for the keyword cascade
/// to fire, never a real value.
const SECRET: &str = "AKIAIOSFODNN7EXAMPLE";

#[test]
fn redaction_survives_multi_byte_text_in_every_position_around_a_secret() {
    for (name, text) in MULTIBYTE {
        // Each arrangement puts the multi-byte run somewhere a byte offset computed from
        // the secret's own span could overshoot into.
        let lines = [
            format!("API_KEY={SECRET}  # {text}"),
            format!("# {text}\nAPI_KEY={SECRET}"),
            format!("{text}_API_KEY={SECRET}"),
            format!("API_KEY={SECRET}{text}"),
            format!("{text} API_KEY = \"{SECRET}\" {text}"),
            format!("API_KEY='{text}{SECRET}{text}'"),
            format!("{text}={SECRET}"),
            format!("Authorization: Bearer {SECRET} {text}"),
        ];

        for (index, line) in lines.iter().enumerate() {
            // The assertion is that this returns at all: a mid-character offset panics.
            let redacted = redact_secrets(line);

            assert!(
                !redacted.contains(SECRET),
                "{name} case {index} left the secret in place: {redacted}"
            );
            assert!(
                redacted.is_char_boundary(0) && std::str::from_utf8(redacted.as_bytes()).is_ok(),
                "{name} case {index} produced invalid UTF-8"
            );
        }
    }
}

/// The same property one level up: a whole project scanned, not a single line, so the
/// file walk, the detectors, the finding records and the report writers all see the
/// multi-byte content too.
#[test]
fn a_project_full_of_multi_byte_text_scans_without_panicking() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    let mut files = Vec::new();
    for (name, text) in MULTIBYTE {
        // A filename can be multi-byte as well, and it reaches the same reporting paths.
        let relative = format!("src/{name}_{text}.py").replace(['/', '\\', ':'], "_");
        let relative = format!("src/{relative}");
        let full = root.join(&relative);
        fs::create_dir_all(full.parent().unwrap()).unwrap();
        fs::write(
            &full,
            format!(
                "# {text}\n\
                 API_KEY = \"{SECRET}\"  # {text}\n\
                 password = '{text}'\n\
                 url = \"https://user:{SECRET}@example.com/{text}\"\n\
                 subprocess.run(f\"echo {text}\", shell=True)\n"
            ),
        )
        .unwrap();
        files.push(Path::new(&relative).to_path_buf());
    }

    let result = scan_project(root, &files, None, &CancellationToken::new())
        .expect("a project of multi-byte text must scan, not fail");

    assert!(
        !result.findings.is_empty(),
        "the planted secrets should still be found in multi-byte files"
    );
    for finding in &result.findings {
        assert!(
            !finding.message.contains(SECRET),
            "invariant I3: a finding message carried the raw secret: {}",
            finding.message
        );
    }
}

/// The boundary cases proper: a secret pressed directly against a multi-byte character
/// with no separator at all, where an offset one byte off is most likely to land inside
/// one rather than between two.
#[test]
fn a_secret_adjacent_to_a_multi_byte_character_with_no_separator() {
    for (name, text) in MULTIBYTE {
        let first = text.chars().next().unwrap();
        let last = text.chars().last().unwrap();

        for (index, line) in [
            format!("API_KEY={first}{SECRET}"),
            format!("API_KEY={SECRET}{last}"),
            format!("API_KEY={first}{SECRET}{last}"),
            format!("{first}API_KEY{last}={SECRET}"),
        ]
        .iter()
        .enumerate()
        {
            let redacted = redact_secrets(line);
            assert!(
                std::str::from_utf8(redacted.as_bytes()).is_ok(),
                "{name} adjacency case {index} produced invalid UTF-8"
            );
        }
    }
}
