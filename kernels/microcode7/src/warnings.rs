// Native Python warning operations, implemented beside the instruction evaluator.
impl Machine<'_> {
    fn warning_get(&mut self, subject: Value, word: &str) -> Result<Value, String> {
        self.core_primitive(Prim::GetMember, "getattr", vec![subject, Value::text(word)], Vec::new())
    }
    fn warning_apply(&mut self, callable: Value, input: Vec<Value>) -> Result<Value, String> {
        match self.apply_held(callable, input) {
            Ok(answer) => Ok(answer),
            Err(Escape::Error(message)) => Err(message),
            Err(raised) => { self.got_away = Some(raised); Err(self.bad_answer()) }
        }
    }
    fn warning_send(&mut self, receiver: Value, word: &str, input: Vec<Value>) -> Result<Value, String> {
        let operation = self.warning_get(receiver, word)?;
        self.warning_apply(operation, input)
    }
    fn warning_namespace(&mut self) -> Result<Value, String> {
        let words = self.table.strings("ext.system.warnings").to_vec();
        let previous = match self.warning_fallback.as_ref() {
            Some(Value::Thing(object)) => object.holds.borrow().clone(), _ => Vec::new(),
        };
        let mut namespace = vec![(String::from("__name__"), Value::text(&words[0]))];
        for serial in 1..words.len() {
            namespace.push((words[serial].clone(), Value::Intrinsic(Prim::WarningOperation((serial-1) as u8), Rc::from(words[serial].as_str()))));
        }
        let kinds = ["DeprecationWarning", "DeprecationWarning", "PendingDeprecationWarning", "ImportWarning", "ResourceWarning"];
        let mut defaults = Vec::with_capacity(5);
        for (index, word) in kinds.iter().enumerate() {
            let kind = self.builtin_named(word)?.ok_or_else(|| format!("NameError: {word}"))?;
            let action = if index == 0 { "default" } else { "ignore" };
            let module = if index == 0 { Value::text("__main__") } else { Value::Nil };
            defaults.push(Value::tuple(vec![Value::text(action), Value::Nil, kind, module, Value::Small(0)]));
        }
        namespace.push((String::from("filters"), Value::Vector(crate::tuples::Sequence::plain(defaults)).keep(true)));
        namespace.push((String::from("_defaultaction"), Value::text("default")));
        namespace.push((String::from("_onceregistry"), Value::Dict(Rc::new(Vec::new().into())).keep(true)));
        let variable = match previous.iter().find(|entry| entry.0 == "_warnings_context") {
            Some(entry) => entry.1.clone(),
            None => {
                let context_module = self.load_namespace("_contextvars")?;
                let context_type = self.warning_get(context_module, "ContextVar")?;
                self.warning_apply(context_type, vec![Value::text("_warnings_context")])?
            }
        };
        namespace.push((String::from("_warnings_context"), variable));
        for entry in namespace.iter_mut().filter(|entry| ["filters", "_defaultaction", "_onceregistry"].contains(&entry.0.as_str())) {
            if let Some(old) = previous.iter().find(|old| old.0 == entry.0) { entry.1 = old.1.clone(); }
        }
        let kind = self.native_kind("module"); self.made += 1;
        let result = Value::Thing(Rc::new(Thing { of: kind, reclassified: RefCell::new(None), holds: RefCell::new(namespace), turn: self.made }));
        self.warning_fallback = Some(result.clone());
        self.imported.insert(words[0].clone(), result.clone());
        self.namespace_places.borrow_mut().take();
        self.refresh_import_table(&words[0]);
        Ok(result)
    }
    fn warning_owner(&self) -> Result<Value, String> {
        match self.cached_import("warnings") {
            Some(value) if !matches!(value, Value::Nil) => Ok(value),
            _ => (if self.shutting_down { self.imported.get("warnings").cloned() } else { None }).or_else(|| self.warning_target.clone()).or_else(|| self.warning_fallback.clone()).ok_or_else(|| String::from("RuntimeError: warnings are not initialized")),
        }
    }
    fn warning_setting(&mut self, public_name: &str, native_name: &str) -> Result<Value, String> {
        let owner = self.warning_owner()?;
        if self.attribute(&owner, public_name).is_some() {
            let current = self.warning_get(owner, public_name)?;
            if let Some(Value::Thing(native)) = &self.warning_fallback {
                let mut fields = native.holds.borrow_mut();
                if let Some(entry) = fields.iter_mut().find(|entry| entry.0 == native_name) { entry.1 = current.clone(); }
            }
            return Ok(current);
        }
        let owner = self.warning_fallback.clone().ok_or_else(|| String::from("RuntimeError: warnings are not initialized"))?;
        self.warning_get(owner, native_name)
    }
    fn warning_operation(&mut self, task: u8, positional: Vec<Value>, keywords: Vec<(String, Value)>) -> Result<Value, String> {
        let bounds = self.extent;
        let result = self.warning_work(task, positional, keywords);
        self.extent = bounds;
        result
    }
    fn warning_work(&mut self, task: u8, positional: Vec<Value>, keywords: Vec<(String, Value)>) -> Result<Value, String> {
        if task > 1 {
            if !keywords.is_empty() { return Err(String::from("TypeError: native warnings operation takes no keyword arguments")); }
            let count = if task == 6 { 1 } else { 0 };
            if positional.len() != count { return Err(String::from("TypeError: wrong number of warning operation arguments")); }
            match task {
                3 if self.warning_lock_count == 0 => return Err(String::from("RuntimeError: warnings lock is not held")),
                2 | 3 => self.warning_epoch += 1,
                4 => self.warning_lock_count += 1,
                5 if self.warning_lock_count == 0 => return Err(String::from("RuntimeError: cannot release un-acquired lock")),
                5 => self.warning_lock_count -= 1,
                6 => self.warning_target = Some(positional[0].clone()),
                _ => return Err(String::from("RuntimeError: unknown warning operation")),
            }
            return Ok(Value::Nil);
        }
        let parameters = if task == 0 { vec!["message", "category", "stacklevel", "source", "skip_file_prefixes"] } else { vec!["message", "category", "filename", "lineno", "module", "registry", "module_globals", "source"] };
        let limit = if task == 0 { 4 } else { 8 };
        let required = if task == 0 { 1 } else { 4 };
        if positional.len() > limit { return Err(String::from("TypeError: too many warning arguments")); }
        let mut supplied = vec![None; parameters.len()];
        for (index, item) in positional.into_iter().enumerate() { supplied[index] = Some(item); }
        for (word, value) in keywords {
            let Some(index) = parameters.iter().position(|parameter| *parameter == word) else { return Err(format!("TypeError: '{word}' is an invalid keyword argument for warn{}()", if task == 0 { "" } else { "_explicit" })); };
            if supplied[index].is_some() { return Err(format!("TypeError: argument for warn() given by name ('{word}') and position ({})", index+1)); }
            supplied[index] = Some(value);
        }
        for index in 0..required {
            if supplied[index].is_none() { return Err(format!("TypeError: warn{}() missing required argument '{}' (pos {})", if task == 0 { "" } else { "_explicit" }, parameters[index], index+1)); }
        }
        if task == 1 {
            let mut explicit = Vec::with_capacity(8);
            for (index, item) in supplied.into_iter().enumerate() { explicit.push(item.unwrap_or(if index == 4 { Value::Unset } else { Value::Nil })); }
            if !matches!(explicit[2].settled(), Value::Text(_) | Value::Unpaired(_)) { return Err(String::from("TypeError: warn_explicit() argument 3 must be str")); }
            explicit[3] = Value::Small(self.warning_number(&explicit[3], 32)?);
            let source_line = self.warning_source_line(&explicit[6], &explicit[3])?;
            explicit.push(source_line);
            return self.warning_at(explicit);
        }
        let message = supplied[0].take().unwrap();
        let warning_kind = self.builtin_named("Warning")?.unwrap();
        let instance_check = self.builtin_named("isinstance")?.unwrap();
        let existing = self.warning_apply(instance_check, vec![message.clone(), warning_kind.clone()])?;
        let category = if self.stands_true(&existing) { self.warning_get(message.clone(), "__class__")? } else {
            supplied[1].take().filter(|v| !matches!(v, Value::Nil)).unwrap_or(self.builtin_named("UserWarning")?.unwrap())
        };
        if !matches!(category.settled(), Value::Blueprint(_)) { return Err(format!("TypeError: category must be a Warning subclass, not '{}'", category.kind_word())); }
        let subclass_check = self.builtin_named("issubclass")?.unwrap();
        let correct = self.warning_apply(subclass_check, vec![category.clone(), warning_kind])?;
        if !self.stands_true(&correct) { return Err(format!("TypeError: category must be a Warning subclass, not '{}'", category.kind_word())); }
        let level = supplied[2].take().unwrap_or(Value::Small(1)).settled();
        let mut distance = self.warning_number(&level, 64)?.max(1);
        let prefixes = supplied[4].take().unwrap_or_else(|| Value::tuple(Vec::new())).settled();
        let Value::Tuple(prefixes) = prefixes else { return Err(String::from("TypeError: skip_file_prefixes must be a tuple of strs.")); };
        let mut omitted = Vec::new();
        for item in prefixes.iter() {
            match item.settled() { Value::Text(word) => omitted.push(word.to_string()), _ => return Err(String::from("TypeError: skip_file_prefixes must be a tuple of strs.")) }
        }
        if !omitted.is_empty() { distance = distance.max(2); }
        let mut caller = self.active_trace.clone();
        let internal = |path: &str| path.contains("importlib") && path.contains("_bootstrap");
        let from_internal = caller.as_ref().is_some_and(|frame| match &frame.holds.borrow()[5].1 { Value::Bound(body, _) => body.written_in.as_ref().is_some_and(|path| internal(path)), _ => false });
        while distance > 1 {
            distance -= 1;
            caller = caller.and_then(|frame| match &frame.holds.borrow()[2].1 { Value::Thing(parent) => Some(parent.clone()), _ => None });
            if from_internal { continue; }
            while caller.as_ref().is_some_and(|frame| match &frame.holds.borrow()[5].1 { Value::Bound(body, _) => body.written_in.as_ref().is_some_and(|path| internal(path) || omitted.iter().any(|prefix| path.starts_with(prefix))), _ => false }) {
                caller = caller.and_then(|frame| match &frame.holds.borrow()[2].1 { Value::Thing(parent) => Some(parent.clone()), _ => None });
            }
        }
        let (world, filename, line) = match caller {
            Some(frame) => {
                let globals = self.activation_member(&Value::Thing(frame.clone()), "f_globals").unwrap_or(Value::Nil);
                let holds = frame.holds.borrow();
                let filename = match &holds[5].1 { Value::Bound(body, _) => body.written_in.as_ref().map(|s| Value::text(s)).unwrap_or_else(|| Value::text(&self.entry_file)), _ => Value::text(&self.entry_file) };
                (globals, filename, holds[0].1.clone())
            }
            None => { let sys = match self.imported.get("sys").cloned() { Some(sys) => sys, None => self.load_namespace("sys")? }; (self.warning_get(sys, "__dict__")?, Value::text("<sys>"), Value::Small(0)) }
        };
        let module = self.warning_send(world.clone(), "get", vec![Value::text("__name__"), Value::text("<string>")])?;
        let empty = Value::Dict(Rc::new(Vec::new().into())).keep(true);
        let registry = self.warning_send(world.clone(), "setdefault", vec![Value::text("__warningregistry__"), empty])?;
        let source = supplied[3].take().unwrap_or(Value::Nil);
        self.warning_at(vec![message, category, filename, line, module, registry, world, source])
    }
    fn warning_at(&mut self, mut input: Vec<Value>) -> Result<Value, String> {
        if !matches!(input[2].settled(), Value::Text(_) | Value::Unpaired(_)) { return Err(String::from("TypeError: warn_explicit() argument 3 must be str")); }
        let line = self.warning_number(&input[3], 32)?;
        if i32::try_from(line).is_err() { return Err(String::from("OverflowError: Python int too large to convert to C int")); }
        input[3] = Value::Small(line);
        if matches!(input[4], Value::Unset) {
            input[4] = match input[2].settled() {
                Value::Unpaired(codepoints) if codepoints.ends_with(&[46,112,121]) => Value::Unpaired(codepoints[..codepoints.len()-3].to_vec().into()),
                Value::Unpaired(_) => input[2].clone(),
                _ => { let path = input[2].bare(); Value::text(if path.is_empty() { "<unknown>" } else { path.strip_suffix(".py").unwrap_or(&path) }) },
            };
        }
        if matches!(input[4], Value::Nil) { return Ok(Value::Nil); }
        let registry = input[5].clone();
        if !matches!(registry.settled(), Value::Nil | Value::Dict(_) | Value::Attributes(_)) { return Err(String::from("TypeError: 'registry' must be a dict or None")); }
        let warning_kind = self.builtin_named("Warning")?.unwrap();
        let test = self.builtin_named("isinstance")?.unwrap();
        let is_instance = self.warning_apply(test, vec![input[0].clone(), warning_kind])?;
        let message; let text; let category;
        if self.stands_true(&is_instance) {
            message = input[0].clone(); category = self.warning_get(message.clone(), "__class__")?;
            let stringify = self.builtin_named("str")?.unwrap(); text = self.warning_apply(stringify, vec![message.clone()])?;
        } else { text = input[0].clone(); category = input[1].clone(); message = self.warning_apply(category.clone(), vec![text.clone()])?; }
        let key = Value::tuple(vec![text.clone(), category.clone(), input[3].clone()]);
        let local = !matches!(registry.settled(), Value::Nil);
        if local && self.warning_registry(&registry, key.clone(), false)? { return Ok(Value::Nil); }
        let owner = self.warning_owner()?;
        let aware = self.attribute(&owner, "_use_context").is_some_and(|flag| self.stands_true(&flag));
        let filters = if aware { self.warning_send(owner.clone(), "_get_filters", Vec::new())? } else { self.warning_setting("filters", "filters")? };
        let Value::Vector(entries) = filters.settled() else { return Err(String::from("ValueError: warnings.filters must be a list")); };
        let mut selected = None;
        let mut matching = Value::Nil;
        for (ordinal, entry) in entries.iter().enumerate() {
            let Value::Tuple(rule) = entry.settled() else { return Err(format!("ValueError: _warnings.filters item {ordinal} isn't a 5-tuple")); };
            if rule.len() != 5 { return Err(format!("ValueError: _warnings.filters item {ordinal} isn't a 5-tuple")); }
            if !matches!(rule[0].settled(), Value::Text(_)) { return Err(format!("TypeError: action must be a string, not '{}'", rule[0].kind_word())); }
            let accepts_message = self.warning_pattern(&rule[1], &text)?;
            let accepts_module = self.warning_pattern(&rule[3], &input[4])?;
            let subclass = self.builtin_named("issubclass")?.unwrap();
            let covered = self.warning_apply(subclass, vec![category.clone(), rule[2].clone()])?;
            let filter_line = self.warning_number(&rule[4], 64)?;
            if !(accepts_message && accepts_module && self.stands_true(&covered)) || (filter_line != 0 && filter_line != line) { continue; }
            selected = Some(rule[0].clone()); matching = entry.clone(); break;
        }
        let action = match selected { Some(value) => value, None => self.warning_setting("defaultaction", "_defaultaction")? };
        if !matches!(action.settled(), Value::Text(_)) { return Err(format!("TypeError: warnings.defaultaction must be a string, not '{}'", action.kind_word())); }
        let word = action.bare();
        if word == "ignore" { return Ok(Value::Nil); }
        if word == "error" { self.got_away = Some(Escape::Thrown(message)); return Err(self.bad_answer()); }
        if word != "always" && word != "all" {
            if local { self.warning_send(registry.clone(), "__setitem__", vec![key, Value::Flag(true)])?; }
            match word.as_str() {
                "default" => {},
                "module" | "once" => {
                    let target = if local { registry } else if word == "once" { self.warning_setting("onceregistry", "_onceregistry")? } else { Value::Nil };
                    if !matches!(target.settled(), Value::Nil) {
                        if !matches!(target.settled(), Value::Dict(_)) { return Err(format!("TypeError: warnings.onceregistry must be a dict, not '{}'", target.kind_word())); }
                        if self.warning_registry(&target, Value::tuple(vec![text.clone(), category.clone()]), true)? { return Ok(Value::Nil); }
                    }
                }
                _ => { let repr = self.builtin_named("repr")?.unwrap(); let shown_action = self.warning_apply(repr.clone(), vec![action])?.bare(); let shown_filter = self.warning_apply(repr, vec![matching])?.bare(); return Err(format!("RuntimeError: Unrecognized action ({shown_action}) in warnings.filters:\n {shown_filter}")); }
            }
        }
        if self.attribute(&owner, "_showwarnmsg").is_none() { return self.warning_fallback_display(&text, &category, &input); }
        let record_type = self.warning_get(owner.clone(), "WarningMessage")?;
        let source_line = input.get(8).cloned().unwrap_or(Value::Nil);
        let record = self.warning_apply(record_type, vec![message, category, input[2].clone(), input[3].clone(), Value::Nil, source_line, input[7].clone()])?;
        self.warning_send(owner, "_showwarnmsg", vec![record])?;
        Ok(Value::Nil)
    }
    fn warning_number(&mut self, argument: &Value, bits: u8) -> Result<i64, String> {
        let settled = argument.settled();
        let whole = if matches!(settled, Value::Small(_) | Value::Huge(_) | Value::Flag(_)) { settled } else {
            match self.stood_for_whole(argument)? { Some(value) => value, None => return Err(format!("TypeError: '{}' object cannot be interpreted as an integer", argument.kind_word())) }
        };
        let complaint = if bits == 32 { "OverflowError: Python int too large to convert to C int" } else { "OverflowError: Python int too large to convert to C ssize_t" };
        let value = match whole { Value::Small(number) => number, Value::Flag(flag) => i64::from(flag), Value::Huge(number) => number.to_i64().ok_or_else(|| complaint.to_owned())?, _ => return Err(complaint.to_owned()) };
        if bits == 32 && i32::try_from(value).is_err() { return Err(complaint.to_owned()); }
        Ok(value)
    }
    fn warning_fallback_display(&mut self, message: &Value, kind: &Value, arguments: &[Value]) -> Result<Value, String> {
        let class_name = self.warning_get(kind.clone(), "__name__")?.bare();
        let stringify = self.builtin_named("str")?.unwrap();
        let contents = self.warning_apply(stringify, vec![message.clone()])?.bare();
        let mut written = format!("{}:{}: {}: {}\n", arguments[2].bare(), arguments[3].bare(), class_name, contents);
        let source = arguments.get(8).filter(|source| !matches!(source, Value::Nil));
        match source {
            Some(line) => { written.push_str(&line.bare()); written.push('\n'); }
            None => if let (Value::Text(path), Value::Small(line)) = (arguments[2].settled(), arguments[3].settled()) {
                if line > 0 {
                    let source = std::fs::read_to_string(path.as_ref()).ok();
                    if let Some(source) = source {
                        if let Some(text) = source.lines().nth(line as usize-1) { written += "  "; written += text.trim_start_matches([' ', '\t', '\x0c']); written.push('\n'); }
                    }
                }
            }
        }
        if let Some(system) = self.imported.get("sys").cloned() {
            if let Some(destination) = self.attribute(&system, "stderr") {
                if !matches!(destination.settled(), Value::Nil) { let _ = self.warning_send(destination, "write", vec![Value::text(&written)]); self.got_away = None; }
            }
        }
        Ok(Value::Nil)
    }
    fn warning_source_line(&mut self, namespace: &Value, row: &Value) -> Result<Value, String> {
        if matches!(namespace.settled(), Value::Nil) { return Ok(Value::Nil); }
        if !matches!(namespace.settled(), Value::Dict(_) | Value::Attributes(_)) { return Err(format!("TypeError: module_globals must be a dict, not '{}'", namespace.kind_word())); }
        let bootstrap = self.load_namespace("importlib._bootstrap_external")?;
        let loader = self.warning_send(bootstrap, "_bless_my_loader", vec![namespace.clone()])?;
        if self.attribute(&loader, "get_source").is_none() { return Ok(Value::Nil); }
        let specification = self.warning_send(namespace.clone(), "get", vec![Value::text("__spec__"), Value::Nil])?;
        let spec_name = self.attribute(&specification, "name").filter(|name| !matches!(name, Value::Nil));
        let name = if let Some(name) = spec_name { name } else { self.warning_send(namespace.clone(), "get", vec![Value::text("__name__"), Value::Nil])? };
        if matches!(name, Value::Nil) { return Ok(Value::Nil); }
        let code = match self.warning_send(loader, "get_source", vec![name]) {
            Ok(code) => code,
            Err(message) if self.got_away.as_ref().map_or(message.starts_with("ImportError:"), |raised| self.escape_names(raised, "ImportError")) => { self.got_away = None; return Ok(Value::Nil); }
            Err(message) => return Err(message),
        };
        if matches!(code.settled(), Value::Nil) { return Ok(Value::Nil); }
        let content = Self::underlying(&code).unwrap_or_else(|| code.settled());
        if !matches!(content, Value::Text(_) | Value::Unpaired(_)) { return Err(format!("TypeError: bad argument type for built-in operation: {}", content.kind_word())); }
        let split = self.prim(Prim::Textual(crate::text::Work::SPLITLINES), "splitlines", &[content])?;
        let Value::Vector(lines) = split.settled() else { return Ok(Value::Nil); };
        let Value::Small(line) = row.settled() else { return Err(String::from("TypeError: 'lineno' must be an integer")); };
        if line <= 0 { return Ok(Value::Nil); }
        Ok(lines.get(line as usize-1).cloned().unwrap_or(Value::Nil))
    }
    fn warning_pattern(&mut self, pattern: &Value, message: &Value) -> Result<bool, String> {
        if matches!(pattern.settled(), Value::Nil) { return Ok(true); }
        if matches!(pattern.settled(), Value::Text(_)) { return Ok(pattern.equals(message)); }
        let answer = self.warning_send(pattern.clone(), "match", vec![message.clone()])?;
        Ok(self.stands_true(&answer))
    }
    fn warning_registry(&mut self, registry: &Value, key: Value, write: bool) -> Result<bool, String> {
        let epoch = self.warning_send(registry.clone(), "get", vec![Value::text("version"), Value::Nil])?;
        if !matches!(epoch.settled(), Value::Small(value) if value == self.warning_epoch) {
            self.warning_send(registry.clone(), "clear", Vec::new())?;
            self.warning_send(registry.clone(), "__setitem__", vec![Value::text("version"), Value::Small(self.warning_epoch)])?;
        }
        let present = self.warning_send(registry.clone(), "get", vec![key.clone(), Value::Nil])?;
        if self.stands_true(&present) { return Ok(true); }
        if write { self.warning_send(registry.clone(), "__setitem__", vec![key, Value::Flag(true)])?; }
        Ok(false)
    }
}

