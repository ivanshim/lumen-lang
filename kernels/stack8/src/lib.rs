// Stack kernel, third design: eight words, the five of stack5 and three
// fused from them where the measurements said a word earns its place.
//
// A definition from langs/ says how a language is spelled; the assembler
// turns a program in that language into five kinds of word over a data
// stack, and the machine runs them. Everything the first stack design had
// a word for is a shape made of these five. This crate never imports the
// other kernels; they meet only in the host.

pub mod compile;
pub mod lang;
pub mod engine;
pub mod arith;
pub mod lex;
pub mod layout;
pub mod value;
pub mod code;

use lang::Lang;
use value::Value;

/// Definitions embedded at build time; one given on the command line is
/// read the same way.
const BUILT_IN: &[&str] = &[
    include_str!("../../../langs/lumen.json"),
    include_str!("../../../langs/rplumen.json"),
    include_str!("../../../langs/python.json"),
    include_str!("../../../langs/rust.json"),
];

/// The embedded languages: each name with its file extensions.
pub fn languages() -> Result<Vec<(String, Vec<String>)>, String> {
    BUILT_IN.iter().map(|text| lang::identify(text)).collect()
}

/// The name of the language a definition describes.
pub fn language_of(definition: &str) -> Result<String, String> {
    lang::identify(definition).map(|(name, _)| name)
}

/// Run `source` as the embedded language `language`.
pub fn run(language: &str, source: &str, program_args: &[String], request: &[(String, String, String, bool)]) -> Result<(), String> {
    for text in BUILT_IN {
        if lang::identify(text)?.0 == language {
            let mut lang = Lang::parse(text).map_err(|e| format!("Error: definition of '{language}': {e}"))?;
            settle_brief(&mut lang, request);
            settle_markup(&mut lang, request);
            return go(&lang, source, program_args, request);
        }
    }
    Err(format!("Error: Unknown language '{}'", language))
}

/// Run `source` under a definition given as JSON text.
pub fn run_definition(definition: &str, source: &str, program_args: &[String], request: &[(String, String, String, bool)]) -> Result<(), String> {
    let mut lang = Lang::parse(definition).map_err(|e| format!("Error: language definition: {e}"))?;
    settle_brief(&mut lang, request);
    settle_markup(&mut lang, request);
    go(&lang, source, program_args, request)
}

/// What a setting the run was started with stands at, with the quotes a
/// setting may be written in taken off, the way the reference reads one
/// written down for it.
fn setting_said(request: &[(String, String, String, bool)], setting: &str) -> Option<String> {
    let said = request.iter().find(|(from, key, ..)| from == "SETTINGS" && *key == setting)?;
    let worth = said.2.trim();
    let bare = worth.strip_prefix('"').and_then(|rest| rest.strip_suffix('"'));
    Some(bare.unwrap_or(worth).to_string())
}

/// Whether a complaint is dressed for a reader of markup. The setting
/// that says so is one the run is started with and cannot change while
/// it goes, so it is settled once, before a word of the program is
/// read. Where it is off the dressing is taken away and the plain words
/// stand; where the language gives no dressing there is nothing to take.
fn settle_markup(lang: &mut Lang, request: &[(String, String, String, bool)]) {
    let Some(setting) = lang.markup_setting.clone() else { return };
    let said = setting_said(request, &setting).map(|worth| worth.to_ascii_lowercase());
    // A setting stands for yes unless it is one of the words for no,
    // which is how the reference reads one written in a file.
    let on = matches!(said.as_deref(), Some(worth) if !matches!(worth, "" | "0" | "off" | "false" | "no"));
    if !on {
        lang.markup_kind = None;
        lang.markup_place = None;
        lang.markup_line = None;
        lang.markup_page = None;
    }
}

/// Where the language's own pages are kept, as the run was started.
/// Nothing where the run names nowhere, and then a complaint about a
/// word of the language points at no page.
fn pages_kept(lang: &Lang, request: &[(String, String, String, bool)]) -> Option<String> {
    let setting = lang.pages_setting.as_ref()?;
    setting_said(request, setting).filter(|where_at| !where_at.is_empty())
}

