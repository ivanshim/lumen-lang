// lumen-lang: command-line host for the three kernels.
//
// Usage: lumen-lang [--kernel stream35|microcode11|microcode4|microcode7|stack5|stack8] [--lang <name|definition.json>]
//                   <file> [--lang <name|definition.json>] [program args...]
//
// The host reads the file, picks the language from `--lang` (also spelled
// `--language`), else from the file extension, else Lumen; prepends the
// embedded Lumen standard library for Lumen programs; and hands the source
// to the selected kernel. A `--lang` value takes a language name (`python`),
// one of its extensions (`py`), or the path of a definition file
// (`langs/php.json`), which is read at run time.
// Nothing here knows how any kernel works, and the kernels never see
// each other.
//
// Three switches of the reference implementation's own are taken as it
// takes them, before the file and not after: `-d setting=worth` gives
// the run a setting, which goes where the surroundings put the same
// setting; `-n` asks for no file of settings behind the run, which is
// how the host stands anyway; and `-v` has the run say what it is and
// stop. Whatever the settings name for the run to load beside itself is
// reached for before any of this, and what came of the reaching is said.

use std::collections::HashSet;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::process;

mod web;
mod python_versions;

mod embedded_modules {
    include!("../langs/lib_python/modules/manifest.rs");
}

/// All the room the host takes is handed out through this, so that a
/// program can be told how much of it the program itself has taken. An
/// allocator is one thing for the whole process, and the kernels are
/// separate crates that know nothing of one another, so the count lives
/// in a crate of its own that each of them may read.
#[global_allocator]
static ROOM: lumen_room::Tally = lumen_room::Tally;

const KERNELS: [&str; 6] = ["stream35", "microcode11", "microcode4", "microcode7", "stack5", "stack8"];
const DEFAULT_KERNEL: &str = "stack8";
const DEFAULT_LANGUAGE: &str = "lumen";

/// Build-time packaging of the Lumen standard library (`langs/lib_lumen/*.lm`).
mod embedded_files {
    include!("../langs/lib_lumen/prelude.rs");
}

/// The prelude manifest: a list of `include "path"` lines.
const PRELUDE_MANIFEST: &str = include_str!("../langs/lib_lumen/prelude.lm");

/// The Lumen library as every other language spells it (`langs/lib_<language>/`,
/// written by scripts/port_examples.py), prepended to programs in that
/// language as `langs/lib_lumen/` is to Lumen programs.
mod mirrors {
    pub mod python { include!("../langs/lib_python/prelude.rs"); }
    pub mod rplumen { include!("../langs/lib_rplumen/prelude.rs"); }
    pub mod rust { include!("../langs/lib_rust/prelude.rs"); }
    pub mod c { include!("../langs/lib_c/prelude.rs"); }
    pub mod javascript { include!("../langs/lib_javascript/prelude.rs"); }
    pub mod pascal { include!("../langs/lib_pascal/prelude.rs"); }
    pub mod php { include!("../langs/lib_php/prelude.rs"); }
    pub mod ruby { include!("../langs/lib_ruby/prelude.rs"); }
    pub mod swift { include!("../langs/lib_swift/prelude.rs"); }
}

/// A language's mirror of the library: its prologue and its files.
fn mirror_for(language: &str) -> Option<(&'static str, &'static str, &'static [(&'static str, &'static str)])> {
    Some(match language {
        "python" => (mirrors::python::PROLOGUE, mirrors::python::EPILOGUE, mirrors::python::FILES),
        "rplumen" => (mirrors::rplumen::PROLOGUE, mirrors::rplumen::EPILOGUE, mirrors::rplumen::FILES),
        "rust" => (mirrors::rust::PROLOGUE, mirrors::rust::EPILOGUE, mirrors::rust::FILES),
        "c" => (mirrors::c::PROLOGUE, mirrors::c::EPILOGUE, mirrors::c::FILES),
        "javascript" => (mirrors::javascript::PROLOGUE, mirrors::javascript::EPILOGUE, mirrors::javascript::FILES),
        "pascal" => (mirrors::pascal::PROLOGUE, mirrors::pascal::EPILOGUE, mirrors::pascal::FILES),
        "php" => (mirrors::php::PROLOGUE, mirrors::php::EPILOGUE, mirrors::php::FILES),
        "ruby" => (mirrors::ruby::PROLOGUE, mirrors::ruby::EPILOGUE, mirrors::ruby::FILES),
        "swift" => (mirrors::swift::PROLOGUE, mirrors::swift::EPILOGUE, mirrors::swift::FILES),
        _ => return None,
    })
}

/// A program on top of its language's library: the Lumen library for a
/// Lumen program, the language's mirror of it for any other. A program
/// that opens with the language's prologue (Python's `import sys`) keeps
/// it first, since the kernels drop a prologue only at the start.
/// The kernels that read the ext.* labels, and so the only ones a
/// mirror's hand-written native source can run on.
const FULL_KERNELS: [&str; 2] = ["stack8", "microcode7"];

/// A file may open with a line saying which program is to run it. That
/// line belongs to the system that runs the file and not to the program,
/// so it is emptied here — emptied and not taken out, so that every line
/// after it keeps the number it was written on.
fn without_shebang(source: String) -> String {
    if !source.starts_with("#!") {
        return source;
    }
    match source.find('\n') {
        Some(end) => source[end..].to_string(),
        None => String::new(),
    }
}

fn with_library(kernel: &str, language: &str, source: String) -> String {
    if env::var_os("LUMEN_BARE").is_some() {
        return source;
    }
    if language == DEFAULT_LANGUAGE {
        let prelude = expand_includes(PRELUDE_MANIFEST).unwrap_or_else(|e| {
            eprintln!("Include error: {}", e);
            process::exit(1);
        });
        return format!("{}\n{}", prelude, source);
    }
    let Some((prologue, epilogue, files)) = mirror_for(language) else { return source };
    // A mirror may carry hand-written source of the language's own under
    // native/, spelling what only the full kernels read; the others are
    // given the ported library alone.
    let full = FULL_KERNELS.contains(&kernel);
    let files: Vec<_> = files.iter().filter(|(path, _)| full || !path.contains("/native/")).collect();
    if files.is_empty() {
        return source;
    }
    let library = files.iter().map(|(_, text)| *text).collect::<Vec<_>>().join("\n");
    // Keep the ported routine for the earlier kernels, while the full
    // Python kernels retain their builtin's optional places argument.
    let library = if full && language == "python" {
        format!("__native_round = round\n{}\nround = __native_round\ndel __native_round\nimport sys", library)
    } else { library };
    let lead = source.len() - source.trim_start().len();
    if !prologue.is_empty() && !source[..lead].contains('\n') && source[lead..].starts_with(prologue) {
        let cut = lead + prologue.len();
        return format!("{}\n{}\n{}", &source[..cut], library, &source[cut..]);
    }
    // A source that is text with code in it opens a run of code for the
    // library and closes it again, so the library is code and the text
    // that follows is still text.
    if !epilogue.is_empty() {
        return format!("{}\n{}\n{}\n{}", prologue, library, epilogue, source);
    }
    format!("{}\n{}", library, source)
}

