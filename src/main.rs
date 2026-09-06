// lumen-lang: command-line host for the three kernels.
//
// Usage: lumen-lang [--kernel stream|microcode|stack|microcode2|microcode3] [--lang <name|definition.json>]
//                   <file> [--lang <name|definition.json>] [program args...]
//
// The host reads the file, picks the language from `--lang` (also spelled
// `--language`), else from the file extension, else Lumen; prepends the
// embedded Lumen standard library for Lumen programs; and hands the source
// to the selected kernel. A `--lang` value takes a language name (`python`),
// one of its extensions (`py`), or the path of a definition file
// (`langs/extras/php.json`), which is read at run time.
// Nothing here knows how any kernel works, and the kernels never see
// each other.

use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::Path;
use std::process;

const KERNELS: [&str; 5] = ["stream", "microcode", "stack", "microcode2", "microcode3"];
const DEFAULT_KERNEL: &str = "microcode";
const DEFAULT_LANGUAGE: &str = "lumen";

/// Build-time packaging of the Lumen standard library (`lib_lumen/*.lm`).
mod embedded_files {
    include!("../lib_lumen/prelude.rs");
}

/// The prelude manifest: a list of `include "path"` lines.
const PRELUDE_MANIFEST: &str = include_str!("../lib_lumen/prelude.lm");

/// Where a language comes from.
enum Language {
    /// An embedded definition, by name.
    Named(String),
    /// A definition read from a file on disk, with the name it declares.
    File { name: String, text: String },
}

impl Language {
    fn name(&self) -> &str {
        match self {
            Language::Named(name) => name,
            Language::File { name, .. } => name,
        }
    }
}

struct Invocation {
    kernel: String,
    file: String,
    language: Language,
    /// A language to write the program in instead of running it (microcode2 only).
    emit: Option<Language>,
    program_args: Vec<String>,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let inv = parse_args(&args);

    let source = fs::read_to_string(&inv.file).unwrap_or_else(|e| {
        eprintln!("Error: Failed to read {}: {}", inv.file, e);
        process::exit(1);
    });

    // Lumen programs run on top of the embedded standard library.
    let source = if inv.language.name() == DEFAULT_LANGUAGE {
        let prelude = expand_includes(PRELUDE_MANIFEST).unwrap_or_else(|e| {
            eprintln!("Include error: {}", e);
            process::exit(1);
        });
        format!("{}\n{}", prelude, source)
    } else {
        source
    };

    if let Some(target) = &inv.emit {
        if inv.kernel != "microcode2" {
            eprintln!("Error: --emit needs --kernel microcode2");
            process::exit(1);
        }
        let text_of = |language: &Language| -> String {
            match language {
                Language::Named(name) => lumen_microcode2::embedded(name).unwrap_or_else(|e| {
                    eprintln!("{}", e);
                    process::exit(1);
                }).to_string(),
                Language::File { text, .. } => text.clone(),
            }
        };
        match lumen_microcode2::emit(&text_of(&inv.language), &source, &text_of(target)) {
            Ok(text) => print!("{}", text),
            Err(message) => {
                eprintln!("{}", message);
                process::exit(1);
            }
        }
        return;
    }

    let result = match (inv.kernel.as_str(), &inv.language) {
        ("stream", Language::Named(name)) => lumen_stream::run(name, &source, &inv.program_args),
        ("stream", Language::File { text, .. }) => lumen_stream::run_definition(text, &source, &inv.program_args),
        ("microcode", Language::Named(name)) => lumen_microcode::run(name, &source, &inv.program_args),
        ("microcode", Language::File { text, .. }) => lumen_microcode::run_definition(text, &source, &inv.program_args),
        ("stack", Language::Named(name)) => lumen_stack::run(name, &source, &inv.program_args),
        ("stack", Language::File { text, .. }) => lumen_stack::run_definition(text, &source, &inv.program_args),
        ("microcode2", Language::Named(name)) => lumen_microcode2::run(name, &source, &inv.program_args),
        ("microcode2", Language::File { text, .. }) => lumen_microcode2::run_definition(text, &source, &inv.program_args),
        ("microcode3", Language::Named(name)) => lumen_microcode3::run(name, &source, &inv.program_args),
        ("microcode3", Language::File { text, .. }) => lumen_microcode3::run_definition(text, &source, &inv.program_args),
        _ => unreachable!("kernel names are validated in parse_args"),
    };

    if let Err(message) = result {
        eprintln!("{}", message);
        process::exit(1);
    }
}

fn usage(program: &str) -> ! {
    eprintln!(
        "Usage: {} [--kernel stream|microcode|stack|microcode2|microcode3] [--lang <name|extension|definition.json>] [--emit <name|extension|definition.json>] <file> [program args...]",
        program
    );
    process::exit(1);
}

/// The embedded languages, each name with its extensions.
fn embedded_languages() -> Vec<(String, Vec<String>)> {
    lumen_microcode::languages().unwrap_or_else(|e| {
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
    let name = lumen_microcode::language_of(&text).unwrap_or_else(|e| {
        eprintln!("Error: language definition {}: {}", path, e);
        process::exit(1);
    });
    Language::File { name, text }
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

fn parse_args(args: &[String]) -> Invocation {
    let program = args.first().map(String::as_str).unwrap_or("lumen-lang");
    let mut rest: &[String] = &args[1..];

    let mut kernel = DEFAULT_KERNEL.to_string();
    let mut language: Option<Language> = None;
    let mut emit: Option<Language> = None;
    let mut file: Option<String> = None;

    // Options may precede the file; `--lang` may also follow it directly.
    loop {
        match rest.first().map(String::as_str) {
            Some("--kernel") if file.is_none() => {
                if rest.len() < 2 {
                    usage(program);
                }
                kernel = rest[1].to_lowercase();
                if !KERNELS.contains(&kernel.as_str()) {
                    eprintln!("Error: Unknown kernel '{}'. Use one of: {}", kernel, KERNELS.join(", "));
                    process::exit(1);
                }
                rest = &rest[2..];
            }
            Some("--lang") | Some("--language") => {
                if rest.len() < 2 {
                    eprintln!("Error: {} requires an argument", rest[0]);
                    process::exit(1);
                }
                language = Some(language_from_flag(&rest[1]));
                rest = &rest[2..];
            }
            Some("--emit") => {
                if rest.len() < 2 {
                    eprintln!("Error: --emit requires an argument");
                    process::exit(1);
                }
                emit = Some(language_from_flag(&rest[1]));
                rest = &rest[2..];
            }
            Some(_) if file.is_none() => {
                file = Some(rest[0].clone());
                rest = &rest[1..];
            }
            _ => break,
        }
    }

    let file = file.unwrap_or_else(|| usage(program));
    let language = language.unwrap_or_else(|| {
        Language::Named(language_from_extension(&file).unwrap_or_else(|| DEFAULT_LANGUAGE.to_string()))
    });

    Invocation { kernel, file, language, emit, program_args: rest.to_vec() }
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
