//! The web side of the host: what a request carries, and a server that
//! answers requests by running the program once for each.
//!
//! A request reaches a program the way CGI has always delivered one: the
//! parts of it stand in the environment, and the body arrives on the
//! input. This host reads them, works them into named values, and hands
//! those to the kernel, which binds them under whatever names the
//! language's definition gives (`$_GET` and the rest for PHP). Serving
//! runs the same path: the server sets those variables for a run of the
//! program and sends back what the program writes.

use std::collections::HashMap;
use std::env;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Command, Stdio};

/// Every part of the request, each named by the group it belongs to and
/// where within it the value goes: ("GET", "name", "value"). The kernels
/// group them by the first of the three.
///
/// A name may point inside a value, the way a form does it: `a[b]` names
/// the place b inside a, and `a[]` the next place in a. The steps of
/// such a name are given apart, joined by the unit separator, so a
/// kernel has only to split them.
/// A part carries text unless it is marked as a number, which a file's
/// size and its error are.
pub type Request = Vec<(String, String, String, bool)>;

/// The mark between the steps of a name that points inside a value.
const BETWEEN_STEPS: char = '\u{1f}';

/// The environment variables a request is made of, in the order a
/// program is likeliest to want them.
const CGI_VARS: [&str; 14] = [
    "REQUEST_METHOD", "REQUEST_URI", "QUERY_STRING", "CONTENT_TYPE", "CONTENT_LENGTH", "SCRIPT_NAME",
    "SCRIPT_FILENAME", "PATH_INFO", "SERVER_NAME", "SERVER_PORT", "SERVER_PROTOCOL", "SERVER_SOFTWARE",
    "REMOTE_ADDR", "HTTP_USER_AGENT",
];

/// The request this run was given: what stands in the environment, and
/// the body on the input when the request says one is coming.
pub fn gathered() -> Request {
    let mut request = Request::new();
    let query = env::var("QUERY_STRING").unwrap_or_default();
    for (key, value) in fields(&query) {
        request.push(("GET".to_string(), key, value, false));
    }
    let (posted, sent, amiss, raw) = body_given();
    if let Some(raw) = raw {
        request.push(("SELF".to_string(), "body".to_string(), raw, false));
    }
    for kind in amiss {
        request.push(("SELF".to_string(), "amiss".to_string(), kind, false));
    }
    for (key, value) in &posted {
        request.push(("POST".to_string(), key.clone(), value.clone(), false));
    }
    for (key, value, counted) in &sent {
        request.push(("FILES".to_string(), key.clone(), value.clone(), *counted));
    }
    let cookies = env::var("HTTP_COOKIE").unwrap_or_default();
    for (key, value) in crumbs(&cookies) {
        request.push(("COOKIE".to_string(), key, value, false));
    }
    for name in CGI_VARS {
        if let Ok(value) = env::var(name) {
            request.push(("SERVER".to_string(), name.to_string(), value, false));
        }
    }
    for (name, value) in env::vars() {
        request.push(("ENV".to_string(), name, value, false));
    }
    request
}

/// A setting counted in bytes, as PHP counts one: the number the text
/// opens with, times what the last letter of the whole stands for.
fn quantity(text: &str) -> Option<i64> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let last = text.chars().last().unwrap_or(' ').to_ascii_lowercase();
    let times: i64 = match last {
        'k' => 1024,
        'm' => 1024 * 1024,
        'g' => 1024 * 1024 * 1024,
        _ => 1,
    };
    let digits = match times > 1 {
        true => &text[..text.len() - last.len_utf8()],
        false => text,
    };
    let digits = digits.trim();
    let (sign, rest) = match digits.strip_prefix('-') {
        Some(rest) => (-1, rest),
        None => (1, digits.strip_prefix('+').unwrap_or(digits)),
    };
    let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
    let opening: i64 = rest[..end].parse().unwrap_or(0);
    Some(sign * opening * times)
}

/// What a setting the run was started with says, as the runner hands it
/// over: every one carries the name it has with PHP_INI_ before it.
fn setting(name: &str) -> Option<String> {
    env::var(format!("PHP_INI_{}", name)).ok()
}

