// Microcode kernel, fourth design: seven forms, the four of microcode4
// and three the kernel lab measured worth a form of their own.
//
// Const, Read, Write, Apply: a constant, a binding read, a binding
// written, a call of a primitive or of a routine value. Cycle: a loop
// run in the frame it appears in. Dyad: an operator on two operands read
// without a visit to a form. Bump: a binding stepped in place. Branching,
// blocks and the three exits are still calls: of `choose` with routine
// values for arms, of a routine run at once, of `yield`, `leave` and
// `resume`. This crate never imports the other kernels.

pub mod math;
pub mod indent;
pub mod scan;
pub mod build;
pub mod exec;
pub mod table;
pub mod form;
pub mod data;

use std::collections::HashMap;

use table::Table;
use data::Value;

const EMBEDDED: &[&str] = &[
    include_str!("../../../langs/lumen.json"),
    include_str!("../../../langs/rplumen.json"),
    include_str!("../../../langs/python.json"),
    include_str!("../../../langs/rust.json"),
];

pub fn languages() -> Result<Vec<(String, Vec<String>)>, String> {
    EMBEDDED.iter().map(|t| table::identify(t)).collect()
}

pub fn language_of(definition: &str) -> Result<String, String> {
    table::identify(definition).map(|(n, _)| n)
}

pub fn run(language: &str, source: &str, program_args: &[String], request: &[(String, String, String, bool)]) -> Result<(), String> {
    for text in EMBEDDED {
        if table::identify(text)?.0 == language {
            return run_definition(text, source, program_args, request);
        }
    }
    Err(format!("Error: Unknown language '{}'", language))
}

pub fn run_definition(definition: &str, source: &str, program_args: &[String], request: &[(String, String, String, bool)]) -> Result<(), String> {
    let table = Table::parse(definition).map_err(|e| format!("Error: language definition: {e}"))?;
    let prefix = table.banner();
    go(&table, source, program_args, request).map_err(|e| format!("{}: {}", prefix, e))
}

/// Which group of the request each label names, and the one that holds
/// what the query, the form and the cookies carry together.
/// What a program calls the file it is written in and the place that
/// file lies in, where it has words for them.
const OWN_PLACE: [(&str, &str); 3] =
    [("file", "ext.system.source.file"), ("directory", "ext.system.source.directory"), ("runner", "ext.system.runner")];

/// What the host may find amiss in a request before the program runs.
/// The host names only which of them it found; the words for each are
/// the language's own.
const REQUEST_AMISS: [(&str, &str); 4] = [
    ("boundary", "ext.system.request.amiss.boundary"),
    ("boundary.wrong", "ext.system.request.amiss.boundary.wrong"),
    ("part", "ext.system.request.amiss.part"),
    ("body.large", "ext.system.request.amiss.body.large"),
];

const REQUEST_PARTS: [(&str, &str); 8] = [
    ("GET", "ext.system.request.query"), ("POST", "ext.system.request.form"), ("COOKIE", "ext.system.request.cookies"),
    ("SERVER", "ext.system.request.server"), ("ENV", "ext.system.request.env"), ("FILES", "ext.system.request.files"),
    ("ALL", "ext.system.request.all"), ("SETTINGS", "ext.system.request.settings"),
];

/// A program that could not be read, told the way this language tells a
/// complaint: written where the run would have written, naming the file
/// and the row the reading stopped on. Where the language has no word
/// for such a stopping, nothing is written and the fault goes back as
/// it came, for the host to tell in its own way.
fn cannot_read(table: &Table, said: &str, row: u32, request: &[(String, String, String, bool)], before: u32, fatally: bool) -> String {
    let key = match fatally {
        true => "ext.system.complaint.fatal",
        false => "ext.system.complaint.reading",
    };
    let Some(word) = table.single(key) else { return said.to_string() };
    let named = |key: &str| request.iter().find(|(from, k, ..)| from == "SELF" && k == key).map(|(.., v, _)| v.clone());
    let file = named("file").unwrap_or_default();
    print!("\n{}: {} in {} on line {}\n", word, said, file, row.saturating_sub(before));
    // The run ends straight after this, and ending does not empty what
    // waits to be written, so it is emptied here.
    use std::io::Write;
    let _ = std::io::stdout().flush();
    said.to_string()
}

fn lines_before(request: &[(String, String, String, bool)]) -> u32 {
    request
        .iter()
        .find(|(from, key, ..)| from == "SELF" && key == "lines_before")
        .and_then(|(.., n, _)| n.parse().ok())
        .unwrap_or(0)
}

