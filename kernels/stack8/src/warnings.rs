// Python's native warning backend; display hooks remain in the public library.
impl Engine<'_> {
    fn warning_member(&mut self, owner: Value, name: &str) -> Res<Value> {
        self.member_of(owner, name)?.ok_or_else(|| format!("AttributeError: missing warning attribute '{name}'"))
    }
    fn warning_method(&mut self, owner: Value, method: &str, args: Vec<Value>) -> Res<Value> {
        let callable = self.warning_member(owner, method)?;
        self.call_held(callable, args)
    }
    fn make_warning_backend(&mut self) -> Res<Value> {
        let names = self.lang.warning_backend.clone();
        let saved = match &self.warning_module {
            Some(Value::Object(module)) => module.fields.borrow().iter().filter(|(word, _)| matches!(word.as_str(), "filters" | "_onceregistry" | "_defaultaction" | "_warnings_context")).cloned().collect::<Vec<_>>(),
            _ => Vec::new(),
        };
        let mut entries = vec![("__name__".into(), Value::text(&names[0]))];
        for (operation, word) in names[1..].iter().enumerate() {
            entries.push((word.clone(), Value::Native(Builtin::WarningBackend(operation as u8), Rc::from(word.as_str()))));
        }
        let mut filters = Vec::new();
        for (action, category, module) in [("default", "DeprecationWarning", true), ("ignore", "DeprecationWarning", false), ("ignore", "PendingDeprecationWarning", false), ("ignore", "ImportWarning", false), ("ignore", "ResourceWarning", false)] {
            let kind = self.builtin_named(category)?.ok_or_else(|| format!("NameError: {category}"))?;
            filters.push(Value::tuple(vec![Value::text(action), Value::Null, kind, if module { Value::text("__main__") } else { Value::Null }, Value::Small(0)]));
        }
        entries.push(("filters".into(), Value::Bond(Rc::new(RefCell::new(Value::array(filters))))));
        entries.push(("_onceregistry".into(), Value::Bond(Rc::new(RefCell::new(Value::Map(Rc::new(Vec::new().into())))))));
        entries.push(("_defaultaction".into(), Value::text("default")));
        let context = if let Some((_, context)) = saved.iter().find(|(word, _)| word == "_warnings_context") { context.clone() } else {
            let module = self.import_module("_contextvars").map_err(|error| { self.carried = Some(error); self.special_fault() })?;
            let factory = self.warning_member(module, "ContextVar")?;
            self.call_held(factory, vec![Value::text("_warnings_context")])?
        };
        entries.push(("_warnings_context".into(), context));
        for (word, value) in saved {
            if let Some(entry) = entries.iter_mut().find(|(key, _)| key == &word) { entry.1 = value; }
        }
        let class = self.kind_class("module");
        self.made += 1;
        let module = Value::Object(Rc::new(Instance { replacement_class: RefCell::new(None), class, fields: RefCell::new(entries), mark: self.made }));
        self.warning_module = Some(module.clone());
        self.modules.insert(names[0].clone(), module.clone());
        self.module_addresses_ready.set(false);
        self.refresh_module_cache(&names[0]);
        Ok(module)
    }
    fn warning_public(&mut self) -> Res<Value> {
        self.module_cache_value("warnings").filter(|v| !matches!(v, Value::Null))
            .or_else(|| if self.finalizing { self.modules.get("warnings").cloned() } else { None })
            .or_else(|| self.warning_bound.clone()).or_else(|| self.warning_module.clone()).ok_or_else(|| "RuntimeError: warning backend is not initialized".into())
    }
    fn warning_attribute(&mut self, name: &str, fallback: &str) -> Res<Value> {
        let public = self.warning_public()?;
        if let Some(value) = self.member_of(public, name)? {
            if let Some(Value::Object(native)) = &self.warning_module {
                if let Some((_, saved)) = native.fields.borrow_mut().iter_mut().find(|(word, _)| word == fallback) { *saved = value.clone(); }
            }
            return Ok(value);
        }
        let native = self.warning_module.clone().ok_or_else(|| "RuntimeError: warning backend is not initialized".to_string())?;
        self.warning_member(native, fallback)
    }
    fn warning_call(&mut self, operation: u8, items: Vec<(Option<String>, Value)>) -> Res<Value> {
        let caller_location = self.location;
        let answer = self.warning_dispatch(operation, items);
        self.location = caller_location;
        answer
    }
    fn warning_dispatch(&mut self, operation: u8, items: Vec<(Option<String>, Value)>) -> Res<Value> {
        if operation >= 2 {
            let args: Vec<_> = items.into_iter().map(|(name, value)| if name.is_none() { Ok(value) } else { Err("TypeError: native warnings operation takes no keyword arguments".to_string()) }).collect::<Result<_,_>>()?;
            match operation {
                2 | 3 => { if !args.is_empty() { return Err("TypeError: _filters_mutated() takes no arguments".into()); } if operation == 3 && self.warning_lock_depth == 0 { return Err("RuntimeError: warnings lock is not held".into()); } self.warning_revision += 1; }
                4 => { if !args.is_empty() { return Err("TypeError: _acquire_lock() takes no arguments".into()); } self.warning_lock_depth += 1; }
                5 => { if !args.is_empty() { return Err("TypeError: _release_lock() takes no arguments".into()); } if self.warning_lock_depth == 0 { return Err("RuntimeError: cannot release un-acquired lock".into()); } self.warning_lock_depth -= 1; }
                6 => { if args.len() != 1 { return Err("TypeError: _set_module() takes one argument".into()); } self.warning_bound = Some(args[0].clone()); }
                _ => return Err("RuntimeError: unknown warnings operation".into()),
            }
            return Ok(Value::Null);
        }
        let fields: &[&str] = if operation == 0 { &["message", "category", "stacklevel", "source", "skip_file_prefixes"] } else { &["message", "category", "filename", "lineno", "module", "registry", "module_globals", "source"] };
        let maximum = if operation == 0 { 4 } else { 8 };
        let mut slots = vec![None; fields.len()];
        let mut position = 0;
        for (name, value) in items {
            let at = if let Some(name) = name { fields.iter().position(|field| *field == name).ok_or_else(|| format!("TypeError: '{name}' is an invalid keyword argument for warn{}()", if operation == 0 { "" } else { "_explicit" }))? } else {
                if position >= maximum { return Err("TypeError: too many warning arguments".into()); }
                let at = position; position += 1; at
            };
            if slots[at].is_some() { return Err(format!("TypeError: argument for warn() given by name ('{}') and position ({})", fields[at], at+1)); }
            slots[at] = Some(value);
        }
        let required = if operation == 0 { 1 } else { 4 };
        for at in 0..required { if slots[at].is_none() { return Err(format!("TypeError: warn{}() missing required argument '{}' (pos {})", if operation == 0 { "" } else { "_explicit" }, fields[at], at+1)); } }
        if operation == 1 {
            let mut args: Vec<_> = slots.into_iter().map(|v| v.unwrap_or(Value::Gap)).collect();
            for index in [5,6,7] { if matches!(args[index], Value::Gap) { args[index] = Value::Null; } }
            if !matches!(args[2].contents(), Value::Text(_) | Value::Codepoints(_)) { return Err("TypeError: warn_explicit() argument 3 must be str".into()); }
            args[3] = Value::Small(self.warning_integer(&args[3], true)?);
            let line = self.warning_source(&args[6], &args[3])?;
            args.push(line);
            return self.warning_explicit(args);
        }
        let message = slots[0].take().unwrap();
        let warning = self.builtin_named("Warning")?.unwrap();
        let is_warning = self.builtin(Builtin::InstanceOf, "isinstance", &mut vec![message.clone(), warning.clone()])?;
        let category = if self.truth(&is_warning) { self.warning_member(message.clone(), "__class__")? } else {
            match slots[1].take() { None | Some(Value::Null) => self.builtin_named("UserWarning")?.unwrap(), Some(kind) => kind }
        };
        let subclass = self.builtin_named("issubclass")?.unwrap();
        if !matches!(category.contents(), Value::Class(_)) { return Err(format!("TypeError: category must be a Warning subclass, not '{}'", category.core_kind())); }
        let valid = self.call_held(subclass, vec![category.clone(), warning])?;
        if !self.truth(&valid) { return Err(format!("TypeError: category must be a Warning subclass, not '{}'", category.core_kind())); }
        let stacklevel = slots[2].take().unwrap_or(Value::Small(1));
        let mut depth = self.warning_integer(&stacklevel, false)?.max(1);
        let prefixes = slots[4].take().unwrap_or_else(|| Value::tuple(Vec::new()));
        let Value::Tuple(prefixes) = prefixes.contents() else { return Err("TypeError: skip_file_prefixes must be a tuple of strs.".into()); };
        let mut skip = Vec::new();
        for prefix in prefixes.iter() { let Value::Text(prefix) = prefix.contents() else { return Err("TypeError: skip_file_prefixes must be a tuple of strs.".into()); }; skip.push(prefix.to_string()); }
        if !skip.is_empty() { depth = depth.max(2); }
        let mut frame = self.trace_frame.clone();
        let internal = |file: &str| file.contains("importlib") && file.contains("_bootstrap");
        let current_internal = frame.as_ref().is_some_and(|frame| matches!(&frame.fields.borrow()[6].1, Value::Routine(routine) if routine.written_in.as_ref().is_some_and(|file| internal(file))));
        for _ in 1..depth {
            frame = frame.and_then(|frame| match &frame.fields.borrow()[2].1 { Value::Object(parent) => Some(parent.clone()), _ => None });
            if !current_internal {
                loop {
                    let hidden = frame.as_ref().is_some_and(|frame| matches!(&frame.fields.borrow()[6].1, Value::Routine(routine) if routine.written_in.as_ref().is_some_and(|file| internal(file) || skip.iter().any(|prefix| file.starts_with(prefix)))));
                    if !hidden { break; }
                    frame = frame.and_then(|frame| match &frame.fields.borrow()[2].1 { Value::Object(parent) => Some(parent.clone()), _ => None });
                }
            }
        }
        let (globals, filename, lineno) = if let Some(frame) = frame {
            let globals = self.frame_member(&frame, "f_globals").unwrap_or(Value::Null);
            let fields = frame.fields.borrow();
            let file = match &fields[6].1 { Value::Routine(routine) => routine.written_in.as_ref().map(|file| Value::text(file)).unwrap_or_else(|| Value::text(&self.root_source)), _ => Value::text(&self.root_source) };
            (globals, file, fields[0].1.clone())
        } else {
            let sys = match self.modules.get("sys").cloned() {
                Some(sys) => sys,
                None => self.import_module("sys").map_err(|error| { self.carried = Some(error); self.special_fault() })?,
            };
            (self.warning_member(sys, "__dict__")?, Value::text("<sys>"), Value::Small(0))
        };
        let module = self.warning_method(globals.clone(), "get", vec![Value::text("__name__"), Value::text("<string>")])?;
        let fresh_registry = Value::Bond(Rc::new(RefCell::new(Value::Map(Rc::new(Vec::new().into())))));
        let registry = self.warning_method(globals.clone(), "setdefault", vec![Value::text("__warningregistry__"), fresh_registry])?;
        self.warning_explicit(vec![message, category, filename, lineno, module, registry, globals, slots[3].take().unwrap_or(Value::Null)])
    }
    fn warning_explicit(&mut self, mut args: Vec<Value>) -> Res<Value> {
        if !matches!(args[2].contents(), Value::Text(_) | Value::Codepoints(_)) { return Err("TypeError: warn_explicit() argument 3 must be str".into()); }
        let lineno = self.warning_integer(&args[3], true)?;
        if lineno < i32::MIN as i64 || lineno > i32::MAX as i64 { return Err("OverflowError: Python int too large to convert to C int".into()); }
        args[3] = Value::Small(lineno);
        if matches!(args[4], Value::Gap) {
            args[4] = match args[2].contents() {
                Value::Codepoints(points) => if points.ends_with(&[46,112,121]) { Value::Codepoints(Rc::new(points[..points.len()-3].to_vec())) } else { args[2].clone() },
                _ => { let path = args[2].plain(); Value::text(if path.is_empty() { "<unknown>" } else { path.strip_suffix(".py").unwrap_or(&path) }) },
            };
        }
        if matches!(args[4], Value::Null) { return Ok(Value::Null); }
        let registry = args[5].clone();
        if !matches!(registry.contents(), Value::Null | Value::Map(_) | Value::Fields(_)) { return Err("TypeError: 'registry' must be a dict or None".into()); }
        let warning = self.builtin_named("Warning")?.unwrap();
        let existing = self.builtin(Builtin::InstanceOf, "isinstance", &mut vec![args[0].clone(), warning])?;
        let (message, text, category) = if self.truth(&existing) {
            let category = self.warning_member(args[0].clone(), "__class__")?;
            let text = self.builtin(Builtin::ToText, "str", &mut vec![args[0].clone()])?;
            (args[0].clone(), text, category)
        } else { (self.call_held(args[1].clone(), vec![args[0].clone()])?, args[0].clone(), args[1].clone()) };
        let key = Value::tuple(vec![text.clone(), category.clone(), args[3].clone()]);
        let has_registry = !matches!(registry.contents(), Value::Null);
        if has_registry && self.warning_seen(registry.clone(), key.clone(), false)? { return Ok(Value::Null); }
        let public = self.warning_public()?;
        let contextual = self.member_of(public.clone(), "_use_context")?.is_some_and(|value| self.truth(&value));
        let filters = if contextual { self.warning_method(public.clone(), "_get_filters", Vec::new())? } else { self.warning_attribute("filters", "filters")? };
        let Value::Array(filters) = filters.contents() else { return Err("ValueError: warnings.filters must be a list".into()); };
        let mut chosen = Value::Null;
        let mut action = None;
        for (index, item) in filters.iter().enumerate() {
            let Value::Tuple(parts) = item.contents() else { return Err(format!("ValueError: _warnings.filters item {index} isn't a 5-tuple")); };
            if parts.len() != 5 { return Err(format!("ValueError: _warnings.filters item {index} isn't a 5-tuple")); }
            if !matches!(parts[0].contents(), Value::Text(_)) { return Err(format!("TypeError: action must be a string, not '{}'", parts[0].core_kind())); }
            let message_matches = self.warning_match(&parts[1], &text)?;
            let module_matches = self.warning_match(&parts[3], &args[4])?;
            let subclass = self.builtin_named("issubclass")?.unwrap();
            let matches = self.call_held(subclass, vec![category.clone(), parts[2].clone()])?;
            let line = self.warning_integer(&parts[4], false)?;
            if !message_matches || !module_matches || !self.truth(&matches) || (line != 0 && line != lineno) { continue; }
            action = Some(parts[0].clone()); chosen = item.clone(); break;
        }
        let action = match action { Some(action) => action, None => self.warning_attribute("defaultaction", "_defaultaction")? };
        if !matches!(action.contents(), Value::Text(_)) { return Err(format!("TypeError: warnings.defaultaction must be a string, not '{}'", action.core_kind())); }
        match action.plain().as_str() {
            "ignore" => return Ok(Value::Null),
            "error" => { self.carried = Some(Fault::Thrown(message)); return Err(self.special_fault()); }
            "always" | "all" => {},
            other => {
                if has_registry { self.warning_method(registry.clone(), "__setitem__", vec![key, Value::Flag(true)])?; }
                if other == "once" || other == "module" {
                    let target = if has_registry { registry } else if other == "once" { self.warning_attribute("onceregistry", "_onceregistry")? } else { Value::Null };
                    if !matches!(target.contents(), Value::Null) {
                        if !matches!(target.contents(), Value::Map(_)) { return Err(format!("TypeError: warnings.onceregistry must be a dict, not '{}'", target.core_kind())); }
                        let alternate = Value::tuple(vec![text.clone(), category.clone()]);
                        if self.warning_seen(target, alternate, true)? { return Ok(Value::Null); }
                    }
                } else if other != "default" {
                    let repr = self.builtin(Builtin::Repr, "repr", &mut vec![action])?.plain();
                    let item = self.builtin(Builtin::Repr, "repr", &mut vec![chosen])?.plain();
                    return Err(format!("RuntimeError: Unrecognized action ({repr}) in warnings.filters:\n {item}"));
                }
            }
        }
        let show = self.member_of(public.clone(), "_showwarnmsg")?;
        if show.is_none() { return self.warning_stderr(&text, &category, &args); }
        let factory = self.warning_member(public.clone(), "WarningMessage")?;
        let line = args.get(8).cloned().unwrap_or(Value::Null);
        let record = self.call_held(factory, vec![message, category, args[2].clone(), args[3].clone(), Value::Null, line, args[7].clone()])?;
        self.call_held(show.unwrap(), vec![record])?;
        Ok(Value::Null)
    }
    fn warning_integer(&mut self, value: &Value, c_int: bool) -> Res<i64> {
        let initial = value.contents();
        let integer = if matches!(initial, Value::Small(_) | Value::Huge(_) | Value::Flag(_)) { initial } else {
            self.special_index(value)?.ok_or_else(|| format!("TypeError: '{}' object cannot be interpreted as an integer", value.core_kind()))?
        };
        let number = match integer { Value::Small(n) => Some(n), Value::Flag(flag) => Some(i64::from(flag)), Value::Huge(n) => n.to_i64(), _ => None };
        let overflow = if c_int { "OverflowError: Python int too large to convert to C int" } else { "OverflowError: Python int too large to convert to C ssize_t" };
        let number = number.ok_or_else(|| overflow.to_string())?;
        if c_int && i32::try_from(number).is_err() { return Err(overflow.into()); }
        Ok(number)
    }
    fn warning_stderr(&mut self, text: &Value, category: &Value, args: &[Value]) -> Res<Value> {
        let name = self.warning_member(category.clone(), "__name__")?.plain();
        let message = self.builtin(Builtin::ToText, "str", &mut vec![text.clone()])?.plain();
        let mut output = format!("{}:{}: {name}: {message}\n", args[2].plain(), args[3].plain());
        if let Some(line) = args.get(8).filter(|line| !matches!(line, Value::Null)) {
            output.push_str(&line.plain()); output.push('\n');
        } else if let (Value::Text(file), Value::Small(row)) = (args[2].contents(), args[3].contents()) {
            if row > 0 {
                if let Ok(source) = std::fs::read_to_string(file.as_ref()) {
                    if let Some(line) = source.lines().nth(row as usize-1) { output.push_str("  "); output.push_str(line.trim_start_matches([' ', '\t', '\x0c'])); output.push('\n'); }
                }
            }
        }
        if let Some(sys) = self.modules.get("sys").cloned() {
            if let Some(stream) = self.member_of(sys, "stderr")? {
                if !matches!(stream.contents(), Value::Null) { let _ = self.warning_method(stream, "write", vec![Value::text(&output)]); self.carried = None; }
            }
        }
        Ok(Value::Null)
    }
    fn warning_source(&mut self, globals: &Value, lineno: &Value) -> Res<Value> {
        if matches!(globals.contents(), Value::Null) { return Ok(Value::Null); }
        if !matches!(globals.contents(), Value::Map(_) | Value::Fields(_)) { return Err(format!("TypeError: module_globals must be a dict, not '{}'", globals.core_kind())); }
        let external = self.import_module("importlib._bootstrap_external").map_err(|error| { self.carried = Some(error); self.special_fault() })?;
        let loader = self.warning_method(external, "_bless_my_loader", vec![globals.clone()])?;
        let Some(get_source) = self.member_of(loader, "get_source")? else { return Ok(Value::Null); };
        let spec = self.warning_method(globals.clone(), "get", vec![Value::text("__spec__"), Value::Null])?;
        let name = self.member_of(spec, "name")?.filter(|v| !matches!(v, Value::Null));
        let name = match name { Some(name) => name, None => self.warning_method(globals.clone(), "get", vec![Value::text("__name__"), Value::Null])? };
        if matches!(name, Value::Null) { return Ok(Value::Null); }
        let source = match self.call_held(get_source, vec![name]) {
            Ok(source) => source,
            Err(error) if error.starts_with("ImportError:") || self.carried.as_ref().is_some_and(|fault| self.fault_names(fault, "ImportError")) => { self.carried = None; return Ok(Value::Null); }
            Err(error) => return Err(error),
        };
        if matches!(source.contents(), Value::Null) { return Ok(Value::Null); }
        let source = Self::worth_of(&source).unwrap_or_else(|| source.contents());
        if !matches!(source, Value::Text(_) | Value::Codepoints(_)) { return Err(format!("TypeError: bad argument type for built-in operation: {}", source.core_kind())); }
        let lines = self.builtin(Builtin::Text(crate::strings::TextOp::Splitlines), "splitlines", &mut vec![source])?;
        let Value::Array(lines) = lines.contents() else { return Ok(Value::Null); };
        let Value::Small(line) = lineno.contents() else { return Err("TypeError: 'lineno' must be an integer".into()); };
        Ok(if line < 1 { Value::Null } else { lines.get(line as usize-1).cloned().unwrap_or(Value::Null) })
    }
    fn warning_match(&mut self, pattern: &Value, text: &Value) -> Res<bool> {
        match pattern.contents() {
            Value::Null => Ok(true), Value::Text(_) => Ok(pattern.equals(text)),
            _ => { let matched = self.warning_method(pattern.clone(), "match", vec![text.clone()])?; Ok(self.truth(&matched)) }
        }
    }
    fn warning_seen(&mut self, registry: Value, key: Value, set: bool) -> Res<bool> {
        let version = self.warning_method(registry.clone(), "get", vec![Value::text("version"), Value::Null])?;
        if !matches!(version.contents(), Value::Small(n) if n == self.warning_revision) {
            self.warning_method(registry.clone(), "clear", Vec::new())?;
            self.warning_method(registry.clone(), "__setitem__", vec![Value::text("version"), Value::Small(self.warning_revision)])?;
        }
        let seen = self.warning_method(registry.clone(), "get", vec![key.clone(), Value::Null])?;
        if self.truth(&seen) { return Ok(true); }
        if set { self.warning_method(registry, "__setitem__", vec![key, Value::Flag(true)])?; }
        Ok(false)
    }
}