/// Whether the shorter marker opens a run of code at all. The setting
/// that says so is one the run is started with and cannot change while
/// it goes, so it is settled once, before a word of the program is
/// read. Where it is off the marker is taken away, and what follows it
/// is page like any other text.
fn settle_brief(lang: &mut Lang, request: &[(String, String, String, bool)]) {
    let Some(setting) = lang.prologue_brief_setting.clone() else { return };
    let said = request
        .iter()
        .find(|(from, key, ..)| from == "SETTINGS" && *key == setting)
        .map(|(.., worth, _)| worth.trim().to_ascii_lowercase());
    // A setting stands for yes unless it is one of the words for no,
    // which is how the reference reads one written in a file.
    let on = matches!(said.as_deref(), Some(worth) if !matches!(worth, "" | "0" | "off" | "false" | "no"));
    if !on {
        lang.prologue_brief = None;
    }
}

fn go(lang: &Lang, source: &str, program_args: &[String], request: &[(String, String, String, bool)]) -> Result<(), String> {
    go_inner(lang, source, program_args, request).map_err(|e| {
        if !lang.exceptions.is_empty() {
            if let Some(told) = e.strip_prefix('\0') { return told.to_string(); }
        }
        let words = &lang.call_builtin_amiss;
        if words.len() == 2 && e.starts_with(&words[0]) && e.ends_with(&words[1]) { e }
        else { format!("{}: {}", lang.banner, e) }
    })
}

/// A program that could not be read, told the way this language tells a
/// complaint: written where the run would have written, naming the file
/// and the line the reading stopped on. Where the language has no word
/// for such a stopping, nothing is written here and the fault goes back
/// as it came, for the host to tell in its own way.
fn cannot_read(lang: &Lang, said: &str, row: usize, request: &[(String, String, String, bool)], before: u32, fatally: bool) -> String {
    let word = match fatally {
        true => lang.complaint_words.iter().find(|(kind, _)| *kind == lang::Complaint::Fatal).map(|(_, word)| word.as_str()),
        false => lang.reading_word.as_deref(),
    };
    let Some(word) = word else { return said.to_string() };
    let named = |key: &str| request.iter().find(|(from, k, ..)| from == "SELF" && k == key).map(|(.., v, _)| v.clone());
    let file = named("file").unwrap_or_default();
    let line = (row as u32).saturating_sub(before);
    print!("{}{}", engine::complaint_opening(lang, word, said), engine::complaint_place(lang, &file, line));
    // The run ends straight after this, and ending does not empty what
    // is waiting to be written, so it is emptied here.
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

/// The host may keep the prologue before the library and put the rest
/// of its line after it. Where that prologue is now an import, join its
/// line again at the beginning of the file's own text. Leave an empty
/// line behind so that every later row keeps its number.
fn whole_import<'a>(source: &'a str, lang: &Lang, before: u32) -> std::borrow::Cow<'a, str> {
    let kept = std::borrow::Cow::Borrowed(source);
    let Some(prologue) = &lang.prologue else { return kept };
    if before == 0 || !prologue.split_whitespace().next().map_or(false, |head| Lang::spells(&lang.import_words, head)) {
        return kept;
    }
    let Some(first_end) = source.find('\n') else { return kept };
    let first = &source[..first_end];
    if first.trim_start() != prologue {
        return kept;
    }
    let Some((last, _)) = source.match_indices('\n').nth(before as usize - 1) else { return kept };
    let cut = last + 1;
    std::borrow::Cow::Owned(format!("{}{}{}", &source[first_end..cut], first, &source[cut..]))
}

