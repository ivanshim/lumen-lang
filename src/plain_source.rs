// Plain UTF-8 library sources are what `_tokenize._decode_source` would
// return, but importing that module pulls the regex compiler into every
// fresh process. BOM, CR newlines, and any coding cookie stay on the
// tokenizer path so those files keep its decoder and error messages.
fn plain_utf8_source(raw: &[u8]) -> Option<String> {
    if raw.starts_with(b"\xef\xbb\xbf") || raw.contains(&b'\r') || has_coding_cookie(raw) {
        return None;
    }
    std::str::from_utf8(raw).ok().map(str::to_string)
}

fn has_coding_cookie(raw: &[u8]) -> bool {
    let mut start = 0usize;
    for _ in 0..2 {
        if start >= raw.len() {
            break;
        }
        let end = raw[start..].iter().position(|byte| *byte == b'\n').map(|i| start + i).unwrap_or(raw.len());
        if line_has_cookie(&raw[start..end]) {
            return true;
        }
        start = end + 1;
    }
    false
}

fn line_has_cookie(line: &[u8]) -> bool {
    let mut i = 0usize;
    while i < line.len() && matches!(line[i], b' ' | b'\t' | 0x0c) {
        i += 1;
    }
    if i >= line.len() || line[i] != b'#' {
        return false;
    }
    let rest = &line[i + 1..];
    rest.windows(7).any(|w| w[..6].eq_ignore_ascii_case(b"coding") && matches!(w[6], b':' | b'='))
}