fn go(table: &Table, source: &str, program_args: &[String], request: &[(String, String, String, bool)]) -> Result<(), String> {
    let ahead = lines_before(request);
    let read = scan::scan_at(source, table).map_err(|(said, row)| cannot_read(table, &said, row, request, ahead, false));
    let shaped = indent::indent(read?, table, ahead).map_err(|(said, row)| cannot_read(table, &said, row, request, ahead, false));
    let tokens = shaped?;
    let system = ["system.args", "ext.system.args.list", "ext.system.args.count", "system.memoization", "system.real_default_precision", "system.entry", "system.kind.integer",
        "system.kind.rational", "system.kind.real", "system.kind.string", "system.kind.boolean", "system.kind.array", "system.kind.null"];
    let mut seeded: Vec<String> = system.iter().filter_map(|k| table.single(k).map(str::to_string)).collect();
    seeded.extend(REQUEST_PARTS.iter().filter_map(|(_, key)| table.single(key).map(str::to_string)));
    seeded.extend(table.single("ext.system.request.amiss").map(str::to_string));
    seeded.extend(table.single("ext.system.request.body").map(str::to_string));
    seeded.extend(OWN_PLACE.iter().filter_map(|(_, key)| table.single(key).map(str::to_string)));
    seeded.extend(table.single("ext.system.source.line").map(str::to_string));
    let before: u32 = request
        .iter()
        .find(|(from, key, ..)| from == "SELF" && key == "lines_before")
        .and_then(|(.., n, _)| n.parse().ok())
        .unwrap_or(0);
    let reduced = if !table.rpn {
        build::build_at(&tokens, table, &seeded, HashMap::new(), true, before)
            .map_err(|(said, row, hard)| cannot_read(table, &said, row, request, ahead, hard))?
    } else {
        // Read leniently until the named programs' arities settle, then strictly.
        let mut assumed: HashMap<String, build::Signature> = HashMap::new();
        let mut settled = None;
        for _ in 0..8 {
            let r = build::build(&tokens, table, &seeded, assumed.clone(), false, before)?;
            let same = r.seen.iter().all(|(n, a)| assumed.get(n) == Some(a));
            assumed = r.seen;
            if same {
                settled = Some(build::build(&tokens, table, &seeded, assumed.clone(), true, before)?);
                break;
            }
        }
        settled.ok_or_else(|| "The programs of this file take and leave values in a way that does not settle".to_string())?
    };
    let mut machine = exec::Machine::new(table, reduced.globals.clone());
    // Text read while the run goes is a piece of this same program, and
    // is built knowing what the whole of it declared about cells.
    machine.knows_cells = (reduced.shared_args.clone(), reduced.arg_names.clone(), reduced.gives_back.clone());
    if let Some(n) = table.single("system.args") {
        machine.define(n, Value::text(&program_args.join(" ")));
    }
    // The same arguments as a list. The file the run was started with
    // stands first in it, as the system that started the run counts it.
    if table.single("ext.system.args.list").is_some() || table.single("ext.system.args.count").is_some() {
        let file = request.iter().find(|(from, k, ..)| from == "SELF" && k == "file").map(|(.., v, _)| v.clone());
        let mut all: Vec<Value> = vec![Value::text(&file.unwrap_or_default())];
        all.extend(program_args.iter().map(|a| Value::text(a)));
        if let Some(n) = table.single("ext.system.args.count") {
            machine.define(n, Value::Small(all.len() as i64));
        }
        if let Some(n) = table.single("ext.system.args.list") {
            machine.define(n, Value::Vector(std::rc::Rc::new(all)));
        }
    }
    // What the request carries, each group a map under the name the
    // definition gives it.
    for (group, key) in REQUEST_PARTS {
        let Some(name) = table.single(key) else { continue };
        let wanted: Vec<&str> = if group == "ALL" { vec!["GET", "POST", "COOKIE"] } else { vec![group] };
        let mut carried: Vec<(Value, Value)> = Vec::new();
        for (_, field, value, counted) in request.iter().filter(|(from, ..)| wanted.contains(&from.as_str())) {
            let steps: Vec<&str> = field.split('\u{1f}').collect();
            let held = match counted {
                true => Value::Small(value.parse().unwrap_or(0)),
                false => Value::text(value),
            };
            written_at(&mut carried, &steps, held);
        }
        machine.define(name, Value::Dict(std::rc::Rc::new(carried)));
    }
    // The body as it came, for a program that would read it itself.
    if let Some(name) = table.single("ext.system.request.body") {
        let found = request.iter().find(|(from, key, ..)| from == "SELF" && key == "body");
        machine.define(name, found.map_or(Value::Nil, |(.., raw, _)| Value::text(raw)));
    }
    // What the host found amiss in the request before the program ran,
    // told in the language's own words.
    if let Some(name) = table.single("ext.system.request.amiss") {
        // Each is the kind the host found, and after it whatever counts
        // the kind is told with; the words come with places for them.
        let said: Vec<Value> = request
            .iter()
            .filter(|(from, key, ..)| from == "SELF" && key == "amiss")
            .filter_map(|(.., told, _)| {
                let mut steps = told.split('\u{1f}');
                let kind = steps.next().unwrap_or_default();
                let (_, key) = REQUEST_AMISS.iter().find(|(k, _)| *k == kind)?;
                let words = table.single(key)?;
                let mut whole = vec![Value::text(words)];
                whole.extend(steps.map(Value::text));
                Some(Value::Vector(std::rc::Rc::new(whole)))
            })
            .collect();
        machine.define(name, Value::Vector(std::rc::Rc::new(said)));
    }
    if let Some((.., place, _)) = request.iter().find(|(from, key, ..)| from == "SELF" && key == "file") {
        machine.found_in(place);
    }
    // Where the program is written, as the request carries it.
    for (part, key) in OWN_PLACE {
        let Some(name) = table.single(key) else { continue };
        if let Some((.., value, _)) = request.iter().find(|(from, field, ..)| from == "SELF" && field == part) {
            machine.define(name, Value::text(value));
        }
    }
    if !table.flag("ext.system.kind.spelled") {
        for (key, sort) in exec::KIND_LABELS {
            if let Some(n) = table.single(key) {
                machine.define(n, Value::KindOf(sort));
            }
        }
    }
    if let Some(n) = table.single("system.real_default_precision") {
        machine.define(n, Value::Small(math::DEFAULT_PLACES as i64));
    }
    if let Err(told) = machine.run_main(&reduced.program.body) {
        let _ = machine.run_afterward();
        machine.let_things_go();
        machine.let_go_all();
        return Err(told);
    }
    if let Err(told) = machine.run_afterward() {
        machine.let_things_go();
        machine.let_go_all();
        return Err(told);
    }
    if let Some(entry) = table.single("system.entry") {
        if let Some(Value::Bound(p, env)) = machine.lookup(entry) {
            let done = machine.invoke(p, env, Vec::new()).map_err(|e| match e {
                exec::Escape::Error(m) => m,
                _ => String::new(),
            });
            machine.let_things_go();
            machine.let_go_all();
            done?;
            return Ok(());
        }
    }
    // Whatever the run was still keeping goes out when it ends.
    machine.let_things_go();
    machine.let_go_all();
    Ok(())
}

