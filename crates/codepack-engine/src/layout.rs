//! Shared layout constants for the plain-text reports this crate writes
//! (`01_structure.txt`, `02_git.txt`, `03_text_dump.txt`).
//!
//! Legacy fixed these widths, and every artifact it produced used them, so they are part
//! of what the reports look like rather than a styling choice this crate is free to
//! change. They were previously written as bare numbers at each of the six places a rule
//! is drawn, which left no way to tell a deliberate width apart from an arbitrary one.

/// Width of the rule under a report's header block.
pub(crate) const SECTION_RULE_WIDTH: usize = 100;

/// Width of the rule around each file's header inside the text dump.
///
/// Wider than [`SECTION_RULE_WIDTH`] on purpose: legacy used 120 here so a per-file
/// banner stands out from the section rules surrounding it when a reader scrolls a dump
/// containing hundreds of files.
pub(crate) const FILE_BANNER_RULE_WIDTH: usize = 120;

/// Draws a rule `width` columns wide using `character`.
pub(crate) fn rule(character: char, width: usize) -> String {
    character.to_string().repeat(width)
}

/// The section rule used under a report header.
pub(crate) fn section_rule(character: char) -> String {
    rule(character, SECTION_RULE_WIDTH)
}

/// The wider rule used around a file banner in the text dump.
pub(crate) fn file_banner_rule() -> String {
    rule('=', FILE_BANNER_RULE_WIDTH)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_have_the_documented_widths_and_characters() {
        assert_eq!(section_rule('=').len(), SECTION_RULE_WIDTH);
        assert!(section_rule('=').chars().all(|c| c == '='));
        assert_eq!(section_rule('-').len(), SECTION_RULE_WIDTH);
        assert!(section_rule('-').chars().all(|c| c == '-'));
        assert_eq!(file_banner_rule().len(), FILE_BANNER_RULE_WIDTH);
    }

    #[test]
    fn the_file_banner_is_wider_than_a_section_rule() {
        // The distinction is the point: a per-file banner must stand out from the
        // section rules around it.
        assert!(FILE_BANNER_RULE_WIDTH > SECTION_RULE_WIDTH);
    }
}
