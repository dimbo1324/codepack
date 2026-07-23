//! `all` mode: no filter, the whole selected project. Ported from legacy
//! `resolve_diff_selection`'s `if mode == "all": return DiffSelection("all", "полный
//! экспорт", None)`.

use super::DiffSelection;

pub fn all_selection() -> DiffSelection {
    DiffSelection {
        mode: "all".to_string(),
        base: "полный экспорт".to_string(),
        paths: None,
        files: Vec::new(),
        warning: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_selection_has_no_filter_and_no_files() {
        let selection = all_selection();
        assert_eq!(selection.mode, "all");
        assert!(!selection.is_limited());
        assert!(selection.files.is_empty());
        assert!(selection.warning.is_none());
    }
}