/// Write a value where a name points: every step names a place in a
/// map, and a step with no name of its own is the next whole number.
/// Maps are made along the way as the steps ask for them.
fn written_at(entries: &mut Vec<(Value, Value)>, steps: &[&str], value: Value) {
    let after = || {
        entries
            .iter()
            .filter_map(|(k, _)| match k {
                Value::Small(n) => Some(n + 1),
                _ => None,
            })
            .chain(std::iter::once(0))
            .max()
            .unwrap_or(0)
    };
    let key = match steps.first() {
        // Digits name a whole-number place, so that `a[]` and `a[0]`
        // are the same place, as a form means them to be.
        Some(step) if !step.is_empty() && step.chars().all(|c| c.is_ascii_digit()) => {
            Value::Small(step.parse().unwrap_or(0))
        }
        Some(step) if !step.is_empty() => Value::text(step),
        _ => Value::Small(after()),
    };
    if steps.len() <= 1 {
        match entries.iter_mut().find(|(k, _)| k.equals(&key)) {
            Some(place) => place.1 = value,
            None => entries.push((key, value)),
        }
        return;
    }
    let at = match entries.iter().position(|(k, _)| k.equals(&key)) {
        Some(at) => at,
        None => {
            entries.push((key, Value::Dict(std::rc::Rc::new(Vec::new()))));
            entries.len() - 1
        }
    };
    let mut deeper = match &entries[at].1 {
        Value::Dict(held) => held.as_ref().clone(),
        _ => Vec::new(),
    };
    written_at(&mut deeper, &steps[1..], value);
    entries[at].1 = Value::Dict(std::rc::Rc::new(deeper));
}