/// What the body carries: the fields of a form, and the files sent with
/// it. A body written as one piece is read as a form; a body written in
/// parts is cut at its boundary, and a part naming a file is written out
/// where the program can read it.
fn body_given() -> (Vec<(String, String)>, Vec<(String, String, bool)>, Vec<String>, Option<String>) {
    let kind = env::var("CONTENT_TYPE").unwrap_or_default();
    let plain = kind.starts_with("application/x-www-form-urlencoded");
    // What a part is cut at runs from `boundary=` to the first comma, as
    // a web server reads it: what follows the comma says something else
    // about the body and is none of the boundary.
    let boundary = kind
        .split(';')
        .map(str::trim)
        .find_map(|part| part.strip_prefix("boundary="))
        .map(|mark| mark.split(',').next().unwrap_or(mark).trim().to_string());
    let in_parts_said = kind.starts_with("multipart/");
    let length: usize = env::var("CONTENT_LENGTH").ok().and_then(|n| n.parse().ok()).unwrap_or(0);
    // A body larger than the run was told to take is not read at all,
    // and how large it was and how large it might have been are told.
    if let Some(most) = setting("post_max_size").and_then(|said| quantity(&said)).filter(|most| *most > 0) {
        if length as i64 > most {
            return (Vec::new(), Vec::new(), vec![format!("body.large{}{}{}{}", BETWEEN_STEPS, length, BETWEEN_STEPS, most)], None);
        }
    }
    let mut body = vec![0u8; length];
    if std::io::stdin().read_exact(&mut body).is_err() {
        return (Vec::new(), Vec::new(), Vec::new(), None);
    }
    // Whatever the body holds is kept as it came, so a program may read
    // it for itself however the run reads it.
    let raw = Some(String::from_utf8_lossy(&body).into_owned());
    // A run may be told not to take anything out of the body; it is
    // still there to be read as it came.
    let reads_body = setting("enable_post_data_reading").map_or(true, |said| !matches!(said.trim(), "0" | "" | "off" | "Off" | "false"));
    if !reads_body {
        return (Vec::new(), Vec::new(), Vec::new(), raw);
    }
    // A body said to be written in parts must say what they are cut at,
    // and say it whole: one opening a quote must close it. A body that
    // never claimed to be written in parts is only left unread.
    if boundary.is_none() {
        let amiss = if in_parts_said { vec!["boundary".to_string()] } else { Vec::new() };
        return match plain {
            true => (fields(raw.as_deref().unwrap_or_default()), Vec::new(), amiss, raw),
            false => (Vec::new(), Vec::new(), amiss, raw),
        };
    }
    let mark = boundary.expect("a boundary");
    if mark.starts_with('"') && !mark.ends_with('"') {
        return (Vec::new(), Vec::new(), vec!["boundary.wrong".to_string()], raw);
    }
    let (posted, sent, amiss) = in_parts(&body, mark.trim_matches('"'));
    (posted, sent, amiss, raw)
}