/// Where a language comes from.
enum Language {
    /// An embedded definition, by name.
    Named(String),
    /// A definition read from a file on disk, with the name it declares
    /// and where it was read from.
    File { name: String, path: String, text: String },
}

impl Language {
    fn name(&self) -> &str {
        match self {
            Language::Named(name) => name,
            Language::File { name, .. } => name,
        }
    }

    /// What to write after `--lang` to ask for this language again.
    fn given(&self) -> Option<String> {
        match self {
            Language::Named(name) => Some(name.clone()),
            Language::File { path, .. } => Some(path.clone()),
        }
    }
}

struct Invocation {
    kernel: String,
    file: String,
    /// Where to answer web requests, when the run is to serve them.
    serve: Option<String>,
    language: Language,
    /// A language to write the program in instead of running it (microcode11 only).
    emit: Option<Language>,
    program_args: Vec<String>,
    python: Option<&'static python_versions::PythonVersion>,
    module_source: Option<String>,
    module_name: Option<String>,
    python_development: bool,
}

/// Whether a language holds text as the bytes it was written in rather
/// than as the letters those bytes spell. It is the definition that
/// says so, not the kernel, so a program is read the same way whichever
/// kernel is to run it.
fn text_is_bytes(language: &Language) -> bool {
    let definition = match language {
        Language::File { text, .. } => text.clone(),
        Language::Named(name) => match lumen_microcode11::embedded(name) {
            Ok(text) => text.to_string(),
            Err(_) => return false,
        },
    };
    lumen_stack8::lang::Lang::parse(&definition).map_or(false, |lang| lang.text_is_bytes)
}

/// Whether a kernel gives the extension labels any meaning. The four
/// reference kernels read past them, so for those a program is read as
/// the letters its bytes spell however the definition holds text — as
/// an unread label leaves everything else it touches unchanged.
fn honours_extensions(kernel: &str) -> bool {
    matches!(kernel, "stack8" | "microcode7")
}

/// The upper halves of the single-byte code pages a Python file's
/// coding line may name, byte 128 standing for a table's first
/// character. The tables are the reference's own.
const CP437_UPPER: &str = "\u{c7}\u{fc}\u{e9}\u{e2}\u{e4}\u{e0}\u{e5}\u{e7}\u{ea}\u{eb}\u{e8}\u{ef}\u{ee}\u{ec}\u{c4}\u{c5}\u{c9}\u{e6}\u{c6}\u{f4}\u{f6}\u{f2}\u{fb}\u{f9}\u{ff}\u{d6}\u{dc}\u{a2}\u{a3}\u{a5}\u{20a7}\u{192}\u{e1}\u{ed}\u{f3}\u{fa}\u{f1}\u{d1}\u{aa}\u{ba}\u{bf}\u{2310}\u{ac}\u{bd}\u{bc}\u{a1}\u{ab}\u{bb}\u{2591}\u{2592}\u{2593}\u{2502}\u{2524}\u{2561}\u{2562}\u{2556}\u{2555}\u{2563}\u{2551}\u{2557}\u{255d}\u{255c}\u{255b}\u{2510}\u{2514}\u{2534}\u{252c}\u{251c}\u{2500}\u{253c}\u{255e}\u{255f}\u{255a}\u{2554}\u{2569}\u{2566}\u{2560}\u{2550}\u{256c}\u{2567}\u{2568}\u{2564}\u{2565}\u{2559}\u{2558}\u{2552}\u{2553}\u{256b}\u{256a}\u{2518}\u{250c}\u{2588}\u{2584}\u{258c}\u{2590}\u{2580}\u{3b1}\u{df}\u{393}\u{3c0}\u{3a3}\u{3c3}\u{b5}\u{3c4}\u{3a6}\u{398}\u{3a9}\u{3b4}\u{221e}\u{3c6}\u{3b5}\u{2229}\u{2261}\u{b1}\u{2265}\u{2264}\u{2320}\u{2321}\u{f7}\u{2248}\u{b0}\u{2219}\u{b7}\u{221a}\u{207f}\u{b2}\u{25a0}\u{a0}";
const CP1251_UPPER: &str = "\u{402}\u{403}\u{201a}\u{453}\u{201e}\u{2026}\u{2020}\u{2021}\u{20ac}\u{2030}\u{409}\u{2039}\u{40a}\u{40c}\u{40b}\u{40f}\u{452}\u{2018}\u{2019}\u{201c}\u{201d}\u{2022}\u{2013}\u{2014}\u{fffd}\u{2122}\u{459}\u{203a}\u{45a}\u{45c}\u{45b}\u{45f}\u{a0}\u{40e}\u{45e}\u{408}\u{a4}\u{490}\u{a6}\u{a7}\u{401}\u{a9}\u{404}\u{ab}\u{ac}\u{ad}\u{ae}\u{407}\u{b0}\u{b1}\u{406}\u{456}\u{491}\u{b5}\u{b6}\u{b7}\u{451}\u{2116}\u{454}\u{bb}\u{458}\u{405}\u{455}\u{457}\u{410}\u{411}\u{412}\u{413}\u{414}\u{415}\u{416}\u{417}\u{418}\u{419}\u{41a}\u{41b}\u{41c}\u{41d}\u{41e}\u{41f}\u{420}\u{421}\u{422}\u{423}\u{424}\u{425}\u{426}\u{427}\u{428}\u{429}\u{42a}\u{42b}\u{42c}\u{42d}\u{42e}\u{42f}\u{430}\u{431}\u{432}\u{433}\u{434}\u{435}\u{436}\u{437}\u{438}\u{439}\u{43a}\u{43b}\u{43c}\u{43d}\u{43e}\u{43f}\u{440}\u{441}\u{442}\u{443}\u{444}\u{445}\u{446}\u{447}\u{448}\u{449}\u{44a}\u{44b}\u{44c}\u{44d}\u{44e}\u{44f}";
const ISO88597_UPPER: &str = "\u{80}\u{81}\u{82}\u{83}\u{84}\u{a}\u{86}\u{87}\u{88}\u{89}\u{8a}\u{8b}\u{8c}\u{8d}\u{8e}\u{8f}\u{90}\u{91}\u{92}\u{93}\u{94}\u{95}\u{96}\u{97}\u{98}\u{99}\u{9a}\u{9b}\u{9c}\u{9d}\u{9e}\u{9f}\u{a0}\u{2018}\u{2019}\u{a3}\u{20ac}\u{20af}\u{a6}\u{a7}\u{a8}\u{a9}\u{37a}\u{ab}\u{ac}\u{ad}\u{fffd}\u{2015}\u{b0}\u{b1}\u{b2}\u{b3}\u{384}\u{385}\u{386}\u{b7}\u{388}\u{389}\u{38a}\u{bb}\u{38c}\u{bd}\u{38e}\u{38f}\u{390}\u{391}\u{392}\u{393}\u{394}\u{395}\u{396}\u{397}\u{398}\u{399}\u{39a}\u{39b}\u{39c}\u{39d}\u{39e}\u{39f}\u{3a0}\u{3a1}\u{fffd}\u{3a3}\u{3a4}\u{3a5}\u{3a6}\u{3a7}\u{3a8}\u{3a9}\u{3aa}\u{3ab}\u{3ac}\u{3ad}\u{3ae}\u{3af}\u{3b0}\u{3b1}\u{3b2}\u{3b3}\u{3b4}\u{3b5}\u{3b6}\u{3b7}\u{3b8}\u{3b9}\u{3ba}\u{3bb}\u{3bc}\u{3bd}\u{3be}\u{3bf}\u{3c0}\u{3c1}\u{3c2}\u{3c3}\u{3c4}\u{3c5}\u{3c6}\u{3c7}\u{3c8}\u{3c9}\u{3ca}\u{3cb}\u{3cc}\u{3cd}\u{3ce}\u{fffd}";

