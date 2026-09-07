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
/// the key within it: ("GET", "name", "value"). The kernels group them
/// by the first of the three.
pub type Request = Vec<(String, String, String)>;

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
        request.push(("GET".to_string(), key, value));
    }
    let posted = form_body();
    for (key, value) in &posted {
        request.push(("POST".to_string(), key.clone(), value.clone()));
    }
    let cookies = env::var("HTTP_COOKIE").unwrap_or_default();
    for (key, value) in pairs(&cookies, ';') {
        request.push(("COOKIE".to_string(), key, value));
    }
    for name in CGI_VARS {
        if let Ok(value) = env::var(name) {
            request.push(("SERVER".to_string(), name.to_string(), value));
        }
    }
    for (name, value) in env::vars() {
        request.push(("ENV".to_string(), name, value));
    }
    request
}

/// The body as a form, when the request says it is one.
fn form_body() -> Vec<(String, String)> {
    let kind = env::var("CONTENT_TYPE").unwrap_or_default();
    if !kind.starts_with("application/x-www-form-urlencoded") {
        return Vec::new();
    }
    let length: usize = env::var("CONTENT_LENGTH").ok().and_then(|n| n.parse().ok()).unwrap_or(0);
    let mut body = vec![0u8; length];
    if std::io::stdin().read_exact(&mut body).is_err() {
        return Vec::new();
    }
    fields(&String::from_utf8_lossy(&body))
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
            Some((key, value)) => (unescaped(key), unescaped(value)),
            None => (unescaped(part), String::new()),
        })
        .collect()
}

/// A part of a URL as the text it stands for: `%41` is `A`, and a plus
/// is a space.
fn unescaped(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
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
