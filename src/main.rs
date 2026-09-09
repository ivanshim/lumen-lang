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

/// What was written in a file, as the kernel is to hold it. Where text
/// is bytes, each byte stands as the character of its own number, so
/// that nothing of what was written is lost and the count of characters
/// is the count of bytes. Where it is letters, the bytes are read as
/// the letters they spell, and a byte spelling none is passed over.
fn source_of(written: Vec<u8>, as_bytes: bool) -> String {
    match as_bytes {
        true => written.into_iter().map(char::from).collect(),
        false => String::from_utf8_lossy(&written).into_owned(),
    }
}

fn main() {
    let args: Vec<OsString> = env::args_os().collect();
    let inv = parse_args(&args);

    let written = fs::read(&inv.file).unwrap_or_else(|e| {
        eprintln!("Error: Failed to read {}: {}", inv.file, e);
        process::exit(1);
    });
    let source = source_of(written, text_is_bytes(&inv.language) && honours_extensions(&inv.kernel));
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
    // Where the program itself lies. A definition may give names to
    // these, and only the full kernels read those labels.
    let whole = std::fs::canonicalize(&inv.file).unwrap_or_else(|_| std::path::PathBuf::from(&inv.file));
    let held = whole.parent().map_or_else(|| ".".to_string(), |p| p.to_string_lossy().into_owned());
    request.push(("SELF".to_string(), "file".to_string(), whole.to_string_lossy().into_owned(), false));
    request.push(("SELF".to_string(), "directory".to_string(), held, false));
    // The program running this one, as the system knows it, which a
    // language may name for a program that wants to find itself again.
    if let Ok(runner) = std::env::current_exe() {
        request.push(("SELF".to_string(), "runner".to_string(), runner.to_string_lossy().into_owned(), false));
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
        eprintln!("{}", message);
        process::exit(1);
    }
}

fn usage(program: &str) -> ! {
    eprintln!(
        "Usage: {} [--kernel stream35|microcode11|microcode4|microcode7|stack5|stack8] [--lang <name|extension|definition.json>] [--emit <name|extension|definition.json>] [--serve <address|port>] [-d setting=worth] [-n] [-v] <file> [program args...]",
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

    let mut kernel = DEFAULT_KERNEL.to_string();
    let mut serve: Option<String> = None;
    let mut language: Option<Language> = None;
    let mut emit: Option<Language> = None;
    let mut file: Option<String> = None;
    // Whether the run was asked only to name itself and stop.
    let mut names_itself = false;

    // An argument as text, for the switches that take one. Bytes
    // spelling no letter stand for the letter of doubt, since a name
    // this host must reach for is a name it must be able to say.
    let said = |a: &OsString| a.to_string_lossy().into_owned();

    // Options may precede the file; `--lang` may also follow it directly.
    loop {
        match rest.first().and_then(|a| a.to_str()) {
            Some("--kernel") if file.is_none() => {
                if rest.len() < 2 {
                    usage(program);
                }
                kernel = said(&rest[1]).to_lowercase();
                if !KERNELS.contains(&kernel.as_str()) {
                    eprintln!("Error: Unknown kernel '{}'. Use one of: {}", kernel, KERNELS.join(", "));
                    process::exit(1);
                }
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
        Language::Named(language_from_extension(&file).unwrap_or_else(|| DEFAULT_LANGUAGE.to_string()))
    });

    Invocation { kernel, file, serve, language, emit, program_args: rest.iter().map(said).collect() }
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