/// A body written in parts: each part says what it is called, and a part
/// that names a file is written out to a place of its own, which the
/// program is told about the way PHP tells it.
fn in_parts(body: &[u8], boundary: &str) -> (Vec<(String, String)>, Vec<(String, String, bool)>, Vec<String>) {
    let (mut posted, mut sent) = (Vec::new(), Vec::new());
    let mut amiss: Vec<String> = Vec::new();
    // A run may be told to take no files at all, and to take none larger
    // than so much.
    let takes_files = setting("file_uploads").map_or(true, |said| !matches!(said.trim(), "0" | "" | "off" | "Off" | "false"));
    let file_limit = setting("upload_max_filesize").and_then(|said| quantity(&said)).filter(|most| *most > 0);
    let mut files = 0usize;
    // A part written before the files may say how large a file the form
    // will take; one larger than that is turned away.
    let mut form_limit: Option<usize> = None;
    let mark = format!("--{}", boundary);
    let mut rest: &[u8] = body;
    while let Some(at) = find_bytes(rest, mark.as_bytes()) {
        let after = &rest[at + mark.len()..];
        if after.starts_with(b"--") {
            break;
        }
        let after = after.strip_prefix(b"\r\n").or_else(|| after.strip_prefix(b"\n")).unwrap_or(after);
        let end = find_bytes(after, mark.as_bytes()).unwrap_or(after.len());
        let piece = &after[..end];
        let head_end = find_bytes(piece, b"\r\n\r\n").map(|p| (p, 4)).or_else(|| find_bytes(piece, b"\n\n").map(|p| (p, 2)));
        rest = &after[end.min(after.len())..];
        let Some((head_end, gap)) = head_end else { continue };
        let head = String::from_utf8_lossy(&piece[..head_end]).into_owned();
        let mut content = &piece[head_end + gap..];
        for tail in [b"\r\n".as_slice(), b"\n".as_slice()] {
            if content.ends_with(tail) {
                content = &content[..content.len() - tail.len()];
            }
        }
        let named = |what: &str| -> Option<String> { attribute(&head, what) };
        // What a part holds is named before the first semicolon; what
        // follows one says something more about it and is not the name.
        let kind = head
            .lines()
            .find_map(|line| {
                line.to_ascii_lowercase()
                    .starts_with("content-type:")
                    .then(|| line[13..].split(';').next().unwrap_or("").trim().to_string())
            })
            .unwrap_or_else(|| "text/plain".to_string());
        // A name that opens a bracket must close on one: a part naming
        // anything after the last bracket is no name at all, and the
        // whole part goes unread. A form's own fields are read more
        // kindly than this, which is why the rule lives here.
        if named("name").map_or(false, |name| name.contains('[') && !name.ends_with(']')) {
            continue;
        }
        match (named("name"), named("filename")) {
            (None, None) => {
                amiss.push("part".to_string());
                continue;
            }
            (Some(name), None) => {
                let said = String::from_utf8_lossy(content).into_owned();
                if name == "MAX_FILE_SIZE" {
                    form_limit = said.trim().parse().ok();
                }
                posted.push((steps_of(&name), said));
            }
            (_, Some(_)) if !takes_files => continue,
            (given, Some(filename)) => {
                // A part naming no file at all sent none: it is counted
                // among the files, and everything said of it is empty
                // but for the word that says none came.
                let none_sent = filename.is_empty();
                // A file larger than the form said it would take is
                // turned away, and nothing of it is kept.
                let too_large = form_limit.map_or(false, |most| content.len() > most);
                // One larger than the run was told to take is turned
                // away too, and said to be turned away for that reason.
                let past_limit = file_limit.map_or(false, |most| content.len() as i64 > most);
                let too_large = too_large || past_limit;
                // The file is written out, since a program is given the
                // place it lies in rather than what it holds.
                let held = env::temp_dir().join(format!("lumenup{}{}", std::process::id(), files));
                let written = !none_sent && !too_large && std::fs::write(&held, content).is_ok();
                let place = held.to_string_lossy().into_owned();
                // A part that says nothing of its name is kept by its
                // turn among the files.
                let steps = given.map_or_else(|| files.to_string(), |name| steps_of(&name));
                files += 1;
                // The six things said of a file are named under the
                // first step of its name, and any deeper steps follow
                // them: `f[2]` becomes f, name, 2, not f, 2, name.
                let (first, deeper) = match steps.split_once(BETWEEN_STEPS) {
                    Some((first, deeper)) => (first, Some(deeper)),
                    None => (steps.as_str(), None),
                };
                // What a file is called is the last step of what was
                // sent: a browser handing over a whole directory names
                // each file by its way in, and only the whole path
                // keeps that.
                let called = filename.rsplit(['/', '\\']).next().unwrap_or(&filename).to_string();
                let nothing = String::new();
                for (what, value, counted) in [
                    ("name", called, false),
                    ("full_path", filename, false),
                    ("type", if none_sent || too_large { nothing.clone() } else { kind.clone() }, false),
                    ("tmp_name", if written { place } else { nothing.clone() }, false),
                    // 0: it came. 2: it was larger than the form said it
                    // would take. 4: none was sent. 1: it came and could
                    // not be put anywhere.
                    ("error", match (none_sent, too_large, written) {
                        (true, ..) => "4".to_string(),
                        (_, true, _) if past_limit => "1".to_string(),
                        (_, true, _) => "2".to_string(),
                        (.., true) => "0".to_string(),
                        _ => "1".to_string(),
                    }, true),
                    ("size", if none_sent || too_large { "0".to_string() } else { content.len().to_string() }, true),
                ] {
                    let path = match deeper {
                        Some(rest) => format!("{first}{BETWEEN_STEPS}{what}{BETWEEN_STEPS}{rest}"),
                        None => format!("{first}{BETWEEN_STEPS}{what}"),
                    };
                    sent.push((path, value, counted));
                }
            }
        }
    }
    (posted, sent, amiss)
}