/// A single-byte page read against its upper half: a byte below 128
/// stands for itself, any other for the character the table gives it.
fn decode_with_upper(bytes: &[u8], upper: &str) -> String {
    let table: Vec<char> = upper.chars().collect();
    bytes.iter().map(|byte| if *byte < 128 { char::from(*byte) } else { table[*byte as usize - 128] }).collect()
}

/// What a line among a file's first two is: a comment, which may carry
/// a coding word, or anything else, past which no coding word is
/// looked for -- the reference stops the search at the first line that
/// is not blank and not only a comment.
enum CodingLine {
    Blank,
    Comment(Option<String>),
    Code,
}

/// Read a line of a file's head the way PEP 263 has the reference read
/// it. A coding word stands in a comment as `coding`, a `:` or `=`,
/// and the name, with or without room about the mark; when one mention
/// is not followed by the mark, the search goes on past it, since a
/// later one in the same comment may still declare the encoding.
fn coding_line(line: &[u8]) -> CodingLine {
    let text = String::from_utf8_lossy(line);
    let trimmed = text.trim_start_matches([' ', '\t', '\x0c']);
    if !trimmed.starts_with('#') {
        return if trimmed.trim_end_matches('\r').is_empty() { CodingLine::Blank } else { CodingLine::Code };
    }
    let mut rest = &trimmed[1..];
    while let Some(at) = rest.find("coding") {
        rest = &rest[at + 6..];
        let Some(marked) = rest.strip_prefix(':').or_else(|| rest.strip_prefix('=')) else { continue };
        let name: String = marked.trim_start_matches([' ', '\t']).chars()
            .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')).collect();
        if !name.is_empty() {
            return CodingLine::Comment(Some(name));
        }
    }
    CodingLine::Comment(None)
}

/// The name the reference knows a coding word by for the UTF-8 and
/// Latin-1 families -- `UTF_8`, `utf-8-sig` and their kin are UTF-8 to
/// it, and the Latin-1 spellings are ISO-8859-1. Any other name stays
/// as it was written.
fn normal_encoding_name(written: &str) -> &str {
    let folded: String = written.chars().take(12).map(|c| if c == '_' { '-' } else { c.to_ascii_lowercase() }).collect();
    if folded == "utf-8" || folded.starts_with("utf-8-") {
        "utf-8"
    } else if matches!(folded.as_str(), "latin-1" | "iso-8859-1" | "iso-latin-1")
        || folded.starts_with("latin-1-") || folded.starts_with("iso-8859-1-") || folded.starts_with("iso-latin-1-") {
        "iso-8859-1"
    } else {
        written
    }
}

/// A string's contents as the reference's reader bounds them: from
/// past the opening quote to the closing one. A backslash skips a
/// byte, a raw mark changes nothing in the reading, and a string a
/// line's end or the file's end cuts off has no contents the codec
/// sees -- the reader faults on it on its own. The answers are the
/// contents' bounds, whether they are decoded (a bytes literal's are
/// not), and where the reading resumes; a reading that resumes where
/// the contents ended means no closing quote was found.
fn string_contents(bytes: &[u8], at: usize, prefix: &[u8]) -> (usize, usize, bool, usize) {
    let n = bytes.len();
    let quote = bytes[at];
    let triple = bytes.get(at + 1) == Some(&quote) && bytes.get(at + 2) == Some(&quote);
    let content_start = at + if triple { 3 } else { 1 };
    let decodes = !prefix.iter().any(|b| matches!(b, b'b' | b'B'));
    let mut i = content_start;
    while i < n {
        let c = bytes[i];
        if c == b'\\' {
            i += 2;
            continue;
        }
        if !triple && (c == b'\n' || c == b'\r') {
            return (content_start, i, false, i);
        }
        if c == quote {
            if triple && !(bytes.get(i + 1) == Some(&quote) && bytes.get(i + 2) == Some(&quote)) {
                i += 1;
                continue;
            }
            return (content_start, i, decodes, i + if triple { 3 } else { 1 });
        }
        i += 1;
    }
    (content_start, n, false, n)
}

/// The first piece of a Python file the reference would decode as
/// UTF-8 and could not: a string's contents between its quotes, or a
/// run of letters, digits and bytes above 127 standing as a name. A
/// byte in a comment, in a bytes literal, or in a string cut off is
/// never decoded by the reference, so it is never faulted here; and
/// past a string cut off the reader faults on its own, so nothing
/// later is looked at. Answers the piece's bounds, or None when every
/// piece decodes.
fn utf8_fault_piece(bytes: &[u8]) -> Option<(usize, usize)> {
    let n = bytes.len();
    let mut i = 0;
    while i < n {
        let byte = bytes[i];
        if byte == b'#' {
            while i < n && bytes[i] != b'\n' { i += 1; }
        } else if byte.is_ascii_alphanumeric() || byte == b'_' || byte >= 128 {
            let start = i;
            while i < n && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] >= 128) { i += 1; }
            let run = &bytes[start..i];
            if i < n && (bytes[i] == b'\'' || bytes[i] == b'"')
                && run.iter().all(|b| matches!(b, b'r' | b'R' | b'b' | b'B' | b'f' | b'F'))
            {
                let (from, unto, decodes, resume) = string_contents(bytes, i, run);
                if decodes && std::str::from_utf8(&bytes[from..unto]).is_err() {
                    return Some((from, unto));
                }
                if resume == unto {
                    return None;
                }
                i = resume;
            } else if std::str::from_utf8(run).is_err() {
                return Some((start, i));
            }
        } else if byte == b'\'' || byte == b'"' {
            let (from, unto, decodes, resume) = string_contents(bytes, i, &[]);
            if decodes && std::str::from_utf8(&bytes[from..unto]).is_err() {
                return Some((from, unto));
            }
            if resume == unto {
                return None;
            }
            i = resume;
        } else {
            i += 1;
        }
    }
    None
}

