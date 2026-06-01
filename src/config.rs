//! Pure transforms over the global `uv.toml` content: set or clear the
//! top-level `[[index]]` configuration. The executor handles locating and
//! reading/writing the file; the interesting logic lives here and is tested.

/// The Tsinghua (TUNA) PyPI mirror that `uva cn` switches to.
pub const TSINGHUA_URL: &str = "https://pypi.tuna.tsinghua.edu.cn/simple";

/// The `[[index]]` block uva writes, using the given line ending.
fn tsinghua_block(nl: &str) -> String {
    format!(
        "[[index]]{nl}url = \"{url}\"{nl}default = true{nl}",
        nl = nl,
        url = TSINGHUA_URL
    )
}

/// Set the index to the Tsinghua mirror: drop any existing top-level
/// `[[index]]` sections, then append our block. Idempotent. Preserves the
/// file's existing line-ending style.
pub fn set_tsinghua_index(content: &str) -> String {
    let nl = crate::text::newline(content);
    let stripped = strip_index_sections(content);
    let body = stripped.trim_end_matches(['\n', '\r']);
    let mut out = String::new();
    if !body.is_empty() {
        out.push_str(body);
        out.push_str(nl);
        out.push_str(nl);
    }
    out.push_str(&tsinghua_block(nl));
    out
}

/// Remove every top-level `[[index]]` array-of-tables section (its header and
/// the key/value lines up to the next section header). Everything else —
/// including other sections and their comments — is preserved.
pub fn strip_index_sections(content: &str) -> String {
    let mut kept: Vec<&str> = Vec::new();
    let mut in_index = false;
    for line in content.lines() {
        let trimmed = line.trim_start();
        if is_table_header(trimmed) {
            in_index = is_index_header(trimmed);
            if in_index {
                continue; // drop the `[[index]]` header itself
            }
            kept.push(line);
        } else if !in_index {
            kept.push(line);
        }
        // else: inside an [[index]] block — drop this line.
    }
    let nl = crate::text::newline(content);
    let mut out = kept.join(nl);
    if content.ends_with('\n') && !out.is_empty() {
        out.push_str(nl);
    }
    out
}

/// A line that begins a TOML table or array-of-tables (`[x]` / `[[x]]`).
fn is_table_header(trimmed: &str) -> bool {
    trimmed.starts_with('[')
}

/// Whether the header is exactly the top-level `[[index]]` (ignoring an inline
/// comment and surrounding whitespace).
fn is_index_header(trimmed: &str) -> bool {
    let head = trimmed.split('#').next().unwrap_or("").trim();
    head == "[[index]]"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_into_empty() {
        assert_eq!(
            set_tsinghua_index(""),
            "[[index]]\nurl = \"https://pypi.tuna.tsinghua.edu.cn/simple\"\ndefault = true\n"
        );
    }

    #[test]
    fn set_preserves_other_settings() {
        let input = "cache-dir = \"/tmp/uvcache\"\n";
        let out = set_tsinghua_index(input);
        assert!(out.starts_with("cache-dir = \"/tmp/uvcache\"\n\n[[index]]"));
        assert!(out.contains("default = true\n"));
    }

    #[test]
    fn set_is_idempotent() {
        let once = set_tsinghua_index("");
        let twice = set_tsinghua_index(&once);
        assert_eq!(once, twice);
    }

    #[test]
    fn set_replaces_existing_index() {
        let input = "[[index]]\nurl = \"https://example.com/simple\"\ndefault = true\n";
        let out = set_tsinghua_index(input);
        assert!(!out.contains("example.com"));
        assert!(out.contains("pypi.tuna.tsinghua.edu.cn"));
        // Exactly one index section.
        assert_eq!(out.matches("[[index]]").count(), 1);
    }

    #[test]
    fn strip_removes_only_index() {
        let input = "\
cache-dir = \"/c\"

[[index]]
url = \"https://example.com/simple\"
default = true

[pip]
universal = true
";
        let out = strip_index_sections(input);
        assert!(!out.contains("[[index]]"));
        assert!(!out.contains("example.com"));
        assert!(out.contains("cache-dir = \"/c\""));
        assert!(out.contains("[pip]"));
        assert!(out.contains("universal = true"));
    }

    #[test]
    fn strip_handles_multiple_index_sections() {
        let input = "\
[[index]]
url = \"https://a/simple\"

[[index]]
url = \"https://b/simple\"
default = true
";
        let out = strip_index_sections(input);
        assert_eq!(out.matches("[[index]]").count(), 0);
        assert!(out.trim().is_empty());
    }

    #[test]
    fn strip_no_index_is_unchanged() {
        let input = "cache-dir = \"/c\"\n";
        assert_eq!(strip_index_sections(input), input);
    }

    #[test]
    fn cn_then_unset_roundtrips_when_uva_owned() {
        // A file containing only uva's own block returns to empty after unset.
        let after_cn = set_tsinghua_index("");
        assert_eq!(strip_index_sections(&after_cn).trim(), "");
    }

    #[test]
    fn strip_preserves_crlf() {
        let input = "cache-dir = \"/c\"\r\n\r\n[[index]]\r\nurl = \"https://x/simple\"\r\n";
        let out = strip_index_sections(input);
        assert!(!out.contains("[[index]]"));
        assert!(out.contains("cache-dir = \"/c\""));
        assert!(out.contains("\r\n"));
    }

    #[test]
    fn set_uses_crlf_when_present() {
        let out = set_tsinghua_index("cache-dir = \"/c\"\r\n");
        assert!(out.contains("[[index]]\r\nurl"));
        assert!(out.contains("default = true\r\n"));
    }
}