impl Machine<'_> {
    fn import_from_finders(&mut self, qualified: &str) -> Result<Option<Value>, String> {
        let Some(system) = self.imported.get("sys").cloned() else { return Ok(None); };
        if self.attribute(&system, "meta_path").is_none() { return Ok(None); }
        let searchers = self.warning_get(system, "meta_path")?;
        let location = match qualified.rsplit_once('.') {
            Some((ancestor, _)) => {
                let parent = self.load_namespace(ancestor)?;
                if self.cached_import(qualified).is_some() { return self.load_namespace(qualified).map(Some); }
                if self.attribute(&parent, "__path__").is_some() { self.warning_get(parent, "__path__")? }
                else { match self.read_class_member(parent, "__path__", false) {
                    Ok(value) => value, Err(fault) if self.missing_member_escape(&fault) => Value::Nil,
                    Err(fault) => return Err(self.carried_native_fault(fault)),
                } }
            }
            None => Value::Nil,
        };
        let searchers = self.gathered_members(&searchers)?;
        self.finding_import.insert(qualified.to_owned());
        let answer = (|| {
            for searcher in searchers {
                // The compiled-in source roster uses its own loader, after custom finders.
                if self.library_sources.contains_key(qualified) || self.sys_path_source(qualified).is_some() {
                    if let Value::Thing(instance) = searcher.settled() {
                        let kind = instance.blueprint();
                        if kind.name.rsplit('.').next() == Some("SourceFinder")
                            && self.inherited_entry(&kind, "__module__").is_some_and(|module| module.bare() == "_runtime_import") { continue; }
                    }
                    if let Value::Blueprint(kind) = searcher.settled() {
                        if matches!(kind.name.rsplit('.').next(), Some("BuiltinImporter" | "FrozenImporter" | "PathFinder")) { continue; }
                    }
                }
                let builtin_searcher = self.imported.get("_imp").and_then(|module| match module { Value::Thing(object) => object.holds.borrow().iter().find(|field| field.0 == "_NativeFinder").map(|field| field.1.clone()), _ => None });
                if builtin_searcher.is_some_and(|native| native.settled().equals(&searcher.settled())) {
                    let exists = self.table.strings("ext.system.warnings").first().is_some_and(|word| word == qualified) || self.library_sources.contains_key(qualified) || self.sys_path_source(qualified).is_some();
                    if exists { return Ok(None); }
                    continue;
                }
                let find_spec = self.core_primitive(Prim::GetMember, "getattr", vec![searcher, Value::text("find_spec"), Value::Nil], Vec::new())?;
                if matches!(find_spec.settled(), Value::Nil) { continue; }
                let specification = self.warning_apply(find_spec, vec![Value::text(qualified), location.clone(), Value::Nil])?;
                if matches!(specification.settled(), Value::Nil) { continue; }
                let bootstrap_module = self.load_namespace("importlib._bootstrap")?;
                // Let the upstream load transaction choose the final public cache entry.
                let completion = self.warning_send(bootstrap_module, "_load_unlocked", vec![specification]);
                if completion.is_err() { self.imported.remove(qualified); }
                self.namespace_places.borrow_mut().take();
                let namespace = completion?.settled();
                self.imported.insert(qualified.to_owned(), namespace.clone());
                if let Some((ancestor, member)) = qualified.rsplit_once('.') {
                    if let Some(parent) = self.cached_import(ancestor).or_else(|| self.imported.get(ancestor).cloned()) {
                        self.alter_class_member(parent, member, Some(namespace.clone()), false).map_err(|fault| self.carried_native_fault(fault))?;
                    }
                }
                return Ok(Some(namespace));
            }
            Ok(None)
        })();
        self.finding_import.remove(qualified);
        answer
    }
}