/// A Python file read as UTF-8, as PEP 263 has it. With neither a
/// coding word nor a signature, every line is looked at raw, and a
/// byte that is not UTF-8 is named on its own, with the file and the
/// line it stands on. With either, the pieces the reader decodes --
/// string contents and names -- are looked at, and the codec's own
/// complaint is told with the file, the line and a mark under the
/// place, as the reference tells it; a file whose undecodable bytes
/// stand only where nothing decodes them is read as far as it goes,
/// for the reader to fault on.
fn utf8_file_source(written: &[u8], bytes: &[u8], signed: bool, declared: bool, given: &str) -> String {
    if let Ok(text) = std::str::from_utf8(bytes) {
        return text.to_owned();
    }
    if !signed && !declared {
        let error = std::str::from_utf8(bytes).unwrap_err();
        let upto = error.valid_up_to();
        let row = bytes[..upto].iter().filter(|byte| **byte == b'\n').count() + 1;
        eprintln!("SyntaxError: Non-UTF-8 code starting with '\\x{:02x}' in file {} on line {}, but no encoding declared; see https://peps.python.org/pep-0263/ for details", bytes[upto], given, row);
        process::exit(1);
    }
    let Some((from, unto)) = utf8_fault_piece(bytes) else {
        return String::from_utf8_lossy(bytes).into_owned();
    };
    let error = std::str::from_utf8(&bytes[from..unto]).unwrap_err();
    let at = from + error.valid_up_to();
    let row = bytes[..at].iter().filter(|byte| **byte == b'\n').count() + 1;
    // The complaint counts the place within the piece being decoded;
    // the mark under the row is set one past the place for a sequence
    // cut short, and on the byte itself otherwise.
    let position = error.valid_up_to();
    let (length, why, past) = match error.error_len() {
        None => (1, "unexpected end of data", 0),
        Some(length) if bytes[at] >= 194 => (length, "invalid continuation byte", length),
        Some(length) => (length, "invalid start byte", 0),
    };
    let complaint = if length == 1 {
        format!("'utf-8' codec can't decode byte 0x{:02x} in position {}", bytes[at], position)
    } else {
        format!("'utf-8' codec can't decode bytes in position {}-{}", position, position + length - 1)
    };
    let line_start = bytes[..at].iter().rposition(|byte| *byte == b'\n').map_or(0, |p| p + 1);
    let col = String::from_utf8_lossy(&bytes[line_start..at]).chars().count() + 1 + past;
    let whole = String::from_utf8_lossy(written);
    let line = whole.split('\n').nth(row - 1).unwrap_or("");
    let shown = line.trim_start();
    let lead = line.chars().count() - shown.chars().count();
    eprintln!("  File \"{}\", line {}", given, row);
    eprintln!("    {}", shown);
    eprintln!("    {}^", " ".repeat(col.saturating_sub(lead + 1)));
    eprintln!("SyntaxError: (unicode error) {}: {}", complaint, why);
    process::exit(1);
}

/// Read a Python file according to PEP 263: a UTF-8 signature is
/// discarded before it reaches the lexer, and a coding word in a
/// comment among the first two lines names how the rest decodes. The
/// same words anywhere else, a string's contents among them, name
/// nothing. A file naming an encoding its bytes do not keep to, one
/// conflicting with the signature, or one the interpreter does not
/// know, is told the way the reference tells it, and the run ends
/// before it begins.
fn python_file_source(written: Vec<u8>, given: &str) -> String {
    let signed = written.starts_with(&[0xef, 0xbb, 0xbf]);
    let bytes = written.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&written);
    let mut cookie: Option<String> = None;
    for line in bytes.split(|byte| *byte == b'\n').take(2) {
        match coding_line(line) {
            CodingLine::Comment(Some(name)) => {
                cookie = Some(name);
                break;
            }
            CodingLine::Comment(None) | CodingLine::Blank => {}
            CodingLine::Code => break,
        }
    }
    let named = cookie.clone().unwrap_or_else(|| "utf-8".to_string());
    let normal = normal_encoding_name(&named);
    if signed && normal != "utf-8" {
        eprintln!("SyntaxError: encoding problem: {} with BOM", normal);
        process::exit(1);
    }
    match named.to_ascii_lowercase().replace(['-', '_'], "").as_str() {
        "utf8" | "utf8sig" => utf8_file_source(&written, bytes, signed, cookie.is_some(), given),
        "ascii" | "usascii" => {
            if bytes.iter().any(|byte| *byte > 127) {
                eprintln!("SyntaxError: encoding problem: {}", normal);
                process::exit(1);
            }
            bytes.iter().copied().map(char::from).collect()
        }
        "latin1" | "latin" | "iso88591" | "isolatin1" => bytes.iter().copied().map(char::from).collect(),
        "cp437" | "437" | "ibm437" => decode_with_upper(bytes, CP437_UPPER),
        "cp1251" => decode_with_upper(bytes, CP1251_UPPER),
        "iso88597" => decode_with_upper(bytes, ISO88597_UPPER),
        _ => {
            eprintln!("SyntaxError: encoding problem: {}", normal);
            process::exit(1);
        }
    }
}

/// Hold source in the form its language asks for: byte values as
/// characters when text is bytes, decoded characters otherwise.
fn source_of(written: Vec<u8>, as_bytes: bool) -> String {
    match as_bytes {
        true => written.into_iter().map(char::from).collect(),
        false => String::from_utf8_lossy(&written).into_owned(),
    }
}

