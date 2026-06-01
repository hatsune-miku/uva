//! Pure transforms over `requirements.txt` content. The executor reads the
//! file, applies one of these, and writes the result — keeping side effects
//! thin and the interesting logic unit-testable.

use std::collections::HashSet;

/// Extract and normalize the distribution name from a requirement spec or a
/// `requirements.txt` line. Returns `None` for blank lines, comments, and
/// option lines (e.g. `-r other.txt`, `--hash ...`).
///
/// Normalization follows a simplified PEP 503: lowercase, and collapse runs of
/// `-`, `_`, `.` into a single `-`.
pub fn normalize_name(line: &str) -> Option<String> {
    // Drop inline comments and surrounding whitespace.
    let line = line.split('#').next().unwrap_or("").trim();
    if line.is_empty() || line.starts_with('-') {
        return None;
    }
    // The name ends at the first version/extras/marker/whitespace delimiter.
    let end = line
        .find(|c: char| "=<>!~;[ (@".contains(c))
        .unwrap_or(line.len());
    let name = line[..end].trim();
    if name.is_empty() {
        return None;
    }
    let mut out = String::new();
    let mut prev_sep = false;
    for c in name.chars() {
        if c == '-' || c == '_' || c == '.' {
            if !prev_sep {
                out.push('-');
                prev_sep = true;
            }
        } else {
            out.push(c.to_ascii_lowercase());
            prev_sep = false;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

/// Append package specs to the given content, skipping any whose normalized
/// name already appears (in the file or earlier in `packages`). Returns the new
/// content; equal to the input when nothing is added.
pub fn append(content: &str, packages: &[String]) -> String {
    let mut seen: HashSet<String> = content.lines().filter_map(normalize_name).collect();
    let mut additions = Vec::new();
    for pkg in packages {
        if let Some(name) = normalize_name(pkg) {
            if seen.insert(name) {
                additions.push(pkg.trim().to_string());
            }
        }
    }
    if additions.is_empty() {
        return content.to_string();
    }
    let nl = crate::text::newline(content);
    let mut out = content.to_string();
    if !out.is_empty() && !out.ends_with('\n') {
        out.push_str(nl);
    }
    for line in additions {
        out.push_str(&line);
        out.push_str(nl);
    }
    out
}

/// Remove any line whose normalized name matches one of `packages`. Comments
/// and blank lines are preserved. Returns the new content; equal to the input
/// when nothing matches.
pub fn remove(content: &str, packages: &[String]) -> String {
    let targets: HashSet<String> = packages.iter().filter_map(|p| normalize_name(p)).collect();
    if targets.is_empty() {
        return content.to_string();
    }
    let nl = crate::text::newline(content);
    let had_trailing_newline = content.ends_with('\n');
    let kept: Vec<&str> = content
        .lines()
        .filter(|line| match normalize_name(line) {
            Some(name) => !targets.contains(&name),
            None => true,
        })
        .collect();
    let mut out = kept.join(nl);
    if had_trailing_newline && !out.is_empty() {
        out.push_str(nl);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn normalize_basic_and_versions() {
        assert_eq!(normalize_name("Flask==2.0"), Some("flask".to_string()));
        assert_eq!(
            normalize_name("requests>=2,<3"),
            Some("requests".to_string())
        );
        assert_eq!(normalize_name("my_pkg"), Some("my-pkg".to_string()));
        assert_eq!(
            normalize_name("ruamel.yaml"),
            Some("ruamel-yaml".to_string())
        );
        assert_eq!(
            normalize_name("requests[security]"),
            Some("requests".to_string())
        );
        assert_eq!(
            normalize_name("foo >= 1 ; python_version < '3'"),
            Some("foo".to_string())
        );
    }

    #[test]
    fn normalize_skips_noise() {
        assert_eq!(normalize_name(""), None);
        assert_eq!(normalize_name("   "), None);
        assert_eq!(normalize_name("# a comment"), None);
        assert_eq!(normalize_name("-r other.txt"), None);
        assert_eq!(normalize_name("--hash=sha256:abc"), None);
        assert_eq!(
            normalize_name("flask  # web framework"),
            Some("flask".to_string())
        );
    }

    #[test]
    fn append_adds_new_line() {
        assert_eq!(append("requests\n", &v(&["flask"])), "requests\nflask\n");
    }

    #[test]
    fn append_adds_trailing_newline_when_missing() {
        assert_eq!(append("requests", &v(&["flask"])), "requests\nflask\n");
    }

    #[test]
    fn append_creates_from_empty() {
        assert_eq!(append("", &v(&["a", "b"])), "a\nb\n");
    }

    #[test]
    fn append_dedups_against_file_and_self() {
        assert_eq!(append("Flask\n", &v(&["flask"])), "Flask\n");
        assert_eq!(append("", &v(&["flask", "Flask==2"])), "flask\n");
    }

    #[test]
    fn remove_drops_matching_lines_only() {
        assert_eq!(remove("requests\nflask\n", &v(&["Flask"])), "requests\n");
        assert_eq!(remove("# keep\nflask\n", &v(&["flask"])), "# keep\n");
    }

    #[test]
    fn remove_no_match_is_unchanged() {
        assert_eq!(remove("requests\n", &v(&["flask"])), "requests\n");
    }

    #[test]
    fn append_preserves_crlf() {
        assert_eq!(
            append("requests\r\n", &v(&["flask"])),
            "requests\r\nflask\r\n"
        );
    }

    #[test]
    fn remove_preserves_crlf() {
        assert_eq!(
            remove("requests\r\nflask\r\n", &v(&["flask"])),
            "requests\r\n"
        );
    }
}
