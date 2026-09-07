// The lab specimen's own host: run a file in the language its extension
// names, a Lumen program on top of the embedded standard library.
use std::collections::HashSet;

mod embedded_files {
    include!("../../../lib_lumen/prelude.rs");
}
const PRELUDE_MANIFEST: &str = include_str!("../../../lib_lumen/prelude.lm");

fn embedded_file(path: &str) -> Option<&'static str> {
    embedded_files::EMBEDDED_FILES.iter().find(|(p, _)| *p == path).map(|(_, c)| *c)
}

fn expand_includes(source: &str, seen: &mut HashSet<String>, out: &mut String) -> Result<(), String> {
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("include ") {
            let path = rest.trim().trim_matches('"').to_string();
            if seen.insert(path.clone()) {
                let contents = embedded_file(&path).ok_or_else(|| format!("File not found in embedded filesystem: {}", path))?;
                expand_includes(contents, seen, out)?;
                out.push('\n');
            }
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let Some(path) = args.get(1) else {
        eprintln!("usage: {} <file> [args...]", args[0]);
        std::process::exit(2);
    };
    let mut source = std::fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("Error: {}", e);
        std::process::exit(2);
    });
    let ext = std::path::Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("");
    let langs = lumen_microlab::languages().unwrap_or_default();
    let Some((name, _)) = langs.iter().find(|(_, exts)| exts.iter().any(|x| x == ext)) else {
        eprintln!("Error: no embedded language for '.{}'", ext);
        std::process::exit(2);
    };
    if name == "lumen" {
        let mut prelude = String::new();
        if let Err(e) = expand_includes(PRELUDE_MANIFEST, &mut HashSet::new(), &mut prelude) {
            eprintln!("Include error: {}", e);
            std::process::exit(1);
        }
        source = format!("{}\n{}", prelude, source);
    }
    if let Err(e) = lumen_microlab::run(name, &source, &args[2..]) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