/// A one-line script that runs this very binary again with the kernel
/// and the language a program was started with, for the one case the
/// binary is not its own start: a host begun through the dynamic
/// loader, whose libraries stand in the loader's own directory and
/// must be named again on every start. The script is written into a
/// directory made fresh for this one process -- private (mode 0700) and
/// named with the process's own number -- so that nothing else can
/// plant a script where a program would run it. No such directory is
/// made on an ordinary start, which names the binary itself.
fn launcher_for(started: &Path, binary: &Path, kernel: &str, language: &str) -> String {
    let mut dir = match fresh_private_dir() {
        Some(dir) => dir,
        None => return String::new(),
    };
    let libraries = started.parent().map_or_else(String::new, |p| p.to_string_lossy().into_owned());
    let quoted = |word: &str| word.replace('\'', r#"'\''"#);
    let script = format!(
        "#!/bin/sh\nexec '{}' --library-path '{}' '{}' --kernel '{}' --lang '{}' \"$@\"\n",
        quoted(&started.to_string_lossy()),
        quoted(&libraries),
        quoted(&binary.to_string_lossy()),
        kernel,
        language,
    );
    dir.push("lumen");
    let _ = std::fs::write(&dir, &script);
    use std::os::unix::fs::PermissionsExt as _;
    let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700));
    dir.to_string_lossy().into_owned()
}

/// A directory of the process's own, made fresh and private (mode
/// 0700), named with the process's number and a count when that name
/// is already taken. A name already standing is never reused: the
/// next name is tried instead, so nothing planted there can be run.
fn fresh_private_dir() -> Option<std::path::PathBuf> {
    use std::os::unix::fs::PermissionsExt as _;
    let pid = std::process::id();
    let mut count: u32 = 0;
    loop {
        let mut dir = std::env::temp_dir();
        if count == 0 {
            dir.push(format!("lumen-lang-{pid}"));
        } else {
            dir.push(format!("lumen-lang-{pid}-{count}"));
        }
        match std::fs::create_dir(&dir) {
            Ok(()) => {
                let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700));
                return Some(dir);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists && count < 1000 => count += 1,
            Err(_) => return None,
        }
    }
}

fn main() {
    // A language may allow a program to go a thousand calls deep before
    // it complains, which wants more of the machine's stack than a run
    // is given by default; the whole run is therefore made on a thread
    // with room for it.
    let roomy = std::thread::Builder::new()
        .stack_size(1 << 30)
        .spawn(run_all)
        .expect("a thread for the run");
    match roomy.join() {
        Ok(()) => {}
        Err(panicked) => std::panic::resume_unwind(panicked),
    }
}

fn run_all() {
    let args: Vec<OsString> = env::args_os().collect();
    let inv = parse_args(&args);

    // The kernel and the language this run was started with are put
    // where the environment can carry them to a child process, so a
    // program that starts this binary again without naming them itself
    // still starts a second interpreter like itself. Nothing is written
    // to a file to say this: the words travel with the run.
    std::env::set_var("LUMEN_KERNEL", inv.kernel.as_str());
    std::env::set_var("LUMEN_LANG", inv.language.name());
    if inv.language.name() == "python" {
        if inv.python_development { std::env::set_var("LUMEN_PYTHON_DEV_MODE", "1"); }
        else { std::env::remove_var("LUMEN_PYTHON_DEV_MODE"); }
    }
    // Only an actual -m invocation carries module execution metadata;
    // a script child must not inherit its parent's module identity.
    if let Some(name) = &inv.module_name { std::env::set_var("LUMEN_RUN_MODULE", name); }
    else { std::env::remove_var("LUMEN_RUN_MODULE"); }
    if let Some(version) = inv.python { std::env::set_var("LUMEN_PYTHON", version.release); }

    let written = inv.module_source.as_ref().map(|source| source.as_bytes().to_vec()).unwrap_or_else(|| fs::read(&inv.file).unwrap_or_else(|e| {
        eprintln!("Error: Failed to read {}: {}", inv.file, e);
        process::exit(1);
    }));
    let source = if inv.language.name() == "python" && honours_extensions(&inv.kernel) {
        python_file_source(written, &inv.file)
    } else {
        source_of(written, text_is_bytes(&inv.language) && honours_extensions(&inv.kernel))
    };
    let source = without_shebang(source);

    // Every program runs on top of its language's library.
    let given_lines = source.lines().count();
    let source = with_library(&inv.kernel, inv.language.name(), source);
    // How many lines the library put before the program's own text, so
    // that a kernel can name the line a program was written on rather
    // than the line of the two run together.
    let lines_before = source.lines().count().saturating_sub(given_lines);

    if let Some(target) = &inv.emit {
        if inv.kernel != "microcode11" {
            eprintln!("Error: --emit needs --kernel microcode11");
            process::exit(1);
        }
        let text_of = |language: &Language| -> String {
            match language {
                Language::Named(name) => lumen_microcode11::embedded(name).unwrap_or_else(|e| {
                    eprintln!("{}", e);
                    process::exit(1);
                }).to_string(),
                Language::File { text, .. } => text.clone(),
            }
        };
        match lumen_microcode11::emit(&text_of(&inv.language), &source, &text_of(target)) {
            Ok(text) => print!("{}", text),
            Err(message) => {
                eprintln!("{}", message);
                process::exit(1);
            }
        }
        return;
    }

    // What a web request carries, gathered once here so that a kernel
    // has only to bind it: the full kernels take it, the others do not
    // read the labels that name it.
    let mut request = web::gathered(text_is_bytes(&inv.language) && honours_extensions(&inv.kernel));
    // Resolve the optional library location before the program can change directory.
    if let Some(root) = std::env::var_os("LUMEN_ROOT") {
        let root = std::path::PathBuf::from(root);
        let root = std::fs::canonicalize(&root).unwrap_or_else(|_| {
            std::env::current_dir().unwrap_or_default().join(root)
        });
        let modules = root.join("langs/lib_python/modules");
        request.push(("SELF".to_string(), "library_root".to_string(), modules.to_string_lossy().into_owned(), false));
    }
    let overlay = inv.python.map_or(&[][..], |v| v.modules);
    for (name, source, file) in overlay.iter().chain(embedded_modules::MODULES.iter().filter(|(name, ..)| !overlay.iter().any(|(overlaid, ..)| overlaid == name))) {
        let source = inv.python.map_or_else(|| source.to_string(), |v| v.module_source(name, source));
        request.push(("MODULE".to_string(), name.to_string(), source, false));
        request.push(("MODULE_FILE".to_string(), name.to_string(), file.to_string(), false));
    }
    let aliases = inv.python.map_or(&[][..], |v| v.aliases);
    for (name, member) in aliases.iter().chain(embedded_modules::MODULE_ALIASES.iter().filter(|(name, ..)| !aliases.iter().any(|(overlaid, ..)| overlaid == name))) {
        request.push(("MODULE_ALIAS".to_string(), name.to_string(), member.to_string(), false));
    }
    // Where the program itself lies. A definition may give names to
    // these, and only the full kernels read those labels.
    let whole = std::fs::canonicalize(&inv.file).unwrap_or_else(|_| std::path::PathBuf::from(&inv.file));
    let held = whole.parent().map_or_else(|| ".".to_string(), |p| p.to_string_lossy().into_owned());
    request.push(("SELF".to_string(), "file".to_string(), whole.to_string_lossy().into_owned(), false));
    request.push(("SELF".to_string(), "directory".to_string(), held, false));
    // The path as the run was started with it, links and all: a
    // complaint about reading the program names it the way its reader
    // named it, which is how the reference names it too.
    request.push(("SELF".to_string(), "given".to_string(), inv.file.clone(), false));
    // The program running this one, as the system knows it, which a
    // language may name for a program that wants to find itself again.
    // A full Python kernel names the binary itself, so a program that
    // runs the interpreter it names is running a second interpreter
    // like itself; the kernel and the language travel in the
    // environment. Only a host begun through the dynamic loader names a
    // launcher instead, since its binary cannot be run bare.
    if let Ok(runner) = std::env::current_exe() {
        let runner = if FULL_KERNELS.contains(&inv.kernel.as_str()) && inv.language.name() == "python" {
            // The host may be started through the dynamic loader, in
            // which case the system names the loader as the running
            // program; the first word of the arguments names the binary
            // itself. Only that start needs a launcher, written into a
            // private directory of the run's own; an ordinary start
            // names the binary itself.
            let binary = match args.first() {
                Some(word) => {
                    let path = std::path::PathBuf::from(word);
                    std::fs::canonicalize(&path).unwrap_or(path)
                }
                None => runner.clone(),
            };
            let through_loader = std::fs::canonicalize(&runner)
                .ok()
                .zip(std::fs::canonicalize(&binary).ok())
                .map_or(false, |(one, other)| one != other);
            if through_loader {
                launcher_for(&runner, &binary, &inv.kernel, inv.language.name())
            } else {
                binary.to_string_lossy().into_owned()
            }
        } else {
            runner.to_string_lossy().into_owned()
        };
        request.push(("SELF".to_string(), "runner".to_string(), runner, false));
    }
    request.push(("SELF".to_string(), "lines_before".to_string(), lines_before.to_string(), true));

    // Serving runs the program once for each request that arrives, with
    // the request in its environment, so a served run is an ordinary run.
    if let Some(address) = &inv.serve {
        let mut program: Vec<String> = Vec::new();
        program.push("--kernel".to_string());
        program.push(inv.kernel.clone());
        if let Some(named) = inv.language.given() {
            program.push("--lang".to_string());
            program.push(named);
        }
        program.push(inv.file.clone());
        program.extend(inv.program_args.iter().cloned());
        if let Err(e) = web::serve(address, &program) {
            eprintln!("{}", e);
            process::exit(1);
        }
        return;
    }

    // The count of room begins here, with the program read, the language
    // read, and what a request carries gathered — everything the host
    // needed before a kernel was called at all. A kernel that goes on to
    // read the program and build a machine for it narrows the count
    // again, to the moment the program itself begins to run; this is the
    // widest mark, and the only one for a kernel that sets none.
    lumen_room::mark();

    let result = match (inv.kernel.as_str(), &inv.language) {
        ("stream35", Language::Named(name)) => lumen_stream35::run(name, &source, &inv.program_args),
        ("stream35", Language::File { text, .. }) => lumen_stream35::run_definition(text, &source, &inv.program_args),
        ("microcode11", Language::Named(name)) => lumen_microcode11::run(name, &source, &inv.program_args),
        ("microcode11", Language::File { text, .. }) => lumen_microcode11::run_definition(text, &source, &inv.program_args),
        ("microcode4", Language::Named(name)) => lumen_microcode4::run(name, &source, &inv.program_args),
        ("microcode4", Language::File { text, .. }) => lumen_microcode4::run_definition(text, &source, &inv.program_args),
        ("stack5", Language::Named(name)) => lumen_stack5::run(name, &source, &inv.program_args),
        ("stack5", Language::File { text, .. }) => lumen_stack5::run_definition(text, &source, &inv.program_args),
        ("stack8", Language::Named(name)) => lumen_stack8::run(name, &source, &inv.program_args, &request),
        ("stack8", Language::File { text, .. }) => lumen_stack8::run_definition(text, &source, &inv.program_args, &request),
        ("microcode7", Language::Named(name)) => lumen_microcode7::run(name, &source, &inv.program_args, &request),
        ("microcode7", Language::File { text, .. }) => lumen_microcode7::run_definition(text, &source, &inv.program_args, &request),
        _ => unreachable!("kernel names are validated in parse_args"),
    };

    if let Err(message) = result {
        if !message.is_empty() { eprintln!("{}", message); }
        process::exit(1);
    }
}