fn go_inner(lang: &Lang, source: &str, program_args: &[String], request: &[(String, String, String, bool)]) -> Result<(), String> {
    let before = lines_before(request);
    let source = whole_import(source, lang, before);
    let read = lex::lex_at(&source, lang).map_err(|(said, row)| cannot_read(lang, &said, row, request, before, false));
    let shaped = layout::layout(read?, lang, before as usize).map_err(|(said, row)| cannot_read(lang, &said, row, request, before, false));
    let tokens = shaped?;
    let mut registry = compile::Registry::default();
    // The system names are globals whether or not the program mentions them.
    let system = [&lang.args_binding, &lang.args_list, &lang.args_count, &lang.memo_binding, &lang.precision_binding, &lang.entry_binding,
        &lang.figures_binding, &lang.figures_shown_binding];
    for name in system.into_iter().flatten() {
        registry.slot(name);
    }
    if !lang.kind_spelled {
        for (name, _) in &lang.sort_bindings {
            registry.slot(name);
        }
    }
    for (_, name) in &lang.request_bindings {
        registry.slot(name);
    }
    if let Some(name) = &lang.amiss_binding {
        registry.slot(name);
    }
    if let Some(name) = &lang.body_binding {
        registry.slot(name);
    }
    for (_, name) in &lang.source_bindings {
        registry.slot(name);
    }
    for name in &lang.module_names {
        registry.slot(name);
    }
    let program = match compile::compile(&tokens, lang, &mut registry, before) {
        Ok(program) => program,
        Err(said) => return Err(cannot_read(lang, &said, registry.stopped_at, request, before, registry.stopped_fatally)),
    };

    let mut machine = engine::Engine::new(lang, registry);
    for name in &lang.module_names {
        machine.define(name, Value::text("__main__"));
    }
    if let Some(name) = &lang.args_binding {
        machine.define(name, Value::text(&program_args.join(" ")));
    }
    // The same arguments as a list. The file the run was started with
    // stands first in it, as the system that started the run counts it.
    if lang.args_list.is_some() || lang.args_count.is_some() {
        let file = request.iter().find(|(from, k, ..)| from == "SELF" && k == "file").map(|(.., v, _)| v.clone());
        let mut all: Vec<Value> = vec![Value::text(&file.unwrap_or_default())];
        all.extend(program_args.iter().map(|a| Value::text(a)));
        if let Some(name) = &lang.args_count {
            machine.define(name, Value::Small(all.len() as i64));
        }
        if let Some(name) = &lang.args_list {
            machine.define(name, Value::array(all));
        }
    }
    // Every part of the request is a map of what it carries, under the
    // name the definition gives it; the one for all of them holds what
    // the query, the form and the cookies carry together.
    for (group, name) in &lang.request_bindings {
        let wanted: Vec<&str> = match group.as_str() {
            "ALL" => vec!["GET", "POST", "COOKIE"],
            other => vec![other],
        };
        let mut carried: Vec<(Value, Value)> = Vec::new();
        for (_, key, value, counted) in request.iter().filter(|(from, ..)| wanted.contains(&from.as_str())) {
            let steps: Vec<&str> = key.split('\u{1f}').collect();
            let held = match counted {
                true => Value::Small(value.parse().unwrap_or(0)),
                false => Value::text(value),
            };
            put_step(&mut carried, &steps, held);
        }
        machine.define(name, Value::Map(std::rc::Rc::new(carried)));
    }
    // The body as it came, for a program that would read it itself.
    if let Some(name) = &lang.body_binding {
        let found = request.iter().find(|(from, key, ..)| from == "SELF" && key == "body");
        machine.define(name, found.map_or(Value::Null, |(.., raw, _)| Value::text(raw)));
    }
    // What the host found amiss in the request before the program ran,
    // in the language's own words: the host names only which of them it
    // found, since the wording is the language's and not the host's.
    if let Some(name) = &lang.amiss_binding {
        // Each is the kind the host found, and after it whatever counts
        // the kind is told with; the words come with places for them.
        let said: Vec<Value> = request
            .iter()
            .filter(|(from, key, ..)| from == "SELF" && key == "amiss")
            .filter_map(|(.., told, _)| {
                let mut steps = told.split('\u{1f}');
                let kind = steps.next().unwrap_or_default();
                let (_, words) = lang.amiss_words.iter().find(|(k, _)| *k == kind)?;
                let mut whole = vec![Value::text(words)];
                whole.extend(steps.map(|n| Value::text(n)));
                Some(Value::array(whole))
            })
            .collect();
        machine.define(name, Value::array(said));
    }
    if let Some((.., place, _)) = request.iter().find(|(from, key, ..)| from == "SELF" && key == "file") {
        machine.written_in(place);
    }
    machine.pages_are_kept(pages_kept(lang, request));
    // Where the program is written, as the request carries it.
    for (part, name) in &lang.source_bindings {
        let found = request.iter().find(|(from, key, ..)| from == "SELF" && key == part);
        if let Some((.., value, _)) = found {
            machine.define(name, Value::text(value));
        }
    }
    if !lang.kind_spelled {
        for (name, kind) in &lang.sort_bindings {
            machine.define(name, engine::sort_value(*kind));
        }
    }
    if let Some(name) = &lang.precision_binding {
        machine.define(name, engine::places_default());
    }
    // How many figures a real is written with is the run's own to
    // settle while it goes, so each count lives in a cell the run
    // reaches by name and the kernel reads again every time it writes
    // a real out. A count of nought or below asks for the fewest
    // figures that read back as the same number, which is where a run
    // showing a real with its kind starts.
    let shared = |v: Value| std::rc::Rc::new(std::cell::RefCell::new(v));
    let mut counts = (None, None);
    if let Some(name) = &lang.figures_binding {
        let cell = shared(Value::Small(lang.real_digits.map_or(-1, |n| n as i64)));
        machine.define(name, Value::Bond(cell.clone()));
        counts.0 = Some(cell);
    }
    if let Some(name) = &lang.figures_shown_binding {
        let cell = shared(Value::Small(-1));
        machine.define(name, Value::Bond(cell.clone()));
        counts.1 = Some(cell);
    }
    value::figures_kept_in(counts.0, counts.1);
    // The room counted against the program is counted from here: what
    // went before was the host reading the program and setting up the
    // machine that runs it, and belongs to the host rather than to the
    // program that asks how much room it has taken.
    lumen_room::mark();
    // A value raised and never caught is a fault like any other, told
    // in the language's own words.
    if let Err(fault) = machine.invoke(&program, Vec::new()) {
        // A run the program itself said was over came out right, and
        // what was named to run at the end runs even then.
        if matches!(fault, engine::Fault::Finished) {
            let done = machine.run_when_done();
            machine.let_things_go();
            machine.let_go_all();
            return done.map_err(|f| f.told(&machine.names()));
        }
        // A program may put a routine in the way of a value nobody
        // took; the run says nothing of its own where one took it up.
        if !machine.taken_up(&fault) {
            machine.ended_uncaught(&fault);
        }
        // What a program named to run at the end may itself be stopped,
        // and that is told as the run's own ending was.
        if let Err(after) = machine.run_when_done() {
            machine.ended_uncaught(&after);
        }
        machine.let_things_go();
        machine.let_go_all();
        return Err(fault.told(&machine.names()));
    }
    if let Err(after) = machine.run_when_done() {
        machine.ended_uncaught(&after);
        machine.let_things_go();
        machine.let_go_all();
        return Err(after.told(&machine.names()));
    }

    // A language with an entry function (Rust's `main`) runs it once the
    // program body has defined it.
    if let Some(entry) = &lang.entry_binding {
        if let Some(Value::Routine(main)) = machine.lookup(entry).cloned() {
            let done = machine.invoke(&main, Vec::new()).map_err(|f| f.told(&machine.names()));
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

/// Put a value where a name points: each step names a place in a map,
/// and a step with no name is the next whole-number place. The maps
/// along the way are made as they are needed.
fn put_step(into: &mut Vec<(Value, Value)>, steps: &[&str], value: Value) {
    let next = || {
        let highest = into
            .iter()
            .filter_map(|(k, _)| match k {
                Value::Small(n) => Some(n + 1),
                _ => None,
            })
            .max();
        Value::Small(highest.unwrap_or(0).max(0))
    };
    let key = match steps.first() {
        // A step of digits names a whole-number place, not one named
        // by that text, so `a[]` and `a[0]` name the same place.
        Some(step) if step.chars().all(|c| c.is_ascii_digit()) && !step.is_empty() => {
            Value::Small(step.parse().unwrap_or(0))
        }
        Some(step) if !step.is_empty() => Value::text(step),
        _ => next(),
    };
    if steps.len() <= 1 {
        match into.iter_mut().find(|(k, _)| k.equals(&key)) {
            Some(place) => place.1 = value,
            None => into.push((key, value)),
        }
        return;
    }
    let at = match into.iter().position(|(k, _)| k.equals(&key)) {
        Some(at) => at,
        None => {
            into.push((key, Value::Map(std::rc::Rc::new(Vec::new()))));
            into.len() - 1
        }
    };
    let mut inside = match &into[at].1 {
        Value::Map(pairs) => pairs.as_ref().clone(),
        _ => Vec::new(),
    };
    put_step(&mut inside, &steps[1..], value);
    into[at].1 = Value::Map(std::rc::Rc::new(inside));
}
