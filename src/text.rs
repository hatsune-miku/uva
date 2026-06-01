//! Tiny text helpers shared by the file-editing transforms.

/// The dominant line ending of `content`: `\r\n` if any CRLF is present,
/// otherwise `\n`. Lets edits preserve a file's existing style instead of
/// forcing LF onto a Windows user's CRLF file.
pub fn newline(content: &str) -> &'static str {
    if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_crlf_and_lf() {
        assert_eq!(newline("a\r\nb"), "\r\n");
        assert_eq!(newline("a\nb"), "\n");
        assert_eq!(newline(""), "\n");
        assert_eq!(newline("no newline"), "\n");
    }
}