impl Machine<'_> {
    fn source_namespace_metadata(&mut self, qualified: &str, value: &Value) -> Result<(), String> {
        if !self.table.has_any("ext.system.module.path") { return Ok(()); }
        let read_export = |module: &Value, wanted: &str| match module {
            Value::Thing(object) => object.holds.borrow().iter().find(|entry| entry.0 == wanted).map(|entry| entry.1.settled()), _ => None,
        };
        let Some(native_loader @ Value::Blueprint(_)) = self.imported.get("_imp").and_then(|module| read_export(module, "_NativeLoader")) else { return Ok(()); };
        let constructor = self.imported.get("importlib._bootstrap").and_then(|bootstrap| read_export(bootstrap, "ModuleSpec"));
        let spec = match constructor {
            Some(callable @ Value::Blueprint(_)) => self.core_run(&callable, vec![Value::text(qualified), native_loader.clone()])?,
            _ => Value::Nil,
        };
        if let Value::Thing(namespace) = value {
            let mut holds = namespace.holds.borrow_mut();
            for entry in holds.iter_mut() {
                match entry.0.as_str() { "__loader__" => entry.1 = native_loader.clone(), "__spec__" => entry.1 = spec.clone(), _ => {} }
            }
        }
        Ok(())
    }
}

impl Machine<'_> {
    fn trace_allocation(&mut self, subject: &Value) {
        if self.allocation_limit == 0 { return; }
        if let Value::Thing(instance) = subject {
            let mut positions = Vec::with_capacity(self.allocation_limit.min(8));
            let mut activation = self.active_trace.clone();
            while let Some(current) = activation.take() {
                let fields = current.holds.borrow();
                let file = match &fields[5].1 { Value::Bound(code, _) => code.written_in.as_deref().unwrap_or(&self.entry_file), _ => &self.entry_file };
                let counterpart = ["/modules/builtins.py", "/modules/_pyio.py", "/modules/_io.py"].iter().any(|suffix| file.ends_with(suffix));
                if !counterpart {
                    if let Value::Small(number) = fields[0].1.settled() { positions.push((file.to_owned(), number)); }
                }
                if positions.len() >= self.allocation_limit { break; }
                if let Value::Thing(previous) = &fields[2].1 { activation = Some(previous.clone()); }
            }
            positions.reverse();
            let address = Rc::as_ptr(instance) as usize;
            if self.made & 63 == 0 { self.allocations.retain(|_, record| record.0.strong_count() > 0); }
            self.allocations.insert(address, (Rc::downgrade(instance), positions));
        }
    }
    fn allocation_operation(&mut self, values: &[Value]) -> Result<Value, String> {
        let task = values.first().map(Value::settled);
        let answer = match task {
            Some(Value::Small(0)) if values.len() == 2 => {
                if let Value::Thing(object) = values[1].settled() {
                    self.allocations.get(&(Rc::as_ptr(&object) as usize)).filter(|(weak, _)| weak.strong_count() != 0)
                        .map(|(_, entries)| Value::tuple(entries.iter().map(|entry| Value::tuple(vec![Value::text(&entry.0), Value::Small(entry.1)])).collect())).unwrap_or(Value::Nil)
                } else { Value::Nil }
            }
            Some(Value::Small(1)) if values.len() == 2 => {
                let Value::Small(depth) = values[1].settled() else { return Err(String::from("TypeError: traceback depth must be an integer")); };
                if !(1..=65535).contains(&depth) { return Err(String::from("ValueError: the number of frames must be in range [1; 65535]")); }
                if self.allocation_limit == 0 { self.allocation_limit = depth as usize; self.last_allocation_limit = self.allocation_limit; } Value::Nil
            }
            Some(Value::Small(2)) if values.len() == 1 => { self.allocations.clear(); self.allocation_limit = 0; Value::Nil }
            Some(Value::Small(3)) if values.len() == 1 => Value::Flag(self.allocation_limit != 0),
            Some(Value::Small(4)) if values.len() == 1 => Value::Small(self.last_allocation_limit as i64),
            _ => return Err(String::from("ValueError: invalid allocation tracing operation")),
        };
        Ok(answer)
    }
}
