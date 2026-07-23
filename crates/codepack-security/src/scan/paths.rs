use std::path::Path;

fn display_backslash(relative_path: &Path) -> String {
    relative_path.to_string_lossy().replace('/', "\\")
}

pub(crate) fn rel_display(relative_path: &Path) -> String {
    if relative_path.as_os_str().is_empty() {
        ".".to_string()
    } else {
        format!(".\\{}", display_backslash(relative_path))
    }
}

pub(crate) fn sarif_uri(display: &str) -> String {
    display
        .trim_start_matches(['.', '\\', '/'])
        .replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn rel_display_prefixes_and_backslash_joins() {
        assert_eq!(
            rel_display(&PathBuf::from("src/main.rs")),
            ".\\src\\main.rs"
        );
    }

    #[test]
    fn rel_display_of_empty_path_is_a_dot() {
        assert_eq!(rel_display(&PathBuf::from("")), ".");
    }

    #[test]
    fn sarif_uri_strips_prefix_and_uses_forward_slashes() {
        assert_eq!(sarif_uri(".\\src\\main.rs"), "src/main.rs");
    }
}
