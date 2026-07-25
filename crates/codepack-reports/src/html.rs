//! Escaping for text embedded in generated HTML. One definition, shared by every
//! HTML-writing report ([`crate::reports::dashboard`], [`crate::reports::overview`])
//! rather than a copy per module — a second, subtly different escaper is exactly the
//! kind of drift that lets one of them miss a character the other handles.

pub(crate) fn escape_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_every_html_special_character() {
        assert_eq!(
            escape_html(r#"<script>alert("x & 'y'")</script>"#),
            "&lt;script&gt;alert(&quot;x &amp; &#39;y&#39;&quot;)&lt;/script&gt;"
        );
    }

    #[test]
    fn leaves_ordinary_text_untouched() {
        assert_eq!(escape_html("plain text 123"), "plain text 123");
    }
}