/// What a head line calls something: `name=x`, `name='x'` or `name="x"`,
/// written after a semicolon like every other thing such a line says.
/// Within a value a backslash standing before the mark that closes it,
/// or before another backslash, is the mark itself; anywhere else a
/// backslash stands for itself, which is how the web has always read
/// these and what a program sending odd names counts on.
fn attribute(head: &str, what: &str) -> Option<String> {
    for line in head.lines() {
        for part in line.split(';').skip(1) {
            let Some(rest) = part.trim_start().strip_prefix(what) else { continue };
            let Some(rest) = rest.trim_start().strip_prefix('=') else { continue };
            let rest = rest.trim_start();
            let (closing, body) = match rest.chars().next() {
                Some(q) if q == '"' || q == '\'' => (Some(q), &rest[q.len_utf8()..]),
                _ => (None, rest),
            };
            let mut out = String::new();
            let mut letters = body.chars().peekable();
            while let Some(letter) = letters.next() {
                if Some(letter) == closing {
                    return Some(out);
                }
                if closing.is_none() && (letter == ' ' || letter == '\t') {
                    return Some(out);
                }
                if letter == '\\' {
                    let next = letters.peek().copied();
                    if next == Some('\\') || (closing.is_some() && next == closing) {
                        out.push(letters.next().expect("a letter was there to see"));
                        continue;
                    }
                }
                out.push(letter);
            }
            return Some(out);
        }
    }
    None
}

fn find_bytes(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// `a=1&b=2` as pairs, each part unescaped.
fn fields(text: &str) -> Vec<(String, String)> {
    pairs(text, '&')
}

fn pairs(text: &str, between: char) -> Vec<(String, String)> {
    text.split(between)
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| match part.split_once('=') {
            Some((key, value)) => (steps_of(&unescaped(key)), unescaped(value)),
            None => (steps_of(&unescaped(part)), String::new()),
        })
        .collect()
}

/// The steps of a name that points inside a value: `a[b][c]` names a,
/// then b, then c, and `a[]` names a and then the next place in it. A
/// bracket runs to the first close after it, so `a[b[]]` names a and
/// `b[`. A name whose brackets never close is a name like any other.
/// The first step is the one a form gives, so a dot or a space in it
/// becomes an underscore, as PHP has always made it.
fn steps_of(name: &str) -> String {
    let (head, mut rest) = match name.split_once('[') {
        Some((head, rest)) => (head, rest),
        None => (name, ""),
    };
    let mut steps = vec![head.replace(['.', ' ', '['], "_")];
    loop {
        match rest.split_once(']') {
            Some((step, tail)) => {
                steps.push(step.to_string());
                rest = match tail.strip_prefix('[') {
                    Some(more) => more,
                    None => break,
                };
            }
            None => {
                // A bracket that never closes opens nothing, so it is
                // part of the name; and a name may hold none of the
                // marks that would make it hard to read, so each of
                // them stands as an underscore.
                return name.replace(['.', ' ', '['], "_");
            }
        }
    }
    steps.join(&BETWEEN_STEPS.to_string())
}

/// The cookies a request carries. A cookie's name is written plainly,
/// not escaped the way a form's is, so it is taken as it stands; only
/// the space before it is dropped, and what follows the `=` is kept to
/// the last letter, trailing spaces and all. Where a name comes twice
/// the first one stands: a cookie is not written over by a later one.
fn crumbs(text: &str) -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = Vec::new();
    for part in text.split(';') {
        let part = part.trim_start_matches([' ', '\t']);
        if part.is_empty() {
            continue;
        }
        let (name, value) = match part.split_once('=') {
            Some((name, value)) => (steps_of(name), unescaped_plainly(value)),
            None => (steps_of(part), String::new()),
        };
        if !found.iter().any(|(had, _)| *had == name) {
            found.push((name, value));
        }
    }
    found
}

/// A part of a URL as the text it stands for: `%41` is `A`, and a plus
/// is a space where the piece was written as a form writes one.
fn unescaped(text: &str) -> String {
    undone(text, true)
}

/// The same, save that a plus stands for itself: what a cookie carries
/// is written plainly and a plus in it is a plus.
fn unescaped_plainly(text: &str) -> String {
    undone(text, false)
}

