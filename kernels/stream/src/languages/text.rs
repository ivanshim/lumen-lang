// Source-text helpers shared by language modules.
//
// This is language-layer code: the kernel lexer has no notion of comments or
// strings, so each language decides what a comment looks like and removes it
// before lexing.

/// Remove line comments introduced by `marker`, leaving newlines in place so
/// line numbers stay stable. Text inside quotes (any of `quotes`, with
/// backslash escapes) is never treated as a comment.
pub fn strip_line_comments(source: &str, marker: &str, quotes: &[char]) -> String {
    let mut out = String::with_capacity(source.len());
    let mut rest = source;
    let mut in_string: Option<char> = None;
    let mut escaped = false;

    while let Some(ch) = rest.chars().next() {
        if let Some(q) = in_string {
            out.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == q {
                in_string = None;
            }
            rest = &rest[ch.len_utf8()..];
            continue;
        }
        if quotes.contains(&ch) {
            in_string = Some(ch);
            out.push(ch);
            rest = &rest[ch.len_utf8()..];
            continue;
        }
        if rest.starts_with(marker) {
            match rest.find('\n') {
                Some(nl) => rest = &rest[nl..],
                None => rest = "",
            }
            continue;
        }
        out.push(ch);
        rest = &rest[ch.len_utf8()..];
    }
    out
}