fn usage(program: &str) -> ! {
    eprintln!(
        "Usage: {} [--kernel stream35|microcode11|microcode4|microcode7|stack5|stack8] [--lang <name|extension|definition.json>] [--emit <name|extension|definition.json>] [--python <version>] [--serve <address|port>] [-d setting=worth] [-n] [-v] <file> [program args...]",
        program
    );
    process::exit(1);
}

/// The embedded languages, each name with its extensions.
fn embedded_languages() -> Vec<(String, Vec<String>)> {
    lumen_stack8::languages().unwrap_or_else(|e| {
        eprintln!("Error: embedded language definitions: {}", e);
        process::exit(1);
    })
}

/// Read a definition file and take the language name it declares.
fn definition_from_file(path: &str) -> Language {
    let text = fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("Error: Failed to read {}: {}", path, e);
        process::exit(1);
    });
    let name = lumen_stack8::language_of(&text).unwrap_or_else(|e| {
        eprintln!("Error: language definition {}: {}", path, e);
        process::exit(1);
    });
    Language::File { name, path: path.to_string(), text }
}

/// Resolve a `--lang` value.
///
/// A value that ends in `.json` or contains a path separator is a definition
/// file. Anything else names an embedded language, by its name or by one of
/// the extensions it declares; a leading dot on an extension is tolerated.
fn language_from_flag(value: &str) -> Language {
    if value.ends_with(".json") || value.contains('/') || value.contains('\\') {
        return definition_from_file(value);
    }
    let wanted = value.trim_start_matches('.').to_lowercase();
    let languages = embedded_languages();
    if let Some((name, _)) = languages.iter().find(|(name, _)| *name == wanted) {
        return Language::Named(name.clone());
    }
    if let Some((name, _)) = languages.iter().find(|(_, exts)| exts.iter().any(|e| *e == wanted)) {
        return Language::Named(name.clone());
    }
    eprintln!(
        "Error: Unknown language '{}'. Use one of: {}, or the path of a definition file.",
        value,
        accepted_language_words(&languages).join(", ")
    );
    process::exit(1);
}

