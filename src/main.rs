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

include!("python_encodings.rs");

use std::collections::HashSet;
use std::env;
use std::io::Read;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::process;

mod web;
mod python_versions;
mod prepared_libraries { include!(concat!(env!("OUT_DIR"), "/prepared_libraries.rs")); }
mod python_sources;

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
        format!("__native_round = round\n{}\nround = __native_round\ndel __native_round\nimport sys{}", library,
            format!("{}{}",
                // The command's text is handed over as a literal: reading it back through os
                // would load the whole os chain, and the import bootstrap is left to the first
                // import that asks for it, as it is for a program run from a file.
                match env::var("LUMEN_COMMAND_SOURCE") {
                    Ok(text) => format!("\nimport linecache\nlinecache._register_code(sys._getframe().f_code, {}, '<string>')\n__spec__ = None\ndel __file__", serde_json::to_string(&text).expect("a source string can be quoted")),
                    Err(_) => String::new(),
                },
                if env::var("LUMEN_WARNOPTIONS").is_ok_and(|options| !options.is_empty()) { "\nimport warnings" } else { "" }))
    } else { library };
    let lead = source.len() - source.trim_start().len();
    let standalone = language != "python" || source.get(lead + prologue.len()..).is_some_and(|tail| {
        let tail = tail.trim_start_matches([' ', '\t']);
        tail.is_empty() || tail.starts_with(['\n', '\r', '#'])
    });
    if !prologue.is_empty() && standalone && !source[..lead].contains('\n') && source[lead..].starts_with(prologue) {
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
    warning_options: Vec<String>,
    python: Option<&'static python_versions::PythonVersion>,
    module_source: Option<String>,
    module_name: Option<String>,
    python_development: bool,
    program_name: Option<String>,
    python_ignore_environment: bool,
    python_isolated: bool,
    python_interactive: bool,
    python_safe_path: bool,
    python_quiet: bool,
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

const KOI8R_UPPER: &str = "\u{2500}\u{2502}\u{250c}\u{2510}\u{2514}\u{2518}\u{251c}\u{2524}\u{252c}\u{2534}\u{253c}\u{2580}\u{2584}\u{2588}\u{258c}\u{2590}\u{2591}\u{2592}\u{2593}\u{2320}\u{25a0}\u{2219}\u{221a}\u{2248}\u{2264}\u{2265}\u{a0}\u{2321}\u{b0}\u{b2}\u{b7}\u{f7}\u{2550}\u{2551}\u{2552}\u{451}\u{2553}\u{2554}\u{2555}\u{2556}\u{2557}\u{2558}\u{2559}\u{255a}\u{255b}\u{255c}\u{255d}\u{255e}\u{255f}\u{2560}\u{2561}\u{401}\u{2562}\u{2563}\u{2564}\u{2565}\u{2566}\u{2567}\u{2568}\u{2569}\u{256a}\u{256b}\u{256c}\u{a9}\u{44e}\u{430}\u{431}\u{446}\u{434}\u{435}\u{444}\u{433}\u{445}\u{438}\u{439}\u{43a}\u{43b}\u{43c}\u{43d}\u{43e}\u{43f}\u{44f}\u{440}\u{441}\u{442}\u{443}\u{436}\u{432}\u{44c}\u{44b}\u{437}\u{448}\u{44d}\u{449}\u{447}\u{44a}\u{42e}\u{410}\u{411}\u{426}\u{414}\u{415}\u{424}\u{413}\u{425}\u{418}\u{419}\u{41a}\u{41b}\u{41c}\u{41d}\u{41e}\u{41f}\u{42f}\u{420}\u{421}\u{422}\u{423}\u{416}\u{412}\u{42c}\u{42b}\u{417}\u{428}\u{42d}\u{429}\u{427}\u{42a}";
const CP1252_UPPER: &str = "\u{20ac}\u{81}\u{201a}\u{192}\u{201e}\u{2026}\u{2020}\u{2021}\u{2c6}\u{2030}\u{160}\u{2039}\u{152}\u{8d}\u{17d}\u{8f}\u{90}\u{2018}\u{2019}\u{201c}\u{201d}\u{2022}\u{2013}\u{2014}\u{2dc}\u{2122}\u{161}\u{203a}\u{153}\u{9d}\u{17e}\u{178}\u{a0}\u{a1}\u{a2}\u{a3}\u{a4}\u{a5}\u{a6}\u{a7}\u{a8}\u{a9}\u{aa}\u{ab}\u{ac}\u{ad}\u{ae}\u{af}\u{b0}\u{b1}\u{b2}\u{b3}\u{b4}\u{b5}\u{b6}\u{b7}\u{b8}\u{b9}\u{ba}\u{bb}\u{bc}\u{bd}\u{be}\u{bf}\u{c0}\u{c1}\u{c2}\u{c3}\u{c4}\u{c5}\u{c6}\u{c7}\u{c8}\u{c9}\u{ca}\u{cb}\u{cc}\u{cd}\u{ce}\u{cf}\u{d0}\u{d1}\u{d2}\u{d3}\u{d4}\u{d5}\u{d6}\u{d7}\u{d8}\u{d9}\u{da}\u{db}\u{dc}\u{dd}\u{de}\u{df}\u{e0}\u{e1}\u{e2}\u{e3}\u{e4}\u{e5}\u{e6}\u{e7}\u{e8}\u{e9}\u{ea}\u{eb}\u{ec}\u{ed}\u{ee}\u{ef}\u{f0}\u{f1}\u{f2}\u{f3}\u{f4}\u{f5}\u{f6}\u{f7}\u{f8}\u{f9}\u{fa}\u{fb}\u{fc}\u{fd}\u{fe}\u{ff}";
const ISO885915_UPPER: &str = "\u{80}\u{81}\u{82}\u{83}\u{84}\u{85}\u{86}\u{87}\u{88}\u{89}\u{8a}\u{8b}\u{8c}\u{8d}\u{8e}\u{8f}\u{90}\u{91}\u{92}\u{93}\u{94}\u{95}\u{96}\u{97}\u{98}\u{99}\u{9a}\u{9b}\u{9c}\u{9d}\u{9e}\u{9f}\u{a0}\u{a1}\u{a2}\u{a3}\u{20ac}\u{a5}\u{160}\u{a7}\u{161}\u{a9}\u{aa}\u{ab}\u{ac}\u{ad}\u{ae}\u{af}\u{b0}\u{b1}\u{b2}\u{b3}\u{17d}\u{b5}\u{b6}\u{b7}\u{17e}\u{b9}\u{ba}\u{bb}\u{152}\u{153}\u{178}\u{bf}\u{c0}\u{c1}\u{c2}\u{c3}\u{c4}\u{c5}\u{c6}\u{c7}\u{c8}\u{c9}\u{ca}\u{cb}\u{cc}\u{cd}\u{ce}\u{cf}\u{d0}\u{d1}\u{d2}\u{d3}\u{d4}\u{d5}\u{d6}\u{d7}\u{d8}\u{d9}\u{da}\u{db}\u{dc}\u{dd}\u{de}\u{df}\u{e0}\u{e1}\u{e2}\u{e3}\u{e4}\u{e5}\u{e6}\u{e7}\u{e8}\u{e9}\u{ea}\u{eb}\u{ec}\u{ed}\u{ee}\u{ef}\u{f0}\u{f1}\u{f2}\u{f3}\u{f4}\u{f5}\u{f6}\u{f7}\u{f8}\u{f9}\u{fa}\u{fb}\u{fc}\u{fd}\u{fe}\u{ff}";

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


/// The first place a file the reader decodes whole is not UTF-8: a
/// signed file and a file naming a coding word have every byte, a
/// comment's and a literal's among them, read by the codec, so the byte
/// that first breaks UTF-8 is where the reader stops.
fn utf8_fault_piece(bytes: &[u8]) -> Option<(usize, usize)> {
    std::str::from_utf8(bytes).err().map(|bad| (bad.valid_up_to(), bytes.len()))
}

/// The reference's report for a file it will not read: the file and the
/// line it stopped on, the line itself where one is shown, a mark under
/// the place the reference marks, and the complaint. The run ends here,
/// before a program begins.
fn source_fault(given: &str, row: usize, line: Option<&str>, caret: Option<usize>, message: &str) -> ! {
    eprintln!("  File \"{}\", line {}", given, row);
    if let Some(text) = line {
        eprintln!("    {}", text);
        if let Some(column) = caret {
            eprintln!("    {}^", " ".repeat(column));
        }
    }
    eprintln!("SyntaxError: {}", message);
    process::exit(1)
}

/// A Python file read as UTF-8, as PEP 263 has it. With neither a
/// coding word nor a signature, every line is looked at raw, and a
/// byte that is not UTF-8 is named on its own, with the file and the
/// line it stands on. With either, the whole file is decoded, and
/// the codec's complaint is told with the file, the line and a mark
/// under the place, as the reference tells it.
fn utf8_file_source(written: &[u8], bytes: &[u8], signed: bool, declared: bool, strict_utf8: bool, given: &str) -> String {
    if let Ok(text) = std::str::from_utf8(bytes) {
        return text.to_owned();
    }
    if !signed && !declared {
        let error = std::str::from_utf8(bytes).unwrap_err();
        let upto = error.valid_up_to();
        let row = bytes[..upto].iter().filter(|byte| **byte == b'\n').count() + 1;
        let line_start = bytes[..upto].iter().rposition(|byte| *byte == b'\n').map_or(0, |p| p + 1);
        let lossy = String::from_utf8_lossy(&bytes[line_start..]);
        let line = lossy.split('\n').next().unwrap_or("");
        let column = String::from_utf8_lossy(&bytes[line_start..upto]).chars().count();
        source_fault(given, row, Some(line), Some(column), &format!("Non-UTF-8 code starting with '\\x{:02x}' in file {} on line {}, but no encoding declared; see https://peps.python.org/pep-0263/ for details", bytes[upto], given, row));
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
    // A name the reference folds to plain UTF-8 is read by the reader's
    // own validator, which names the line and the place; any other
    // spelling is read by the codec, whose complaint stands alone.
    if !strict_utf8 {
        source_fault(given, 0, None, None, &format!("{}: {}", complaint, why));
    }
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

/// Decode source with the same CPython-derived legacy mapping data as codecs.
fn legacy_source(bytes: &[u8], encoding: &str, given: &str) -> String {
    static TABLES: std::sync::OnceLock<serde_json::Value> = std::sync::OnceLock::new();
    let tables = TABLES.get_or_init(|| serde_json::from_str(include_str!("../langs/lib_python/modules/_legacy_charmaps.json")).expect("valid codec mapping data"));
    let rows = tables[encoding].as_array().expect("a registered mapping family");
    let first = if encoding == "gbk" { 0x81 } else { 0xa1 };
    let mut text = String::new();
    let mut at = 0;
    while at < bytes.len() {
        let lead = bytes[at];
        if lead < 128 { text.push(char::from(lead)); at += 1; continue; }
        let mapped = if lead >= first && lead <= 0xfe && at + 1 < bytes.len() {
            let trail = bytes[at + 1];
            let column = if encoding == "gbk" && (0x40..=0xfe).contains(&trail) && trail != 0x7f {
                Some(usize::from(trail - 0x40 - u8::from(trail > 0x7f)))
            } else if encoding == "gb2312" && (0xa1..=0xfe).contains(&trail) { Some(usize::from(trail - 0xa1)) } else { None };
            column.and_then(|column| rows[usize::from(lead - first)].as_str()?.chars().nth(column)).filter(|character| *character != '\0')
        } else { None };
        if let Some(character) = mapped { text.push(character); at += 2; }
        else {
            eprintln!("SyntaxError: encoding problem: {encoding} ({given})");
            process::exit(1);
        }
    }
    text
}
/// The reference's byte-to-byte codecs, known but not text encodings: a
/// coding word naming one is refused as the reference refuses it.
const NON_TEXT_CODECS: [&str; 7] = ["base64codec", "bz2codec", "hexcodec", "quopricodec", "rot13", "uucodec", "zlibcodec"];

/// Read a Python file according to PEP 263: a UTF-8 signature is
/// discarded before it reaches the lexer, and a coding word in a
/// comment among the first two lines names how the rest decodes. The
/// same words anywhere else, a string's contents among them, name
/// nothing. A file naming an encoding its bytes do not keep to, one
/// conflicting with the signature, or one the interpreter does not
/// know, is told the way the reference tells it, and the run ends
/// before it begins.
/// Where a byte stands in its line, one past the line's start, as the
/// reference counts it when it decodes a file line by line.
fn fault_position(bytes: &[u8], at: usize) -> usize {
    let start = bytes[..at].iter().rposition(|byte| *byte == b'\n').map_or(0, |place| place + 1);
    at - start + 1
}

fn python_file_source(written: Vec<u8>, given: &str) -> String {
    let signed = written.starts_with(&[0xef, 0xbb, 0xbf]);
    let bytes = written.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&written);
    // A nought in the file is refused before any byte is read as text:
    // the reference names the nought itself, not a decoding complaint
    // that a later invalid byte would otherwise raise first.
    if let Some(at) = bytes.iter().position(|byte| *byte == 0) {
        let row = bytes[..at].iter().filter(|byte| **byte == b'\n').count() + 1;
        let start = bytes[..at].iter().rposition(|byte| *byte == b'\n').map_or(0, |place| place + 1);
        let line = String::from_utf8_lossy(&bytes[start..at]).into_owned();
        source_fault(given, row, Some(&line), None, "source code cannot contain null bytes");
    }
    let mut cookie: Option<String> = None;
    let mut cookie_row = 1usize;
    for (at, line) in bytes.split(|byte| *byte == b'\n').take(2).enumerate() {
        cookie_row = at + 1;
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
    let shown = String::from_utf8_lossy(bytes);
    let row_line = |row: usize| shown.split('\n').nth(row - 1).unwrap_or("").to_string();
    if signed && normal != "utf-8" {
        let line = row_line(cookie_row);
        source_fault(given, cookie_row, Some(&line), None, &format!("encoding problem: {} with BOM", normal));
    }
    let stripped = normal.to_ascii_lowercase().replace(['-', '_'], "");
    match stripped.as_str() {
        "utf8" | "utf8sig" => utf8_file_source(&written, bytes, signed, cookie.is_some(), normal == "utf-8", given),
        "ascii" | "usascii" => {
            if let Some(at) = bytes.iter().position(|byte| *byte > 127) {
                let row = bytes[..at].iter().filter(|byte| **byte == b'\n').count() + 1;
                let line = row_line(row);
                let message = format!("(unicode error) 'ascii' codec can't decode byte 0x{:02x} in position {}: ordinal not in range(128)", bytes[at], at);
                source_fault(given, row, Some(&line), None, &message);
            }
            bytes.iter().copied().map(char::from).collect()
        }
        "latin1" | "latin" | "iso88591" | "isolatin1" => bytes.iter().copied().map(char::from).collect(),
        "gbk" | "cp936" | "ms936" | "936" => legacy_source(bytes, "gbk", given),
        "gb2312" | "gb231280" | "euccn" | "chinese" | "csiso58gb231280" => legacy_source(bytes, "gb2312", given),
        "latin9" | "iso885915" => decode_with_upper(bytes, ISO885915_UPPER),
        "cp437" | "437" | "ibm437" => decode_with_upper(bytes, CP437_UPPER),
        "cp1251" => decode_with_upper(bytes, CP1251_UPPER),
        "iso88597" => decode_with_upper(bytes, ISO88597_UPPER),
        "koi8r" => decode_with_upper(bytes, KOI8R_UPPER),
        "cp1252" | "1252" | "windows1252" => {
            if let Some(at) = bytes.iter().position(|byte| matches!(*byte, 0x81 | 0x8D | 0x8F | 0x90 | 0x9D)) {
                source_fault(given, 0, None, None, &format!("'charmap' codec can't decode byte 0x{:02x} in position {}: character maps to <undefined>", bytes[at], fault_position(bytes, at)));
            }
            decode_with_upper(bytes, CP1252_UPPER)
        }
        "cp949" => match decode_cp949(bytes) {
            Ok(text) => text,
            Err(fault) => source_fault(given, 0, None, None, &format!("'cp949' codec can't decode byte 0x{:02x} in position {}: {}", fault.byte, fault_position(bytes, fault.at), fault.reason)),
        },
        "cp932" => match decode_cp932(bytes) {
            Ok(text) => text,
            Err(fault) => source_fault(given, 0, None, None, &format!("'cp932' codec can't decode byte 0x{:02x} in position {}: {}", fault.byte, fault_position(bytes, fault.at), fault.reason)),
        },
        "shiftjis" | "sjis" => match decode_shift_jis(bytes) {
            Ok(text) => text,
            Err(fault) => source_fault(given, 0, None, None, &format!("'shift_jis' codec can't decode byte 0x{:02x} in position {}: {}", fault.byte, fault_position(bytes, fault.at), fault.reason)),
        },
        _ if NON_TEXT_CODECS.contains(&stripped.as_str()) => source_fault(given, 0, None, None, &format!("'{}' is not a text encoding", named)),
        _ => source_fault(given, 0, None, None, &format!("unknown encoding: {}", named)),
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

/// Native cache markers written by the two full Python kernels.
const PYC_MAGICS: [[u8; 4]; 2] = [
    lumen_stack8::PYTHON_CACHE_MAGIC,
    lumen_microcode7::PYTHON_CACHE_MAGIC,
];

/// Whether bytes open with the bytecode marker, and so are a compiled
/// module rather than source.
fn is_pyc(bytes: &[u8]) -> bool {
    bytes.len() >= 4 && PYC_MAGICS.iter().any(|magic| bytes[..4] == *magic)
}

/// A Python double-quoted literal for a path handed to a bootstrap.
fn python_string_literal(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\x{:02x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Source that runs a compiled module: it reads the pyc back through
/// the very marshal that wrote it and runs the code object it holds,
/// the way the reference runs a pyc by marshalling its code in again.
fn compiled_source(pyc_path: &str) -> String {
    format!(
        "import marshal as _marshal\n_lumen_pyc_code = _marshal.loads(open({}, 'rb').read()[16:])\nexec(_lumen_pyc_code)\n",
        python_string_literal(pyc_path)
    )
}

/// A Python bytes literal for a compiled module's marshalled code, read
/// out of a ZIP where there is no file of its own to open.
fn python_bytes_literal(bytes: &[u8]) -> String {
    let mut out = String::from("b\"");
    for &b in bytes {
        match b {
            b'\\' => out.push_str("\\\\"),
            b'"' => out.push_str("\\\""),
            b'\n' => out.push_str("\\n"),
            b'\r' => out.push_str("\\r"),
            b'\t' => out.push_str("\\t"),
            0x20..=0x7e => out.push(b as char),
            _ => out.push_str(&format!("\\x{:02x}", b)),
        }
    }
    out.push('"');
    out
}

/// Source that runs a compiled module whose marshalled code is carried
/// here as bytes (from a ZIP), reading it back through the very marshal
/// that wrote it.
fn compiled_bytes_source(marshalled: &[u8]) -> String {
    format!(
        "import marshal as _marshal\n_lumen_pyc_code = _marshal.loads({})\nexec(_lumen_pyc_code)\n",
        python_bytes_literal(marshalled)
    )
}

/// A little-endian u16 at `at`, or nothing where the bytes run out.
fn le16(bytes: &[u8], at: usize) -> Option<u16> {
    let slice = bytes.get(at..at + 2)?;
    Some(u16::from_le_bytes([slice[0], slice[1]]))
}

/// A little-endian u32 at `at`, or nothing where the bytes run out.
fn le32(bytes: &[u8], at: usize) -> Option<u32> {
    let slice = bytes.get(at..at + 4)?;
    Some(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

/// The ZIP file a program path stands for, and the directory within it:
/// the whole path when it names a ZIP file itself, else the ZIP an
/// ancestor names and the remainder as the directory carried inside it.
fn zip_entry(path: &str) -> Option<(String, String)> {
    let p = Path::new(path);
    if p.is_file() && p.extension().and_then(|e| e.to_str()) == Some("zip") {
        return Some((path.to_string(), String::new()));
    }
    let mut rest = p;
    let mut tail: Vec<String> = Vec::new();
    while let Some(parent) = rest.parent() {
        let name = rest.file_name().and_then(|n| n.to_str())?.to_string();
        tail.push(name);
        if parent.is_file() && parent.extension().and_then(|e| e.to_str()) == Some("zip") {
            tail.reverse();
            return Some((parent.to_string_lossy().into_owned(), tail.join("/")));
        }
        rest = parent;
    }
    None
}

/// A file's stored (uncompressed) bytes read out of a ZIP, or nothing
/// where the ZIP or the entry is not there. Only the stored form the
/// host's own writer lays down is read back.
fn zip_read(zip_path: &str, name: &str) -> Option<Vec<u8>> {
    let bytes = fs::read(zip_path).ok()?;
    let eocd = bytes.windows(4).rposition(|w| w == b"PK\x05\x06")?;
    let entries = le16(&bytes, eocd + 10)? as usize;
    let mut at = le32(&bytes, eocd + 16)? as usize;
    for _ in 0..entries {
        if bytes.get(at..at + 4)? != b"PK\x01\x02" { return None; }
        let method = le16(&bytes, at + 10)?;
        let name_len = le16(&bytes, at + 28)? as usize;
        let extra_len = le16(&bytes, at + 30)? as usize;
        let comment_len = le16(&bytes, at + 32)? as usize;
        let local = le32(&bytes, at + 42)? as usize;
        let entry_name = std::str::from_utf8(bytes.get(at + 46..at + 46 + name_len)?).ok()?;
        if entry_name == name {
            if method != 0 { return None; }
            let lname_len = le16(&bytes, local + 26)? as usize;
            let lextra_len = le16(&bytes, local + 28)? as usize;
            let start = local + 30 + lname_len + lextra_len;
            let size = le32(&bytes, local + 18)? as usize;
            return bytes.get(start..start + size).map(|d| d.to_vec());
        }
        at += 46 + name_len + extra_len + comment_len;
    }
    None
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
        "#!/bin/sh\nexport LUMEN_LAUNCHER=\"$0\"\nexec '{}' --library-path '{}' '{}' --kernel '{}' --lang '{}' \"$@\"\n",
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
    // The waiting host thread must not take signals meant for the interpreter.
    let previous_mask = unsafe {
        let mut blocked = std::mem::zeroed();
        let mut previous = std::mem::zeroed();
        libc::sigemptyset(&mut blocked);
        libc::sigaddset(&mut blocked, libc::SIGINT);
        let error = libc::pthread_sigmask(libc::SIG_BLOCK, &blocked, &mut previous);
        if error != 0 { panic!("Failed to set signal mask: {}", std::io::Error::from_raw_os_error(error)); }
        previous
    };
    let (python_run, learn) = std::sync::mpsc::channel::<()>();
    let roomy = std::thread::Builder::new()
        .stack_size(1 << 30)
        .spawn(move || {
            let error = unsafe { libc::pthread_sigmask(libc::SIG_SETMASK, &previous_mask, std::ptr::null_mut()) };
            if error != 0 { panic!("Failed to restore signal mask: {}", std::io::Error::from_raw_os_error(error)); }
            run_all(python_run);
        })
        .expect("a thread for the run");
    // A Python run only: the waiting thread takes no signals, so one sent to
    // the process lands on the running thread and interrupts its blocking
    // calls (waitpid, select) instead of sitting unseen while they carry on.
    // (The run ending without a word drops the sender and ends the wait.)
    #[cfg(target_os = "linux")]
    if learn.recv().is_ok() {
        unsafe {
            let mut every = std::mem::zeroed();
            libc::sigfillset(&mut every);
            libc::pthread_sigmask(libc::SIG_BLOCK, &every, std::ptr::null_mut());
        }
    }
    match roomy.join() {
        Ok(()) => {}
        Err(panicked) => std::panic::resume_unwind(panicked),
    }
}

fn run_all(python_run: std::sync::mpsc::Sender<()>) {
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
        let _ = python_run.send(());
        if inv.python_development { std::env::set_var("LUMEN_PYTHON_DEV_MODE", "1"); }
        else { std::env::remove_var("LUMEN_PYTHON_DEV_MODE"); }
        std::env::set_var("LUMEN_PYTHON_SAFE_PATH", if inv.python_safe_path { "1" } else { "0" });
        if inv.python_ignore_environment { std::env::set_var("LUMEN_PYTHON_IGNORE_ENV", "1"); }
        else { std::env::remove_var("LUMEN_PYTHON_IGNORE_ENV"); }
        if inv.python_isolated { std::env::set_var("LUMEN_PYTHON_ISOLATED", "1"); }
        else { std::env::remove_var("LUMEN_PYTHON_ISOLATED"); }
    }
    // Only an actual -m invocation carries module execution metadata;
    // a script child must not inherit its parent's module identity.
    if let Some(name) = &inv.module_name { std::env::set_var("LUMEN_RUN_MODULE", name); }
    else { std::env::remove_var("LUMEN_RUN_MODULE"); }
    if let Some(version) = inv.python { std::env::set_var("LUMEN_PYTHON", version.release); }

    // Carry Python startup warning options without confusing script arguments
    // or splitting a message that itself contains a separator character.
    if inv.language.name() == "python" {
        let mut options = Vec::new();
        if let Some(environment) = env::var("PYTHONWARNINGS").ok().filter(|_| !inv.python_ignore_environment) {
            options.extend(environment.split(',').filter(|part| !part.is_empty()).map(str::to_owned));
        }
        options.extend(inv.warning_options.iter().cloned());
        let mut ordered = Vec::new();
        for option in options {
            ordered.retain(|previous| previous != &option);
            ordered.push(option);
        }
        let encoded = ordered.iter().map(|option| format!("{}:{option}", option.chars().count())).collect::<String>();
        env::set_var("LUMEN_WARNOPTIONS", encoded);
    } else {
        if !inv.warning_options.is_empty() {
            eprintln!("Error: -W requires the Python language");
            process::exit(2);
        }
        env::remove_var("LUMEN_WARNOPTIONS");
    }

    let written = inv.module_source.as_ref().map(|source| source.as_bytes().to_vec()).unwrap_or_else(|| (if inv.file == "-" || inv.file == "<stdin>" {
        let mut input = Vec::new();
        std::io::stdin().read_to_end(&mut input).map(|_| input)
    } else { fs::read(&inv.file) }).unwrap_or_else(|e| {
        eprintln!("Error: Failed to read {}: {}", inv.file, e);
        process::exit(1);
    }));
    let source = if inv.language.name() == "python" && honours_extensions(&inv.kernel) {
        // A compiled module handed as the program is not read as
        // source; its bytecode is marshalled back in and run, the way
        // the reference runs a pyc. Only a file handed straight to the
        // host is looked at here: a -m module or a directory already
        // named its own source while the arguments were read.
        if inv.module_source.is_none() && is_pyc(&written) {
            compiled_source(&inv.file)
        } else {
            python_file_source(written, &inv.file)
        }
    } else {
        source_of(written, text_is_bytes(&inv.language) && honours_extensions(&inv.kernel))
    };
    let source = without_shebang(source);
    let source = if inv.python_interactive {
        let initial = serde_json::to_string(&source).expect("interactive initial source");
        let filename = serde_json::to_string(&inv.file).expect("interactive source filename");
        format!("{}\n_lumen_interact({initial}, {filename}, globals(), {}, {})\n", include_str!("python_interactive.py"), if inv.python_quiet { "True" } else { "False" }, serde_json::to_string(inv.python.map_or("3.14.8", |version| version.release)).expect("interactive version"))
    } else { source };
    if inv.language.name() == "python" && inv.file == "<string>" { env::set_var("LUMEN_COMMAND_SOURCE", &source); }
    else { env::remove_var("LUMEN_COMMAND_SOURCE"); }

    // Every program runs on top of its language's library.
    let given_lines = source.lines().count();
    let command_source = (inv.language.name() == "python" && inv.file == "<string>" && inv.module_source.is_some()).then(|| source.clone());
    // Initialize the language library before registering a command's code.
    // Command strings have no filesystem source and share the interactive cache.
    let source = if let Some(text) = command_source {
        let literal = serde_json::to_string(&text).expect("a source string can be quoted");
        let registered = format!("__import__('linecache')._register_code(compile({literal}, '<string>', 'exec'), {literal}, '<string>')\n{source}");
        with_library(&inv.kernel, inv.language.name(), registered)
    } else { with_library(&inv.kernel, inv.language.name(), source) };
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
    let overlay = inv.python.map_or(&[][..], |v| v.modules);
    for (name, source, file) in overlay.iter().chain(embedded_modules::MODULES.iter().filter(|(name, ..)| !overlay.iter().any(|(overlaid, ..)| overlaid == name))) {
        let source = inv.python.map_or_else(|| source.to_string(), |v| v.module_source(name, source));
        request.push(("MODULE".to_string(), name.to_string(), source, false));
        request.push(("MODULE_FILE".to_string(), name.to_string(), file.to_string(), false));
    }
    if inv.language.name() == "python" && FULL_KERNELS.contains(&inv.kernel.as_str()) {
        let root = python_library_sources(&request).unwrap_or_else(|error| {
            eprintln!("Error: cannot provide Python library sources: {error}");
            process::exit(1);
        });
        request.push(("SELF".to_string(), "library_root".to_string(), root.to_string_lossy().into_owned(), false));
    }
    let aliases = inv.python.map_or(&[][..], |v| v.aliases);
    for (name, member) in aliases.iter().chain(embedded_modules::MODULE_ALIASES.iter().filter(|(name, ..)| !aliases.iter().any(|(overlaid, ..)| overlaid == name))) {
        request.push(("MODULE_ALIAS".to_string(), name.to_string(), member.to_string(), false));
    }
    // Where the program itself lies. A definition may give names to
    // these, and only the full kernels read those labels.
    let whole = if inv.file == "-" { std::path::PathBuf::from("<stdin>") } else {
        std::fs::canonicalize(&inv.file).unwrap_or_else(|_| std::path::PathBuf::from(&inv.file))
    };
    let held = whole.parent().map_or_else(|| ".".to_string(), |p| p.to_string_lossy().into_owned());
    request.push(("SELF".to_string(), "file".to_string(), whole.to_string_lossy().into_owned(), false));
    request.push(("SELF".to_string(), "directory".to_string(), held, false));
    // The path as the run was started with it, links and all: a
    // complaint about reading the program names it the way its reader
    // named it, which is how the reference names it too.
    request.push(("SELF".to_string(), "given".to_string(), if inv.file == "<string>" { "-c".to_owned() } else { inv.file.clone() }, false));
    // The command that named the program, where it is not the source
    // file itself; a program reads it as its own first argument.
    if let Some(name) = &inv.program_name {
        request.push(("SELF".to_string(), "argv0".to_string(), name.clone(), false));
    }
    if inv.file == "-" {
        request.push(("SELF".to_string(), "argv0".to_string(), "-".to_string(), false));
    }
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
                // A child launched through our wrapper reports the same interpreter path.
                std::env::var("LUMEN_LAUNCHER").ok().filter(|path| Path::new(path).is_file())
                    .unwrap_or_else(|| launcher_for(&runner, &binary, &inv.kernel, inv.language.name()))
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

    prepared_libraries::install(&inv.kernel);
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
        let interrupted = message.strip_suffix("\u{1}SIGINT");
        let report = interrupted.unwrap_or(&message);
        if !report.is_empty() { eprintln!("{}", report); }
        if interrupted.is_some() {
            use std::io::Write;
            let _ = std::io::stdout().flush();
            let _ = std::io::stderr().flush();
            // Restore the signal disposition after Python reports an uncaught interrupt.
            unsafe {
                libc::signal(libc::SIGINT, libc::SIG_DFL);
                let mut signals = std::mem::zeroed();
                libc::sigemptyset(&mut signals);
                libc::sigaddset(&mut signals, libc::SIGINT);
                libc::pthread_sigmask(libc::SIG_UNBLOCK, &signals, std::ptr::null_mut());
                libc::raise(libc::SIGINT);
            }
        }
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
    let mut python_bytes_warning = 0usize;
    // Startup transfers belong to one invocation; child processes parse their
    // own flags rather than inheriting these internal option records.
    for option in ["LUMEN_PYTHON_BYTES_WARNING", "LUMEN_PYTHON_WARNOPTIONS", "LUMEN_PYTHON_XOPTIONS", "LUMEN_PYTHON_NO_SITE", "LUMEN_PYTHON_UNBUFFERED"] {
        env::remove_var(option);
    }

    let mut module_name = None;
    let mut python_development = false;
    let mut warning_options = Vec::new();
    let mut tracing_limit = None;
    let mut extended_options = Vec::new();
    let mut python_ignore_environment = false;
    let mut python_isolated = false;
    let mut python_interactive = false;
    let mut python_safe_path = !env::var("LUMEN_LANG").is_ok_and(|held| held == "python");
    let mut explicit_safe_path = false;
    let mut python_quiet = false;
    // Whether the run was asked only to name itself and stop.
    let mut names_itself = false;
    // The name a Python program sees as its own first argument. It is
    // the source name for an ordinary run; `-c` names the code as the
    // source and stands in argv[0] itself.
    let mut program_name = None;

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
            Some(flag) if file.is_none() && (flag == "-W" || flag.starts_with("-W") && flag.len() > 2) => {
                let option = if flag == "-W" {
                    if rest.len() < 2 {
                        eprintln!("Error: -W requires an argument");
                        process::exit(2);
                    }
                    let option = said(&rest[1]);
                    rest = &rest[2..];
                    option
                } else {
                    let option = flag[2..].to_string();
                    rest = &rest[1..];
                    option
                };
                warning_options.push(option);
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
                python_safe_path = explicit_safe_path;
                let (module, used) = if flag == "-m" {
                    if rest.len() < 2 { usage(program); }
                    (said(&rest[1]), 2)
                } else { (flag[2..].to_owned(), 1) };
                if language.as_ref().is_some_and(|definition| definition.name() != "python") {
                    eprintln!("Error: -m requires the Python language");
                    process::exit(1);
                }
                file = Some("-m".to_owned());
                module_source = Some(format!("import runpy\nrunpy._run_module_as_main({})\n", python_string_literal(&module)));
                language.get_or_insert_with(|| Language::Named("python".to_string()));
                rest = &rest[used..];
                break;
            }
            // Python command strings retain their conventional virtual filename.
            Some("-c") if file.is_none() && language.as_ref().is_none_or(|lang| lang.name() == "python") => {
                if rest.len() < 2 { usage(program); }
                module_source = Some(said(&rest[1]));
                python_safe_path = explicit_safe_path;
                file = Some("<string>".to_owned());
                program_name = Some("-c".to_owned());
                language = Some(Language::Named("python".to_owned()));
                rest = &rest[2..];
                break;
            }
            Some("-q") if file.is_none() && language.as_ref().is_none_or(|lang| lang.name() == "python") => {
                python_quiet = true;
                language = Some(Language::Named("python".to_owned()));
                rest = &rest[1..];
            }
            Some("-i") if file.is_none() && language.as_ref().is_none_or(|lang| lang.name() == "python") => {
                python_interactive = true;
                language = Some(Language::Named("python".to_owned()));
                rest = &rest[1..];
            }
            // These Python startup switches describe services already isolated
            // from site imports and configure byte diagnostics before execution.
            Some(flag) if file.is_none() && (flag == "-S" || flag == "-u"
                || flag.len() > 1 && flag.starts_with('-') && flag[1..].chars().all(|letter| letter == 'b'))
                && language.as_ref().is_none_or(|lang| lang.name() == "python") => {
                if flag == "-S" { env::set_var("LUMEN_PYTHON_NO_SITE", "1"); }
                else if flag == "-u" { env::set_var("LUMEN_PYTHON_UNBUFFERED", "1"); }
                else {
                    let previous = python_bytes_warning;
                    python_bytes_warning = previous + flag.len() - 1;
                    env::set_var("LUMEN_PYTHON_BYTES_WARNING", python_bytes_warning.to_string());
                }
                language = Some(Language::Named("python".to_owned()));
                rest = &rest[1..];
            }
            // Preserve Python warning options in command-line order.
            Some(flag) if file.is_none() && (flag == "-W" || flag.starts_with("-W"))
                && language.as_ref().is_none_or(|lang| lang.name() == "python") => {
                let (option, used) = if flag == "-W" {
                    if rest.len() < 2 { usage(program); }
                    (said(&rest[1]), 2)
                } else { (flag[2..].to_owned(), 1) };
                let mut options: Vec<String> = env::var("LUMEN_PYTHON_WARNOPTIONS").ok()
                    .and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default();
                options.push(option);
                env::set_var("LUMEN_PYTHON_WARNOPTIONS", serde_json::to_string(&options).unwrap());
                language = Some(Language::Named("python".to_owned()));
                rest = &rest[used..];
            }
            // Apply Python's debug-range switch before compiling any code.
            Some(flag) if file.is_none() && (flag == "-X" || flag.starts_with("-X"))
                && language.as_ref().is_none_or(|lang| lang.name() == "python") => {
                let (option, used) = if flag == "-X" {
                    if rest.len() < 2 { usage(program); }
                    (said(&rest[1]), 2)
                } else { (flag[2..].to_owned(), 1) };
                extended_options.push(option.clone());
                if option == "tracemalloc" { tracing_limit = Some(1); }
                else if let Some(limit) = option.strip_prefix("tracemalloc=") {
                    tracing_limit = Some(limit.parse::<usize>().ok().filter(|n| *n > 0 && *n <= 65535).unwrap_or_else(|| {
                        eprintln!("invalid number of frames for tracemalloc"); process::exit(1);
                    }));
                }
                let (key, value) = option.split_once('=').unwrap_or((&option, "True"));
                let mut options: std::collections::BTreeMap<String, String> = env::var("LUMEN_PYTHON_XOPTIONS").ok()
                    .and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default();
                options.insert(key.to_owned(), value.to_owned());
                env::set_var("LUMEN_PYTHON_XOPTIONS", serde_json::to_string(&options).unwrap());
                if key == "no_debug_ranges" { env::set_var("PYTHONNODEBUGRANGES", "1"); }
                if key == "dev" { python_development = true; }
                language = Some(Language::Named("python".to_owned()));
                rest = &rest[used..];
            }
            Some("-P") if file.is_none() => {
                explicit_safe_path = true;
                python_safe_path = true;
                language.get_or_insert_with(|| Language::Named("python".to_owned()));
                rest = &rest[1..];
            }
            Some("-I") if file.is_none() => {
                python_isolated = true;
                explicit_safe_path = true;
                python_safe_path = true;
                python_ignore_environment = true;
                language = Some(Language::Named("python".to_string()));
                rest = &rest[1..];
            }
            Some("-E") if file.is_none() => {
                python_safe_path = explicit_safe_path;
                python_ignore_environment = true;
                language = Some(Language::Named("python".to_string()));
                rest = &rest[1..];
            }
            Some("-") if file.is_none() => {
                python_safe_path = explicit_safe_path;
                file = Some("-".to_string());
                language = Some(Language::Named("python".to_string()));
                rest = &rest[1..];
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

    if let Ok(setting) = env::var("LUMEN_PYTHON_BYTES_WARNING") {
        let level = setting.parse::<usize>().unwrap_or(0);
        if level != 0 {
            let mut options: Vec<String> = env::var("LUMEN_PYTHON_WARNOPTIONS").ok()
                .and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default();
            options.push(format!("{}::BytesWarning", if level > 1 { "error" } else { "default" }));
            env::set_var("LUMEN_PYTHON_WARNOPTIONS", serde_json::to_string(&options).unwrap());
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

    if let Some(limit) = tracing_limit { env::set_var("LUMEN_TRACE_FRAMES", limit.to_string()); }
    else { env::remove_var("LUMEN_TRACE_FRAMES"); }
    let language = language.unwrap_or_else(|| {
        // Explicit language wins over the environment. With neither, infer
        // a supplied file's language or start Python for no-file input.
        match env::var("LUMEN_LANG") {
            Ok(held) if !held.trim().is_empty() => language_from_flag(&held),
            _ => Language::Named(file.as_ref().map_or_else(|| "python".to_owned(), |file| {
                language_from_extension(file).unwrap_or_else(|| DEFAULT_LANGUAGE.to_string())
            })),
        }
    });
    python_interactive &= language.name() == "python";
    // Only Python reads no-file input as a script or terminal session.
    // Keep an explicit dash as argv[0], and leave resolved -c/-m source intact.
    if language.name() == "python" && file.as_deref().is_none_or(|name| name == "-") {
        python_interactive |= std::io::IsTerminal::is_terminal(&std::io::stdin());
        if file.is_none() {
            file = Some("<stdin>".to_owned());
            program_name = Some(String::new());
        }
        if python_interactive { module_source = Some(String::new()); }
    }
    let file = file.unwrap_or_else(|| usage(program));
    // A directory handed as the program is run, for Python alone, as the
    // __main__.py inside it, the way the reference runs it; it keeps its
    // own name as the program's, so a script asking for sys.argv[0] finds
    // the directory it was reached through rather than the file that ran,
    // and its __main__ module names itself __main__ the way the
    // reference's does. A directory handed to any other language stays a
    // directory, as it always was.
    if language.name() == "python" && module_source.is_none() && module_name.is_none() && std::path::Path::new(&file).is_dir() {
        let inside = format!("{file}/__main__.py");
        if let Ok(source) = fs::read_to_string(&inside) {
            module_source = Some(source);
            module_name = Some("__main__".to_string());
        } else {
            // A directory whose __main__ was compiled keeps only the
            // pyc; it is run the same way, marshalled back in.
            let inside_pyc = format!("{file}/__main__.pyc");
            if fs::metadata(&inside_pyc).is_ok() {
                module_source = Some(compiled_source(&inside_pyc));
                module_name = Some("__main__".to_string());
            }
        }
    }
    // A ZIP handed as the program runs the __main__.py it carries, and
    // a directory inside a ZIP runs that directory's __main__.py, the
    // way the reference runs either; the module names itself __main__ so
    // argparse reads the path it was reached through.
    if language.name() == "python" && module_source.is_none() && module_name.is_none() {
        if let Some((zip_path, inner)) = zip_entry(&file) {
            let mut target = String::new();
            if !inner.is_empty() { target = format!("{inner}/"); }
            if let Some(source) = zip_read(&zip_path, &format!("{target}__main__.py")) {
                module_source = Some(String::from_utf8_lossy(&source).into_owned());
                module_name = Some("__main__".to_string());
            } else if let Some(pyc) = zip_read(&zip_path, &format!("{target}__main__.pyc")) {
                if pyc.len() >= 16 && is_pyc(&pyc) {
                    module_source = Some(compiled_bytes_source(&pyc[16..]));
                    module_name = Some("__main__".to_string());
                }
            }
        }
    }

    if (python_development || python_ignore_environment) && language.name() != "python" {
        eprintln!("Error: {} requires the Python language", if python_ignore_environment { "-E" } else { "-X dev" });
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
    if language.name() == "python" {
        let encoded = extended_options.iter().map(|option| format!("{}:{option}", option.chars().count())).collect::<String>();
        env::set_var("LUMEN_XOPTIONS", encoded);
    } else { env::remove_var("LUMEN_XOPTIONS"); }
    Invocation { kernel, file, serve, language, emit, python, module_source, module_name, program_name, python_ignore_environment, python_isolated, python_quiet, python_interactive, python_safe_path, python_development, warning_options, program_args: rest.iter().map(said).collect() }
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


// Installed binaries expose the same source that their embedded loader runs.
fn python_library_sources(request: &[(String, String, String, bool)]) -> std::io::Result<std::path::PathBuf> {
    let original = env::var_os("LUMEN_ROOT").map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")))
        .join("langs/lib_python/modules");
    python_sources::library(request, &original)
}
