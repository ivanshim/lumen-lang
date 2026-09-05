// lumen-lang: command-line host for the two kernels.
//
// Usage: lumen-lang [--kernel stream|microcode] <file> [--lang <language>] [program args...]
//
// The host reads the file, picks the language from `--lang` or the file
// extension, prepends the embedded Lumen standard library for Lumen programs,
// and hands the source to the selected kernel. Nothing here knows how either
// kernel works, and the kernels never see each other.

use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::Path;
use std::process;

const KERNELS: [&str; 2] = ["stream", "microcode"];
const DEFAULT_KERNEL: &str = "microcode";
const DEFAULT_LANGUAGE: &str = "lumen";

/// Build-time packaging of the Lumen standard library (`lib_lumen/*.lm`).
mod embedded_files {
    include!("../lib_lumen/prelude.rs");
}

/// The prelude manifest: a list of `include "path"` lines.
const PRELUDE_MANIFEST: &str = include_str!("../lib_lumen/prelude.lm");

struct Invocation {
    kernel: String,
    file: String,
    language: String,
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
    let source = if inv.language == "lumen" {
        let prelude = expand_includes(PRELUDE_MANIFEST).unwrap_or_else(|e| {
            eprintln!("Include error: {}", e);
            process::exit(1);
        });
        format!("{}\n{}", prelude, source)
    } else {
        source
    };

    let result = match inv.kernel.as_str() {
        "stream" => lumen_stream::run(&inv.language, &source, &inv.program_args),
        "microcode" => lumen_microcode::run(&inv.language, &source, &inv.program_args),
        _ => unreachable!("kernel names are validated in parse_args"),
    };

    if let Err(message) = result {
        eprintln!("{}", message);
        process::exit(1);
    }
}

fn usage(program: &str) -> ! {
    eprintln!("Usage: {} [--kernel stream|microcode] <file> [--lang <language>] [program args...]", program);
    process::exit(1);
}

fn parse_args(args: &[String]) -> Invocation {
    let program = args.first().map(String::as_str).unwrap_or("lumen-lang");
    let mut rest: &[String] = &args[1..];

    let mut kernel = DEFAULT_KERNEL.to_string();
    if rest.first().map(String::as_str) == Some("--kernel") {
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

    let file = match rest.first() {
        Some(f) => f.clone(),
        None => usage(program),
    };
    rest = &rest[1..];

    let mut language = String::new();
    if rest.first().map(String::as_str) == Some("--lang") {
        match rest.get(1) {
            Some(l) => language = l.to_lowercase(),
            None => {
                eprintln!("Error: --lang requires an argument");
                process::exit(1);
            }
        }
        rest = &rest[2..];
    }
    if language.is_empty() {
        language = language_from_extension(&file).unwrap_or(DEFAULT_LANGUAGE).to_string();
    }

    Invocation { kernel, file, language, program_args: rest.to_vec() }
}

fn language_from_extension(file: &str) -> Option<&'static str> {
    match Path::new(file).extension()?.to_str()? {
        "lm" => Some("lumen"),
        "rs" => Some("rust_core"),
        "py" => Some("python_core"),
        _ => None,
    }
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