/// Every word `--lang` accepts: each language's name, then its extensions.
fn accepted_language_words(languages: &[(String, Vec<String>)]) -> Vec<String> {
    let mut words = Vec::new();
    for (name, exts) in languages {
        words.push(name.clone());
        words.extend(exts.iter().filter(|e| *e != name).cloned());
    }
    words
}

/// A setting the run was started with, put where the run's surroundings
/// put one: under its own name with PHP_INI_ before it, which is the
/// road by which the host already reads a setting. The name is the
/// bytes before the first mark of equals and the worth the bytes after
/// it; a name with no such mark stands at one, as the reference has it.
fn settle_setting(given: &[u8]) {
    let (name, worth) = match given.iter().position(|b| *b == b'=') {
        Some(at) => (&given[..at], &given[at + 1..]),
        None => (given, b"1".as_slice()),
    };
    if name.is_empty() {
        return;
    }
    let named = format!("PHP_INI_{}", String::from_utf8_lossy(name));
    env::set_var(named, OsStr::from_bytes(worth));
}

/// Whatever the run was told to load beside itself, reached for as it
/// was told. This host carries nothing it can take in while it runs, so
/// nothing can come of the reaching; what is said here is what actually
/// happened when the file was reached for, named by the very path that
/// was tried, and nothing at all is said where nothing was tried.
fn extensions_reached_for() {
    let Some(named) = env::var_os("PHP_INI_extension") else { return };
    let named = named.as_bytes().to_vec();
    if named.is_empty() {
        return;
    }
    let held = env::var_os("PHP_INI_extension_dir").map(|d| d.as_bytes().to_vec()).unwrap_or_default();
    let mut tried: Vec<u8> = Vec::new();
    // The bare name first, and then the name wearing the ending a
    // shared library wears, which is the pair the reference tries.
    for (nth, ending) in [b"".as_slice(), b".so".as_slice()].into_iter().enumerate() {
        let mut path = held.clone();
        if !path.is_empty() && !path.ends_with(b"/") {
            path.push(b'/');
        }
        path.extend_from_slice(&named);
        path.extend_from_slice(ending);
        let stood = match fs::File::open(OsStr::from_bytes(&path)) {
            Err(e) => plainly(&e),
            Ok(_) => "found, but this host takes nothing in while it runs".to_string(),
        };
        if nth > 0 {
            tried.extend_from_slice(b", ");
        }
        tried.extend_from_slice(&path);
        tried.extend_from_slice(format!(" ({stood})").as_bytes());
    }
    // The words go where the run writes, which is where the reference
    // puts a complaint it is telling on the page. A path may be spelled
    // with bytes that spell no letter, so the whole is put together as
    // bytes and written out as bytes.
    let mut said: Vec<u8> = b"\nWarning: PHP Startup: Unable to load dynamic library '".to_vec();
    said.extend_from_slice(&named);
    said.extend_from_slice(b"' (tried: ");
    said.extend_from_slice(&tried);
    said.extend_from_slice(b") in Unknown on line 0\n");
    use std::io::Write as _;
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(&said);
    let _ = out.flush();
}

/// What the host said went wrong, without the number it puts after the
/// words, which says the same thing over again in figures.
fn plainly(e: &std::io::Error) -> String {
    let said = e.to_string();
    match said.find(" (os error ") {
        Some(at) => said[..at].to_string(),
        None => said,
    }
}