impl Engine<'_> {
    /// Load a private bridge without redispatching that request to public finders.
    fn bootstrap_import(&mut self, path: &str) -> Flow<Value> {
        if let Some(held) = self.modules.get(path) { return Ok(held.clone()); }
        let marked = self.finder_active.insert(path.to_owned());
        let loaded = self.import_module(path);
        if marked { self.finder_active.remove(path); }
        loaded
    }

    fn finder_import(&mut self, path: &str) -> Flow<Option<Value>> {
        let Some(sys) = self.modules.get("sys").cloned() else { return Ok(None); };
        let Some(finders) = self.member_of(sys, "meta_path")? else { return Ok(None); };
        let search_path = if let Some((parent, _)) = path.rsplit_once('.') {
            let owner = self.import_module(parent)?;
            if self.module_cache_value(path).is_some() { return self.import_module(path).map(Some); }
            self.member_of(owner, "__path__")?.unwrap_or(Value::Null)
        } else { Value::Null };
        let finders = self.core_members(&finders)?;
        self.finder_active.insert(path.to_string());
        let result: Res<Option<Value>> = (|| {
            for finder in finders {
                // Embedded source uses the native loader; custom finders still precede it.
                if self.module_sources.contains_key(path) || self.sys_path_source(path).is_some() {
                    if let Value::Object(instance) = finder.contents() {
                        let kind = instance.class_now();
                        if kind.name.rsplit('.').next() == Some("SourceFinder")
                            && self.class_value(&kind, "__module__").is_some_and(|module| module.plain() == "_runtime_import") { continue; }
                    }
                    if let Value::Class(kind) = finder.contents() {
                        if matches!(kind.name.rsplit('.').next(), Some("BuiltinImporter" | "FrozenImporter" | "PathFinder")) { continue; }
                        if kind.name.rsplit('.').next() == Some("_PathFinder")
                            && self.class_value(&kind, "__module__").is_some_and(|module| module.plain() == "sys") { continue; }
                    }
                }
                let own_finder = self.modules.get("_imp").and_then(|module| match module { Value::Object(object) => object.fields.borrow().iter().find(|(word, _)| word == "_NativeFinder").map(|(_, value)| value.clone()), _ => None });
                if own_finder.is_some_and(|own| own.contents().equals(&finder.contents())) {
                    if self.lang.warning_backend.first().is_some_and(|word| word == path) || self.module_sources.contains_key(path) || self.sys_path_source(path).is_some() { return Ok(None); }
                    continue;
                }
                let Some(find_spec) = self.member_of(finder, "find_spec")? else { continue; };
                let spec = self.call_held(find_spec, vec![Value::text(path), search_path.clone(), Value::Null])?;
                if matches!(spec.contents(), Value::Null) { continue; }
                if !self.importing.contains("_zipimport_path") && (self.modules.contains_key("_zipimport_path") || self.modules.get("importlib").cloned().is_some_and(|module| self.member_of(module, "_native_spec").ok().flatten().is_some())) {
                    let bridge = self.bootstrap_import("_zipimport_path").map_err(|fault| { self.carried = Some(fault); self.special_fault() })?;
                    let loaded = self.warning_method(bridge, "load", vec![spec])?;
                    self.modules.insert(path.to_string(), loaded.clone());
                    self.module_addresses_ready.set(false); self.changed_module_positions.borrow_mut().clear();
                    return Ok(Some(loaded));
                }
                let bootstrap = self.bootstrap_import("importlib._bootstrap").map_err(|fault| { self.carried = Some(fault); self.special_fault() })?;
                // The pinned loader owns namespace creation, replacement and failure cleanup.
                let loaded = self.warning_method(bootstrap, "_load_unlocked", vec![spec]);
                let module = match loaded {
                    Ok(value) => value.contents(),
                    Err(fault) => {
                        self.modules.remove(path);
                        self.module_addresses_ready.set(false);
                        return Err(fault);
                    }
                };
                self.modules.insert(path.to_string(), module.clone());
                self.module_addresses_ready.set(false);
                if let Some((parent, child)) = path.rsplit_once('.') {
                    if let Some(owner) = self.module_cache_value(parent).or_else(|| self.modules.get(parent).cloned()) {
                        let setter = self.builtin_named("setattr")?.unwrap();
                        self.call_held(setter, vec![owner, Value::text(child), module.clone()])?;
                    }
                }
                return Ok(Some(module));
            }
            Ok(None)
        })();
        self.finder_active.remove(path);
        result.map_err(|message| self.carried.take().unwrap_or(Fault::Note(message)))
    }
}