fn undone(text: &str, plus_is_space: bool) -> String {
    let bytes = text.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' if plus_is_space => out.push(b' '),
            b'%' if i + 2 < bytes.len() => {
                let digits = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                match u8::from_str_radix(digits, 16) {
                    Ok(byte) => {
                        out.push(byte);
                        i += 2;
                    }
                    Err(_) => out.push(b'%'),
                }
            }
            byte => out.push(byte),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Answer requests on the given address by running the program once for
/// each, as a web server has always run a program: the request in the
/// environment, the answer on the output.
pub fn serve(where_to: &str, program: &[String]) -> Result<(), String> {
    let address = if where_to.contains(':') { where_to.to_string() } else { format!("127.0.0.1:{}", where_to) };
    let door = TcpListener::bind(&address).map_err(|e| format!("Cannot listen on {}: {}", address, e))?;
    eprintln!("lumen-lang serving on http://{}", address);
    for caller in door.incoming() {
        match caller {
            Ok(stream) => {
                if let Err(e) = answer(stream, program) {
                    eprintln!("request: {}", e);
                }
            }
            Err(e) => eprintln!("connection: {}", e),
        }
    }
    Ok(())
}

/// One request: read it, run the program with it, send back what the
/// program wrote.
fn answer(mut stream: TcpStream, program: &[String]) -> Result<(), String> {
    let asked = read_request(&mut stream)?;
    let written = run_for(&asked, program)?;
    let (headers, body) = split_headers(&written);
    let mut answer = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n", body.len());
    let mut typed = false;
    for line in &headers {
        typed |= line.to_ascii_lowercase().starts_with("content-type:");
        answer.push_str(line);
        answer.push_str("\r\n");
    }
    if !typed {
        answer.push_str("Content-Type: text/html; charset=UTF-8\r\n");
    }
    answer.push_str("\r\n");
    let mut out = answer.into_bytes();
    out.extend_from_slice(body.as_bytes());
    stream.write_all(&out).map_err(|e| e.to_string())?;
    stream.flush().map_err(|e| e.to_string())
}

/// What one request says: its method, where it points, its headers and
/// its body.
struct Asked {
    method: String,
    target: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

fn read_request(stream: &mut TcpStream) -> Result<Asked, String> {
    let peer = stream.peer_addr().map(|a| a.ip().to_string()).unwrap_or_default();
    let mut reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
    let mut first = String::new();
    reader.read_line(&mut first).map_err(|e| e.to_string())?;
    let mut parts = first.split_whitespace();
    let method = parts.next().unwrap_or("GET").to_string();
    let target = parts.next().unwrap_or("/").to_string();
    let mut headers = HashMap::new();
    headers.insert("REMOTE_ADDR".to_string(), peer);
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).map_err(|e| e.to_string())? == 0 {
            break;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_ascii_uppercase().replace('-', "_"), value.trim().to_string());
        }
    }
    let length: usize = headers.get("CONTENT_LENGTH").and_then(|n| n.parse().ok()).unwrap_or(0);
    let mut body = vec![0u8; length];
    if length > 0 {
        reader.read_exact(&mut body).map_err(|e| e.to_string())?;
    }
    Ok(Asked { method, target, headers, body })
}

/// Run the program for one request, with the request in its environment
/// and its body on its input, and take what it writes.
fn run_for(asked: &Asked, program: &[String]) -> Result<String, String> {
    let me = env::current_exe().map_err(|e| e.to_string())?;
    let (path, query) = asked.target.split_once('?').unwrap_or((asked.target.as_str(), ""));
    let mut child = Command::new(me);
    child
        .args(program)
        .env("REQUEST_METHOD", &asked.method)
        .env("REQUEST_URI", &asked.target)
        .env("QUERY_STRING", query)
        .env("SCRIPT_NAME", path)
        .env("PATH_INFO", path)
        .env("SERVER_PROTOCOL", "HTTP/1.1")
        .env("SERVER_SOFTWARE", "lumen-lang")
        .env("CONTENT_LENGTH", asked.body.len().to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (name, value) in &asked.headers {
        let named = match name.as_str() {
            "CONTENT_TYPE" | "CONTENT_LENGTH" | "REMOTE_ADDR" => name.clone(),
            other => format!("HTTP_{}", other),
        };
        child.env(named, value);
    }
    let mut running = child.spawn().map_err(|e| e.to_string())?;
    if let Some(input) = running.stdin.as_mut() {
        input.write_all(&asked.body).map_err(|e| e.to_string())?;
    }
    let done = running.wait_with_output().map_err(|e| e.to_string())?;
    let told = String::from_utf8_lossy(&done.stdout).into_owned();
    if done.status.success() {
        return Ok(told);
    }
    let why = String::from_utf8_lossy(&done.stderr);
    Ok(format!("{}{}", told, why))
}

/// A program may write headers first, as CGI has always let it: the
/// lines before the first blank one are headers when they look like
/// headers.
fn split_headers(written: &str) -> (Vec<&str>, &str) {
    let Some(end) = written.find("\n\n").or_else(|| written.find("\r\n\r\n")) else {
        return (Vec::new(), written);
    };
    let head = &written[..end];
    let looks = !head.is_empty()
        && head.lines().all(|line| match line.split_once(':') {
            Some((name, _)) => !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'),
            None => false,
        });
    if !looks {
        return (Vec::new(), written);
    }
    let body = &written[end..];
    let body = body.strip_prefix("\n\n").or_else(|| body.strip_prefix("\r\n\r\n")).unwrap_or(body);
    (head.lines().collect(), body)
}