fn parse_args(args: &[OsString]) -> Invocation {
    let held = args.first().map(|a| a.to_string_lossy().into_owned()).unwrap_or_else(|| "lumen-lang".to_string());
    let program = held.as_str();
    let mut rest: &[OsString] = &args[1..];

    let mut kernel: Option<String> = None;
    let mut serve: Option<String> = None;
    let mut language: Option<Language> = None;
    let mut emit: Option<Language> = None;
    let mut python: Option<String> = None;
    let mut file: Option<String> = None;
    let mut module_source = None;
    let mut module_name = None;
    let mut python_development = false;
    // Whether the run was asked only to name itself and stop.
    let mut names_itself = false;

    // An argument as text, for the switches that take one. Bytes
    // spelling no letter stand for the letter of doubt, since a name
    // this host must reach for is a name it must be able to say.
    let said = |a: &OsString| a.to_string_lossy().into_owned();

    // Options may precede the file; `--lang` may also follow it directly.
    loop {
        match rest.first().and_then(|a| a.to_str()) {
            Some("-X") if file.is_none() => {
                if rest.len() < 2 { usage(program); }
                if said(&rest[1]) != "dev" {
                    eprintln!("Error: unsupported Python -X option '{}'", said(&rest[1]));
                    process::exit(2);
                }
                python_development = true;
                rest = &rest[2..];
            }
            Some("-Xdev") if file.is_none() => {
                python_development = true;
                rest = &rest[1..];
            }
            Some("--kernel") if file.is_none() => {
                if rest.len() < 2 {
                    usage(program);
                }
                let held = said(&rest[1]).to_lowercase();
                if !KERNELS.contains(&held.as_str()) {
                    eprintln!("Error: Unknown kernel '{}'. Use one of: {}", held, KERNELS.join(", "));
                    process::exit(1);
                }
                kernel = Some(held);
                rest = &rest[2..];
            }
            Some("--lang") | Some("--language") => {
                if rest.len() < 2 {
                    eprintln!("Error: {} requires an argument", said(&rest[0]));
                    process::exit(1);
                }
                language = Some(language_from_flag(&said(&rest[1])));
                rest = &rest[2..];
            }
            Some("--python") => {
                // For other languages, options after the source stay program arguments.
                if let Some(source) = &file {
                    let python_source = language.as_ref().map_or_else(|| {
                        match env::var("LUMEN_LANG") {
                            Ok(value) if !value.trim().is_empty() => language_from_flag(&value).name() == "python",
                            _ => language_from_extension(source).as_deref() == Some("python"),
                        }
                    }, |language| language.name() == "python");
                    if !python_source { break; }
                }
                if rest.len() < 2 {
                    eprintln!("Error: --python requires a version; Supported releases: {}", python_versions::VERSIONS.iter().map(|v| v.release).collect::<Vec<_>>().join(", "));
                    process::exit(1);
                }
                python = Some(said(&rest[1]));
                rest = &rest[2..];
            }
            Some("--serve") => {
                if rest.len() < 2 {
                    eprintln!("Error: --serve requires an address or a port");
                    process::exit(1);
                }
                serve = Some(said(&rest[1]));
                rest = &rest[2..];
            }
            // A setting given where the run starts, spelled as the
            // reference spells one, and written either as two words or
            // as one. It is put where the surroundings put the same
            // setting, under its own name with PHP_INI_ before it, so
            // that one road serves them both; a setting named with
            // nothing after it stands at one. These three switches are
            // the reference's own, and like its own they are looked for
            // only before the file: what stands after the file belongs
            // to the program and is left to it.
            Some(flag) if file.is_none() && (flag == "-m" || flag.starts_with("-m") && flag.len() > 2
                && (python.is_some() || language.as_ref().is_some_and(|held| held.name() == "python") || env::var("LUMEN_LANG").is_ok_and(|held| held == "python"))) => {
                let (module, used) = if flag == "-m" {
                    if rest.len() < 2 { usage(program); }
                    (said(&rest[1]), 2)
                } else { (flag[2..].to_owned(), 1) };
                if language.as_ref().is_some_and(|definition| definition.name() != "python") {
                    eprintln!("Error: -m requires the Python language");
                    process::exit(1);
                }
                let path = module.replace('.', "/");
                let mut found = None;
                for candidate in [format!("{path}.py"), format!("{path}/__main__.py")] {
                    if let Ok(source) = fs::read_to_string(&candidate) { found = Some((candidate, source)); break; }
                }
                if found.is_none() {
                    let executable_module = format!("{module}.__main__");
                    if let Some((_, source, filename)) = embedded_modules::MODULES.iter().find(|(name, _, _)| *name == executable_module)
                        .or_else(|| embedded_modules::MODULES.iter().find(|(name, _, _)| *name == module)) {
                        found = Some((format!("langs/lib_python/modules/{filename}"), source.to_string()));
                    }
                }
                let Some((filename, source)) = found else {
                    eprintln!("No module named {module}");
                    process::exit(1);
                };
                module_name = Some(if filename.ends_with("/__main__.py") { format!("{module}.__main__") } else { module });
                file = Some(filename);
                module_source = Some(source);
                language = Some(Language::Named("python".to_string()));
                rest = &rest[used..];
                break;
            }
            Some("-d") if file.is_none() => {
                if rest.len() < 2 {
                    eprintln!("Error: -d requires a setting");
                    process::exit(1);
                }
                settle_setting(rest[1].as_bytes());
                rest = &rest[2..];
            }
            Some(joined) if file.is_none() && joined.len() > 2 && joined.starts_with("-d") => {
                settle_setting(&rest[0].as_bytes()[2..]);
                rest = &rest[1..];
            }
            // The run asked only to say what it is.
            Some("-v") | Some("--version") if file.is_none() => {
                names_itself = true;
                rest = &rest[1..];
            }
            // The run asked to start with no file of settings behind
            // it. This host reads no such file whatever it is told, so
            // the asking is taken and nothing follows from it.
            Some("-n") if file.is_none() => {
                rest = &rest[1..];
            }
            Some("--emit") => {
                if rest.len() < 2 {
                    eprintln!("Error: --emit requires an argument");
                    process::exit(1);
                }
                emit = Some(language_from_flag(&said(&rest[1])));
                rest = &rest[2..];
            }
            Some(_) | None if file.is_none() && !rest.is_empty() => {
                file = Some(said(&rest[0]));
                rest = &rest[1..];
            }
            _ => break,
        }
    }

    // A kernel not named on the command line may be carried in the
    // environment, by a host that started this binary again without
    // naming it; a flag always wins over either. Where neither names
    // one, the default stands.
    let kernel = kernel.unwrap_or_else(|| {
        let held = env::var("LUMEN_KERNEL").unwrap_or_default().to_lowercase();
        if held.is_empty() {
            DEFAULT_KERNEL.to_string()
        } else if KERNELS.contains(&held.as_str()) {
            held
        } else {
            eprintln!("Error: Unknown kernel '{}'. Use one of: {}", held, KERNELS.join(", "));
            process::exit(1);
        }
    });

    // Whatever the run was told to load beside itself is reached for
    // here, before anything else it was told to do, as the reference
    // reaches for one at its own start.
    extensions_reached_for();
    if names_itself {
        println!("{} {} (kernel {})", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"), kernel);
        process::exit(0);
    }

    let file = file.unwrap_or_else(|| usage(program));
    let language = language.unwrap_or_else(|| {
        // A language not named on the command line may be carried in
        // the environment, by a host that started this binary again
        // without naming it; a flag always wins. Where neither names
        // one, the file's own extension, and then the default, stand.
        match env::var("LUMEN_LANG") {
            Ok(held) if !held.trim().is_empty() => language_from_flag(&held),
            _ => Language::Named(language_from_extension(&file).unwrap_or_else(|| DEFAULT_LANGUAGE.to_string())),
        }
    });

    if python_development && language.name() != "python" {
        eprintln!("Error: -X dev requires the Python language");
        process::exit(2);
    }
    let mut language = language;
    let python = if language.name() == "python" {
        let version = python_versions::select(python.as_deref(), &file).unwrap_or_else(|e| { eprintln!("{e}"); process::exit(1) });
        let base = match &language {
            Language::Named(name) => lumen_microcode11::embedded(name).unwrap().to_string(),
            Language::File { text, .. } => text.clone(),
        };
        let path = language.given().unwrap();
        let text = version.definition(&base).unwrap_or_else(|e| { eprintln!("Error: {e}"); process::exit(1) });
        language = Language::File { name: "python".to_string(), path, text };
        Some(version)
    } else { None };
    Invocation { kernel, file, serve, language, emit, python, module_source, module_name, python_development, program_args: rest.iter().map(said).collect() }
}

/// The language whose embedded definition claims the file's extension.
fn language_from_extension(file: &str) -> Option<String> {
    let ext = Path::new(file).extension()?.to_str()?;
    embedded_languages().into_iter().find(|(_, exts)| exts.iter().any(|x| x == ext)).map(|(name, _)| name)
}

fn embedded_file(path: &str) -> Option<&'static str> {
    embedded_files::EMBEDDED_FILES.iter().find(|(p, _)| *p == path).map(|(_, c)| *c)
}

/// Expand `include "path"` lines recursively from the embedded library.
/// Each file is included at most once; every other line is copied through.
fn expand_includes(source: &str) -> Result<String, String> {
    fn walk(source: &str, seen: &mut HashSet<String>, out: &mut String) -> Result<(), String> {
        for line in source.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("include ") {
                let rest = rest.trim();
                if !(rest.starts_with('"') && rest.ends_with('"') && rest.len() >= 2) {
                    return Err(format!("Invalid include syntax: {}", line));
                }
                let path = &rest[1..rest.len() - 1];
                if !seen.insert(path.to_string()) {
                    continue;
                }
                let contents = embedded_file(path)
                    .ok_or_else(|| format!("File not found in embedded filesystem: {}", path))?;
                walk(contents, seen, out)?;
                out.push('\n');
            } else {
                out.push_str(line);
                out.push('\n');
            }
        }
        Ok(())
    }
    let mut out = String::new();
    walk(source, &mut HashSet::new(), &mut out)?;
    Ok(out)
}