impl Engine<'_> {
    fn source_loader_metadata(&mut self, name: &str, namespace: &Value) -> Flow<()> {
        if self.lang.module_path.is_none() { return Ok(()); }
        let loader = self.modules.get("_imp").and_then(|module| match module {
            Value::Object(owner) => owner.fields.borrow().iter().find(|(key, _)| key == "_NativeLoader").map(|(_, value)| value.contents()), _ => None,
        });
        let Some(loader @ Value::Class(_)) = loader else { return Ok(()); };
        let spec_type = self.modules.get("importlib._bootstrap").and_then(|module| match module {
            Value::Object(owner) => owner.fields.borrow().iter().find(|(key, _)| key == "ModuleSpec").map(|(_, value)| value.contents()), _ => None,
        });
        let specification = if let Some(constructor @ Value::Class(_)) = spec_type {
            self.call_held(constructor, vec![Value::text(name), loader.clone()])?
        } else { Value::Null };
        if let Value::Object(owner) = namespace {
            for (key, value) in [("__loader__", loader), ("__spec__", specification)] {
                let mut fields = owner.fields.borrow_mut();
                if let Some(field) = fields.iter_mut().find(|(word, _)| word == key) { field.1 = value; }
            }
        }
        Ok(())
    }
}

impl Engine<'_> {
    fn remember_allocation(&mut self, value: &Value) {
        if self.allocation_depth == 0 { return; }
        let Value::Object(object) = value else { return; };
        let mut sites = Vec::new();
        let mut frame = self.trace_frame.clone();
        while let Some(current) = frame {
            let row = current.fields.borrow();
            if let Value::Routine(body) = &row[6].1 {
                let file = body.written_in.as_deref().unwrap_or(&self.root_source);
                // These Python counterparts implement native I/O allocation.
                if !["/modules/_io.py", "/modules/_pyio.py", "/modules/builtins.py"].iter().any(|suffix| file.ends_with(suffix)) {
                    if let Value::Small(line) = row[0].1.contents() { sites.push((file.to_string(), line)); }
                }
            }
            if sites.len() == self.allocation_depth { break; }
            frame = match &row[2].1 { Value::Object(parent) => Some(parent.clone()), _ => None };
        }
        sites.reverse();
        if self.made % 64 == 0 { self.allocation_sites.retain(|_, (weak, _)| weak.strong_count() != 0); }
        self.allocation_sites.insert(Rc::as_ptr(object) as usize, (Rc::downgrade(object), sites));
    }
    fn allocation_command(&mut self, args: &[Value]) -> Res<Value> {
        let Some(Value::Small(operation)) = args.first().map(Value::contents) else { return Err("TypeError: allocation operation must be an integer".into()); };
        match (operation, args) {
            (0, [_, value]) => {
                let Value::Object(object) = value.contents() else { return Ok(Value::Null); };
                let Some((weak, sites)) = self.allocation_sites.get(&(Rc::as_ptr(&object) as usize)) else { return Ok(Value::Null); };
                if weak.upgrade().is_none() { return Ok(Value::Null); }
                Ok(Value::tuple(sites.iter().map(|(file, line)| Value::tuple(vec![Value::text(file), Value::Small(*line)])).collect()))
            }
            (1, [_, Value::Small(limit)]) if *limit > 0 && *limit <= 65535 => { if self.allocation_depth == 0 { self.allocation_depth = *limit as usize; self.allocation_retained_depth = self.allocation_depth; } Ok(Value::Null) }
            (2, [_]) => { self.allocation_depth = 0; self.allocation_sites.clear(); Ok(Value::Null) }
            (3, [_]) => Ok(Value::Flag(self.allocation_depth > 0)),
            (4, [_]) => Ok(Value::Small(self.allocation_retained_depth as i64)),
            _ => Err("ValueError: invalid allocation tracing operation".into()),
        }
    }
}
