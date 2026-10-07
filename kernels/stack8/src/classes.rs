// Classes whose ancestry is an ordered company, and the members which
// belong to classes and routines rather than to their callers.
use super::*;

impl<'a> Engine<'a> {
    pub(super) fn class_word(&self, part: &str) -> &str {
        match part {
            "name" => self.member_words[0],
            "qualified" => self.member_words[1],
            "module" => self.member_words[2],
            "kind" => self.member_words[3],
            "get" => self.member_words[4],
            "set" => self.member_words[5],
            "remove" => self.member_words[6],
            "call" => self.member_words[7],
            "namespace" => self.member_words[8],
            "root" => self.member_words[9],
            "allocate" => self.member_words[10],
            "descriptor.get" => self.member_words[11],
            "descriptor.set" => self.member_words[12],
            "descriptor.delete" => self.member_words[13],
            "doc" => self.member_words[14],
            "defaults" => self.member_words[15],
            "base" => self.member_words[16],
            "bases" => self.member_words[17],
            "mro" => self.member_words[18],
            "order" => self.member_words[19],
            "receiver" => self.member_words[20],
            "unready" => self.member_words[21],
            _ => self.lang.class_details.get(part).and_then(|v| v.first()).map_or("", String::as_str),
        }
    }
    // Read once when the language itself was read, since no program
    // still running can change which words a class stands under: every
    // step of every program asks this, so it stands as a field on the
    // language rather than a hashmap looked into afresh each time.
    pub(super) fn fuller_classes(&self) -> bool { self.lang.fuller_classes }
    fn class_refusal(&self) -> Fault { self.class_word("unready").to_string().into() }
    pub(super) fn root_class(&mut self) -> Rc<Class> {
        if let Some(c) = &self.class_root { return c.clone(); }
        let name = self.class_word("root").to_string();
        let c = Rc::new(Class { outline: Some(format!("<class '{name}'>")), name,
            direct: vec![], lineage: RefCell::new(vec![]), base: None, answers: vec![], fields: vec![], reaches: vec![],
            methods: vec![], constants: vec![], shared: RefCell::new(vec![]), weak_storage: std::cell::Cell::new(None), declares_slots: false, sealed: std::cell::Cell::new(false), mro_adopted: std::cell::Cell::new(false), adopted_order: RefCell::new(Vec::new()), python_names: std::cell::RefCell::new(None) });
        self.class_root = Some(c.clone());
        c
    }
    /// The class standing for a builtin kind, made once for each word
    /// the definition names: a thing of a class beneath it keeps a worth
    /// of that kind among its members, under a name no program can spell.
    pub(super) fn kind_class(&mut self, word: &str) -> Rc<Class> {
        if self.lang.builtins.get(word) == Some(&Builtin::ClassTool(11)) && !self.class_word("descriptor.get").is_empty() { return self.property_class(); }
        if let Some((_, c)) = self.kind_classes.iter().find(|(w, _)| w == word) { return c.clone(); }
        let mut root = self.root_class();
        if self.lang.builtins.get(word) == Some(&Builtin::Bool) {
            if let Some(integer) = self.lang.builtin_words.iter().find(|(op, _)| *op == Builtin::ToInt).map(|(_, word)| word.clone()) {
                root = self.kind_class(&integer);
            }
        }
        let mut ancestry = vec![root.clone()]; ancestry.extend(root.lineage.borrow().iter().cloned());
        let mut hooks = Vec::new();
        if matches!(self.lang.builtins.get(word), Some(Builtin::Zip | Builtin::Map | Builtin::Filter)) {
            for (place, mode) in [(15, 4), (16, 3)] {
                if let Some(name) = self.lang.class_special.get(place) { hooks.push((name.clone(), Self::adapter(119, vec![Value::Small(mode)]))); }
            }
        }
        if matches!(self.lang.builtins.get(word), Some(Builtin::Set | Builtin::Frozen | Builtin::Enumerate | Builtin::Zip | Builtin::Map | Builtin::Filter | Builtin::Reversed)) {
            if let Some(name) = self.lang.class_special.get(79) { hooks.push((name.clone(), Self::adapter(133, vec![Value::text(word)]))); }
        }
        if word == "Union" {
            hooks.push(("__repr__".into(), Self::adapter(126, Vec::new())));
            // The reference builds a union by subscription, which the
            // special form's own item method does; the folded members
            // are the very shape the class questions read a union by.
            hooks.push(("__class_getitem__".into(), Self::adapter(140, Vec::new())));
        }
        if self.lang.bind_names && matches!(word, "getset_descriptor" | "member_descriptor") {
            for part in ["descriptor.get", "descriptor.set", "descriptor.delete"] {
                let name = self.class_word(part).to_string();
                if !name.is_empty() { hooks.push((name.clone(), Self::adapter(29, vec![Value::text(word), Value::text(&name)]))); }
            }
        }
        if self.lang.bind_names && word == "code" {
            let mut fields = self.lang.class_details.get("code.fields").cloned().unwrap_or_default();
            fields.extend([self.class_word("argcount").to_string(), self.class_word("varnames").to_string()]);
            for name in fields.into_iter().filter(|name| name.starts_with("co_")) {
                hooks.push((name.clone(), Self::adapter(29, vec![Value::text(word), Value::text(&name)])));
            }
        }
        if word == "GenericAlias" {
            hooks.push(("__iter__".into(), Self::adapter(124, vec![Value::Small(5)])));
            for (name, mode) in [(self.class_word("allocate").to_string(), 0), ("__repr__".to_string(), 1),
                ("__call__".to_string(), 2), ("__mro_entries__".to_string(), 3), ("__eq__".to_string(), 4)] {
                hooks.push((name, Self::adapter(124, vec![Value::Small(mode)])));
            }
        }
        if word == "SimpleNamespace" {
            for (name, mode) in [(self.class_word("allocate").to_string(), 0), (self.lang.constructor.clone().unwrap_or_default(), 1),
                                 ("__repr__".to_string(), 2), ("__eq__".to_string(), 3), ("__ne__".to_string(), 4),
                                 ("__reduce__".to_string(), 5), ("__replace__".to_string(), 6)] {
                hooks.push((name, Self::adapter(122, vec![Value::Small(mode)])));
            }
            hooks.push(("__hash__".to_string(), Value::Null));
        }
        if word == "super" {
            hooks.push((self.lang.constructor.clone().unwrap_or_default(), Self::adapter(236, Vec::new())));
        }
        let public = if matches!(word, "TypeVar" | "ParamSpec" | "TypeVarTuple" | "TypeAliasType" | "Generic" | "NoDefaultType" | "ParamSpecArgs" | "ParamSpecKwargs") { format!("typing.{word}") } else if matches!(word, "SimpleNamespace" | "GenericAlias") { format!("types.{word}") } else if word == "Union" { String::from("typing.Union") } else { word.to_owned() };
        let c = Rc::new(Class { outline: Some(format!("<class '{public}'>")), name: word.to_string(),
            direct: vec![root.clone()], lineage: RefCell::new(ancestry), base: Some(root), answers: vec![], fields: vec![], reaches: vec![],
            methods: vec![], constants: vec![("\0kind".to_string(), Value::text(word))], shared: RefCell::new(hooks), weak_storage: std::cell::Cell::new(None), declares_slots: false, sealed: std::cell::Cell::new(false), mro_adopted: std::cell::Cell::new(false), adopted_order: RefCell::new(Vec::new()), python_names: std::cell::RefCell::new(None) });
        if !self.lang.class_builder.is_empty() && matches!(self.lang.builtins.get(word), Some(Builtin::ClassTool(9..=10))) {
            c.shared.borrow_mut().push((self.class_word("descriptor.get").to_string(), Self::adapter(79, vec![])));
            if self.lang.builtins.get(word) == Some(&Builtin::ClassTool(9)) {
                c.shared.borrow_mut().push((self.class_word("call").to_string(), Self::adapter(80, vec![])));
            }
        }
        if word == "classmethod_descriptor" && !self.class_word("descriptor.get").is_empty() {
            c.shared.borrow_mut().push((self.class_word("descriptor.get").to_owned(), Self::adapter(235, Vec::new())));
        }
        if word == "module" && !self.class_word("kind").is_empty() {
            if let Some(init) = &self.lang.constructor {
                c.shared.borrow_mut().push((init.clone(), Self::adapter(2, vec![Value::text("module")])));
            }
        }
        if word == "module" && !self.class_word("kind").is_empty() {
            c.shared.borrow_mut().push((self.class_word("namespace").to_string(), Self::adapter(16,
                vec![Value::text(self.class_word("namespace")), Value::Class(c.clone()), Value::text("\0module-namespace")])));
            if let Some(repr) = self.lang.class_special.get(1) {
                c.shared.borrow_mut().push((repr.clone(), Self::adapter(29, vec![Value::text(word), Value::text(repr)])));
            }
        }
        if self.lang.bind_names && word == "function" {
            c.shared.borrow_mut().push((self.class_word("descriptor.get").to_owned(), Self::adapter(15, vec![])));
        }
        if self.lang.bind_names && matches!(word, "getset_descriptor" | "member_descriptor" | "method_descriptor" | "wrapper_descriptor" | "classmethod_descriptor") {
            let get = self.class_word("descriptor.get").to_owned();
            c.shared.borrow_mut().push((get.clone(), self.held_kind_descriptor(word, &get)));
        }
        self.kind_classes.push((word.to_string(), c.clone()));
        if matches!(word, "function" | "builtin_function_or_method" | "method" | "method_descriptor" | "wrapper_descriptor" | "type" | "NoneType") {
            for name in self.lang.class_special.iter().enumerate().filter(|(at, _)| *at == 8 || *at == 17 && word != "NoneType").map(|(_, name)| name) {
                c.shared.borrow_mut().push((name.clone(), self.held_kind_descriptor(word, name)));
            }
        }
        if matches!(word, "int" | "float" | "str" | "tuple" | "bytes" | "bytearray" | "dict" | "set" | "frozenset" | "complex" | "list") {
            let allocation = self.class_word("allocate").to_string();
            c.shared.borrow_mut().push((allocation, self.native_allocator(word)));
        }
        if Lang::spells(&self.lang.builtin_bases, "bytes") && matches!(word, "bytes" | "bytearray") {
            c.shared.borrow_mut().push(("__buffer__".into(), self.held_kind_descriptor(word, "__buffer__")));
            if word == "bytearray" { c.shared.borrow_mut().push(("__release_buffer__".into(), self.held_kind_descriptor(word, "__release_buffer__"))); }
        }
        if let Some(sample) = self.kind_sample(word) {
            let names = self.kind_member_names(&sample);
            for name in names {
                if matches!(name.as_str(), "start" | "stop" | "step" | "real" | "imag" | "numerator" | "denominator" | "__reduce_ex__" | "__dir__") { continue; }
                if word == "generator" && name == "__setstate__" { continue; }
                if self.lang.builtins.get(word) == Some(&Builtin::Frozen) && self.lang.constructor.as_deref() == Some(name.as_str()) { continue; }
                if c.shared.borrow().iter().any(|(key, _)| key == &name) { continue; }
                if let Some(descriptor) = self.loose_kind_member(&Value::Class(c.clone()), &name) {
                    c.shared.borrow_mut().push((name, descriptor));
                }
            }
        }
        if word == "bytearray" {
            if let Some(operation) = self.lang.builtins.get("bytearray.maketrans") {
                c.shared.borrow_mut().push((String::from("maketrans"), Self::adapter(4, vec![Value::Native(*operation, Rc::from("bytearray.maketrans"))])));
            }
        }
        if word == "dict" {
            let descriptor = Self::adapter(29, vec![Value::text("dict"), Value::text("fromkeys"), Value::Flag(true)]);
            let mut shared = c.shared.borrow_mut();
            if let Some((_, member)) = shared.iter_mut().find(|(name, _)| name == "fromkeys") { *member = descriptor; }
            else { shared.push((String::from("fromkeys"), descriptor)); }
        }
        c
    }
    /// The class every metaclass stands on: the kind builtin read as a
    /// class. A class standing on it is a metaclass, and the things it
    /// makes are classes rather than objects.
    pub(super) fn metaclass_root(&mut self) -> Rc<Class> {
        if let Some(c) = &self.class_maker { return c.clone(); }
        let root = self.root_class();
        let name = self.lang.builtins.iter().find(|(_, b)| **b == Builtin::SortOf).map_or(String::new(), |(w, _)| w.clone());
        let c = Rc::new(Class { outline: Some(format!("<class '{name}'>")), name,
            direct: vec![root.clone()], lineage: RefCell::new(vec![root.clone()]), base: Some(root), answers: vec![], fields: vec![], reaches: vec![],
            methods: vec![], constants: vec![], shared: RefCell::new(vec![]), weak_storage: std::cell::Cell::new(None), declares_slots: false, sealed: std::cell::Cell::new(false), mro_adopted: std::cell::Cell::new(false), adopted_order: RefCell::new(Vec::new()), python_names: std::cell::RefCell::new(None) });
        self.class_maker = Some(c.clone());
        for detail in ["mro", "namespace", "order", "name"] {
            let key = self.class_word(detail);
            if !key.is_empty() { c.shared.borrow_mut().push((key.to_string(), self.held_kind_descriptor("type", key))); }
        }
        for at in [8, 17] { if let Some(name) = self.lang.class_special.get(at) {
            c.shared.borrow_mut().push((name.clone(), self.held_kind_descriptor("type", name)));
        } }
        c
    }
    pub(super) fn is_metaclass_root(&self, c: &Rc<Class>) -> bool {
        self.class_maker.as_ref().map_or(false, |m| Rc::ptr_eq(m, c))
    }
    /// The metaclass a class was made by, where its header named one.
    fn own_maker(c: &Class) -> Option<Rc<Class>> {
        match c.constants.iter().find(|(n, _)| n == MAKER_MEMBER).map(|(_, v)| v) {
            Some(Value::Class(m)) => Some(m.clone()),
            _ => None,
        }
    }
    /// That metaclass, through the class's whole line: a class is made by
    /// the metaclass of the nearest forebear that named one.
    pub(super) fn maker_beneath(c: &Class) -> Option<Rc<Class>> {
        std::iter::once(c).chain(c.lineage.borrow().iter().map(Rc::as_ref)).find_map(Self::own_maker)
    }
    /// The metaclass that will make a class: the one its header named,
    /// else the one its forebears were made by. Where both speak, the
    /// one standing on the other is taken, as the deeper answer.
    pub(super) fn maker_in_force(&mut self, asked: Option<Value>, bases: &[Rc<Class>]) -> Flow<Option<Rc<Class>>> {
        let named = match asked.map(|v| v.contents()) {
            None => None,
            // The kind builtin names the plainest maker there is, which
            // is no metaclass of its own; so does the class it stands for.
            Some(Value::Native(Builtin::SortOf, _)) => None,
            Some(Value::Class(c)) if self.is_metaclass_root(&c) => None,
            Some(Value::Class(c)) => Some(c),
            Some(other) => return Err(self.core_fault("core.uncallable", &other.core_kind()).into()),
        };
        let mut winner = named;
        for inherited in bases.iter().filter_map(|b| Self::maker_beneath(b)) {
            match &winner {
                None => winner = Some(inherited),
                Some(current) if Rc::ptr_eq(current, &inherited) || current.lineage.borrow().iter().any(|b| Rc::ptr_eq(b, &inherited)) => {},
                Some(current) if inherited.lineage.borrow().iter().any(|b| Rc::ptr_eq(b, current)) => winner = Some(inherited),
                _ => return Err("TypeError: metaclass conflict: the metaclass of a derived class must be a (non-strict) subclass of the metaclasses of all its bases".into()),
            }
        }
        Ok(winner)
    }
    /// The reference's own docstring for a builtin kind's word, where
    /// this runtime keeps one. Nothing for a word left undocumented,
    /// which reads as the kind having no attribute of that name.
    pub(super) fn builtin_kind_doc(word: &str) -> Option<&'static str> {
        match word {
            "enumerate" => Some("Return an enumerate object.\n\n  iterable\n    an object supporting iteration\n\nThe enumerate object yields pairs containing a count (from start, which\ndefaults to zero) and a value yielded by the iterable argument.\n\nenumerate is useful for obtaining an indexed list:\n    (0, seq[0]), (1, seq[1]), (2, seq[2]), ..."),
            "reversed" => Some("Return a reverse iterator over the values of the given sequence."),
            _ => None,
        }
    }
    /// The builtin kind a class itself stands for, if it is one.
    pub(super) fn own_kind(c: &Class) -> Option<String> {
        c.constants.iter().find(|(n, _)| n == "\0kind").map(|(_, v)| v.plain())
    }
    /// The builtin kind a class stands on, through any of its line.
    pub(super) fn kind_beneath(c: &Class) -> Option<String> {
        std::iter::once(c).chain(c.lineage.borrow().iter().map(Rc::as_ref)).find_map(Self::own_kind)
    }

    /// Whether a class stands on the builtin kind named, through any of
    /// its line: a class written with several builtin bases stands on
    /// each of them, and answers to any of them asked after.
    pub(super) fn kind_among(c: &Class, word: &str) -> bool {
        std::iter::once(c).chain(c.lineage.borrow().iter().map(Rc::as_ref))
            .any(|held| Self::own_kind(held).as_deref() == Some(word))
    }
    /// Whether the class was sealed against change: a sealed class
    /// refuses writes and removals among its members and cannot stand
    /// as a base. Only the library's own sealing builtin sets the mark,
    /// which no member of the class can stand for.
    pub(super) fn class_sealed(c: &Class) -> bool {
        c.sealed.get()
    }
    /// The worth a thing keeps of the builtin kind its class stands on,
    /// as it is kept: a row or a map in its cell, so that what is done
    /// to it through the thing is done to the thing's own.
    pub(super) fn worth_of(value: &Value) -> Option<Value> {
        let Value::Object(o) = value else { return None };
        o.fields.borrow().iter().find(|(n, _)| n == "\0worth").map(|(_, v)| v.clone())
    }
    /// That worth, settled, where the thing's class has no method of its
    /// own at any of the given places of the protocol: what the class
    /// left unsaid is answered by the worth.
    pub(super) fn worth_free_of(&self, value: &Value, places: &[usize]) -> Option<Value> {
        let worth = Self::worth_of(value)?;
        if places.iter().any(|&place| self.special_value(value, place).is_some()) { return None; }
        Some(worth.contents())
    }
    /// A thing of a class standing on a builtin kind, its worth made by
    /// the builtin of that kind from the arguments given.
    fn thing_of_kind(&mut self, c: Rc<Class>, word: &str, args: Vec<Value>) -> Flow<Value> {
        if word == "Union" { return Err("TypeError: cannot create 'typing.Union' instances".into()); }
        if word == "cell" {
            if args.len() > 1 { return Err(format!("TypeError: cell expected at most 1 argument, got {}", args.len()).into()); }
            return Ok(Self::adapter(31, vec![Value::Binding(Rc::new(RefCell::new(args.first().cloned().unwrap_or(Value::Blank))))]));
        }
        if word == "method" {
            if args.len() != 2 { return Err("TypeError: method expected 2 arguments".into()); }
            if !matches!(self.class_work(2, vec![args[0].clone()])?, Value::Flag(true)) { return Err("TypeError: first argument must be callable".into()); }
            if matches!(args[1], Value::Null) { return Err("TypeError: instance must not be None".into()); }
            return Ok(Self::adapter(131, args));
        }
        if word == "mappingproxy" {
            let entries = self.call_items(args)?;
            if entries.len() != 1 || entries[0].0.is_some() { return Err("TypeError: mappingproxy() takes exactly one argument".into()); }
            let mapping = entries[0].1.clone();
            let held = mapping.contents();
            if !matches!(&mapping, Value::View(view) if view.1 == "mapping") && !matches!(held, Value::Map(_) | Value::Fields(_)) && !(matches!(held, Value::Object(_)) && self.special_value(&held, 11).is_some()) {
                return Err(format!("TypeError: mappingproxy() argument must be a mapping, not {}", held.core_kind()).into());
            }
            return Ok(Value::View(Rc::new((mapping, "mapping".to_string()))));
        }
        // None, Ellipsis and NotImplemented are singletons no kind
        // builtin makes twice over: calling the kind itself hands back
        // the one value there is of it, and any argument at all is
        // refused, since there is no other one to build from it.
        if let Some((singleton, shown)) = match word {
            "NoneType" => Some((Value::Null, "NoneType")),
            "ellipsis" => Some((Value::Ellipsis, "EllipsisType")),
            "NotImplementedType" => Some((Value::Declined(Rc::from(self.lang.special_declined.first().map(String::as_str).unwrap_or("NotImplemented"))), "NotImplementedType")),
            _ => None,
        } {
            return if args.is_empty() { Ok(singleton) } else { Err(format!("TypeError: {shown} takes no arguments").into()) };
        }
        if word == "module" {
            let supplied = self.call_items(args)?;
            let mut name = None;
            let mut doc = Value::Null;
            let mut count = 0;
            for (key, value) in supplied {
                match key.as_deref() {
                    None if count == 0 => { name = Some(value); count += 1; }
                    None if count == 1 => { doc = value; count += 1; }
                    Some("name") if name.is_none() => name = Some(value),
                    Some("doc") if count < 2 => doc = value,
                    _ => return Err("TypeError: invalid module arguments".into()),
                }
            }
            let Some(name @ Value::Text(_)) = name else { return Err("TypeError: module name must be str".into()); };
            self.made += 1;
            return Ok(Value::Object(Rc::new(Instance { replacement_class: RefCell::new(None), class: c,
                fields: RefCell::new(vec![("__name__".to_string(), name), ("__doc__".to_string(), doc),
                    ("__package__".to_string(), Value::Null), ("__loader__".to_string(), Value::Null), ("__spec__".to_string(), Value::Null)]), mark: self.made })));
        }
        // A routine built by hand out of a code value and a
        // dictionary of names: the reference's way of making a
        // function, which keeps the dictionary it was handed.
        if word == "function" {
            let (Some(Value::Adapter(code)), Some(globe)) = (args.first(), args.get(1)) else { return Err(self.class_refusal()); };
            if code.0 != 7 { return Err(self.class_refusal()); }
            let Some(Value::Routine(template)) = code.1.first() else { return Err(self.class_refusal()); };
            let born = self.ambient_builtins();
            let mut made = (**template).clone();
            if let Some(named) = self.captured_title(globe) { made.home = Some(named); }
            made.globe = Some(globe.clone());
            made.born = Some(born);
            made.revised = RefCell::new(None);
            self.made += 1;
            return Ok(Value::Routine(Rc::new(made)));
        }
        if word == "method" {
            if args.len() != 2 { return Err("TypeError: method expected 2 arguments".into()); }
            if matches!(args[1].contents(), Value::Null) { return Err("TypeError: instance must not be None".into()); }
            return Ok(Self::adapter(3, args));
        }
        // A module made by hand: the reference's ModuleType is the run's
        // own module kind, so calling it makes a thing carrying the name
        // it was handed and, where one was handed, its documentation.
        if word == "module" {
            // Allocation leaves argument binding to the initializer,
            // including an initializer supplied by a module subclass.
            let fields = Vec::new();
            self.made += 1;
            return Ok(Value::Object(Rc::new(Instance { replacement_class: RefCell::new(None), class: c, fields: RefCell::new(fields), mark: self.made })));
        }
        let Some(op) = self.lang.builtins.get(word).copied() else { return Err(self.class_refusal()); };
        let items = self.call_items(args)?;
        let made = self.builtin_call(op, word, items).map_err(|words| self.carried.take().unwrap_or(Fault::Note(words)))?;
        let made = if word == "str" { Self::worth_of(&made).unwrap_or(made) } else { made };
        let kept = match made.contents() {
            held @ (Value::Array(_) | Value::Map(_)) => Value::Collection(Rc::new(RefCell::new(held)), true),
            other => other,
        };
        self.made += 1;
        Ok(Value::Object(Rc::new(Instance {replacement_class: RefCell::new(None),  class: c, fields: RefCell::new(vec![("\0worth".to_string(), kept)]), mark: self.made })))
    }
    fn module_member_or(&mut self, module: &Value, name: &str, fallback: Value) -> Flow<Value> {
        match self.class_get(module.clone(), name, false) {
            Ok(value) => Ok(value.contents()),
            Err(fault) if self.attribute_fault(&fault) => { self.absent_member = None; Ok(fallback) },
            Err(fault) => Err(fault),
        }
    }
    fn module_rendered(&mut self, value: &Value, quoted: bool) -> Flow<String> {
        self.special_text(value, quoted).map_err(|message| self.carried.take().unwrap_or(Fault::Note(message)))
    }
    fn module_namespace_paths(&mut self, loader: &Value) -> Flow<Option<Value>> {
        let external = self.modules.get("_frozen_importlib_external").or_else(|| self.modules.get("importlib._bootstrap_external")).cloned();
        let Some(external) = external else { return Ok(None); };
        let category = self.module_member_or(&external, "NamespaceLoader", Value::Null)?;
        if matches!(category, Value::Null) { return Ok(None); }
        if !self.class_work(0, vec![loader.clone(), category])?.is_true() { return Ok(None); }
        let path = self.class_get(loader.clone(), "_path", false)?;
        Ok(Some(Value::Array(Rc::new(self.comprehension_items(&path)?).into())))
    }
    pub(super) fn module_repr_value(&mut self, module: Value) -> Flow<Value> {
        if self.module_holding(&module).is_none() && !matches!(&module, Value::Object(o) if Self::kind_beneath(&o.class_now()).as_deref() == Some("module")) {
            return Err(format!("TypeError: descriptor '__repr__' requires a 'module' object but received a '{}'", Self::type_argument_kind(&module)).into());
        }
        let loader = self.module_member_or(&module, "__loader__", Value::Null)?;
        let spec = self.module_member_or(&module, "__spec__", Value::Null)?;
        let mut check = vec![spec.clone()];
        let has_spec = self.builtin(Builtin::Bool, "bool", &mut check)?.is_true();
        let text = if has_spec {
            let raw_name = self.class_get(spec.clone(), "name", false)?.contents();
            let name = if matches!(raw_name, Value::Null) { Value::text("?") } else { raw_name.clone() };
            let origin = self.class_get(spec.clone(), "origin", false)?.contents();
            if matches!(origin, Value::Null) {
                let provider = self.class_get(spec, "loader", false)?.contents();
                let title = self.module_rendered(&name, true)?;
                if matches!(provider, Value::Null) { format!("<module {title}>") }
                else if let Some(paths) = self.module_namespace_paths(&provider)? {
                    format!("<module {title} (namespace) from {}>", self.module_rendered(&paths, true)?)
                } else { format!("<module {title} ({})>", self.module_rendered(&provider, true)?) }
            } else {
                let location = self.class_get(spec, "has_location", false)?;
                let mut check = vec![location];
                if self.builtin(Builtin::Bool, "bool", &mut check)?.is_true() {
                    format!("<module {} from {}>", self.module_rendered(&name, true)?, self.module_rendered(&origin, true)?)
                } else {
                    format!("<module {} ({})>", self.module_rendered(&raw_name, true)?, self.module_rendered(&origin, false)?)
                }
            }
        } else {
            let name = self.module_member_or(&module, "__name__", Value::text("?"))?;
            let title = self.module_rendered(&name, true)?;
            match self.class_get(module, "__file__", false) {
                Ok(file) => format!("<module {title} from {}>", self.module_rendered(&file, true)?),
                Err(fault) if self.attribute_fault(&fault) => {
                    self.absent_member = None;
                    if matches!(loader, Value::Null) { format!("<module {title}>") }
                    else { format!("<module {title} ({})>", self.module_rendered(&loader, true)?) }
                }
                Err(fault) => return Err(fault),
            }
        };
        Ok(Value::text(&text))
    }
    fn initialise_module(&mut self, receiver: Value, args: Vec<Value>) -> Flow<Value> {
        let valid = matches!(&receiver, Value::Object(o) if Self::kind_beneath(&o.class_now()).as_deref() == Some("module"))
            || self.module_holding(&receiver).is_some();
        if !valid { return Err(format!("TypeError: descriptor '__init__' requires a 'module' object but received a '{}'", Self::type_argument_kind(&receiver)).into()); }
        let items = self.call_items(args)?;
        if items.len() > 2 { return Err(format!("TypeError: module() takes at most 2 arguments ({} given)", items.len()).into()); }
        let mut values = [None, None];
        let mut positional = 0;
        let mut extra = None;
        for (key, value) in items {
            if let Some(key) = key {
                let slot = match key.as_str() { "name" => 0, "doc" => 1, _ => { extra = Some(key); continue; } };
                if slot < positional { return Err(format!("TypeError: argument for module() given by name ('{key}') and position ({})", slot + 1).into()); }
                values[slot] = Some(value);
            } else {
                values[positional] = Some(value);
                positional += 1;
            }
        }
        let name = values[0].take().ok_or("TypeError: module() missing required argument 'name' (pos 1)")?;
        if let Some(key) = extra { return Err(format!("TypeError: module() got an unexpected keyword argument '{key}'").into()); }
        let text = Self::worth_of(&name).unwrap_or_else(|| name.contents());
        if !matches!(text, Value::Text(_) | Value::Codepoints(_)) {
            let kind = if matches!(text, Value::Null) { "None".to_string() } else { Self::type_argument_kind(&name) };
            return Err(format!("TypeError: module() argument 'name' must be str, not {kind}").into());
        }
        let doc = values[1].take().unwrap_or(Value::Null);
        for (key, value) in [("__name__", name), ("__doc__", doc), ("__package__", Value::Null), ("__loader__", Value::Null), ("__spec__", Value::Null)] {
            let Value::Object(object) = &receiver else { unreachable!() };
            let dictionary = object.fields.borrow().iter().find(|(name, _)| name == "\0namespace").map(|(_, held)| held.clone());
            if let Some(dictionary) = dictionary { Self::book_write(&dictionary, key, Some(value)); }
            else {
                let link = object.fields.borrow().iter().find(|(name, _)| name == key).and_then(|(_, held)| match held { Value::Bond(cell) => Some(cell.clone()), _ => None });
                if let Some(link) = link { *link.borrow_mut() = value; }
                else { Self::write_members(&mut object.fields.borrow_mut(), key, Some(value), true).map_err(|_| self.class_refusal())?; }
            }
        }
        Ok(Value::Null)
    }
    pub(super) fn form_class(&mut self, name: String, bases: Vec<Rc<Class>>, members: Vec<(String, Value)>) -> Flow<Value> {
        self.form_named_class(Value::text(&name), bases, members)
    }
    fn form_named_class(&mut self, title_value: Value, mut bases: Vec<Rc<Class>>, mut members: Vec<(String, Value)>) -> Flow<Value> {
        let name = self.type_title(&title_value)?;
        let prepared = members.iter().position(|(key, _)| key == "\0prepared").map(|i| members.remove(i).1);
        if let Some(at) = members.iter().position(|(word, _)| word == "\0header") {
            let (_, header) = members.remove(at);
            let Value::Tuple(arguments) = header else { return Err(self.class_refusal()); };
            let opened = self.call_items(arguments.as_ref().clone())?;
            let mut raw_bases = Vec::new();
            let mut maker = None;
            let mut keywords = Vec::new();
            for (word, value) in opened {
                match word {
                    Some(word) if Lang::spells(&self.lang.metaclass_word, &word) => maker = Some(value),
                    Some(word) => keywords.push((word, value)), None => raw_bases.push(value),
                }
            }
            let original = Value::tuple(raw_bases.clone());
            let hook_name = self.class_word("mro.entries").to_string();
            let mut replaced = false;
            let mut normalized = Vec::new();
            for value in raw_bases {
                if !hook_name.is_empty() && matches!(value.contents(), Value::Object(_)) {
                    let hook = self.class_get(value.clone(), &hook_name, false)?;
                    let result = self.class_apply(hook, vec![original.clone()])?;
                    let Value::Tuple(listed) = result.contents() else { return Err("TypeError: __mro_entries__ must return a tuple".into()); };
                    normalized.extend(listed.iter().cloned());
                    replaced = true;
                } else { normalized.push(value); }
            }
            if replaced { members.push((self.class_word("original.bases").to_string(), original)); }
            if let Some(factory) = maker.filter(|v| !matches!(v, Value::Native(Builtin::SortOf, _))) {
                let entries = members.iter().filter(|(_, value)| !matches!(value, Value::Blank))
                    .map(|(word, value)| (Value::text(word), value.clone())).collect::<Vec<_>>();
                let namespace = prepared.unwrap_or_else(|| Value::Collection(Rc::new(RefCell::new(Value::Map(Rc::new(entries.into())))), true));
                let mut given = vec![Value::text(&name), Value::tuple(normalized), namespace];
                given.extend(keywords.into_iter().map(|(word, value)| Value::Tie(Rc::new((Value::text(&word), value)))));
                return self.class_apply(factory, given);
            }
            bases.clear();
            for value in normalized {
                if !self.class_word("mro.entries").is_empty() && matches!(value.contents(), Value::Null) { return Err("TypeError: NoneType takes no arguments".into()); }
                bases.push(self.type_base(&value)?);
            }
            members.extend(keywords.into_iter().map(|(word, value)| (format!("\0keyword:{word}"), value)));
        }
        let declared_bases = bases.clone();
        if bases.is_empty() { bases.push(self.root_class()); }
        // The keywords the header carried are no members: they go by
        // name to the forebear's subclass hook.
        let mut carried = Vec::new();
        members.retain(|(n, v)| match n.strip_prefix("\0keyword:") {
            Some(word) => { carried.push(Value::Tie(Rc::new((Value::text(word), v.clone())))); false }
            None => true,
        });
        // The metaclass the header named is no member either: it says
        // what makes the class.
        let mut asked = None;
        members.retain(|(n, v)| if n == MAKER_MEMBER { asked = Some(v.clone()); false } else { true });
        let maker = self.maker_in_force(asked, &bases)?;
        // A metaclass with a making of its own makes the class: it is
        // handed itself, the name, the bases and the body's namespace as
        // a map it may write into, and what it answers is the class.
        if let Some(m) = maker.clone() {
            if let Some(f) = self.class_value(&m, self.class_word("allocate")) {
                let listed = Value::tuple(declared_bases.iter().cloned().map(Value::Class).collect());
                let pairs: Vec<(Value, Value)> = members.iter().filter(|(_, v)| !matches!(v, Value::Blank))
                    .map(|(n, v)| (Value::text(n), v.clone())).collect();
                let namespace = prepared.unwrap_or_else(|| Value::Collection(Rc::new(RefCell::new(Value::Map(Rc::new(pairs.into())))), true));
                let mut given = vec![Value::Class(m.clone()), Value::text(&name), listed.clone(), namespace.clone()];
                given.extend(carried.iter().cloned());
                let made = self.class_apply(f, given)?;
                // The metaclass is then told of what it has made.
                if let Some(begun) = self.lang.constructor.as_deref().and_then(|n| self.class_value(&m, n)) {
                    let bound = self.bind_class_value(begun, Some(made.clone()), m)?;
                    let mut told = vec![Value::text(&name), listed, namespace];
                    told.extend(carried);
                    self.class_apply(bound, told)?;
                }
                return Ok(made);
            }
        }
        self.forge_class(name, bases, members, maker, carried, title_value)
    }
    /// A class laid out from what the kind builtin is given: the
    /// metaclass to remember, the name, the bases and the namespace,
    /// with any further keyword kept for a forebear's subclass hook.
    pub(super) fn class_from_parts(&mut self, args: Vec<Value>) -> Flow<Value> {
        let (mut plain, mut named) = (Vec::new(), Vec::new());
        for (key, v) in self.call_items(args)? {
            match key { Some(k) => named.push(Value::Tie(Rc::new((Value::text(&k), v)))), None => plain.push(v) }
        }
        let Some(first) = plain.first().map(Value::contents) else { return Err("TypeError: type.__new__(): not enough arguments".into()) };
        let given = match first {
            Value::Native(Builtin::SortOf, _) => self.metaclass_root(),
            Value::Class(m) => m,
            Value::Native(_, word) if self.lang.builtins.get(word.as_ref()).is_some_and(Self::kind_builtin) => self.kind_class(&word),
            other => return Err(format!("TypeError: type.__new__(X): X is not a type object ({})", other.core_kind()).into()),
        };
        if !self.is_metaclass_root(&given) && !given.lineage.borrow().iter().any(|b| self.is_metaclass_root(b)) {
            return Err(format!("TypeError: type.__new__({0}): {0} is not a subtype of type", given.name).into());
        }
        if plain.len() != 4 { return Err(format!("TypeError: type.__new__() takes exactly 3 arguments ({} given)", plain.len() - 1).into()); }
        let title = self.type_title(&plain[1])?;
        let bases_value = plain[2].contents();
        let bases_value = Self::worth_of(&bases_value).map_or(bases_value.clone(), |v| v.contents());
        let Value::Tuple(listed) = bases_value else { return Err(format!("TypeError: type.__new__() argument 2 must be tuple, not {}", plain[2].core_kind()).into()) };
        let namespace = plain[3].contents();
        let namespace = Self::worth_of(&namespace).map_or(namespace.clone(), |v| v.contents());
        let Value::Map(entries) = namespace else { return Err(format!("TypeError: type.__new__() argument 3 must be dict, not {}", plain[3].core_kind()).into()) };
        let mut parents = Vec::new();
        for b in listed.iter() {
            if !self.stands_for_kind(&b.contents()) {
                // A base that is no class but answers __mro_entries__ is
                // refused by name the way the reference refuses it:
                // type() does not resolve entries, types.new_class()
                // does.
                let hook = self.class_word("mro.entries").to_string();
                if !hook.is_empty() && matches!(b.contents(), Value::Object(_)) {
                    match self.class_get(b.clone(), &hook, false) {
                        Ok(_) => return Err("TypeError: type() doesn't support MRO entry resolution; use types.new_class()".to_string().into()),
                        Err(fault) if self.attribute_fault(&fault) => {}
                        Err(fault) => return Err(fault),
                    }
                }
                return Err("TypeError: metaclass conflict: the metaclass of a derived class must be a (non-strict) subclass of the metaclasses of all its bases".into());
            }
            parents.push(self.type_base(b)?);
        }
        if parents.is_empty() { parents.push(self.root_class()); }
        let winner = self.maker_in_force(Some(Value::Class(given.clone())), &parents)?;
        if let Some(m) = &winner {
            if !Rc::ptr_eq(m, &given) {
                if let Some(new) = self.class_value(m, self.class_word("allocate")) {
                    plain[0] = Value::Class(m.clone()); plain.extend(named);
                    return self.class_apply(new, plain);
                }
            }
        }
        let members = entries.iter().map(|(k, v)| (k.plain(), v.clone())).collect();
        self.forge_class(title, parents, members, winner, named, plain[1].contents())
    }
    pub(super) fn type_argument_kind(value: &Value) -> String {
        if let Value::Object(object) = value.contents() {
            if let Some(names) = object.class_now().python_names.borrow().as_ref() {
                return names.0.type_text().plain();
            }
        }
        value.core_kind()
    }
    fn type_title(&mut self, value: &Value) -> Flow<String> {
        if self.class_word("name").is_empty() { return Ok(Self::worth_of(value).unwrap_or_else(|| value.contents()).plain()); }
        let text = value.type_text();
        self.type_utf8(&text)?;
        let Value::Text(title) = text else { return Err(format!("TypeError: type.__new__() argument 1 must be str, not {}", Self::type_argument_kind(value)).into()); };
        if title.contains('\0') { return Err("ValueError: type name must not contain null characters".into()); }
        Ok(title.to_string())
    }
    pub(super) fn type_utf8(&mut self, value: &Value) -> Flow<()> {
        if let Value::Codepoints(row) = value.contents() {
            if let Some(start) = row.iter().position(|n| (0xd800..=0xdfff).contains(n)) {
                let end = row[start..].iter().take_while(|n| (0xd800..=0xdfff).contains(*n)).count() + start;
                let class = self.furnished(43).ok_or_else(|| self.class_refusal())?;
                let error = self.exception_instance(class, vec![Value::text("utf-8"), value.clone(), Value::Small(start as i64), Value::Small(end as i64), Value::text("surrogates not allowed")], Value::Null);
                return Err(Fault::Thrown(error));
            }
        }
        Ok(())
    }
    pub(super) fn contains_class(class: &Rc<Class>, target: &Rc<Class>) -> bool {
        fn visit(class: &Rc<Class>, target: &Rc<Class>, seen: &mut Vec<*const Class>) -> bool {
            if class.mro_adopted.get() { return class.adopted_order.borrow().iter().filter_map(std::rc::Weak::upgrade).any(|base| Rc::ptr_eq(&base, target)); }
            if Rc::ptr_eq(class, target) || class.lineage.borrow().iter().any(|base| Rc::ptr_eq(base, target)) { return true; }
            let address = Rc::as_ptr(class);
            if seen.contains(&address) { return false; }
            seen.push(address);
            if class.fields.iter().any(|(key, held)| key == "\0also-beneath"
                && matches!(held, Value::Class(other) if visit(other, target, seen))) { return true; }
            if class.direct.iter().any(|base| visit(base, target, seen)) { return true; }
            class.base.as_ref().is_some_and(|base| visit(base, target, seen))
        }
        visit(class, target, &mut Vec::new())
    }
    pub(super) fn class_order(c: &Rc<Class>) -> Vec<Rc<Class>> {
        let ancestors = c.lineage.borrow();
        if c.mro_adopted.get() { return c.adopted_order.borrow().iter().filter_map(std::rc::Weak::upgrade).collect(); }
        std::iter::once(c.clone()).chain(ancestors.iter().cloned()).collect()
    }
    fn instance_root_visible(&self, c: &Rc<Class>) -> bool {
        if !c.mro_adopted.get() { return true; }
        self.class_root.as_ref().is_some_and(|root| c.adopted_order.borrow().iter()
            .filter_map(std::rc::Weak::upgrade).any(|base| Rc::ptr_eq(&base, root)))
    }
    fn object_receiver(&mut self, value: &Value) -> Flow<(bool, String)> {
        let actual = self.class_type(vec![value.clone()])?;
        let name = actual.kind_it_names().ok_or_else(|| self.class_refusal())?;
        let class = self.super_type_arg(&actual)?;
        let root = self.root_class();
        Ok((Self::contains_class(&class, &root), name))
    }
    fn generic_instance_access(&self, c: &Rc<Class>) -> bool {
        if !c.mro_adopted.get() { return true; }
        Self::class_order(c).iter().any(|base| {
            self.class_root.as_ref().is_some_and(|root| Rc::ptr_eq(root, base))
                || Self::own_kind(base).as_deref() == Some("module")
        })
    }
    pub(super) fn public_class(&self, class: Rc<Class>) -> Value {
        if self.is_metaclass_root(&class) { return self.kind_maker_word(); }
        let visible = Self::own_kind(&class).and_then(|word| self.spelled_kind(&word));
        match visible {
            Some(Value::Native(Builtin::Bytes(mode @ 0..=1), _)) if self.lang.bind_names => self.byte_kind(mode == 1),
            Some(value) => value,
            None => Value::Class(class),
        }
    }
    fn solid_parent(&self, class: &Rc<Class>) -> Rc<Class> {
        if Self::own_kind(class).is_some() || self.is_metaclass_root(class) || class.declares_slots { return class.clone(); }
        class.base.as_ref().map_or_else(|| class.clone(), |parent| self.solid_parent(parent))
    }
    fn layout_parent(&self, bases: &[Rc<Class>]) -> Flow<Option<Rc<Class>>> {
        let mut chosen: Option<Rc<Class>> = None;
        for base in bases {
            if let Some(previous) = &chosen {
                let old = self.solid_parent(previous); let next = self.solid_parent(base);
                if Rc::ptr_eq(&old, &next) || old.lineage.borrow().iter().any(|p| Rc::ptr_eq(p, &next)) { continue; }
                if !next.lineage.borrow().iter().any(|p| Rc::ptr_eq(p, &old)) {
                    return Err(self.lang.layout_amiss.clone().unwrap_or_else(|| self.class_word("unready").to_string()).into());
                }
            }
            chosen = Some(base.clone());
        }
        Ok(chosen)
    }
    /// The class itself, laid out from its name, its bases, its members
    /// and the metaclass it is to remember. This is the making the kind
    /// builtin does, and what a metaclass reaches for through its
    /// forebears when it has made a namespace of its own.
    fn merge_class_orders(&self, bases: &[Rc<Class>]) -> Flow<Vec<Rc<Class>>> {
        let mut lines: Vec<Vec<Rc<Class>>> = bases.iter().map(|b| {
            Self::class_order(b)
        }).collect();
        lines.push(bases.to_vec());
        let mut lineage = Vec::new();
        while lines.iter().any(|s| !s.is_empty()) {
            let head = lines.iter().filter_map(|s| s.first()).find(|head|
                !lines.iter().any(|s| s.iter().skip(1).any(|tail| Rc::ptr_eq(head, tail)))).cloned()
                .ok_or_else(|| self.class_word("mro.amiss").to_string())?;
            if lineage.iter().any(|c| Rc::ptr_eq(c, &head)) { return Err(self.class_word("mro.amiss").to_string().into()); }
            for line in &mut lines { if line.first().map_or(false, |c| Rc::ptr_eq(c, &head)) { line.remove(0); } }
            lineage.push(head);
        }
        Ok(lineage)
    }
    pub(super) fn forge_class(&mut self, name: String, bases: Vec<Rc<Class>>, mut members: Vec<(String, Value)>,
        maker: Option<Rc<Class>>, carried: Vec<Value>, title_value: Value) -> Flow<Value> {
        let mut class_cell = None;
        let cell_word = self.class_word("classcell").to_owned();
        if !cell_word.is_empty() {
            if let Some(at) = members.iter().position(|(key, _)| key == &cell_word) {
                let cell = members.remove(at).1;
                if !matches!(cell.contents(), Value::Adapter(ref wrapped) if wrapped.0 == 31) { return Err("TypeError: __classcell__ must be a nonlocal cell".into()); }
                class_cell = Some(cell);
            }
        }

        // A place only an arm of a conditional writes to may never have
        // been written. Nothing stands there, and the class keeps no
        // member for it: a name a conditional never bound is no member.
        members.retain(|(_, held)| !matches!(held, Value::Blank));
        if !self.lang.class_details.is_empty() {
            for (key, value) in &mut members {
                if key == self.lang.class_details.get("allocate").and_then(|v| v.first()).map(String::as_str).unwrap_or("") && matches!(value, Value::Routine(_)) {
                    *value = Self::adapter(4, vec![value.clone()]);
                }
            }
        }
        if !self.lang.module_cache.is_empty() && members.iter().any(|(key, value)| key == "__abc_tpflags__" && matches!(value.contents(), Value::Small(flags) if flags & 96 == 96)) {
            return Err(format!("TypeError: type {name} has both Py_TPFLAGS_SEQUENCE and Py_TPFLAGS_MAPPING set").into());
        }
        let lineage = self.merge_class_orders(&bases)?;
        // Two builtin kinds cannot both be kept in one thing.
        let mut kinds: Vec<String> = lineage.iter().filter_map(|b| Self::own_kind(b)).collect();
        kinds.dedup();
        if kinds.len() > 1 { return Err(self.lang.layout_amiss.clone().unwrap_or_else(|| self.class_word("unready").to_string()).into()); }
        let primary = self.layout_parent(&bases)?;
        let calling_code = self.trace_frame.as_ref().and_then(|frame| frame.fields.borrow().iter()
            .find_map(|(key, value)| if key == "\0routine" { match value { Value::Routine(code) => Some(code.clone()), _ => None } } else { None }));
        let module = calling_code.as_ref().map_or_else(|| Value::text(self.class_word("main")), |code| self.routine_module(code));
        if !members.iter().any(|(n,_)| n == self.class_word("module")) {
            members.push((self.class_word("module").to_string(), module));
        }
        if let Some((_, held)) = members.iter().find(|(n, _)| n == self.class_word("qualified")) {
            if !matches!(if self.class_word("name").is_empty() { Self::worth_of(held).unwrap_or_else(|| held.contents()) } else { held.type_text() }, Value::Text(_) | Value::Codepoints(_)) { return Err(format!("TypeError: type __qualname__ must be a str, not {}", Self::type_argument_kind(held)).into()); }
        }
        let python_names = if !self.class_word("name").is_empty() {
            self.type_title(&Value::text(&name))?;
            let qualified = members.iter().find(|(key, _)| key == self.class_word("qualified")).map(|(_, v)| v.clone()).unwrap_or_else(|| title_value.clone());
            if let Some((_, doc)) = members.iter().find(|(key, _)| key == self.class_word("doc")) {
                let raw = doc.type_text();
                self.type_utf8(&raw)?;
            } else {
                let doc_slot = members.iter().find(|(key, _)| key == self.class_word("slots")).is_some_and(|(_, value)| match value.contents() {
                    Value::Text(word) => word.as_ref() == self.class_word("doc"),
                    Value::Tuple(words) | Value::Array(words) => words.iter().any(|word| word.plain() == self.class_word("doc")),
                    _ => false,
                });
                if !doc_slot { members.push((self.class_word("doc").to_string(), Value::Null)); }
            }
            members.retain(|(key, _)| key != self.class_word("qualified"));
            Some((title_value, qualified.clone(), self.class_word("module").to_string(), qualified))
        } else { None };
        let mut constants = maker.map(|m| vec![(MAKER_MEMBER.to_string(), Value::Class(m))]).unwrap_or_default();
        if let Some(word) = &self.lang.native_type_name {
            if let Some(at) = members.iter().position(|(key, _)| key == word) {
                let (_, value) = members.remove(at);
                if !matches!(value.contents(), Value::Text(_)) { return Err("TypeError: native type name must be a str".into()); }
                constants.push(("\0native-name".to_string(), value.contents()));
            }
        }
        let display=members.iter().find(|(n,_)|n==self.class_word("qualified")).map(|(_,v)|v.plain()).unwrap_or_else(||name.clone());
        // Slot storage is fixed when the class is made, independently of
        // later replacement or mutation of its public declaration.
        let owns_storage = members.iter().find(|(key, _)| key == self.class_word("slots")).is_some_and(|(_, value)| {
            let slots = match value.contents() { Value::Tuple(items) | Value::Array(items) => items.as_ref().clone(), single => vec![single] };
            slots.iter().any(|slot| !matches!(slot, Value::Text(key) if key.as_ref() == "__dict__" || key.as_ref() == "__weakref__"))
        });
        let module = members.iter().find(|(key, _)| key == self.class_word("module")).map(|(_, value)| value.plain()).unwrap_or_default();
        let c = Rc::new(Class { name: name.clone(), outline: Some(format!("<class '{module}.{display}'>")),
            base: primary, direct: bases, lineage: RefCell::new(lineage), answers: vec![], fields: vec![], reaches: vec![],
            methods: vec![], constants,
            shared: RefCell::new(members), weak_storage: std::cell::Cell::new(None), declares_slots: owns_storage, sealed: std::cell::Cell::new(false), mro_adopted: std::cell::Cell::new(false), adopted_order: RefCell::new(Vec::new()), python_names: RefCell::new(python_names) });
        if !self.lang.weak_refused.is_empty() { crate::faint::remember(crate::faint::Hold::Class(Rc::downgrade(&c))); }
        if let Some(cell) = &class_cell { let word = self.class_word("cell.contents").to_owned(); self.class_write(cell.clone(), &word, Some(Value::Class(c.clone())), true)?; }

        // A metaclass whose own answering of the order is written out
        // is asked for it once while the class is made, and its answer
        // is the order the class keeps, measured the way the reference
        // measures it: never empty, every entry a class, and no two
        // builtin kinds kept in the one thing.
        let mro_word = self.class_word("order");
        if !mro_word.is_empty() {
            if let Some(meta) = Self::maker_beneath(&c) {
                let meta_order = Self::class_order(&meta);
                let reader = meta_order.iter().find_map(|base| Self::own_class_value(base, mro_word));
                if let Some(reader @ Value::Routine(_)) = reader {
                    let bound = self.bind_class_value(reader, Some(Value::Class(c.clone())), meta)?;
                    let answer = self.class_apply(bound, Vec::new())?;
                    let given = self.comprehension_items(&answer)?;
                    if given.is_empty() { return Err("TypeError: type MRO must not be empty".into()); }
                    let mut adopted = Vec::new();
                    for item in &given {
                        let base = match item.contents() {
                            Value::Class(b) => b,
                            Value::Native(op, word) if Self::kind_builtin(&op) => self.kind_class(&word),
                            other => return Err(format!("TypeError: mro() returned a non-class ('{}')", self.super_tp_name(&other)).into()),
                        };
                        adopted.push(base);
                    }
                    let layout = std::iter::once(&c).chain(c.lineage.borrow().iter()).find_map(|b| Self::own_kind(b));
                    if let Some(amiss) = adopted.iter().find(|b| Self::own_kind(b).is_some() && Self::own_kind(b) != layout) {
                        return Err(format!("TypeError: mro() returned base with unsuitable layout ('{}')", amiss.name).into());
                    }
                    // A supplied MRO is a complete order, independent of the
                    // declared bases. Its owner need not occur first, or at all.
                    *c.adopted_order.borrow_mut() = adopted.iter().map(Rc::downgrade).collect();
                    *c.lineage.borrow_mut() = adopted.into_iter().filter(|base| !Rc::ptr_eq(base, &c)).collect();
                    c.mro_adopted.set(true);
                }
            }
        }


        self.furnish_slots(&c)?;
        if !self.lang.class_builder.is_empty() && self.slots_allow(&c, self.class_word("namespace"))
            && !c.direct.iter().any(|base| self.slots_allow(base, self.class_word("namespace")))
            && Self::own_class_value(&c, self.class_word("namespace")).is_none() {
            c.shared.borrow_mut().push((self.class_word("namespace").to_owned(), Self::adapter(16,
                vec![Value::text(self.class_word("namespace")), Value::Class(c.clone()), Value::text("\0instance-namespace")])));
        }
        if Self::own_class_value(&c, self.class_word("doc")).is_none() { c.shared.borrow_mut().push((self.class_word("doc").into(), Value::Null)); }
        // Each member that asks to be told its name is told it, once the
        // class stands, before any forebear hears of the new class.
        if !self.class_word("descriptor.name").is_empty() {
            let declared = c.shared.borrow().clone();
            for (member, held) in declared {
                let Some(told) = self.descriptor_hook(&held, "descriptor.name") else { continue };
                self.call_descriptor(&held, told, vec![Value::Class(c.clone()), Value::text(&member)])?;
            }
        }
        // Class creation invokes the subclass hook through super(cls, cls).
        // In particular, an MRO that omits cls fails at this binding step.
        if c.mro_adopted.get() { self.super_check(&c, &Value::Class(c.clone()))?; }
        let order = Self::class_order(&c);
        let start = order.iter().position(|base| Rc::ptr_eq(base, &c)).unwrap_or(0);
        if let Some(hook) = order[start + 1..].iter().find_map(|b| Self::own_class_value(b, self.class_word("subclass"))) {
            if matches!(&hook,Value::Adapter(w) if w.0==5){let bound=self.bind_class_value(hook,None,c.clone())?;self.class_apply(bound,carried)?;}
            else{let mut given=vec![Value::Class(c.clone())];given.extend(carried);self.class_apply(hook, given)?;}
        }
        // Keywords with no hook to receive them are refused.
        else if !carried.is_empty() { return Err(format!("TypeError: {}.__init_subclass__() takes no keyword arguments", c.name).into()); }
        // The class is told of to each forebear it was written under,
        // loosely, so the forebear's __subclasses__ names it while it
        // stands and never after.
        for parent in &c.direct {
            self.class_children.borrow_mut().entry(Rc::as_ptr(parent) as usize).or_default().push(Rc::downgrade(&c));
        }
        Ok(Value::Class(c))
    }
    /// The slots a class names become members of it, each a descriptor
    /// that keeps the slot's value in the thing under a name of its own,
    /// so that two classes of one line naming the same slot keep two.
    fn furnish_slots(&mut self, c: &Rc<Class>) -> Flow<()> {
        if self.class_word("descriptor.get").is_empty() { return Ok(()); }
        c.weak_storage.set(Some(self.weak_layout(c)));
        let Some(slots) = Self::own_class_value(c, self.class_word("slots")) else {
            if !self.lang.weak_refused.is_empty() && self.weak_layout(c) && !c.direct.iter().any(|base| self.weak_layout(base)) {
                c.shared.borrow_mut().push(("__weakref__".into(), Self::adapter(16, vec![Value::text("__weakref__"), Value::Class(c.clone())])));
            }
            return Ok(());
        };
        // The names may come from a row, a mapping's keys, a set's
        // members or a single name, as the reference reads them.
        let named: Vec<Value> = match slots.contents() {
            Value::Tuple(v) | Value::Array(v) => v.as_ref().clone(),
            Value::Map(v) => v.iter().map(|(key, _)| key.clone()).collect(),
            Value::Set(v) => v.borrow().items(),
            single => vec![single],
        };
        if let Some(kind @ ("int" | "tuple" | "bytes")) = Self::kind_beneath(c).as_deref() {
            if !named.is_empty() {
                return Err(format!("TypeError: nonempty __slots__ not supported for subtype of '{kind}'").into());
            }
        }
        // Every slot is a name of its own: a string holding a plain
        // identifier, never another kind of value and never one that
        // could not be written after `self.`. `__dict__` and
        // `__weakref__` are written at most once each, since each
        // stands for one layout a thing may carry, not several.
        let mut dict_seen = 0u32;
        let mut weakref_seen = 0u32;
        for slot in &named {
            let Value::Text(word) = slot else { return Err("TypeError: __slots__ items must be strings".to_string().into()) };
            if !Self::valid_slot_name(word) { return Err("TypeError: __slots__ items must be identifiers".to_string().into()); }
            match word.as_ref() { "__dict__" => dict_seen += 1, "__weakref__" => weakref_seen += 1, _ => {} }
        }
        if dict_seen > 1 { return Err("TypeError: __dict__ slot disallowed: we already got one".to_string().into()); }
        if weakref_seen > 1 { return Err("TypeError: __weakref__ slot disallowed: we already got one".to_string().into()); }
        // A forebear built of a program's own classes, none of which
        // named any slots, already carries a dict and a weak reference
        // wherever it stands, so naming either again here only doubles
        // what is already had. A forebear standing on a builtin kind
        // carries neither by itself, no more than the common ancestor
        // does, so it is passed over exactly as that ancestor is.
        if dict_seen == 1 || weakref_seen == 1 {
            let root = self.root_class();
            let slots_word = self.class_word("slots").to_string();
            let already = if weakref_seen == 1 {
                c.direct.iter().any(|b| self.weak_layout(b))
            } else {
                c.lineage.borrow().iter().any(|b| !Rc::ptr_eq(b, &root) && Self::own_kind(b).is_none() && Self::own_class_value(b, &slots_word).is_none())
            };
            if already {
                let which = if dict_seen == 1 { "__dict__" } else { "__weakref__" };
                return Err(format!("TypeError: {which} slot disallowed: we already got one").into());
            }
        }
        for slot in named {
            let Value::Text(word) = slot else { unreachable!() };
            let word: Rc<str> = Rc::from(crate::compile::private_name(&c.name, &word));
            if word.as_ref() == self.class_word("namespace") { continue; }
            if Self::own_class_value(c, &word).is_some() { return Err(format!("ValueError: '{word}' in __slots__ conflicts with class variable").into()); }
            c.shared.borrow_mut().push((word.to_string(), Self::adapter(16, vec![Value::Text(word.clone()), Value::Class(c.clone())])));
        }
        Ok(())
    }
    /// Whether a slot's own name could follow `self.` in this language:
    /// not empty, opening on a letter or an underscore, and holding
    /// nothing after but letters, figures and underscores.
    fn valid_slot_name(word: &str) -> bool {
        let mut letters = word.chars();
        match letters.next() {
            Some(c) if c == '_' || c.is_alphabetic() => {}
            _ => return false,
        }
        letters.all(|c| c == '_' || c.is_alphanumeric())
    }
    /// Where a slot's value is kept in a thing: under the slot's name and
    /// the class that declared it. A thing not of that class has no such
    /// place, and the descriptor says so.
    pub(super) fn slot_place(&self, thing: &Value, parts: &[Value]) -> Flow<String> {
        let (Value::Object(o), Some(Value::Class(owner))) = (thing, parts.get(1)) else { return Err(self.class_refusal()) };
        let word = parts[0].plain();
        if !Rc::ptr_eq(&o.class_now(), owner) && !o.class_now().lineage.borrow().iter().any(|b| Rc::ptr_eq(b, owner)) {
            let pieces = self.lang.class_details.get("descriptor.foreign").cloned().unwrap_or_default();
            if pieces.len() != 4 { return Err(self.class_refusal()); }
            return Err(format!("{}{word}{}{}{}{}{}", pieces[0], pieces[1], owner.name, pieces[2], o.class_now().name, pieces[3]).into());
        }
        Ok(format!("\0slot:{word}:{:p}", Rc::as_ptr(owner)))
    }
    fn slot_read(&self, thing: &Value, parts: &[Value]) -> Flow<Value> {
        if parts.get(2).is_some_and(|part| part.plain() == "\0module-namespace") {
            let valid = self.module_holding(thing).is_some() || matches!(thing, Value::Object(o) if Self::kind_beneath(&o.class_now()).as_deref() == Some("module"));
            if !valid { return Err(format!("TypeError: descriptor '__dict__' for 'module' objects doesn't apply to a '{}' object", Self::type_argument_kind(thing)).into()); }
            let Value::Object(module) = thing else { unreachable!() };
            return Ok(module.fields.borrow().iter().find(|(key, _)| key == "\0namespace")
                .map_or_else(|| Value::Fields(module.clone()), |(_, book)| book.clone()));
        }
        let place = self.slot_place(thing, parts)?;
        let Value::Object(o) = thing else { return Err(self.class_refusal()) };
        if parts.get(2).is_some_and(|part| part.plain() == "\0instance-namespace") {
            return Ok(o.fields.borrow().iter().find(|(key, _)| key == "\0namespace")
                .map_or_else(|| Value::Fields(o.clone()), |(_, book)| book.clone()));
        }
        if parts[0].plain() == "__weakref__" && !self.lang.weak_refused.is_empty() { return Ok(crate::faint::references(thing).into_iter().next().unwrap_or(Value::Null)); }
        let kept = o.fields.borrow().iter().find(|(n, _)| *n == place).map(|(_, v)| v.clone());
        kept.ok_or_else(|| self.missing_member(thing, &parts[0].plain()))
    }
    fn slot_write(&self, thing: &Value, parts: &[Value], value: Option<Value>) -> Flow<Value> {
        if parts.get(2).is_some_and(|part| part.plain() == "\0module-namespace") {
            self.slot_read(thing, parts)?;
            return Err(self.class_word("property.readonly").to_string().into());
        }
        let place = self.slot_place(thing, parts)?;
        let Value::Object(o) = thing else { return Err(self.class_refusal()) };
        if parts.get(2).is_some_and(|part| part.plain() == "\0instance-namespace") {
            return self.replace_instance_namespace(o, value);
        }
        if parts[0].plain() == "__weakref__" && !self.lang.weak_refused.is_empty() { return Err(format!("AttributeError: attribute '__weakref__' of '{}' objects is not writable", o.class_now().name).into()); }
        Self::write_members(&mut o.fields.borrow_mut(), &place, value, false).map_err(|_| self.missing_member(thing, &parts[0].plain()))?;
        Ok(Value::Null)
    }
    /// The class every property is a thing of, made once. Its members
    /// are the workings of the protocol -- reading, writing and removing
    /// through the accessors a property keeps -- and the calls that make
    /// a fresh property from an old one with one accessor changed. A
    /// class written to stand on it inherits all of them.
    pub(super) fn property_class(&mut self) -> Rc<Class> {
        if let Some(c) = &self.property_class { return c.clone(); }
        let root = self.root_class();
        let name = self.lang.builtins.iter().find(|(_, b)| **b == Builtin::ClassTool(11)).map(|(n, _)| n.clone()).unwrap_or_default();
        let mut members = Vec::new();
        let workings = [("descriptor.get", 20), ("descriptor.set", 21), ("descriptor.delete", 22), ("property.getter", 23), ("property.deleter", 25), ("descriptor.name", 27)];
        for (part, tag) in workings { members.push((self.class_word(part).to_string(), Self::adapter(tag, vec![]))); }
        if let Some(word) = self.lang.property_setter.first() { members.push((word.clone(), Self::adapter(24, vec![]))); }
        if let Some(word) = &self.lang.constructor { members.push((word.clone(), Self::adapter(26, vec![]))); }
        members.push((self.class_word("name").to_string(), Self::adapter(28, vec![Value::text("\0name")])));
        for part in ["property.fget", "property.fset", "property.fdel", "doc", "property.is_abstract"] {
            members.push((self.class_word(part).to_string(), Self::adapter(28, vec![Value::text(Self::accessor_place(part))])));
        }
        let c = Rc::new(Class { outline: Some(format!("<class '{name}'>")), name,
            direct: vec![root.clone()], lineage: RefCell::new(vec![root.clone()]), base: Some(root), answers: vec![], fields: vec![], reaches: vec![],
            methods: vec![], constants: vec![], shared: RefCell::new(members), weak_storage: std::cell::Cell::new(None), declares_slots: false, sealed: std::cell::Cell::new(false), mro_adopted: std::cell::Cell::new(false), adopted_order: RefCell::new(Vec::new()), python_names: std::cell::RefCell::new(None) });
        self.property_class = Some(c.clone());
        c
    }
    /// Where a property keeps each accessor among its own members, under
    /// names no program can spell.
    fn accessor_place(part: &str) -> &'static str {
        match part { "property.fget" => "\0fget", "property.fset" => "\0fset", "property.fdel" => "\0fdel", "doc" => "\0doc", "property.is_abstract" => "\0abstract", _ => "\0name" }
    }
    /// The class every classmethod stands on, made once: it keeps the
    /// routine it was given under a name no program can spell, and read
    /// through a class it binds the routine to that class, as the
    /// reference's own classmethod does.
    pub(super) fn classmethod_class(&mut self) -> Rc<Class> {
        if let Some(c) = &self.classmethod_class { return c.clone(); }
        let root = self.root_class();
        let name = self.lang.builtins.iter().find(|(_, b)| **b == Builtin::ClassTool(10)).map(|(n, _)| n.clone()).unwrap_or_default();
        let c = Rc::new(Class { outline: Some(format!("<class '{name}'>")), name,
            direct: vec![root.clone()], lineage: RefCell::new(vec![root.clone()]), base: Some(root), answers: vec![], fields: vec![], reaches: vec![],
            methods: vec![], constants: vec![],
            shared: RefCell::new(vec![
                (self.class_word("descriptor.get").to_string(), Self::adapter(201, vec![])),
                (self.lang.constructor.clone().unwrap_or_default(), Self::adapter(200, vec![])),
            ]),
            weak_storage: std::cell::Cell::new(None), declares_slots: false, sealed: std::cell::Cell::new(false), mro_adopted: std::cell::Cell::new(false), adopted_order: RefCell::new(Vec::new()), python_names: std::cell::RefCell::new(None) });
        self.classmethod_class = Some(c.clone());
        c
    }
    /// The class every staticmethod stands on, made once: it keeps the
    /// routine it was given the same way, and read through anything it
    /// hands the routine back unbound.
    pub(super) fn staticmethod_class(&mut self) -> Rc<Class> {
        if let Some(c) = &self.staticmethod_class { return c.clone(); }
        let root = self.root_class();
        let name = self.lang.builtins.iter().find(|(_, b)| **b == Builtin::ClassTool(9)).map(|(n, _)| n.clone()).unwrap_or_default();
        let c = Rc::new(Class { outline: Some(format!("<class '{name}'>")), name,
            direct: vec![root.clone()], lineage: RefCell::new(vec![root.clone()]), base: Some(root), answers: vec![], fields: vec![], reaches: vec![],
            methods: vec![], constants: vec![],
            shared: RefCell::new(vec![
                (self.class_word("descriptor.get").to_string(), Self::adapter(203, vec![])),
                (self.lang.constructor.clone().unwrap_or_default(), Self::adapter(202, vec![])),
            ]),
            weak_storage: std::cell::Cell::new(None), declares_slots: false, sealed: std::cell::Cell::new(false), mro_adopted: std::cell::Cell::new(false), adopted_order: RefCell::new(Vec::new()), python_names: std::cell::RefCell::new(None) });
        self.staticmethod_class = Some(c.clone());
        c
    }
    /// Whether a value read as a class names the classmethod class: the
    /// builtin's word, read before anything was written to that name.
    pub(super) fn names_classmethod_class(&self, value: &Value) -> bool {
        self.names_wrapper_class(value, 10)
    }
    /// Whether a value read as a class names the staticmethod class.
    pub(super) fn names_staticmethod_class(&self, value: &Value) -> bool {
        self.names_wrapper_class(value, 9)
    }
    fn names_wrapper_class(&self, value: &Value, which: u8) -> bool {
        let word = match value { Value::Native(_, word) => word.as_ref(), Value::Adapter(w) if w.0 == 8 => match &w.1[0] { Value::Text(t) => t.as_ref(), _ => return false }, _ => return false };
        self.lang.builtins.get(word) == Some(&Builtin::ClassTool(which))
    }
    /// Whether a value read as a class names the property class: the
    /// builtin's word, read before anything was written to that name.
    pub(super) fn names_property_class(&self, value: &Value) -> bool {
        let word = match value { Value::Native(_, word) => word.as_ref(), Value::Adapter(w) if w.0 == 8 => match &w.1[0] { Value::Text(t) => t.as_ref(), _ => return false }, _ => return false };
        self.lang.builtins.get(word) == Some(&Builtin::ClassTool(11))
    }
    fn property_accessor(property: &Instance, place: &str) -> Option<Value> {
        property.fields.borrow().iter().find(|(n, _)| n == place).map(|(_, v)| v.clone()).filter(|v| !matches!(v, Value::Null))
    }
    /// A property's complaint about an accessor it has not: the words of
    /// the label around the property's name, where it was told one, and
    /// the class of the thing it was asked about.
    fn property_complaint(&self, part: &str, property: &Instance, thing: &Value) -> Fault {
        let pieces = self.lang.class_details.get(part).cloned().unwrap_or_default();
        if pieces.len() != 3 { return self.class_refusal(); }
        let named = Self::property_accessor(property, "\0name").map(|n| format!(" '{}'", n.plain())).unwrap_or_default();
        let of = match thing { Value::Object(o) => { let class = o.class_now(); let shown = class.python_names.borrow().as_ref().map_or_else(|| class.name.clone(), |names| names.1.type_text().plain()); shown }, Value::Class(c) => if self.class_word("name").is_empty() { c.name.clone() } else { Self::maker_beneath(c).map_or_else(|| c.name.clone(), |maker| maker.python_names.borrow().as_ref().map_or_else(|| maker.name.clone(), |names| names.0.type_text().plain())) }, other => other.plain() };
        format!("{}{named}{}{of}{}", pieces[0], pieces[1], pieces[2]).into()
    }
    /// The workings of the property class, each handed the property
    /// first and then what the program gave.
    fn property_work(&mut self, tag: u8, args: Vec<Value>) -> Flow<Value> {
        let Some(Value::Object(property)) = args.first().cloned() else { return Err(self.class_refusal()) };
        let given = &args[1..];
        match tag {
            20 => {
                let thing = given.first().cloned().unwrap_or(Value::Null);
                if matches!(thing, Value::Null) { return Ok(args[0].clone()); }
                let Some(getter) = Self::property_accessor(&property, "\0fget") else { return Err(self.property_complaint("property.unreadable", &property, &thing)) };
                self.class_apply(getter, vec![thing])
            }
            21 => {
                let [thing, value] = given else { return Err(self.class_refusal()) };
                let Some(setter) = Self::property_accessor(&property, "\0fset") else { return Err(self.property_complaint("property.unwritable", &property, thing)) };
                self.class_apply(setter, vec![thing.clone(), value.clone()])?;
                Ok(Value::Null)
            }
            22 => {
                let [thing] = given else { return Err(self.class_refusal()) };
                let Some(remover) = Self::property_accessor(&property, "\0fdel") else { return Err(self.property_complaint("property.undeletable", &property, thing)) };
                self.class_apply(remover, vec![thing.clone()])?;
                Ok(Value::Null)
            }
            // A fresh property of the same class, one accessor changed;
            // its stored name, including None, is carried over.
            23..=25 => {
                let [accessor] = given else { return Err(self.class_refusal()) };
                let place = ["\0fget", "\0fset", "\0fdel"][(tag - 23) as usize];
                let kept = |part: &str| if place == part { Some(accessor.clone()) } else { Self::property_accessor(&property, part) };
                // A copy is built the way any property is, from the
                // accessors and the docstring the reference carries over:
                // where the docstring came from a getter it is picked
                // again from the accessor the copy is made with.
                let carried = property.fields.borrow().iter().find(|(n, _)| n == "\0getterdoc").map(|(_, v)| matches!(v, Value::Flag(true))).unwrap_or(false);
                let doc = if carried && kept("\0fget").is_some() { Value::Null } else {
                    property.fields.borrow().iter().find(|(n, _)| n == "\0doc").map(|(_, v)| v.clone()).unwrap_or(Value::Null)
                };
                // The copy is made by calling the class, as the reference
                // does, so a subclass's own making has its say.
                let made = self.class_apply(Value::Class(property.class_now().clone()), vec![kept("\0fget").unwrap_or(Value::Null), kept("\0fset").unwrap_or(Value::Null), kept("\0fdel").unwrap_or(Value::Null), doc])?;
                if let Value::Object(instance) = &made {
                    let class = instance.class_now();
                    if self.property_class.as_ref().map_or(false, |known| Rc::ptr_eq(&class, known) || class.lineage.borrow().iter().any(|base| Rc::ptr_eq(base, known))) {
                        let stored_name = property.fields.borrow().iter().find(|(key, _)| key == "\0name").map(|(_, value)| value.clone());
                        if let Some(name) = stored_name {
                            let _ = Self::write_members(&mut instance.fields.borrow_mut(), "\0name", Some(name), false);
                        }
                    }
                }
                Ok(made)
            }
            26 => {
                let parts = ["property.fget", "property.fset", "property.fdel", "property.doc"];
                let mut kept: Vec<Value> = vec![Value::Null; 4];
                let mut at = 0;
                for (key, value) in self.call_items(given.to_vec())? {
                    let place = match key {
                        Some(key) => parts.iter().position(|p| self.class_word(p) == key).ok_or_else(|| Self::named_fault(&self.lang.call_unknown, &key))?,
                        None => { at += 1; at - 1 }
                    };
                    if place >= 4 { return Err(self.class_refusal()); }
                    kept[place] = value;
                }
                // Accessors change before the getter's doc is requested, so
                // a failed lookup leaves the same partial state as Python.
                let plain = self.property_class.as_ref().map_or(false, |known| Rc::ptr_eq(known, &property.class_now()));
                {
                    let mut fields = property.fields.borrow_mut();
                    fields.retain(|(n, _)| !matches!(n.as_str(), "\0fget" | "\0fset" | "\0fdel" | "\0name" | "\0doc" | "\0getterdoc"));
                    for (place, value) in ["\0fget", "\0fset", "\0fdel"].iter().zip(kept.iter().take(3)) { fields.push((place.to_string(), value.clone())); }
                    fields.push(("\0doc".to_string(), Value::Null));
                    fields.push(("\0getterdoc".to_string(), Value::Flag(false)));
                }
                // A docstring given is kept as it is; where none was given
                // the getter's own is taken, and that it came from there is
                // remembered so a later copy takes the new getter's.
                let mut doc = kept[3].clone();
                let mut getter_doc = false;
                let doc_word = self.class_word("doc").to_string();
                if matches!(doc, Value::Null) && !matches!(kept[0], Value::Null) {
                    match self.class_get(kept[0].clone(), &doc_word, false) {
                        Ok(found) => { if !matches!(found.contents(), Value::Null) { doc = found; getter_doc = true; } }
                        Err(fault) if self.attribute_fault(&fault) => {}
                        Err(fault) => return Err(fault),
                    }
                }
                {
                    let mut fields = property.fields.borrow_mut();
                    let _ = Self::write_members(&mut fields, "\0doc", Some(doc.clone()), false);
                    let _ = Self::write_members(&mut fields, "\0getterdoc", Some(Value::Flag(getter_doc)), false);
                }
                if !plain {
                    // A subclass keeps its docstring on the thing itself, as
                    // the reference does, so the class's own __doc__ does not
                    // shadow it. A class with nowhere to keep one drops an
                    // ordinary docstring and refuses one that came from a
                    // getter, which is the reference's own exception.
                    let writable = self.property_keeps_doc(&property.class_now());
                    if writable {
                        self.class_write(Value::Object(property.clone()), &doc_word, Some(doc), false)?;
                    } else if getter_doc {
                        return Err("AttributeError: readonly attribute".into());
                    }
                }
                Ok(Value::Null)
            }
            27 => {
                if given.len() != 2 { return Err(format!("TypeError: __set_name__() takes 2 positional arguments but {} were given", given.len()).into()); }
                let _ = Self::write_members(&mut property.fields.borrow_mut(), "\0name", Some(given[1].clone()), false);
                Ok(Value::Null)
            }
            _ => Err(self.class_refusal()),
        }
    }
    /// Whether a property still waits on an answer: any accessor it keeps
    /// carries a mark that weighs true. An accessor with no mark counts as
    /// not abstract; a mark that cannot be weighed raises as it would.

    /// What a property's kept accessor reads as: the accessor itself, or
    /// for its first string, the one it was given, else its getter's.
    fn property_reading(&mut self, property: &Instance, place: &str) -> Flow<Value> {
        if place == "\0name" { return self.property_name(property); }
        // Whether the property stands for a question nobody has
        // answered: a plain yes or no, taken from the marks on its
        // getter, its setter and its deleter alike. What a mark holds
        // is weighed as any truth is, and a complaint other than a
        // mark simply not being there travels on.
        if place == "\0abstract" {
            let word = self.class_word("property.is_abstract").to_string();
            for accessor in ["\0fget", "\0fset", "\0fdel"] {
                let Some(held) = Self::property_accessor(property, accessor) else { continue };
                let marked = match self.class_get(held, &word, false) {
                    Ok(v) => v,
                    Err(fault) if self.attribute_fault(&fault) => continue,
                    Err(fault) => return Err(fault),
                };
                if self.special_truth(&marked)? { return Ok(Value::Flag(true)); }
            }
            return Ok(Value::Flag(false));
        }
        if let Some(v) = Self::property_accessor(property, place) { return Ok(v); }
        if place == "\0doc" {
            if let Some(Value::Routine(getter)) = Self::property_accessor(property, "\0fget") {
                return Ok(getter.doc.clone().map_or(Value::Null, |s| Value::text(&s)));
            }
        }
        Ok(Value::Null)
    }
    /// Whether a traceback, or the chain it leads, comes back around to
    /// another: assigning such a chain would make a loop the reference
    /// refuses.
    fn traceback_reaches(from: &Value, target: &Rc<crate::value::Traceback>) -> bool {
        let mut here = from.clone();
        loop {
            let Value::Trace(trace) = here else { return false };
            if Rc::ptr_eq(&trace, target) { return true; }
            here = trace.next.borrow().clone();
        }
    }
    /// A traceback built by hand, as `types.TracebackType` builds one:
    /// what it follows, the frame it stands in, and where in that frame.
    fn traceback_from_parts(&mut self, args: Vec<Value>) -> Flow<Value> {
        let parts = ["tb_next", "tb_frame", "tb_lasti", "tb_lineno"];
        let mut slots: [Option<Value>; 4] = [None, None, None, None];
        let mut at = 0usize;
        for (key, value) in self.call_items(args)? {
            let slot = match key {
                Some(key) => parts.iter().position(|part| *part == key).ok_or_else(|| format!("TypeError: traceback() got an unexpected keyword argument '{key}'"))?,
                None => { let slot = at; at += 1; if slot >= 4 { return Err(format!("TypeError: traceback() takes at most 4 arguments ({} given)", at).into()); } slot }
            };
            if slots[slot].replace(value).is_some() { return Err(format!("TypeError: traceback() got multiple values for argument '{}'", parts[slot]).into()); }
        }
        let mut take = |slot: usize| -> Flow<Value> {
            slots[slot].take().ok_or_else(|| format!("TypeError: traceback() missing required argument '{}' (pos {})", parts[slot], slot + 1).into())
        };
        let next = take(0)?;
        let frame = take(1)?;
        let lasti = take(2)?;
        let lineno = take(3)?;
        let link = match next.contents() {
            Value::Null => Value::Null,
            Value::Trace(_) => next.clone(),
            other => return Err(format!("TypeError: expected traceback object or None, got '{}'", other.core_kind()).into()),
        };
        let Value::Object(holder) = frame.contents() else {
            return Err(format!("TypeError: traceback() argument 'tb_frame' must be frame, not {}", Self::type_argument_kind(&frame)).into());
        };
        if self.frame_class.as_ref().map_or(false, |kind| !Rc::ptr_eq(&holder.class_now(), kind)) {
            return Err(format!("TypeError: traceback() argument 'tb_frame' must be frame, not {}", Self::type_argument_kind(&frame)).into());
        }
        let instruction = self.traceback_number(&lasti)?;
        let line = self.traceback_number(&lineno)?;
        Ok(Value::Trace(Rc::new(crate::value::Traceback { instruction, location: None, line, frame: holder.clone(), next: RefCell::new(link) })))
    }
    /// A traceback's position or line: a whole number through `__index__`,
    /// which must fit where the reference keeps it.
    fn traceback_number(&mut self, value: &Value) -> Flow<i64> {
        let number = match value.contents() {
            Value::Small(n) => n,
            Value::Flag(flag) => i64::from(flag),
            Value::Huge(_) => match self.special_index(value)? {
                Some(Value::Huge(big)) => big.to_i64().ok_or_else(|| "OverflowError: Python int too large to convert to C int".to_string())?,
                _ => return Err("OverflowError: Python int too large to convert to C int".into()),
            },
            Value::Object(_) => match self.special_index(value)? {
                Some(Value::Small(n)) => n,
                Some(Value::Huge(big)) => big.to_i64().ok_or_else(|| "OverflowError: Python int too large to convert to C int".to_string())?,
                Some(other) => return Err(format!("TypeError: __index__ returned non-int (type {})", other.core_kind()).into()),
                None => return Err(format!("TypeError: '{}' object cannot be interpreted as an integer", value.core_kind()).into()),
            },
            other => return Err(format!("TypeError: '{}' object cannot be interpreted as an integer", other.core_kind()).into()),
        };
        if number < i32::MIN as i64 || number > i32::MAX as i64 {
            return Err("OverflowError: Python int too large to convert to C int".into());
        }
        Ok(number)
    }
    /// Whether a property subclass can keep a docstring on the thing
    /// itself: a class laying out slots keeps one only where a slot
    /// names it, and the property class, which keeps none, is no
    /// dictionary for a subclass.
    fn property_keeps_doc(&self, class: &Rc<Class>) -> bool {
        let Some(property) = &self.property_class else { return true };
        let slots_word = self.class_word("slots");
        let doc_word = self.class_word("doc");
        let names = self.class_word("namespace");
        let names_it = |listed: &[Value]| listed.iter().any(|s| matches!(s.contents(), Value::Text(t) if t.as_ref() == doc_word || t.as_ref() == names));
        let mut current = class.clone();
        loop {
            if Rc::ptr_eq(&current, property) { return false; }
            match Self::own_class_value(&current, &slots_word) {
                Some(slots) => {
                    let listed: Vec<Value> = match slots.contents() { Value::Array(v) | Value::Tuple(v) => v.to_vec(), other => vec![other] };
                    if names_it(&listed) { return true; }
                }
                None => return true,
            }
            let Some(base) = current.base.clone() else { return false };
            current = base;
        }
    }
    /// A property's name: the one a class gave it at taking, else the
    /// getter's own, else a complaint, as the reference has it.
    fn property_name(&mut self, property: &Instance) -> Flow<Value> {
        if let Some((_, held)) = property.fields.borrow().iter().find(|(n, _)| n == "\0name") { return Ok(held.clone()); }
        if let Some(getter) = Self::property_accessor(property, "\0fget") {
            let name_word = self.class_word("name").to_string();
            return match self.class_get(getter, &name_word, false) {
                Ok(held) => Ok(held),
                Err(fault) if self.attribute_fault(&fault) => Err("AttributeError: 'property' object has no attribute '__name__'".into()),
                Err(fault) => Err(fault),
            };
        }
        Err("AttributeError: 'property' object has no attribute '__name__'".into())
    }
    /// The hook a class member answers the protocol with, where the
    /// member is a thing whose class furnishes one.
    fn descriptor_hook(&self, member: &Value, part: &str) -> Option<Value> {
        let word = self.class_word(part);
        if word.is_empty() { return None; }
        let Value::Object(o) = member else { return None };
        self.class_value(&o.class_now(), word)
    }
    fn call_descriptor(&mut self, member: &Value, hook: Value, args: Vec<Value>) -> Flow<Value> {
        let Value::Object(o) = member else { return Err(self.class_refusal()) };
        // Native descriptor slots already receive their descriptor explicitly.
        // Resolve overrides normally and omit only the temporary bound wrapper.
        if matches!(&hook, Value::Adapter(slot) if matches!(slot.0, 50..=57 | 201 | 203)) {
            let mut supplied = Vec::with_capacity(args.len() + 1);
            supplied.push(member.clone()); supplied.extend(args);
            if let Value::Adapter(slot) = &hook {
                if matches!(slot.0, 50..=57) { return self.property_work(slot.0, supplied); }
            }
            return self.class_apply(hook, supplied);
        }
        let bound = self.bind_class_value(hook, Some(member.clone()), o.class_now().clone())?;
        self.class_apply(bound, args)
    }
    /// A member that takes writes as well as reads: it is asked before a
    /// thing's own fields, where one that only reads gives way to them.
    pub(super) fn reads_descriptor(&self, subject: &Value, name: &str) -> bool {
        let Value::Object(instance) = subject else { return false };
        self.class_value(&instance.class_now(), name).is_some_and(|entry| {
            matches!(&entry, Value::Adapter(parts) if matches!(parts.0, 6 | 28) || parts.0 == 16 && parts.1.get(2).is_some_and(|v| v.plain() == "\0module-namespace"))
                || self.descriptor_hook(&entry, "descriptor.get").is_some()
        })
    }
    fn takes_writes(&self, member: &Value) -> bool {
        if let Value::Adapter(w) = member { return matches!(w.0, 6 | 16 | 28); }
        if self.lang.bind_names && matches!(member, Value::Null | Value::Small(_) | Value::Huge(_) | Value::Flag(_) | Value::Real(_) | Value::Text(_) | Value::Array(_) | Value::Tuple(_) | Value::Map(_) | Value::Set(_) | Value::Bytes(..) | Value::Routine(_) | Value::Method(..)) {
            return false;
        }
        self.descriptor_hook(member, "descriptor.set").is_some() || self.descriptor_hook(member, "descriptor.delete").is_some()
    }
    /// Whether a fault is a missing member's, however it was raised.
    pub(super) fn attribute_fault(&self, fault: &Fault) -> bool {
        let kind = self.class_word("attribute.amiss").split(':').next().unwrap_or_default();
        if kind.is_empty() { return false; }
        match fault {
            Fault::Note(told) => told.split(':').next() == Some(kind),
            Fault::Thrown(Value::Object(o)) => std::iter::once(&o.class_now()).chain(o.class_now().lineage.borrow().iter()).any(|c| c.name == kind),
            _ => false,
        }
    }
    fn own_class_value(c: &Class, name: &str) -> Option<Value> {
        if let Some((_,v)) = c.shared.borrow().iter().find(|(n,_)| n == name) {
            return if matches!(v.contents(), Value::Blank | Value::Gap) { None } else { Some(v.clone()) };
        }
        c.methods.iter().find(|(n,_)| n == name).map(|(_,p)| Value::Routine(p.clone()))
            .or_else(|| c.constants.iter().find(|(n,_)| n == name).map(|(_,v)| v.clone()))
    }
    pub(super) fn kind_owns_protocol(&self, kind: &str, name: &str) -> bool {
        self.lang.class_details.get("native.protocols").is_some_and(|entries| {
            entries.chunks_exact(2).any(|entry| entry[0] == kind && entry[1].split_whitespace().any(|word| word == name))
        })
    }
    pub(super) fn allocation_is_custom(&self, c: &Class) -> bool {
        if !self.lang.class_details.get("native.protocols").is_some_and(|entries| !entries.is_empty()) { return false; }
        let name = self.class_word("allocate");
        Self::ordered_lookup(c, |base| {
            if let Some(value) = Self::own_class_value(base, name) { return Some(!matches!(value, Value::Adapter(hook) if hook.0 == 14)); }
            if Self::own_kind(base).is_some_and(|kind| self.kind_owns_protocol(&kind, name)) { return Some(false); }
            None
        }).unwrap_or(false)
    }
    fn ordered_lookup<T>(c: &Class, mut visit: impl FnMut(&Class) -> Option<T>) -> Option<T> {
        if c.mro_adopted.get() {
            return c.adopted_order.borrow().iter().filter_map(std::rc::Weak::upgrade).find_map(|base| visit(&base));
        }
        std::iter::once(c).chain(c.lineage.borrow().iter().map(Rc::as_ref)).find_map(visit)
    }

    pub(super) fn class_value(&self, c: &Class, name: &str) -> Option<Value> {
        Self::ordered_lookup(c, |base| {
            Self::own_class_value(base, name).or_else(|| {
                if !self.lang.fuller_classes { return None; }
                let word = Self::own_kind(base)?;
                if self.lang.class_special.get(8).is_some_and(|hash| hash == name)
                    && matches!(word.as_str(), "list" | "dict" | "set" | "bytearray") { return Some(Value::Null); }
                if !self.kind_owns_protocol(&word, name) { return None; }
                let sample = self.kind_sample(&word)?;
                if self.lang.class_special.get(8).is_some_and(|hash| hash == name)
                    && self.lang.class_special.get(2).is_some_and(|eq| self.native_special(&sample, eq))
                    && !self.native_special(&sample, name) { return Some(Value::Null); }
                if !self.native_special(&sample, name) { return None; }
                Some(self.held_kind_descriptor(&word, name))
            })
        })
    }
    fn closure_title(name: &str) -> &str {
        if name.starts_with("#class_cell") { "__class__" } else { name }
    }
    pub(super) fn adapter(kind: u8, values: Vec<Value>) -> Value { Value::Adapter(Rc::new((kind,values))) }
    fn descriptor_apply(&mut self, callable: Value, args: Vec<Value>) -> Flow<Value> {
        // Native callables use ordinary dispatch, and stored strings stay
        // strings. Routine calls retain their attached namespace here.
        if !self.lang.class_builder.is_empty() && matches!(callable.contents(), Value::Native(..) | Value::ByteKind(..) | Value::SortOf(..) | Value::Text(_)) {
            self.call_held(callable, args).map_err(|words| self.carried.take().unwrap_or_else(|| words.into()))
        } else { self.class_apply(callable, args) }
    }
    pub(super) fn validate_class_spreads(&self, class: &Rc<Class>, args: &[Value]) -> Flow<()> {
        for argument in args {
            if let Value::Tie(pair)=argument {
                if matches!(pair.0,Value::Flag(true)) && !matches!(Self::worth_of(&pair.1).unwrap_or_else(||pair.1.contents()).contents(),Value::Map(_)|Value::Fields(_)) {
                    let title=class.python_title().unwrap_or_else(||class.name.clone());
                    return Err(format!("TypeError: {title}() argument after ** must be a mapping, not {}",pair.1.core_kind()).into());
                }
            }
        }
        Ok(())
    }
    pub(super) fn class_apply(&mut self, callable: Value, mut args: Vec<Value>) -> Flow<Value> {
        if let Value::Class(class)=&callable { self.validate_class_spreads(class, &args)?; }
        let callable = match callable {
            held @ (Value::Bond(_) | Value::Binding(_)) => self.what_it_spells(held),
            held => held,
        };
        match callable {
            Value::Bond(cell) => { let held = cell.borrow().clone(); self.class_apply(held, args) },
            Value::Routine(p) => { self.invoke(&p,args)?; Ok(self.drop_top()?) }
            Value::Native(operation, name) => {
                let items = self.call_items_named(&name, args)?;
                let answer = self.builtin_call(operation, &name, items);
                if let Some(raised) = self.carried.take() { return Err(raised); }
                Ok(answer?)
            }
            // A method bound to a value of a builtin kind, reached as a
            // value in its own right and then called.
            Value::ValueMethod(bound) => {
                if self.lang.class_details.get("native.protocols").is_some_and(|names| !names.is_empty()) {
                    let mut positional = Vec::new(); let mut named = Vec::new();
                    for (key, value) in self.call_items(args)? {
                        match key { Some(key) => named.push((key, value)), None => positional.push(value) }
                    }
                    let answer = self.value_method(&bound.0, &bound.1, positional, named);
                    if let Some(fault) = self.carried.take() { return Err(fault); }
                    Ok(answer?)
                } else { Ok(self.value_method(&bound.0, &bound.1, args, Vec::new())?) }
            },
            Value::Method(o,p,_) => { args.insert(0,Value::Object(o)); self.invoke(&p,args)?; Ok(self.drop_top()?) }
            // A thing called stands on its own call member, which may be
            // a thing again: each such step is counted with the calls
            // standing, so a thing whose call member is a thing of its
            // own kind is refused at the depth any endless call is.
            Value::Object(o) => {let f=self.class_value(&o.class_now(),self.class_word("call")).ok_or_else(||self.class_refusal())?;self.reaching_further()?;args.insert(0,Value::Object(o));let answer=self.class_apply(f,args);self.answered();answer},
            Value::Class(c) => self.class_make(c,args),
            Value::Adapter(w) => match w.0 {
                236 => self.super_initialised(args),
                235 => {
                    let items = self.call_items(args)?;
                    let mut positional = Vec::new(); let mut keywords = Vec::new();
                    for (key, value) in items { match key { None => positional.push(value), Some(name) => keywords.push((name, value)) } }
                    if positional.is_empty() { return Err("TypeError: descriptor '__get__' of 'classmethod_descriptor' object needs an argument".into()); }
                    let descriptor = positional.remove(0).contents();
                    if !matches!(&descriptor, Value::Adapter(parts) if parts.0 == 29 && parts.1.len() == 3) {
                        return Err(format!("TypeError: descriptor '__get__' requires a 'classmethod_descriptor' object but received a '{}'", descriptor.core_kind()).into());
                    }
                    self.explicit_classmethod(&descriptor, positional, keywords)
                }

                82 => {
                    // A group's own maker: the class to make, then its
                    // heading and exceptions, as the reference's __new__.
                    let Some(receiver) = args.first().map(Value::contents) else {
                        return Err("TypeError: BaseExceptionGroup.__new__(): not enough arguments".into());
                    };
                    let Value::Class(class) = &receiver else {
                        if let Some(name) = receiver.kind_it_names().or_else(|| self.kind_spelled(&receiver).map(|word| word.to_string())) {
                            return Err(format!("TypeError: BaseExceptionGroup.__new__({name}): {name} is not a subtype of BaseExceptionGroup").into());
                        }
                        return Err(format!("TypeError: BaseExceptionGroup.__new__(X): X is not a type object ({})", Self::type_argument_kind(&receiver)).into());
                    };
                    if !self.stands_on(class, 37) {
                        let name = &class.name;
                        return Err(format!("TypeError: BaseExceptionGroup.__new__({name}): {name} is not a subtype of BaseExceptionGroup").into());
                    }
                    self.exception_allocate(class.clone(), args[1..].to_vec())
                }
                129 => {
                    if !args.is_empty() { return Err("TypeError: function takes no arguments".into()); }
                    Ok(self.builtin_call(Builtin::Eval, "eval", w.1.iter().cloned().map(|value| (None, value)).collect())?)
                }
                126 => {
                    let Some(Value::Object(union)) = args.first() else { return Err(self.class_refusal()); };
                    let mut parts = Vec::new();
                    if let Some((_, Value::Tuple(members))) = union.fields.borrow().iter().find(|(key, _)| key == "__args__") {
                        for member in members.iter() {
                            let shown = if matches!(member.contents(), Value::Class(_)) {
                                let module = self.class_get(member.clone(), "__module__", false)?.plain();
                                let name = self.class_get(member.clone(), "__qualname__", false)?.plain();
                                if module == "builtins" { name } else { format!("{module}.{name}") }
                            } else if let Some(name) = member.kind_it_names() { name }
                            else { self.special_text(member, true)? };
                            parts.push(if shown == "NoneType" { "None".into() } else { shown });
                        }
                    }
                    Ok(Value::text(&parts.join(" | ")))
                }
                140 => {
                    let subscript = args.last().cloned().unwrap_or(Value::Null).contents();
                    let members: Vec<Value> = match subscript { Value::Tuple(row) => row.iter().cloned().collect(), other => vec![other] };
                    if members.is_empty() { return Err("TypeError: Cannot take a Union of no types.".into()); }
                    let mut folded: Option<Value> = None;
                    for member in members {
                        folded = Some(match folded { None => member, Some(acc) => self.join_types(&acc, &member) });
                    }
                    Ok(folded.unwrap_or(Value::Null))
                }
                124 => self.alias_work(match w.1[0] { Value::Small(mode) => mode, _ => -1 }, args),
                122 => self.namespace_work(match w.1[0] { Value::Small(mode) => mode, _ => -1 }, args),
                9 if w.1.is_empty() && args.len() == 2 => {
                    let parent = args[0].contents();
                    if !matches!(parent, Value::Class(_)) { return Err(self.class_refusal()); }
                    Ok(Self::adapter(9, vec![parent, args[1].contents()]))
                },
                // C-only draw methods can read the native stream without
                // building a Python frame. Other calls use the full reader.
                63 => {
                    let bits = w.1[0].plain() == "bits";
                    let valid = args.len() == if bits { 2 } else { 1 }
                        && matches!(args.first().map(Value::contents), Some(Value::Object(_)))
                        && (!bits || matches!(args[1].contents(), Value::Small(n) if n >= 0));
                    if valid {
                        match self.class_get(args[0].clone(), "_stream", false) {
                            Ok(mark @ Value::Small(_)) => {
                                let mut native = vec![w.1[0].clone(), mark];
                                if bits { native.push(args[1].clone()); }
                                return self.class_apply(Value::text("__random"), native);
                            }
                            Err(fault) if !self.attribute_fault(&fault) => return Err(fault),
                            _ => {},
                        }
                    }
                    self.class_apply(w.1[1].clone(), args)
                }
                // Ordinary binary64 inputs need no Python conversion frame.
                // Domain edges and custom numeric protocols keep that frame.
                64 => {
                    let operation = w.1[0].plain();
                    if operation == "normal_pdf" && args.len() == 2 {
                        let x = args[1].contents();
                        if matches!(x, Value::Real(_)) {
                            let mu = self.class_get(args[0].clone(), "_mu", false)?.contents();
                            let sigma = self.class_get(args[0].clone(), "_sigma", false)?.contents();
                            if let (Value::Real(m), Value::Real(s), Value::Real(point)) = (&mu, &sigma, &x) {
                                let scale = crate::value::as_binary(&s.p, &s.q);
                                let mean = crate::value::as_binary(&m.p, &m.q);
                                let input = crate::value::as_binary(&point.p, &point.q);
                                if mean.is_finite() && input.is_finite() && scale.is_finite() && scale * scale > 0.0 && (scale * scale).is_finite() {
                                    return self.class_apply(Value::text("__math"), vec![w.1[0].clone(), x, mu, sigma]);
                                }
                            }
                        }
                    }
                    if operation == "sqrt_frac_rto" && args.len() == 2 && args.iter().all(|value| matches!(value.contents(), Value::Small(_) | Value::Huge(_) | Value::Flag(_))) {
                        return self.class_apply(Value::text("__math"), vec![w.1[0].clone(), args[0].clone(), args[1].clone()]);
                    }
                    if operation == "isqrt" && args.len() == 1 {
                        let number = args[0].contents();
                        if matches!(number, Value::Small(_) | Value::Huge(_) | Value::Flag(_)) {
                            return self.class_apply(Value::text("__math"), vec![w.1[0].clone(), number]);
                        }
                    }
                    if operation == "fsum" && self.lang.math_fsum && args.len() == 1 {
                        let values = match args[0].contents() { Value::Array(values) | Value::Tuple(values) => Some(values), _ => None };
                        if values.is_some_and(|row| row.iter().all(|item| matches!(item.contents(), Value::Small(_) | Value::Huge(_) | Value::Real(_) | Value::Flag(_)))) {
                            return self.class_apply(Value::text("__math"), vec![w.1[0].clone(), args[0].clone()]);
                        }
                    }
                    if args.len() == 1 {
                        let number = args[0].contents();
                        if operation == "floor" && matches!(number, Value::Small(_) | Value::Huge(_)) {
                            return Ok(number);
                        }
                        // The representation already records finiteness and
                        // sign. Do not round it just to inspect the domain;
                        // the native operation performs that conversion once.
                        let signs = match number {
                            Value::Small(n) => Some((n >= 0, n > 0)),
                            Value::Real(real) if !real.q.is_zero() => {
                                let positive = !real.p.is_zero() && real.p.sign() == real.q.sign();
                                Some((real.p.is_zero() || positive, positive))
                            }
                            _ => None,
                        };
                        if let Some((nonnegative, positive)) = signs {
                            let domain = match operation.as_str() {
                                "log" | "lgamma" | "log2" => positive,
                                "sqrt" => nonnegative,
                                "exp" | "floor" | "fabs" => true,
                                "frexp" => self.lang.math_frexp,
                                _ => false,
                            };
                            if domain {
                                return self.class_apply(Value::text("__math"), vec![w.1[0].clone(),args[0].clone()]);
                            }
                        }
                    }
                    self.class_apply(w.1[1].clone(),args)
                }
                0 => Ok(w.1[0].clone()),
                133 => {
                    let owner = w.1[0].plain();
                    if args.is_empty() { return Err(format!("TypeError: unbound method {owner}.__reduce__() needs an argument").into()); }
                    let native = Self::worth_of(&args[0]).unwrap_or_else(|| args[0].contents()).contents();
                    if matches!(self.lang.builtins.get(&owner), Some(Builtin::Enumerate | Builtin::Zip | Builtin::Map | Builtin::Filter | Builtin::Reversed)) && matches!(native, Value::Cursor(_)) && native.core_kind() == owner {
                        if args.len() != 1 { return Err(format!("TypeError: {owner}.__reduce__() takes no arguments ({} given)", args.len()-1).into()); }
                        return self.pickle_reduction(&args[0]).map_err(Fault::Note);
                    }
                    if !matches!(&native, Value::Set(_)) || native.set_fixed() != (self.lang.builtins.get(&owner) == Some(&Builtin::Frozen)) {
                        return Err(format!("TypeError: descriptor '__reduce__' for '{owner}' objects doesn't apply to a '{}' object", args[0].core_kind()).into());
                    }
                    if args.len() > 1 { return Err(format!("TypeError: {}.__reduce__() takes no arguments ({} given)", args[0].core_kind(), args.len() - 1).into()); }
                    return self.set_reduction(&args[0]);
                },
                132 => {
                    if !args.is_empty() { return Err(format!("TypeError: builtin_function_or_method.__reduce__() takes no arguments ({} given)", args.len()).into()); }
                    Ok(w.1[0].clone())
                },
                1 => {
                    let Some(Value::Class(c)) = args.first() else { return Err(format!("TypeError: object.__new__(X): X is not a type object ({})", args.first().map_or("NoneType".into(), Value::core_kind)).into()); };
                    if c.shared.borrow().iter().any(|(key, flag)| key == "\0buffer_allocator" && flag.is_true()) {
                        let name = c.python_title().unwrap_or_else(|| c.name.clone());
                        return Err(format!("TypeError: object.__new__({name}) is not safe, use {name}.__new__()").into());
                    }
                    if self.exception_class(c) { return Ok(self.exception_instance(c.clone(), args[1..].to_vec(), Value::Null)); }
                    self.abstract_refusal(c)?;
                    if args.len()!=1 { self.root_refuses_arguments(c,true)?; }
                    self.made += 1;
                    Ok(Value::Object(Rc::new(Instance {replacement_class: RefCell::new(None), class:c.clone(),fields:RefCell::new(vec![]),mark:self.made})))
                }
                2 if w.1.first().is_some_and(|v| v.plain() == "module") => {
                    if args.is_empty() { return Err("TypeError: descriptor '__init__' of 'module' object needs an argument".into()); }
                    let receiver = args.remove(0);
                    self.initialise_module(receiver, args)
                    }
                2 if w.1.first().is_some_and(|v| v.plain() == "__init_subclass__") => {
                    let given = self.call_items(args)?;
                    let Some((None, Value::Class(class))) = given.first() else { return Err(self.class_refusal()); };
                    if given.len() != 1 { return Err(format!("TypeError: {}.__init_subclass__() takes no keyword arguments", class.name).into()); }
                        Ok(Value::Null)
                    }
                2 if !w.1.is_empty() => {
                    let word=w.1[0].plain();
                    let Some(Value::Object(object))=args.first().cloned() else { return Err(self.class_refusal()); };
                    args.remove(0);
                    let op=*self.lang.builtins.get(&word).ok_or_else(||self.class_refusal())?;
                    if op == Builtin::List && self.class_value(&object.class_now(),self.class_word("allocate")).is_some_and(|maker| !matches!(maker,Value::Adapter(_))) {
                        args=self.call_items(args)?.into_iter().filter_map(|(key,value)|key.is_none().then_some(value)).collect();
                    }
                    let initialized=self.descriptor_apply(Value::Native(op,Rc::from(word)),args)?;
                    let mut fields=object.fields.borrow_mut();
                    let slot=fields.iter_mut().find(|(name,_)|name=="\0worth").ok_or_else(||self.class_refusal())?;
                    if let Value::Collection(cell,_)|Value::Bond(cell)=&slot.1 { *cell.borrow_mut()=initialized.contents(); }
                    else { slot.1=initialized.held(false); }
                    Ok(Value::Null)
                }
                2 if matches!(args.first(), Some(Value::Object(o)) if self.exception_class(&o.class_now())) => {
                    let Value::Object(o) = args.remove(0) else { unreachable!() };
                    let name = self.lang.constructor.clone().unwrap_or_default();
                    self.exception_method(o, &name, &args)
                }
                2 if args.first().is_some_and(|value| matches!(value.contents(), Value::Tuple(_))) => Ok(Value::Null),
                2 if args.len()==1 => Ok(Value::Null),
                2 => {
                    let Some(Value::Object(o)) = args.first() else { return Err(self.class_refusal()); };
                    self.root_refuses_arguments(&o.class_now(),false)?;
                    Ok(Value::Null)
                }
                45 if !args.is_empty() => {
                    let alias = self.lazy_type_alias(args.remove(0))?;
                    if let (Value::Object(instance), Some(parameters)) = (&alias, args.first()) {
                        if let Some((_, held)) = instance.fields.borrow_mut().iter_mut().find(|(key, _)| key == "__type_params__") { *held = parameters.clone(); }
                    }
                    Ok(alias)
                }
                74 if args.len() == 3 => {
                    if let Value::Class(owner) = args[0].contents() {
                        if let Some(value) = Self::own_class_value(&owner, &args[1].plain()) { return Ok(value.contents()); }
                    }
                    self.class_apply(args[2].clone(), Vec::new())
                }
                77 if w.1.first().is_some_and(|v| v.plain() == "#union") && args.len() == 2 => {
                    let entries = match args[1].contents() { Value::Tuple(row) => row.to_vec(), item => vec![item] };
                    if entries.is_empty() { return Err("TypeError: Cannot take a Union of no types.".into()); }
                    let typing = self.import_module("typing")?;
                    let check = self.class_get(typing, "_type_check", false)?.contents();
                    let mut checked = Vec::new();
                    for entry in entries { checked.push(self.class_apply(check.clone(), vec![entry, Value::text("Union[arg, ...]: each arg must be a type.")])?); }
                    let mut result = checked.remove(0);
                    for item in checked { result = self.join_types(&result, &item); }
                    Ok(result)
                }
                77 if self.lang.type_parameters && args.len() == 2 && matches!(args[0], Value::Class(_)) => {
                    Ok(args.remove(0))
                }
                154 => {
                    let module = self.import_module("typing")?;
                    let function = self.class_get(module, &w.1[0].plain(), false)?;
                    args.insert(0, w.1[1].clone());
                    self.class_apply(function, args)
                }
                152 | 153 => {
                    if w.0 == 152 {
                        if let Some(Value::Class(owner)) = args.first() {
                            if let Some(parameters) = Self::own_class_value(owner, "__type_params__") {
                                if parameters.is_true() {
                                    owner.shared.borrow_mut().push(("__parameters__".into(), parameters));
                                    return Ok(Value::Null);
                                }
                            }
                        }
                    }
                    let module = self.import_module("typing")?;
                    let word = if w.0 == 152 { "_generic_init_subclass" } else { "_generic_class_getitem" };
                    let function = self.class_get(module, word, false)?;
                    self.class_apply(function, args)
                }
                49 if args.len() == 1 => {
                    let module = self.import_module("typing")?;
                    let alias = self.class_get(module, "_GenericAlias", false)?;
                    let generic = Value::Class(self.kind_class("Generic"));
                    self.class_apply(alias.contents(), vec![generic, args.remove(0)])
                }
                49 if args.is_empty() => {
                    self.typing_module();
                    Ok(Value::Class(self.kind_class("Generic")))
                }
                48 => {
                    if args.len() != 1 { return Err("TypeError: constevaluator.__call__() takes exactly 1 argument (0 given)".into()); }
                    self.data.extend([args[0].clone(), Value::Small(4)]);
                    self.perform(&Action::Eq, 2)?;
                    if self.drop_top()?.is_true() {
                        let shown = w.1[0].plain();
                        let text = shown.strip_prefix("<class '").and_then(|s| s.strip_suffix("'>")).unwrap_or(&shown);
                        return Ok(Value::text(text));
                    }
                    Ok(w.1[0].clone())
                }
                47 => {
                    if args.len() > 1 { return Err("TypeError: evaluator takes at most one argument".into()); }
                    let format = args.first().cloned().unwrap_or(Value::Small(1));
                    self.data.extend([format, Value::Small(2)]);
                    self.perform(&Action::Gt, 2)?;
                    if self.drop_top()?.is_true() { return Err("NotImplementedError: ".into()); }
                    self.class_apply(w.1[0].clone(), Vec::new())
                }
                46 if args.len() == 5 => {
                    let class = self.kind_class(&args[1].plain());
                    let parameter = self.typing_instance(class, vec![args[0].clone()])?;
                    let Value::Object(instance) = &parameter else { unreachable!() };
                    let mut fields = instance.fields.borrow_mut();
                    if !matches!(args[2], Value::Null) {
                        fields.push(("\0type_bound".into(), args[2].clone()));
                        fields.push(("\0type_constraints".into(), args[4].clone()));
                    }
                    if !matches!(args[3], Value::Null) {
                        fields.retain(|(key, _)| key != "__default__");
                        fields.push(("\0lazy:__default__".into(), args[3].clone()));
                    }
                    if instance.class_now().name == "ParamSpec" {
                        if let Some((_, bound)) = fields.iter_mut().find(|(key, _)| key == "__bound__") { *bound = Value::Null; }
                    }
                    if let Some((_, variance)) = fields.iter_mut().find(|(key, _)| key == "__infer_variance__") { *variance = Value::Flag(true); }
                    if let Some((_, shown)) = fields.iter_mut().find(|(key, _)| key == "\0typing_repr") { *shown = args[0].clone(); }
                    drop(fields);
                    Ok(parameter)
                }
                // A member a native forebear carries, read off the
                // parent walk: it works upon the worth the thing keeps,
                // reached past the class the walk was made against, as
                // the parent call of a method spelled out does.
                44 if w.1.len() == 3 => {
                    let receiver = w.1[0].clone();
                    let owner = w.1[1].plain();
                    let name = w.1[2].plain();
                    self.class_super(receiver, &owner, &name, args)
                }
                44 => {
                    if args.len() != 1 { return Err("TypeError: __annotate__() requires one argument".into()); }
                    self.data.extend([args[0].clone(), Value::Small(2)]);
                    self.perform(&Action::Gt, 2)?;
                    if self.drop_top()?.is_true() { return Err("NotImplementedError: ".into()); }
                    match &w.1[0] {
                        Value::Class(owner) => self.evaluate_class_annotations(owner),
                        Value::Array(row) => {
                            let mut entries = Vec::new();
                            for pair in row.chunks(2) {
                                let [key, evaluator] = pair else { break; };
                                let evaluator = evaluator.contents();
                                if matches!(evaluator, Value::Blank) { continue; }
                                let value = match evaluator { Value::Routine(_) | Value::Method(..) => self.class_apply(evaluator, Vec::new())?, value => value };
                                entries.push((key.clone(), value));
                            }
                            Ok(self.keep_collection(Value::Map(Rc::new(entries.into()))))
                        }
                        _ => Err(self.class_refusal()),
                    }
                }
                78 => self.type_initialiser(args),
                40 => self.class_from_parts(args),
                41 => {
                    let opened = self.call_items(args)?;
                    let mut values: Vec<Value> = opened.into_iter().map(|(key, value)| match key {
                        Some(key) => Value::Tie(Rc::new((Value::text(&key), value))), None => value,
                    }).collect();
                    if values.is_empty() { return Err(self.class_refusal()); }
                    let target = values.remove(0);
                    if let Value::Class(class) = target { self.class_construct(class, values) }
                    else if self.stands_for_kind(&target) { self.class_apply(target, values) }
                    else { Err(self.class_refusal()) }
                }
                180 => {
                    if !args.is_empty() { return Err("TypeError: function takes no arguments".into()); }
                    self.text_run(true, "eval", vec![w.1[0].clone(), w.1[1].clone()]).map_err(Fault::from)
                }
                143 => {
                    if !args.is_empty() { return Err("TypeError: this code object takes no arguments".into()); }
                    let result = self.text_run(true, "eval", w.1[..2].to_vec());
                    match result { Ok(value) => Ok(value), Err(message) => Err(self.carried.take().unwrap_or_else(|| message.into())) }
                }
                43 => {

                    self.class_apply(w.1[0].clone(), args)
                }
                200|202 => {
                    // A wrapper's own making: the callable it is given
                    // is kept under a name no program can spell.
                    let Some(Value::Object(o)) = args.first() else { return Err(self.class_refusal()); };
                    let Some(callable) = args.get(1) else { return Err(self.class_refusal()); };
                    o.fields.borrow_mut().push(("\0callable".to_string(), callable.clone()));
                    Ok(Value::Null)
                }
                201 => {
                    // classmethod.__get__: the kept callable is bound to
                    // the class the read came through, as the reference
                    // binds it, never to nothing.
                    let Some(Value::Object(o)) = args.first() else { return Err(self.class_refusal()); };
                    let Some(callable) = o.fields.borrow().iter().find(|(n, _)| n == "\0callable").map(|(_, v)| v.clone()) else { return Err(self.class_refusal()) };
                    let bound = args[1..].iter().find_map(|v| match v { c @ Value::Class(_) => Some(c.clone()), _ => None })
                        .unwrap_or_else(|| args.get(1).cloned().unwrap_or(Value::Null));
                    Ok(Self::adapter(3, vec![callable, bound]))
                }
                203 => {
                    // staticmethod.__get__: the kept callable itself,
                    // unbound.
                    let Some(Value::Object(o)) = args.first() else { return Err(self.class_refusal()) };
                    let Some(callable) = o.fields.borrow().iter().find(|(n, _)| n == "\0callable").map(|(_, v)| v.clone()) else { return Err(self.class_refusal()) };
                    Ok(callable)
                }
                204 => {
                    let Value::Class(c) = &w.1[0] else { return Err(self.class_refusal()); };
                    let mut live = Vec::new();
                    let mut children = self.class_children.borrow_mut();
                    if let Some(list) = children.get_mut(&(Rc::as_ptr(c) as usize)) {
                        list.retain(|weak| weak.upgrade().is_some());
                        live = list.iter().filter_map(|weak| weak.upgrade().map(Value::Class)).collect();
                    }
                    drop(children);
                    Ok(Value::array(live))
                }
                42 => Ok(Value::Collection(Rc::new(RefCell::new(Value::Map(Rc::new(Vec::new().into())))), true)),
                30 => {
                    let operation = w.1[0].plain();
                    // State and reduction answer for the thing itself, not
                    // for the native worth beneath it, so they are given
                    // the thing whole.
                    let holds_state = self.lang.class_details.get("root.members").map_or(false, |names| names.get(10) == Some(&operation) || names.get(11) == Some(&operation) || names.get(12) == Some(&operation));
                    if !holds_state && w.1.get(1).is_some_and(|owner| owner.plain() != self.class_word("root")) {
                        if let Some(first) = args.first_mut() {
                            if let Some(native) = Self::worth_of(first) { *first = native.contents(); }
                        }
                    }
                    self.root_work(&operation,args)
                },
                // The maker of a builtin kind: given the class to make a
                // thing of and what the kind's builtin takes.
                233 if args.len() == 1 => self.calendar_repr(&args[0]),
                234 if args.len() == 1 => self.calendar_reduce(&args[0]),
                14 if !args.is_empty() => {
                    let kind = args.remove(0);
                    let word = w.1[0].plain();
                    if word == "struct_time" { return self.calendar_sequence(kind, args); }
                    // The parent class's own allocation lays out a bare
                    // thing of the class asked for; what it is to hold
                    // comes from the constructing after.
                    if self.lang.fuller_classes && word == "super" {
                        let Value::Class(c) = &kind else { return Err(self.class_refusal()); };
                        if !Self::super_descended(c) { return Err(self.class_refusal()); }
                        self.made += 1;
                        return Ok(Value::Object(Rc::new(Instance { replacement_class: RefCell::new(None), class: c.clone(), fields: RefCell::new(Vec::new()), mark: self.made })));
                    }
                    if let Value::Native(op @ (Builtin::Set | Builtin::Frozen), name) = &kind {
                        if name.as_ref() != word { return Err(self.class_refusal()); }
                        let mut given = if *op == Builtin::Set { Vec::new() } else { args };
                        return Ok(self.builtin(*op, name, &mut given)?);
                    }
                    if self.lang.builtins.get(&word) == Some(&Builtin::Bool) {
                        return match &kind {
                            Value::Native(Builtin::Bool, name) if name.as_ref() == word => {
                                if args.len() > 1 {
                                    Err(format!("TypeError: bool expected at most 1 argument, got {}", args.len()).into())
                                } else { Ok(self.builtin(Builtin::Bool, name, &mut args)?) }
                            }
                            Value::Native(_, name) => Err(format!("TypeError: bool.__new__({name}): {name} is not a subtype of bool").into()),
                            Value::Class(class) => Err(format!("TypeError: bool.__new__({0}): {0} is not a subtype of bool", class.name).into()),
                            other => Err(format!("TypeError: bool.__new__(X): X is not a type object ({})", other.core_kind()).into()),
                        };
                    }
                    if let Value::Native(Builtin::Bool, _) = &kind {
                        if self.lang.builtins.get(&word) == Some(&Builtin::ToInt) {
                            return Err("TypeError: int.__new__(bool) is not safe, use bool.__new__()".into());
                        }
                    }
                    if let Value::Native(operation,name)=&kind {
                        if Self::kind_builtin(operation) && name.as_ref()==word {return self.class_apply(kind,args)}
                        if Self::kind_builtin(operation) {return Err(format!("TypeError: {word}.__new__({name}): {name} is not a subtype of {word}").into())}
                    }
                    let Value::Class(c) = kind else { return Err(format!("TypeError: {word}.__new__(X): X is not a type object ({})",kind.core_kind()).into()); };
                    if Self::kind_beneath(&c).as_deref()!=Some(word.as_str()) {return Err(format!("TypeError: {word}.__new__({0}): {0} is not a subtype of {word}",c.name).into())}
                    let given = if matches!(self.lang.builtins.get(&word), Some(Builtin::Set | Builtin::List | Builtin::Dict)) { Vec::new() } else { args };
                    self.thing_of_kind(c, &word, given)
                }
                119 if args.len() == 1 => {
                    let Value::Small(mode) = w.1[0] else { return Err(self.class_refusal()); };
                    self.iterator_recipe_next(mode, &args[0])?.ok_or_else(|| self.core_fault("core.exhausted", "").into())
                }
                3 | 131 => {
                    args.insert(0,w.1[1].clone());
                    self.descriptor_apply(w.1[0].clone(), args)
                }
                // A member a builtin kind carries, standing loose: the
                // first value it is called with is the one it works
                // upon, and the rest are what the member itself takes.
                // Called with none at all, it names the kind and itself
                // as CPython's unbound method does; handed a receiver
                // of the wrong kind, it names the member, the kind and
                // the receiver's own, as CPython's descriptor does.
                29 => {
                    let member = w.1[1].plain();
                    let word = w.1[0].plain();
                    if args.is_empty() {
                        if word == "module" && self.lang.class_special.get(1).is_some_and(|name| name == &member) {
                            return Err("TypeError: descriptor '__repr__' of 'module' object needs an argument".into());
                        }
                        let pieces = self.lang.class_details.get("descriptor.unbound").cloned().unwrap_or_default();
                        return if pieces.len() == 3 {
                            Err(format!("{}{word}{}{member}{}", pieces[0], pieces[1], pieces[2]).into())
                        } else {
                            Err(self.class_refusal())
                        };
                    }
                    if word == "type" && member == self.class_word("order") {
                        let items = self.call_items(args)?;
                        let keywords = items.iter().any(|(key, _)| key.is_some());
                        let mut positional: Vec<_> = items.into_iter().filter_map(|(key, value)| key.is_none().then_some(value)).collect();
                        if positional.is_empty() { return Err("TypeError: unbound method type.mro() needs an argument".into()); }
                        let received = positional.remove(0).contents();
                        if !self.stands_for_kind(&received) {
                            return Err(format!("TypeError: descriptor 'mro' for 'type' objects doesn't apply to a '{}' object", self.super_tp_name(&received)).into());
                        }
                        let title = if w.1.get(2).is_some_and(Value::is_true) { self.super_tp_name(&received) } else { word };
                        if keywords { return Err(format!("TypeError: {title}.mro() takes no keyword arguments").into()); }
                        if !positional.is_empty() { return Err(format!("TypeError: {title}.mro() takes no arguments ({} given)", positional.len()).into()); }
                        let owner = self.super_type_arg(&received)?;
                        let mut classes = vec![self.public_class(owner.clone())];
                        classes.extend(self.merge_class_orders(&owner.direct)?.into_iter().map(|base| self.public_class(base)));
                        return Ok(self.keep_collection(Value::array(classes)));
                    }
                    let supplied = args.remove(0);
                    let subject = match supplied.contents() { thing @ Value::Object(_) => thing, _ => supplied };
                    // A thing of a class standing on the very kind this
                    // word names answers as its worth would, since the
                    // loose member is the kind's own and not the
                    // class's: `set.union(s, ...)` for `s` a subclass of
                    // `set` works upon what `s` keeps of a set.
                    if word == "module" && self.lang.class_special.get(1).is_some_and(|key| key == &member) {
                        if !args.is_empty() { return Err(format!("TypeError: expected 0 arguments, got {}", args.len()).into()); }
                        return self.module_repr_value(subject);
                    }
                    let receiver = match &subject {
                        // The worth is kept as it stands, cell and all,
                        // where it is one that a method writes into (a
                        // row or a map, behind a cell of its own): the
                        // writing must reach the very thing the subclass
                        // instance keeps, not a copy taken out of it.
                        Value::Object(o) if Self::kind_among(&o.class_now(), &word) => {
                            Self::worth_of(&subject).unwrap_or_else(|| subject.clone())
                        }
                        _ => subject.clone(),
                    };
                    if word == "dict" && args.len() == 1 && self.lang.class_special.get(11).map_or(false, |slot| slot == &member) {
                        if let (Value::Object(object), Value::Map(rows)) = (&subject, receiver.contents()) {
                            let (position, _) = self.map_locate(&rows, Some(&rows), &args[0])?;
                            if let Some(position) = position { return Ok(rows[position].1.clone()); }
                            if let Some(hook) = self.lang.missing_key.as_ref().and_then(|name| self.class_value(&object.class_now(), name)) {
                                let bound = self.bind_class_value(hook, Some(subject.clone()), object.class_now().clone())?;
                                return self.class_apply(bound, args);
                            }
                            return Err(self.key_absent(&args[0]).into());
                        }
                    }
                    // A loose member is the kind's own alone, so a
                    // receiver of some other kind is refused before the
                    // member is even looked up, even where it happens
                    // to answer to a member of the same name some other
                    // kind carries (`list.count`, `tuple.count`); a
                    // receiver of the kind itself, or standing under it
                    // the way a flag stands under the whole-number kind,
                    // still reaches the member as before.
                    let of_own_kind = self.lang.builtins.get(word.as_str()).copied().filter(Self::kind_builtin)
                        .map_or_else(|| receiver.core_kind() == word, |op| self.kind_holds(&op, &word, &receiver.contents()));
                    if word == "type" && self.stands_for_kind(&receiver) {
                        if self.lang.class_special.get(17).map_or(false, |slot| slot == &member) {
                            // type.__call__ performs construction directly; routing
                            // through the metaclass again would call itself forever.
                            return match receiver {
                                Value::Class(class) => self.class_construct(class, args),
                                other => self.class_apply(other, args),
                            };
                        }
                        if self.lang.class_special.get(8).map_or(false, |slot| slot == &member) && args.is_empty() {
                            return Ok(self.builtin_call(Builtin::Hash, &member, vec![(None, receiver)])?);
                        }
                    }
                    // Buffer wrappers share native method validation and the canonical exporter.
                    if of_own_kind && Lang::spells(&self.lang.builtin_bases, "bytes")
                        && matches!(word.as_str(), "bytes" | "bytearray")
                        && matches!(member.as_str(), "__buffer__" | "__release_buffer__") {
                        let mut inputs = vec![subject]; inputs.extend(args);
                        return self.class_apply(Value::Native(Builtin::ValueMethod, Rc::from(format!("{word}.{member}"))), inputs);
                    }
                    if member == self.class_word("get") && (of_own_kind || !matches!(word.as_str(), "type" | "module")) {
                        let inputs = self.call_items(args)?;
                        if inputs.iter().any(|(key, _)| key.is_some()) {
                            return Err(format!("TypeError: wrapper {member}() takes no keyword arguments").into());
                        }
                        if inputs.len() != 1 {
                            return Err(format!("TypeError: expected 1 argument, got {}", inputs.len()).into());
                        }
                        let attribute = inputs[0].1.contents();
                        let Value::Text(attribute) = attribute else {
                            return Err(format!("TypeError: attribute name must be string, not '{}'", attribute.core_kind()).into());
                        };
                        return self.class_get(subject, &attribute, true);
                    }
                    let found = if of_own_kind && self.native_special(&receiver, &member) {
                        Some(Value::ValueMethod(Rc::new((subject.clone(), member.clone()))))
                    } else if of_own_kind { self.builtin_member(&receiver,&member)? } else { None };
                    match found {
                        Some(bound) => self.class_apply(bound,args),
                        None => {
                            let pieces = self.lang.class_details.get("descriptor.foreign").cloned().unwrap_or_default();
                            if pieces.len() == 4 {
                                Err(format!("{}{member}{}{word}{}{}{}", pieces[0], pieces[1], pieces[2], Self::shown_kind(&receiver), pieces[3]).into())
                            } else {
                                Err(self.missing_member(&subject,&member))
                            }
                        }
                    }
                }
                79 => {
                    if self.call_items(args.clone())?.iter().any(|(key, _)| key.is_some()) {
                        return Err(format!("TypeError: wrapper {}() takes no keyword arguments", self.class_word("descriptor.get")).into());
                    }
                    let count = args.len().saturating_sub(1);
                    if count == 0 { return Err(format!("TypeError: {} expected at least 1 argument, got 0", self.class_word("descriptor.get")).into()); }
                    if count > 2 { return Err(format!("TypeError: {} expected at most 2 arguments, got {}", self.class_word("descriptor.get"), count).into()); }
                    let [descriptor, instance, rest @ ..] = args.as_slice() else { return Err(self.class_refusal()); };
                    let explicit = rest.first().filter(|owner| !matches!(owner.contents(), Value::Null));
                    if matches!(instance.contents(), Value::Null) && explicit.is_none() { return Err("TypeError: __get__(None, None) is invalid".into()); }
                    let held = Self::worth_of(descriptor).unwrap_or_else(|| descriptor.contents());
                    let Value::Adapter(wrapped) = held else { return Err(self.class_refusal()); };
                    match wrapped.0 {
                        4 => Ok(wrapped.1[0].clone()),
                        5 => {
                            let owner = match explicit { Some(owner) => owner.clone(), None => self.call_held(self.kind_maker_word(), vec![instance.clone()])? };
                            Ok(Self::adapter(3, vec![wrapped.1[0].clone(), owner]))
                        }
                        _ => Err(self.class_refusal()),
                    }
                }
                80 => {
                    if args.is_empty() { return Err(self.class_refusal()); }
                    let descriptor = args.remove(0);
                    let held = Self::worth_of(&descriptor).ok_or_else(|| self.class_refusal())?;
                    let Value::Adapter(wrapped) = held else { return Err(self.class_refusal()); };
                    if wrapped.0 != 4 { return Err(self.class_refusal()); }
                    self.descriptor_apply(wrapped.1[0].clone(), args)
                }
                4 => self.descriptor_apply(w.1[0].clone(), args),
                8 => self.class_apply(w.1[0].clone(),args),
                5 => Err(self.core_fault("core.uncallable", "classmethod").into()),
                13 if args.len() == 1 => {
                    let Value::Adapter(property) = &w.1[0] else { return Err(self.class_refusal()); };
                    let mut members = property.1.clone();
                    members.resize(2, Value::Null); members[1] = args.remove(0);
                    Ok(Self::adapter(6, members))
                }
                10..=12 => {
                    let Some(subject) = args.first().cloned() else { return Err(self.class_refusal()); };
                    if w.1.first().is_some_and(|owner| owner.plain() == self.class_word("root")) {
                        let (valid, actual) = self.object_receiver(&subject)?;
                        if !valid {
                            let method = self.class_word(match w.0 { 10 => "get", 11 => "set", _ => "remove" });
                            return Err(format!("TypeError: descriptor '{method}' requires a '{}' object but received a '{actual}'", self.class_word("root")).into());
                        }
                    }
                    let Some(Value::Text(name)) = args.get(1) else { return Err("TypeError: attribute name must be string".to_string().into()); };
                    if w.0 == 10 { self.class_get(subject,name,true) }
                    else { self.class_write(subject,name,if w.0 == 11 {args.get(2).cloned()} else {None},true) }
                }
                // The reader of a member that binds: given the thing, or
                // nothing and the class, it answers what a read through
                // that thing or class would.
                15 if w.1.is_empty() => {
                    if self.call_items(args.clone())?.iter().any(|(key, _)| key.is_some()) {
                        return Err("TypeError: wrapper __get__() takes no keyword arguments".into());
                    }
                    if args.is_empty() { return Err("TypeError: descriptor '__get__' of 'function' object needs an argument".into()); }
                    let function = args.remove(0).contents();
                    if !(matches!(&function, Value::Routine(_)) || matches!(&function, Value::Adapter(part) if matches!(part.0, 129 | 180))) {
                        return Err(format!("TypeError: descriptor '__get__' requires a 'function' object but received a '{}'", Self::shown_kind(&function)).into());
                    }
                    if args.is_empty() { return Err("TypeError: __get__ expected at least 1 argument, got 0".into()); }
                    if args.len() > 2 { return Err(format!("TypeError: __get__ expected at most 2 arguments, got {}", args.len()).into()); }
                    let instance = args.remove(0);
                    if matches!(instance.contents(), Value::Null) {
                        if args.first().map_or(true, |owner| matches!(owner.contents(), Value::Null)) { return Err("TypeError: __get__(None, None) is invalid".into()); }
                        return Ok(function);
                    }
                    let owner = match args.first() { Some(Value::Class(class)) => class.clone(), _ => self.root_class() };
                    if matches!(&function, Value::Routine(_)) { self.bind_class_value(function, Some(instance), owner) }
                    else { Ok(Self::adapter(3, vec![function, instance])) }
                }
                15 if !args.is_empty() && args.len() <= 2 => {
                    let thing = match &args[0] { Value::Null => None, other => Some(other.clone()) };
                    let owner = match (args.get(1), &thing) {
                        (Some(Value::Class(c)), _) => c.clone(),
                        (Some(native), _) if self.stands_for_kind(native) => {
                            let word = native.kind_it_names().map(Rc::<str>::from).or_else(|| self.kind_spelled(native)).ok_or_else(|| self.class_refusal())?;
                            self.kind_class(&word)
                        }
                        (_, Some(Value::Object(o))) => o.class_now().clone(),
                        (_, Some(_)) => self.root_class(),
                        (_, None) => return Err(self.class_refusal()),
                    };
                    self.bind_class_value(w.1[0].clone(), thing, owner)
                }
                17 if args.len() == 2 => self.slot_write(&args[0], &w.1, Some(args[1].clone())),
                18 if args.len() == 1 => self.slot_write(&args[0], &w.1, None),
                19 if args.len() == 2 => {
                    let spec = match &args[1] {
                        Value::Text(spec) => spec.to_string(),
                        other => match Self::worth_of(other).map(|worth| worth.contents()) {
                            Some(Value::Text(spec)) => spec.to_string(),
                            _ => {
                                let word = Self::format_given_kind(other);
                                return Err(Fault::Note(format!("{}{}", self.lang.format_argument.first().map_or("", String::as_str), word)));
                            }
                        },
                    };
                    if spec.is_empty() { self.special_text(&args[0], false).map(|shown| Value::text(&shown)).map_err(Fault::Note) }
                    else { Err(Fault::Note(format!("TypeError: unsupported format string passed to {}.__format__", args[0].core_kind()))) }
                }
                20..=27 => self.property_work(w.0, args),
                _ => Err(self.class_refusal()),
            },
            Value::Text(word) => {
                let Some(op) = self.lang.builtins.get(word.as_ref()).copied() else { return Err(self.class_refusal()); };
                if let Builtin::ClassTool(i) = op { self.class_work(i,args) }
                else if op == Builtin::SortOf { self.class_type(args) }
                else { self.class_apply(Value::Native(op, word), args) }
            }
            other => Err(self.core_fault("core.uncallable", &other.core_kind()).into()),
        }
    }
    /// Making a thing of a class. A class made by a metaclass is called
    /// through that metaclass's own call, which decides what comes of it.
    pub(super) fn typing_module(&mut self) -> Value {
        if let Some(module) = self.modules.get("_typing") { return module.clone(); }
        let mut fields = vec![("__name__".into(), Value::text("_typing"))];
        fields.push(("Union".into(), Value::Class(self.kind_class("Union"))));
        for name in ["TypeVar", "ParamSpec", "TypeVarTuple", "TypeAliasType", "Generic", "NoDefaultType", "ParamSpecArgs", "ParamSpecKwargs"] {
            let class = self.kind_class(name);
            if name != "NoDefaultType" {
                class.shared.borrow_mut().push(("__module__".into(), Value::text("typing")));
            }
            if name == "Generic" && self.lang.type_parameters {
                for (word, operation) in [("__class_getitem__", 153), ("__init_subclass__", 152)] {
                    class.shared.borrow_mut().push((word.into(), Self::adapter(5, vec![Self::adapter(operation, Vec::new())])));
                }
            }
            fields.push((name.into(), Value::Class(class)));
        }
        let sentinel_class = self.kind_class("NoDefaultType");
        self.made += 1;
        let sentinel = Value::Object(Rc::new(Instance { replacement_class: RefCell::new(None), class: sentinel_class,
            fields: RefCell::new(vec![("\0typing_repr".into(), Value::text("typing.NoDefault"))]), mark: self.made }));
        fields.push(("NoDefault".into(), sentinel));
        fields.push(("_idfunc".into(), Value::Native(Builtin::ClassTool(23), Rc::from("_idfunc"))));
        let class = self.kind_class("module");
        self.made += 1;
        let module = Value::Object(Rc::new(Instance { replacement_class: RefCell::new(None), class, fields: RefCell::new(fields), mark: self.made }));
        self.modules.insert("_typing".into(), module.clone()); self.module_addresses_ready.set(false);
        module
    }
    fn no_type_default(&mut self) -> Value {
        let Value::Object(module) = self.typing_module() else { unreachable!() };
        let result = module.fields.borrow().iter().find(|(key, _)| key == "NoDefault").expect("type default sentinel").1.clone();
        result
    }
    fn typing_instance(&mut self, class: Rc<Class>, args: Vec<Value>) -> Flow<Value> {
        let mut positional = Vec::new();
        let mut keywords = Vec::new();
        for (key, value) in self.call_items(args)? {
            match key { Some(key) => keywords.push((key, value)), None => positional.push(value) }
        }
        if class.name == "NoDefaultType" {
            if !positional.is_empty() || !keywords.is_empty() { return Err("TypeError: NoDefaultType takes no arguments".into()); }
            return Ok(self.no_type_default());
        }
        if class.name == "TypeVar" && positional.len() == 2 { return Err("TypeError: A single constraint is not allowed".into()); }
        if matches!(class.name.as_str(), "ParamSpec" | "TypeVarTuple") && positional.len() > 1 { return Err(format!("TypeError: {}() takes exactly 1 positional argument ({} given)", class.name.to_lowercase(), positional.len()).into()); }
        let Some(Value::Text(name)) = positional.first().map(Value::contents) else {
            return Err("TypeError: name must be a str".into());
        };
        let module = self.module_named().unwrap_or_else(|| "__main__".into());
        let mut fields: Vec<(String, Value)> = vec![("__name__".into(), Value::Text(name.clone())), ("__module__".into(), Value::text(&module)),
            ("\0typing_repr".into(), Value::Text(name))];
        if class.name == "TypeAliasType" {
            if positional.len() != 2 { return Err("TypeError: TypeAliasType requires a name and a value".into()); }
            fields.push(("__value__".into(), positional[1].clone()));
            fields.push(("__type_params__".into(), Value::tuple(Vec::new())));
        } else {
            fields.extend([( "__bound__".into(), Value::Null), ("__constraints__".into(), Value::tuple(positional[1..].to_vec())),
                ("__default__".into(), self.no_type_default()), ("__covariant__".into(), Value::Flag(false)),
                ("__contravariant__".into(), Value::Flag(false)), ("__infer_variance__".into(), Value::Flag(false))]);
        }
        if class.name == "TypeVarTuple" { fields.retain(|(key, _)| !matches!(key.as_str(), "__bound__" | "__constraints__" | "__covariant__" | "__contravariant__" | "__infer_variance__")); }
        if class.name == "ParamSpec" {
            fields.retain(|(key, _)| key != "__constraints__");
            if let Some((_, bound)) = fields.iter_mut().find(|(key, _)| key == "__bound__") { *bound = Value::Class(self.kind_class("NoneType")); }
        }
        for (key, value) in keywords {
            let attribute = match key.as_str() {
                "bound" => "__bound__", "default" => "__default__", "covariant" => "__covariant__",
                "contravariant" => "__contravariant__", "infer_variance" => "__infer_variance__", "type_params" => "__type_params__",
                _ => return Err(format!("TypeError: {}() got an unexpected keyword argument '{key}'", class.name).into()),
            };
            let value = if matches!(key.as_str(), "covariant" | "contravariant" | "infer_variance") {
                self.class_apply(Value::Native(Builtin::Bool, Rc::from("bool")), vec![value])?
            } else if key == "bound" && (class.name == "ParamSpec" || !matches!(value.contents(), Value::Null)) {
                let module = self.import_module("typing")?;
                let check = self.class_get(module, "_type_check", false)?.contents();
                self.class_apply(check, vec![value, Value::text("Bound must be a type.")])?
            } else { value };
            if let Some((_, held)) = fields.iter_mut().find(|(key, _)| key == attribute) { *held = value; }
            else { return Err(format!("TypeError: {}() got an unexpected keyword argument '{key}'", class.name.to_lowercase()).into()); }
        }
        if matches!(class.name.as_str(), "TypeVar" | "ParamSpec") {
            let flag = |name: &str| fields.iter().find(|(key, _)| key == name).is_some_and(|(_, v)| v.is_true());
            let (co, contra, inferred) = (flag("__covariant__"), flag("__contravariant__"), flag("__infer_variance__"));
            if co && contra { return Err("ValueError: Bivariant types are not supported.".into()); }
            if inferred && (co || contra) { return Err("ValueError: Variance cannot be specified with infer_variance.".into()); }
            if class.name == "TypeVar" && positional.len() > 1 && fields.iter().any(|(key, v)| key == "__bound__" && !matches!(v, Value::Null)) { return Err("TypeError: Constraints cannot be combined with bound=...".into()); }
            let prefix = if inferred { "" } else if co { "+" } else if contra { "-" } else { "~" };
            let title = positional[0].plain();
            fields.iter_mut().find(|(key, _)| key == "\0typing_repr").unwrap().1 = Value::text(&format!("{prefix}{title}"));
        }
        self.made += 1;
        Ok(Value::Object(Rc::new(Instance { replacement_class: RefCell::new(None), class, fields: RefCell::new(fields), mark: self.made })))
    }
    fn lazy_type_alias(&mut self, evaluator: Value) -> Flow<Value> {
        let Value::Routine(routine) = evaluator.contents() else { return Err(self.class_refusal()); };
        let class = self.kind_class("TypeAliasType");
        let mut alias = self.typing_instance(class, vec![Value::text(&routine.ident), Value::Null])?;
        if let Value::Object(instance) = &mut alias {
            let mut fields = instance.fields.borrow_mut();
            fields.retain(|(key, _)| key != "__value__");
            fields.push(("\0lazy:__value__".into(), evaluator));
        }
        Ok(alias)
    }
    pub(super) fn class_make(&mut self, c: Rc<Class>, args: Vec<Value>) -> Flow<Value> {
        self.validate_class_spreads(&c, &args)?;
        if self.lang.fuller_classes && Self::super_descended(&c) { return self.super_made(c, args); }
        if Self::own_kind(&c).is_some() && matches!(c.name.as_str(), "TypeVar" | "ParamSpec" | "TypeVarTuple" | "TypeAliasType" | "NoDefaultType") {
            return self.typing_instance(c, args);
        }
        if matches!(Self::own_kind(&c).as_deref(), Some("range_iterator" | "longrange_iterator")) {
            return Err(format!("TypeError: cannot create '{}' instances", c.name).into());
        }
        if Self::own_kind(&c).as_deref() == Some("function") && self.lang.trace_fields.len() > 18 {
            let mut parts = vec![None; 6];
            let mut next = 0;
            for (named, value) in self.call_items(args)? {
                let at = match named {
                    Some(word) => ["code", "globals", "name", "argdefs", "closure", "kwdefaults"].iter().position(|part| *part == word)
                        .ok_or_else(|| format!("TypeError: function() got an unexpected keyword argument '{word}'"))?,
                    None => { let at = next; next += 1; at }
                };
                if at >= parts.len() || parts[at].replace(value).is_some() { return Err("TypeError: invalid function arguments".into()); }
            }
            if let Some(code @ Value::Object(_)) = parts[0].as_ref().map(Value::contents) {
                if let Value::Object(object) = &code {
                    if self.code_class.as_ref().is_some_and(|kind| Rc::ptr_eq(kind, &object.class_now())) {
                        let globals = parts[1].clone().ok_or_else(|| "TypeError: function() missing required argument 'globals'".to_string())?;
                        if !matches!(globals.contents(), Value::Map(_)) && !Self::worth_of(&globals).is_some_and(|base| matches!(base.contents(), Value::Map(_))) { return Err("TypeError: function() argument 'globals' must be dict".into()); }
                        let name = match parts[2].as_ref().map(Value::contents) {
                            None | Some(Value::Null) => Value::text("<module>"),
                            Some(name @ Value::Text(_)) => name,
                            _ => return Err("TypeError: arg 3 (name) must be None or string".into()),
                        };
                        let defaults = match parts[3].as_ref().map(Value::contents) {
                            None | Some(Value::Null) => Value::Null,
                            Some(value @ Value::Tuple(_)) => value,
                            _ => return Err("TypeError: arg 4 (defaults) must be None or tuple".into()),
                        };
                        let closure = match parts[4].as_ref().map(Value::contents) {
                            None | Some(Value::Null) => Value::Null,
                            Some(value @ Value::Tuple(_)) => {
                                if let Value::Tuple(cells) = &value {
                                    if !cells.is_empty() { return Err("ValueError: module code requires closure of length 0".into()); }
                                }
                                value
                            }
                            _ => return Err("TypeError: arg 5 (closure) must be None or tuple".into()),
                        };
                        return Ok(Self::adapter(180, vec![code, globals, name, defaults, closure]));
                    }
                }
            }
            let Some(Value::Adapter(code)) = parts[0].as_ref().map(Value::contents) else { return Err("TypeError: function() argument 'code' must be code".into()); };
            let (7, Some(Value::Routine(origin))) = (code.0, code.1.first()) else { return Err("TypeError: function() argument 'code' must be code".into()); };
            let Some(globals) = parts[1].clone() else { return Err("TypeError: function() missing required argument 'globals'".into()); };
            if !matches!(globals.contents(), Value::Map(_)) && !Self::worth_of(&globals).is_some_and(|base| matches!(base.contents(), Value::Map(_))) { return Err("TypeError: function() argument 'globals' must be dict".into()); }
            let mut made = (**origin).clone();
            let keywords = match parts[5].as_ref().map(Value::contents) {
                None | Some(Value::Null) => Vec::new(),
                Some(Value::Map(entries)) => entries.iter().map(|(key, value)| (key.plain(), value.clone())).collect(),
                _ => return Err("TypeError: arg 6 (kwdefaults) must be None or dict".into()),
            };
            made = Self::with_spare_arguments(&made, None, Some(keywords));
            made.globe = Some(globals.clone());
            made.born = Some(self.ambient_builtins());
            if let Some(Value::Text(name)) = parts[2].as_ref().map(Value::contents) { made.ident = name.to_string(); made.qualified = name.to_string(); }
            if let Some(Value::Tuple(defaults)) = parts[3].as_ref().map(Value::contents) {
                made = Self::with_spare_arguments(&made, Some(defaults.as_ref().clone()), None);
            }
            let closure = match parts[4].as_ref().map(Value::contents) {
                Some(Value::Tuple(cells)) => cells.to_vec(),
                None | Some(Value::Null) => Vec::new(),
                _ => return Err("TypeError: arg 5 (closure) must be tuple".into()),
            };
            if closure.len() != made.enclosing.len() { return Err("ValueError: function requires a closure of the right length".into()); }
            let mut free = made.enclosing.clone();
            free.sort_by(|(one, _), (two, _)| made.idents[*one].cmp(&made.idents[*two]));
            for ((at, _), cell) in free.iter().zip(closure.iter()) {
                let Value::Adapter(wrapped) = cell.contents() else { return Err("TypeError: arg 5 (closure) must contain cells".into()); };
                if wrapped.0 != 31 { return Err("TypeError: arg 5 (closure) must contain cells".into()); }
                let Some(held @ (Value::Binding(_) | Value::Bond(_))) = wrapped.1.first() else { return Err("TypeError: arg 5 (closure) must contain cells".into()); };
                made.enclosed.push((*at, held.clone()));
            }
            if let Some(named) = self.captured_title(&globals) { made.home = Some(named); }
            made.globe = Some(globals.clone());
            let builtins_word = self.lang.module_builtins.first().cloned().unwrap_or_default();
            made.born = Some(match self.dyn_lookup(&globals.contents(), &builtins_word)? {
                Some(held) => held,
                None => self.ambient_builtins(),
            });
            made.revised = RefCell::new(None);
            let created = Value::Routine(Rc::new(made));
            if origin.ident == "<genexpr>" || origin.ident.starts_with("#generator") { return Ok(Self::adapter(43, vec![created])); }
            return Ok(created);
        }
        if let Some(maker) = Self::maker_beneath(&c) {
            if let Some(f) = self.class_value(&maker, self.class_word("call")) {
                let mut given = vec![Value::Class(c)];
                given.extend(args);
                return self.class_apply(f, given);
            }
        }
        self.class_construct(c, args)
    }
    fn type_initialiser(&mut self, args: Vec<Value>) -> Flow<Value> {
        let items = self.call_items(args)?;
        let plain: Vec<_> = items.iter().filter(|(key, _)| key.is_none()).map(|(_, value)| value.contents()).collect();
        let Some(receiver) = plain.first() else { return Err("TypeError: descriptor '__init__' of 'type' object needs an argument".into()) };
        if !self.stands_for_kind(receiver) { return Err(format!("TypeError: descriptor '__init__' requires a 'type' object but received a '{}'", receiver.core_kind()).into()); }
        if !matches!(plain.len(), 2 | 4) { return Err("TypeError: type.__init__() takes 1 or 3 arguments".into()); }
        if plain.len() == 2 && items.iter().any(|(key, _)| key.is_some()) { return Err("TypeError: type.__init__() takes no keyword arguments".into()); }
        Ok(Value::Null)
    }
    fn initialise_made_class(&mut self, value: &Value, requested: &Rc<Class>, args: Vec<Value>) -> Flow<()> {
        let Value::Class(made) = value else { return Ok(()) };
        let actual = Self::maker_beneath(made).unwrap_or_else(|| self.metaclass_root());
        if !Rc::ptr_eq(&actual, requested) && !actual.lineage.borrow().iter().any(|m| Rc::ptr_eq(m, requested)) { return Ok(()) }
        if let Some(init) = self.lang.constructor.as_deref().and_then(|name| self.class_value(&actual, name)) {
            let bound = self.bind_class_value(init, Some(value.clone()), actual)?;
            let answer = self.class_apply(bound, args)?;
            if !matches!(answer, Value::Null) { return Err(format!("TypeError: __init__() should return None, not '{}'", answer.core_kind()).into()); }
        }
        Ok(())
    }
    /// The making itself, as the kind builtin does it: the class
    /// allocates a thing and constructs it.
    pub(super) fn class_construct(&mut self, c: Rc<Class>, args: Vec<Value>) -> Flow<Value> {
        if self.traceback_class.as_ref().map_or(false, |known| Rc::ptr_eq(known, &c)) { return self.traceback_from_parts(args); }
        self.abstract_refusal(&c)?;
        let allocation = self.class_value(&c,self.class_word("allocate")).filter(|value| {
            !matches!(value, Value::Adapter(entry) if entry.0 == 14)
                || Self::own_class_value(&c, self.class_word("allocate")).is_some()
        });
        // A metaclass called outright makes a class, the way the kind
        // builtin does, from a name, bases and a namespace.
        let makes_classes = self.is_metaclass_root(&c) || c.lineage.borrow().iter().any(|b| self.is_metaclass_root(b));
        let kind = Self::kind_beneath(&c).filter(|word| !self.lang.type_parameters || word != "Generic");
        let object = if allocation.is_none() && makes_classes {
            let mut given = vec![Value::Class(c.clone())]; given.extend(args.clone());
            self.class_from_parts(given)?
        } else if let Some(f) = allocation {
            let mut given = vec![Value::Class(c.clone())]; given.extend(args.clone());
            self.class_apply(f,given)?
        } else if let Some(word) = &kind {
            if matches!(word.as_str(), "str_iterator" | "str_ascii_iterator") {
                return Err(format!("TypeError: cannot create '{}' instances", word).into());
            }
            let mut initial = args.clone();
            match self.lang.builtins.get(word) {
                Some(Builtin::Set) => initial.clear(),
                // Mutable dictionary allocation precedes the subclass initializer;
                // its arguments belong to that initializer, not dict.update.
                Some(Builtin::Dict) if self.lang.constructor.as_deref().and_then(|name| self.class_value(&c, name)).is_some() => initial.clear(),
                Some(Builtin::AsReal) if self.lang.constructor.as_deref().and_then(|name| self.class_value(&c, name)).is_some() => {
                    initial = self.call_items(initial)?.into_iter().filter_map(|(key, value)| key.is_none().then_some(value)).take(1).collect();
                }
                // A mutable container takes its members in its own
                // `__init__`; where the class writes one, the builtin is
                // asked only to allocate the empty thing and the class's
                // method is handed the arguments itself.
                Some(Builtin::List | Builtin::Dict) if self.lang.constructor.as_deref().and_then(|name| self.class_value(&c, name)).is_some() => {
                    initial.clear();
                }
                Some(Builtin::Filter) if self.lang.constructor.as_deref().and_then(|name| self.class_value(&c, name)).is_some() => {
                    initial = self.call_items(initial)?.into_iter().filter_map(|(key, value)| key.is_none().then_some(value)).collect();
                }
                Some(Builtin::Frozen | Builtin::Tuple) if self.lang.constructor.as_deref().and_then(|name| self.class_value(&c, name)).is_some() => {
                    initial = self.call_items(initial)?.into_iter().filter_map(|(key, value)| key.is_none().then_some(value)).collect();
                }
                _ => (),
            }
            self.thing_of_kind(c.clone(), word, initial)?
        } else {
            self.made += 1;
            Value::Object(Rc::new(Instance {replacement_class: RefCell::new(None), class:c.clone(),fields:RefCell::new(vec![]),mark:self.made}))
        };
        if makes_classes { self.initialise_made_class(&object, &c, args.clone())?; }
        if let Value::Object(o) = &object {
            // One whose class has last words to say is remembered, so that
            // a round holding it may be found when the program asks.
            if self.lang.finaliser.is_some() && crate::faint::last_word_of(&o.class).is_some() {
                crate::faint::remember(crate::faint::Hold::Object(Rc::downgrade(o)));
                if !self.lang.trace_fields.is_empty() { crate::faint::anchor(o); }
            }
            if self.lang.destructor.is_some() || self.lang.finaliser.is_some() {
                self.things_made.borrow_mut().push(Rc::downgrade(o));
            }
            if Rc::ptr_eq(&o.class_now(),&c) || o.class_now().lineage.borrow().iter().any(|b| Rc::ptr_eq(b,&c)) {
                let init = self.lang.constructor.as_deref().and_then(|n| self.class_value(&o.class_now(),n));
                if let Some(f) = init {
                    let bound=self.bind_class_value(f,Some(object.clone()),o.class_now().clone())?;
                    let answer = self.class_apply(bound,args)?;
                    if !matches!(answer.contents(),Value::Null) { return Err(self.class_refusal()); }
                } else if let Some(worth @ Value::Set(_)) = Self::worth_of(&object).filter(|v| !v.set_fixed()) {
                    let mut positional = Vec::new();
                    let mut keywords = Vec::new();
                    for (key, value) in self.call_items(args)? {
                        match key { Some(key) => keywords.push((key, value)), None => positional.push(value) }
                    }
                    let name = self.lang.constructor.clone().unwrap_or_default();
                    self.value_method(&worth, &name, positional, keywords)?;
                } else if kind.is_none() && self.class_value(&c,self.class_word("allocate")).is_none() {
                    if !self.call_items(args)?.is_empty() { self.root_refuses_arguments(&c,true)?; }
                }
            }
        }
        Ok(object)
    }
    /// The root's making (`allocating`) or constructing, handed more
    /// than the class or the thing: refused as CPython refuses it,
    /// naming the root where the class wrote over the one working and
    /// the class where it wrote over neither.
    fn root_refuses_arguments(&self, c: &Rc<Class>, allocating: bool) -> Flow<()> {
        let made_own = self.class_value(c,self.class_word("allocate")).is_some();
        let built_own = self.lang.constructor.as_deref().map_or(false,|n| self.class_value(c,n).is_some());
        let (own, other, part) = if allocating {(made_own, built_own, "arguments.new")} else {(built_own, made_own, "arguments.init")};
        let (part, named) = if own {(part, self.class_word("root").to_string())}
            else if !other {(if allocating {"arguments.none"} else {part}, c.name.clone())}
            else {return Ok(())};
        let pieces = self.lang.class_details.get(part).cloned().unwrap_or_default();
        if pieces.len() != 2 { return Err(self.class_refusal()); }
        Err(format!("{}{named}{}", pieces[0], pieces[1]).into())
    }
    /// A class whose maker left it a set of names nobody answered is
    /// refused on making, the names told in order: the label's words are
    /// the member the set is kept under, the opening of the complaint and
    /// the two middles, one name or many.
    fn abstract_refusal(&self, c: &Rc<Class>) -> Flow<()> {
        let Some(words) = self.lang.class_details.get("abstract").filter(|words| words.len() == 4) else { return Ok(()); };
        let pointer = Rc::as_ptr(c) as usize;
        if self.concrete_kinds.borrow().get(&pointer).is_some_and(|previous| previous.as_ptr() == Rc::as_ptr(c) && previous.strong_count() != 0) { return Ok(()); }
        let Some(held) = Self::own_class_value(c, &words[0]) else {
            let mut ordinary = self.concrete_kinds.borrow_mut();
            if ordinary.len() > 128 { ordinary.retain(|_, class| class.strong_count() != 0); }
            ordinary.insert(pointer, Rc::downgrade(c));
            return Ok(());
        };
        let mut names: Vec<String> = match held.contents() {
            Value::Set(members) => members.borrow().items().iter().filter_map(|v| match v { Value::Text(t) => Some(t.to_string()), _ => None }).collect(),
            _ => Vec::new(),
        };
        if names.is_empty() { return Ok(()); }
        names.sort();
        let listed = names.iter().map(|n| format!("'{n}'")).collect::<Vec<_>>().join(", ");
        let middle = if names.len() == 1 { &words[2] } else { &words[3] };
        Err(format!("{}{}{}{}", words[1], c.name, middle, listed).into())
    }
    /// A member every thing and every class has from the root, where
    /// the name is one: the members the language names for it, and
    /// hooks for making, constructing, reading, writing, removing and
    /// formatting. Given the class of a thing, the hooks come bare, to
    /// be bound to the thing.
    fn root_allocator(&self) -> Value {
        let key = "root allocator".to_string();
        if let Some(callable) = self.loose_members.borrow().get(&key) { return callable.clone(); }
        let value = Self::adapter(1, Vec::new());
        self.loose_members.borrow_mut().insert(key, value.clone());
        value
    }
    fn root_initialiser(&self) -> Value {
        let name = self.lang.constructor.as_deref().unwrap_or_default();
        let mut entries = self.kind_descriptors.borrow_mut();
        if let Some((_, _, callable)) = entries.iter().find(|(kind, member, _)| kind == "object" && member == name) { return callable.clone(); }
        let callable = Self::adapter(2, Vec::new());
        entries.push((String::from("object"), name.to_string(), callable.clone()));
        callable
    }
    fn root_member(&mut self, name: &str, of: Option<&Rc<Class>>) -> Option<Value> {
        if name.is_empty() { return None; }
        if self.lang.class_details.get("root.members").map_or(false,|names| names.iter().any(|n| n==name)) {
            let owner = of.and_then(|class| Self::kind_beneath(class)).unwrap_or_else(|| self.class_word("root").to_string());
            let key = format!("root:{owner}:{name}");
            if let Some(member) = self.loose_members.borrow().get(&key) { return Some(member.clone()); }
            let member = Self::adapter(30, vec![Value::text(name), Value::text(&owner)]);
            self.loose_members.borrow_mut().insert(key, member.clone());
            return Some(member);
        }
        of?;
        let hook = if name==self.class_word("allocate") {1}
            else if self.lang.constructor.as_deref()==Some(name) || name==self.class_word("subclass") {2}
            else if name==self.class_word("get") {10} else if name==self.class_word("set") {11}
            else if name==self.class_word("remove") {12}
            else if self.lang.class_special.get(72).map_or(false,|word| word==name) {19}
            else {return None};
        Some(if hook == 1 { self.root_allocator() } else if hook == 2 && self.lang.constructor.as_deref() == Some(name) { self.root_initialiser() } else if (10..=12).contains(&hook) && !self.lang.class_builder.is_empty() { Self::adapter(hook, vec![Value::text(self.class_word("root"))]) } else { Self::adapter(hook, vec![]) })
    }
    /// The root's own working of one of its members, handed the value
    /// it works upon first.
    pub(super) fn root_work(&mut self, name: &str, args: Vec<Value>) -> Flow<Value> {
        let place = self.lang.class_details.get("root.members").and_then(|names| names.iter().position(|n| n==name)).unwrap_or(usize::MAX);
        if place == 9 && args.len() != 1 { return Err(self.lang.method_errors["arguments"].clone().into()); }
        let declined = || Value::Declined(Rc::from(self.lang.special_declined.first().map(String::as_str).unwrap_or("NotImplemented")));
        let Some(subject) = args.first().cloned() else { return Err(self.class_refusal()); };
        let other = args.get(1).cloned();
        Ok(match place {
            // Alike only with itself; unlike wherever it is not alike.
            0 => match other { Some(v) if Self::member_matches(&subject,&v) => Value::Flag(true), _ => declined() },
            1 => {
                let Some(v) = other else { return Err(self.class_refusal()); };
                match self.special_call(&subject,2,vec![v.clone()]).map_err(Fault::Note)? {
                    Some(told @ Value::Declined(_)) => told,
                    Some(told) => Value::Flag(!self.special_truth(&told).map_err(Fault::Note)?),
                    None => if Self::member_matches(&subject,&v) {Value::Flag(false)} else {declined()},
                }
            }
            2..=5 | 14 => declined(),
            6 => match &subject {
                Value::Object(o) => Value::Small(o.mark as i64),
                other => other.core_hash().map(Value::Small).ok_or_else(|| self.core_fault("core.unhashable",&other.core_kind()))?,
            },
            7 | 8 => match &subject {
                Value::Object(o) if place == 7 => {
                    let module = self.class_word("main");
                    Value::text(&if o.class_now().base.is_none() && o.class_now().name == "object" { "<object object at 0x1>".to_owned() }
                        else if let Some(title) = o.class_now().python_title() { format!("<{title} object at 0x1>") }
                        else if module.is_empty() { format!("<{} object>", o.class_now().name) }
                        else { format!("<{module}.{} object at 0x1>", o.class_now().name) })
                }
                other => Value::text(&self.special_text(other,true).map_err(Fault::Note)?),
            },
            9 => self.default_directory(&subject)?,
            10 => self.root_state(&subject),
            11 | 12 => {
                let protocol = if place == 11 { 0 } else {
                    if args.len() != 2 { return Err("TypeError: __reduce_ex__() takes exactly one argument".into()); }
                    let indexed = match args[1].contents() {
                        number @ (Value::Small(_) | Value::Huge(_) | Value::Flag(_)) => number,
                        _ => self.special_index(&args[1]).map_err(Fault::Note)?.ok_or_else(|| format!("TypeError: '{}' object cannot be interpreted as an integer", args[1].core_kind()))?,
                    };
                    indexed.as_big().map_err(Fault::Note)?.to_i32().ok_or("OverflowError: Python int too large to convert to C int")?
                };
                if place == 12 {
                    if let Some(answer) = self.special_call(&subject, 79, Vec::new()).map_err(Fault::Note)? { return Ok(answer); }
                    if let Value::Object(instance) = &subject {
                        if self.exception_class(&instance.class_now()) {
                            let name = self.lang.reduce_method.clone().unwrap_or_default();
                            return self.exception_method(instance.clone(), &name, &[]);
                        }
                    }
                }
                if matches!(Self::worth_of(&subject).unwrap_or_else(|| subject.contents()).contents(), Value::Set(_)) { return self.set_reduction(&subject); }
                return self.object_reduction(&subject, protocol);
            }
            13 => Value::Small(match &subject { Value::Object(o) if o.class_now().name != self.class_word("root") => 24, _ => 16 }),
            _ => return Err(self.class_refusal()),
        })
    }
    /// What a thing holds of its own, as a dictionary, or nothing where
    /// it holds nothing.
    pub(super) fn set_reduction(&mut self, subject: &Value) -> Flow<Value> {
        let kind = self.class_type(vec![subject.clone()])?;
        let members = self.core_members(subject).map_err(Fault::Note)?;
        let state = match self.class_get(subject.clone(), "__getstate__", false) {
            Ok(reader) => self.class_apply(reader, Vec::new())?,
            Err(fault) if self.attribute_fault(&fault) => self.root_state(subject),
            Err(fault) => return Err(fault),
        };
        Ok(Value::tuple(vec![kind, Value::tuple(vec![Value::array(members)]), state]))
    }
    fn object_reduction(&mut self, subject: &Value, protocol: i32) -> Flow<Value> {
        self.check_native_reduction(subject).map_err(Fault::Note)?;
        let registry = self.import_module("copyreg")?;
        if protocol < 2 {
            let worker = self.class_get(registry, "_reduce_ex", false)?;
            return self.class_apply(worker.contents(), vec![subject.clone(), Value::Small(i64::from(protocol))]);
        }
        let class = self.class_type(vec![subject.clone()])?;
        let mut arguments = Value::tuple(Vec::new());
        let mut keywords = Value::Map(Rc::new(Vec::new().into()));
        let mut has_extended = false;
        match self.class_get(subject.clone(), "__getnewargs_ex__", true) {
            Ok(method) => {
                let result = self.class_apply(method, Vec::new())?;
                let Value::Tuple(parts) = result.contents() else { return Err(format!("TypeError: __getnewargs_ex__ should return a tuple, not '{}'", result.core_kind()).into()) };
                if parts.len() != 2 { return Err(format!("ValueError: __getnewargs_ex__ should return a tuple of length 2, not {}", parts.len()).into()); }
                arguments = parts[0].contents(); keywords = parts[1].contents(); has_extended = true;
                if !matches!(arguments, Value::Tuple(_)) { return Err(format!("TypeError: first item of the tuple returned by __getnewargs_ex__ must be a tuple, not '{}'", arguments.core_kind()).into()); }
                if !matches!(keywords, Value::Map(_)) { return Err(format!("TypeError: second item of the tuple returned by __getnewargs_ex__ must be a dict, not '{}'", keywords.core_kind()).into()); }
            }
            Err(error) if self.attribute_fault(&error) => (),
            Err(error) => return Err(error),
        }
        if !has_extended {
            match self.class_get(subject.clone(), "__getnewargs__", true) {
                Ok(method) => {
                    arguments = self.class_apply(method, Vec::new())?.contents();
                    if !matches!(arguments, Value::Tuple(_)) { return Err(format!("TypeError: __getnewargs__ should return a tuple, not '{}'", arguments.core_kind()).into()); }
                }
                Err(error) if self.attribute_fault(&error) => (),
                Err(error) => return Err(error),
            }
        }
        let (constructor, newargs) = if matches!(&keywords, Value::Map(items) if !items.is_empty()) {
            (self.class_get(registry, "__newobj_ex__", false)?, Value::tuple(vec![class, arguments, keywords]))
        } else {
            let Value::Tuple(items) = arguments else { unreachable!() };
            let mut combined = vec![class]; combined.extend(items.iter().cloned());
            (self.class_get(registry, "__newobj__", false)?, Value::tuple(combined))
        };
        let state = match self.class_get(subject.clone(), "__getstate__", false) {
            Ok(method) => self.class_apply(method, Vec::new())?,
            Err(error) if self.attribute_fault(&error) => self.root_state(subject),
            Err(error) => return Err(error),
        };
        let base = Self::worth_of(subject).unwrap_or_else(|| subject.contents());
        let listitems = if matches!(base.contents(), Value::Array(_)) { self.core_iterator(subject).map_err(Fault::Note)? } else { Value::Null };
        let dictitems = if matches!(base.contents(), Value::Map(_)) {
            let items = self.class_get(subject.clone(), "items", false)?;
            let pairs = self.class_apply(items, Vec::new())?;
            self.core_iterator(&pairs).map_err(Fault::Note)?
        } else { Value::Null };
        Ok(Value::tuple(vec![constructor.contents(), newargs, state, listitems, dictitems]))
    }
    pub(super) fn root_state(&self, subject: &Value) -> Value {
        let settled = subject.contents();
        let Value::Object(o) = &settled else { return Value::Null };
        let mut entries: Vec<(Value, Value)> = o.fields.borrow().iter().filter(|(n,v)| !n.starts_with(['\0', '#']) && !matches!(v, Value::Blank)).map(|(n,v)| (Value::text(n), v.clone())).collect();
        // A class that names no slots keeps its own names in a namespace
        // dictionary; those are among what the object holds, as the
        // reference reports them.
        if let Some((_, held)) = o.fields.borrow().iter().find(|(key, _)| key == "\0namespace") {
            if let Value::Map(pairs) = held.contents() {
                for (key, value) in pairs.iter() {
                    let name = match key { Value::Hashed(pair) => pair.0.clone(), other => other.clone() };
                    if let Value::Text(text) = &name {
                        if !text.starts_with(['\0', '#']) && !matches!(value, Value::Blank) {
                            entries.push((Value::text(text), value.clone()));
                        }
                    }
                }
            }
        }
        let dictionary = if entries.is_empty() { Value::Null } else { Value::Map(Rc::new(entries.into())) };
        let slots: Vec<_> = o.fields.borrow().iter().filter_map(|(key, value)| {
            let slot = key.strip_prefix("\0slot:")?.rsplit_once(':')?.0;
            (!matches!(value, Value::Blank)).then(|| (Value::text(slot), value.clone()))
        }).collect();
        if slots.is_empty() { dictionary }
        else { Value::tuple(vec![dictionary, Value::Map(Rc::new(slots.into()))]) }
    }
    pub(super) fn qualified_class(&self, class: &Class) -> String {
        let fields = class.shared.borrow();
        let module = fields.iter().find(|(key, _)| key == self.class_word("module")).map(|(_, value)| value.plain());
        let local = class.python_names.borrow().as_ref().map(|names| names.1.plain()).unwrap_or_else(|| fields.iter().find(|(key, _)| key == self.class_word("qualified")).map(|(_, value)| value.plain()).unwrap_or_else(|| class.name.clone()));
        module.map_or(local.clone(), |prefix| format!("{prefix}.{local}"))
    }

    fn attribute_from_hook(&self, failure: Fault, subject: &Value, name: &str) -> Fault {
        let Fault::Thrown(Value::Object(error)) = &failure else { return failure };
        if !self.attribute_fault(&failure) { return failure; }
        let qualified = match subject {
            Value::Class(class) => format!("type object '{}' has no attribute '{name}'", self.qualified_class(class)),
            Value::Object(instance) if !self.lang.module_names.is_empty() && Self::kind_beneath(&instance.class_now()).as_deref() == Some("module") => {
                let fields = instance.fields.borrow();
                let namespace = fields.iter().find(|(key, _)| key == "_namespace").map(|(_, value)| value.contents());
                let named = match &namespace {
                    Some(Value::Map(entries)) => entries.iter().find(|(key, _)| matches!(key, Value::Text(word) if word.as_ref() == "__name__")).map(|(_, value)| value),
                    _ => fields.iter().find(|(key, _)| key == "__name__").map(|(_, value)| value),
                };
                let label = named.and_then(|value| match value.contents() { Value::Text(word) => Some(word.to_string()), _ => None });
                label.map_or_else(|| format!("module has no attribute '{name}'"), |label| format!("module '{label}' has no attribute '{name}'"))
            }
            Value::Object(instance) => format!("'{}' object has no attribute '{name}'", self.qualified_class(&instance.class_now())),
            _ => return failure,
        };
        let mut fields = error.fields.borrow_mut();
        let arguments = fields.iter().find(|(key, _)| key == "\0arguments").and_then(|(_, value)| match value { Value::Tuple(items) => Some(items.clone()), _ => None });
        let use_default = arguments.as_ref().is_some_and(|items| items.is_empty() || items.len() == 1 && items[0].plain() == name);
        if use_default {
            let args = Value::tuple(vec![Value::text(&qualified)]);
            for (key, value) in fields.iter_mut() {
                if key == "\0arguments" || self.lang.exception_args.as_deref() == Some(key) { *value = args.clone(); }
            }
        }
        for (key, value) in fields.iter_mut() {
            if self.lang.absent_name_member.as_deref() == Some(key) { *value = Value::text(name); }
            if self.lang.absent_object_member.as_deref() == Some(key) { *value = subject.clone(); }
        }
        drop(fields);
        failure
    }

    fn missing_member(&self, subject: &Value, name: &str) -> Fault {
        // A module and a kind are named by their own name in words of
        // their own, as CPython names them.
        let named = self.member_named_amiss(subject, name);
        if !named.is_empty() { return named.into(); }
        // A thing and a class are named by their own name; anything
        // else by the name its kind goes under.
        let class = match subject { Value::Object(o) => {
            let kind = o.class_now();
            Self::own_kind(&kind).and_then(|_| kind.outline.as_deref().and_then(|title| title.strip_prefix("<class '")?.strip_suffix("'>")).map(str::to_owned))
                .unwrap_or_else(|| kind.name.clone())
        }, Value::Class(c)=>c.name.clone(), other=>other.contents().core_kind() };
        let pieces = self.lang.class_details.get("attribute.amiss").cloned().unwrap_or_default();
        if pieces.len()!=3 { return self.class_refusal(); }
        format!("{}{class}{}{name}{}",pieces[0],pieces[1],pieces[2]).into()
    }

    /// A member a thing may read but neither write over nor take away:
    /// its class names the members its things hold, and holds a value
    /// of its own under this name, which a write to a thing does not
    /// reach.
    fn readonly_member(&self, subject: &Value, name: &str) -> Fault {
        let pieces = self.lang.class_details.get("attribute.readonly").cloned().unwrap_or_default();
        if pieces.len() != 3 { return self.unwritable_member(subject, name); }
        let class = match subject { Value::Object(o)=>o.class_now().name.clone(), other=>other.contents().core_kind() };
        format!("{}{class}{}{name}{}", pieces[0], pieces[1], pieces[2]).into()
    }

    /// A member a thing cannot take: it keeps no namespace of its own
    /// to put one in, whether because its class names the members it
    /// holds or because it is a value of a builtin kind.
    fn unwritable_member(&self, subject: &Value, name: &str) -> Fault {
        let told = self.member_unwritable(subject, name);
        if told.is_empty() { return self.missing_member(subject, name); }
        told.into()
    }
    pub(super) fn explicit_classmethod(&mut self, descriptor: &Value, args: Vec<Value>, named: Vec<(String, Value)>) -> Flow<Value> {
        if !named.is_empty() { return Err("TypeError: wrapper __get__() takes no keyword arguments".into()); }
        if args.is_empty() { return Err("TypeError: __get__ expected at least 1 argument, got 0".into()); }
        if args.len() > 2 { return Err(format!("TypeError: __get__ expected at most 2 arguments, got {}", args.len()).into()); }
        let owner = match args.get(1).map(Value::contents) {
            Some(value) if !matches!(value, Value::Null) => value,
            _ if matches!(args[0].contents(), Value::Null) => return Err("TypeError: __get__(None, None) is invalid".into()),
            _ => self.class_type(vec![args[0].clone()])?,
        };
        let Value::Adapter(parts) = descriptor else { return Err(self.class_refusal()); };
        let clip = |text: String| {
            let end = text.char_indices().map(|(at, ch)| at + ch.len_utf8()).take_while(|end| *end <= 100).last().unwrap_or(0);
            text[..end].to_owned()
        };
        let defining = parts.1[0].plain();
        let word = parts.1[1].plain();
        let argument_kind = match &owner {
            Value::Object(object) => {
                let kind = object.class_now();
                let exported = kind.shared.borrow().iter().any(|(key, value)| key == "\0buffer_allocator" && value.is_true());
                if exported || kind.python_names.borrow().is_none() { self.qualified_class(&kind) } else { Self::type_argument_kind(&owner) }
            }
            _ => Self::type_argument_kind(&owner),
        };
        let class = if let Value::Class(held) = &owner { held.clone() } else if matches!(&owner, Value::Native(Builtin::Bool, _)) { self.kind_class("bool") } else { self.type_base(&owner).map_err(|_| Fault::Note(format!("TypeError: descriptor '{word}' for type '{defining}' needs a type, not a '{}' as arg 2", clip(argument_kind))))? };
        let base = self.spelled_kind(&defining).ok_or_else(|| self.class_refusal())?;
        let registered_buffer = class.shared.borrow().iter().any(|(key, value)| key == "\0buffer_allocator" && value.is_true());
        let title = if registered_buffer || class.python_names.borrow().is_none() { self.qualified_class(&class) }
            else { class.python_names.borrow().as_ref().unwrap().0.type_text().plain() };
        if !self.beneath(&owner, &base, true)? { return Err(format!("TypeError: descriptor '{word}' requires a subtype of '{defining}' but received '{}'", clip(title)).into()); }
        self.bind_class_value(descriptor.clone(), None, class)
    }
    pub(super) fn bind_class_value(&mut self, value: Value, subject: Option<Value>, class: Rc<Class>) -> Flow<Value> {
        if let (Value::Native(Builtin::Text(_), word), Some(receiver)) = (&value, &subject) {
            if word.contains('.') { return Ok(Self::adapter(3, vec![value.clone(), receiver.clone()])); }
        }
        if let Value::Adapter(w) = &value {
            if w.0 == 29 && w.1.len() == 3 {
                let class = match &subject { Some(Value::Object(object)) => object.class_now(), Some(Value::Class(owner)) => owner.clone(), _ => class };
                let name = w.1[1].plain();
                if Self::own_kind(&class).as_deref() == Some("dict") {
                    let qualified = format!("dict.{name}");
                    if let Some(operation) = self.lang.builtins.get(&qualified) { return Ok(Value::Native(*operation, Rc::from(qualified))); }
                }
                return Ok(Value::ValueMethod(Rc::new((Value::Class(class), name))));
            }
            if (10..=12).contains(&w.0) && w.1.first().is_some_and(|owner| owner.plain() == self.class_word("root")) {
                if let Some(receiver) = subject {
                    let (valid, actual) = self.object_receiver(&receiver)?;
                    if !valid {
                        let method = self.class_word(match w.0 { 10 => "get", 11 => "set", _ => "remove" });
                        return Err(format!("TypeError: descriptor '{method}' for '{}' objects doesn't apply to a '{actual}' object", self.class_word("root")).into());
                    }
                    return Ok(Self::adapter(3, vec![value, receiver]));
                }
                return Ok(value);
            }
            // Native super init is a descriptor for super instances only.
            if w.0 == 236 {
                if let Some(receiver) = subject.as_ref().map(Value::contents) {
                    if !matches!(&receiver, Value::Object(o) if Self::super_descended(&o.class_now())) {
                        let received = self.class_type(vec![receiver])?.kind_it_names().ok_or_else(|| self.class_refusal())?;
                        return Err(format!("TypeError: descriptor '__init__' for 'super' objects doesn't apply to a '{received}' object").into());
                    }
                }
            }
            if w.0 == 29 && w.1[0].plain() == "type" && subject.is_some() {
                let name = w.1[1].plain();
                if ["mro", "namespace", "name"].iter().any(|part| name == self.class_word(part)) {
                    let target = subject.as_ref().unwrap().contents();
                    let c = match &target {
                        Value::Class(c) => c.clone(),
                        Value::Native(Builtin::SortOf, _) => self.metaclass_root(),
                        other => match other.kind_it_names().map(Rc::<str>::from).or_else(|| self.kind_spelled(other)) {
                            Some(word) if self.stands_for_kind(other) => self.kind_class(&word),
                            _ => return Err(self.class_refusal()),
                        },
                    };
                    if name == self.class_word("name") {
                        let title = c.python_names.borrow().as_ref().map_or_else(|| c.name.clone(), |names| names.0.plain());
                        return Ok(Value::text(&title));
                    }
                    if name == self.class_word("namespace") { return Ok(Value::View(Rc::new((Value::Class(c), "mapping".into())))); }
                    let line = Self::class_order(&c).into_iter().map(|base| self.public_class(base)).collect();
                    return Ok(Value::tuple(line));
                }
            }
            if self.lang.bind_names && w.0 == 29 && subject.is_some() && Value::loose_member_descriptor(&w.1[0].plain(), &w.1[1].plain())
                .is_some_and(|(form, _)| matches!(form, "attribute" | "member")) {
                let raw = subject.as_ref().unwrap();
                let stored = Self::worth_of(raw).unwrap_or_else(|| raw.clone()).contents();
                let field = w.1[1].plain();
                if let Some(answer) = self.builtin_member(&stored, &field).map_err(Fault::from)? { return Ok(answer); }
                return self.class_get(stored, &field, false);
            }
            if w.0 == 29 && w.1[0].plain() == "int" {
                let receiver = subject.clone().unwrap_or_else(|| Value::Class(class.clone()));
                if let Some(method) = self.integer_member(&receiver, &w.1[1].plain()) { return Ok(method); }
            }
            if w.0 == 29 && w.1[0].plain() == "dict" && self.lang.value_methods.get(&w.1[1].plain()).map(String::as_str) == Some("fromkeys") {
                return Ok(Value::ValueMethod(Rc::new((Value::Class(class), w.1[1].plain()))));
            }
            if w.0 == 29 && w.1[0].plain() == "type" && w.1[1].plain() == self.class_word("order") {
                if let Some(receiver) = subject {
                    let mut state = w.1.clone();
                    state.push(Value::Flag(true));
                    return Ok(Self::adapter(3, vec![Self::adapter(29, state), receiver]));
                }
                return Ok(value);
            }
            return match w.0 {
                2 if !w.1.is_empty() && subject.is_some() => Ok(Self::adapter(3, vec![value.clone(), subject.unwrap()])),
                29 | 122 | 124 | 126 | 133 | 180 | 236 if subject.is_some() => Ok(Self::adapter(3, vec![value.clone(), subject.unwrap()])),
                64 if w.1[0].plain() == "normal_pdf" && subject.is_some() => Ok(Self::adapter(3, vec![value.clone(), subject.unwrap()])),
                63 if subject.is_some() => Ok(Self::adapter(3, vec![value.clone(), subject.unwrap()])),
                119 if subject.is_some() => Ok(Self::adapter(3, vec![value.clone(), subject.unwrap()])),
                4 => Ok(w.1[0].clone()),
                5 => Ok(Self::adapter(3,vec![w.1[0].clone(),Value::Class(class)])),
                6 if subject.is_some() => self.class_apply(w.1[0].clone(),vec![subject.unwrap()]),
                16 if subject.is_some() => self.slot_read(&subject.unwrap(), &w.1),
                29 if subject.is_some() => Ok(Self::adapter(3, vec![value.clone(), subject.unwrap()])),
                // A working of the property class, read through a
                // property: bound to it. Its kept accessors read plainly.
                20..=27 | 30 | 79..=80 | 200..=203 | 233..=235 if subject.is_some() => Ok(Self::adapter(3, vec![value.clone(), subject.unwrap()])),
                28 => match subject { Some(Value::Object(o)) => self.property_reading(&o, &w.1[0].plain()), _ => Ok(value) },
                _ => Ok(value),
            };
        }
        // A member whose class furnishes a reader is read through it,
        // told the thing -- or nothing, for a read on the class -- and
        // the class the read went through.
        if let Some(reader) = self.descriptor_hook(&value, "descriptor.get") {
            return self.call_descriptor(&value, reader, vec![subject.unwrap_or(Value::Null), Value::Class(class)]);
        }
        match (value,subject) {
            (Value::Routine(f),Some(Value::Object(o))) => Ok(Value::method(o,f)),
            (Value::Routine(f), Some(receiver)) if self.lang.bind_names => {
                Ok(Self::adapter(131, vec![Value::Routine(f), receiver]))
            }
            (Value::Routine(f),Some(other)) => Ok(Self::adapter(3, vec![Value::Routine(f), other])),
            (v,_) => Ok(v),
        }
    }
    pub(super) fn calendar_kind(&mut self) -> Rc<Class> {
        if let Some((_, found)) = self.kind_classes.iter().find(|(word, _)| word == "struct_time") { return found.clone(); }
        let tuple = self.kind_class("tuple");
        let mut lineage = vec![tuple.clone()]; lineage.extend(tuple.lineage.borrow().iter().cloned());
        let fields = vec![("__module__".into(), Value::text("time")), ("__new__".into(), self.native_allocator("struct_time")), ("__repr__".into(), Self::adapter(233, vec![])), ("__str__".into(), Self::adapter(233, vec![])), ("__reduce__".into(), Self::adapter(234, vec![])), ("n_sequence_fields".into(), Value::Small(9)), ("n_fields".into(), Value::Small(11)), ("n_unnamed_fields".into(), Value::Small(0))];
        let kind = Rc::new(Class { name: "struct_time".into(), outline: Some("<class 'time.struct_time'>".into()), base: Some(tuple.clone()), direct: vec![tuple], lineage: RefCell::new(lineage), answers: vec![], fields: vec![], reaches: vec![], methods: vec![], constants: vec![("\0kind".into(), Value::text("struct_time"))], shared: RefCell::new(fields), weak_storage: std::cell::Cell::new(None), declares_slots: true, sealed: std::cell::Cell::new(true), mro_adopted: std::cell::Cell::new(false), adopted_order: RefCell::new(Vec::new()), python_names: RefCell::new(None) });
        self.kind_classes.push(("struct_time".into(), kind.clone()));
        kind
    }
    pub(super) fn calendar_sequence(&mut self, class: Value, args: Vec<Value>) -> Flow<Value> {
        let Value::Class(class) = class else { return Err("TypeError: struct_time.__new__(X): X is not a type object".into()); };
        let mut parts = vec![Value::Blank, Value::Map(Rc::new(Vec::new().into()))];
        let mut count = 0;
        for (name, value) in self.call_items(args)? {
            let position = match name.as_deref() { Some("sequence") => 0, Some("dict") => 1, Some(word) => return Err(format!("TypeError: '{word}' is an invalid keyword argument for struct_time()").into()), None => { let position=count; count+=1; position } };
            if position > 1 { return Err("TypeError: struct_time() takes at most 2 arguments".into()); }
            parts[position] = value;
        }
        if matches!(parts[0], Value::Blank) { return Err("TypeError: struct_time() missing required argument 'sequence' (pos 1)".into()); }
        let mut sequence = self.core_members(&parts[0]).map_err(Fault::Note)?;
        let size = sequence.len();
        if size < 9 || size > 11 { return Err(format!("TypeError: time.struct_time() takes an at {} {}-sequence ({}-sequence given)", if size < 9 { "least" } else { "most" }, if size < 9 { 9 } else { 11 }, size).into()); }
        let metadata = Self::worth_of(&parts[1]).unwrap_or_else(|| parts[1].contents()).contents();
        let Value::Map(dictionary) = metadata else { return Err("TypeError: time.struct_time() takes a dict as second arg, if any".into()); };
        const NAMES: [&str; 11] = ["tm_year", "tm_mon", "tm_mday", "tm_hour", "tm_min", "tm_sec", "tm_wday", "tm_yday", "tm_isdst", "tm_zone", "tm_gmtoff"];
        while sequence.len() < 11 {
            let name = NAMES[sequence.len()];
            sequence.push(dictionary.iter().find(|(key, _)| matches!(key, Value::Text(word) if word.as_ref() == name)).map_or(Value::Null, |(_, value)| value.clone()));
        }
        let mut fields = vec![("\0worth".into(), Value::tuple(sequence[..9].to_vec()))];
        fields.extend(NAMES.iter().zip(sequence).map(|(name,value)| (name.to_string(),value)));
        self.made += 1;
        Ok(Value::Object(Rc::new(Instance { class, replacement_class: RefCell::new(None), fields: RefCell::new(fields), mark:self.made })))
    }
    fn calendar_repr(&mut self, value: &Value) -> Flow<Value> {
        let mut fields=Vec::new();
        for name in ["tm_year", "tm_mon", "tm_mday", "tm_hour", "tm_min", "tm_sec", "tm_wday", "tm_yday", "tm_isdst"] {
            let member=self.class_get(value.clone(),name,false)?;
            fields.push(format!("{name}={}",self.special_text(&member,true).map_err(Fault::Note)?));
        }
        Ok(Value::text(&format!("time.struct_time({})",fields.join(", "))))
    }
    fn calendar_reduce(&mut self, value: &Value) -> Flow<Value> {
        let payload=Self::worth_of(value).ok_or_else(|| Fault::Note("TypeError: invalid struct_time receiver".into()))?;
        let mut extra=Vec::new();
        for name in ["tm_zone", "tm_gmtoff"] { extra.push((Value::text(name),self.class_get(value.clone(),name,false)?)); }
        Ok(Value::tuple(vec![Value::Class(self.calendar_kind()),Value::tuple(vec![payload,Value::Map(Rc::new(extra.into()))])]))
    }
    fn native_allocator(&self, word: &str) -> Value {
        self.loose_members.borrow_mut().entry(format!("allocator:{word}")).or_insert_with(|| Self::adapter(14, vec![Value::text(word)])).clone()
    }
    pub(super) fn integer_member(&self, subject: &Value, name: &str) -> Option<Value> {
        if let Value::Native(Builtin::Bool, word) = subject {
            if name == self.class_word("allocate") {
                return Some(self.native_allocator(word));
            }
        }
        let layout = self.lang.class_details.get("integer.layout")?;
        if layout.len() != 7 { return None; }
        let (kind, subclass) = match subject {
            Value::Native(Builtin::ToInt | Builtin::Bool, _) => (true, false),
            Value::Class(c) if Self::kind_beneath(c).as_deref().and_then(|word| self.lang.builtins.get(word)) == Some(&Builtin::ToInt) => (true, Self::own_kind(c).is_none()),
            Value::Object(_) if Self::worth_of(subject).map_or(false, |v| matches!(v, Value::Small(_) | Value::Huge(_))) => (false, true),
            Value::Small(_) | Value::Huge(_) | Value::Flag(_) => (false, false),
            _ => return None,
        };
        if !kind && Lang::spells(&self.lang.byte_words["ext.builtin.bytes.from_int"], name) {
            return Some(Value::ValueMethod(Rc::new((subject.clone(), "integer_bytes".to_string()))));
        }
        if self.lang.byte_words["ext.builtin.bytes.to_int"].iter().any(|word| word.rsplit('.').next() == Some(name)) {
            let owner = match subject {
                Value::Object(o) => Value::Class(o.class_now().clone()),
                _ if kind => subject.clone(),
                _ => Value::Native(Builtin::ToInt, Rc::from(self.lang.builtins.iter().find(|(_, b)| **b == Builtin::ToInt)?.0.as_str())),
            };
            return Some(Value::ValueMethod(Rc::new((owner, "integer_from_bytes".into()))));
        }
        if kind && name == layout[0] { return Some(Value::Small(layout[if subclass { 6 } else { 3 }].parse().ok()?)); }
        if kind && name == layout[1] { return Some(Value::Small(layout[4].parse().ok()?)); }
        if name == layout[2] { return Some(Value::ValueMethod(Rc::new((subject.clone(), "integer_size".to_string())))); }
        None
    }
    /// A member read that ends in a missing member -- whether the class's
    /// own reading hook said so, or a property's getter, or nothing was
    /// found -- is offered to the class's fallback reader before it is
    /// reported. A plain read, the root's own, has no fallback.
    pub(super) fn class_get(&mut self, subject: Value, name: &str, plain: bool) -> Flow<Value> {
        if self.names_property_class(&subject) { let class = self.property_class(); return self.class_get(Value::Class(class), name, plain); }
        if name == self.class_word("doc") && matches!(subject.contents(), Value::Null) {
            return Ok(Value::text("The type of the None singleton."));
        }
        if name == self.class_word("allocate") && matches!(subject.contents(), Value::Null | Value::Ellipsis | Value::Declined(_)) {
            let kind = self.named_kind(&subject.contents());
            return self.class_get(kind, name, plain);
        }


        let mut subject = subject;
        if !self.lang.class_builder.is_empty() {
            loop {
                let next = match &subject {
                    Value::Bond(cell) | Value::Binding(cell) => Some(cell.borrow().clone()),
                    _ => None,
                };
                match next { Some(value) => subject = value, None => break }
            }
        }
        if self.lang.bind_names && !name.starts_with("__") {
            if let Value::Object(instance) = &subject {
                let class = instance.class_now();
                let slotted = class.declares_slots || class.lineage.borrow().iter().any(|base| base.declares_slots);
                if slotted && (plain || self.class_value(&class, self.class_word("get")).is_none()) {
                    if let Some(Value::Adapter(slot)) = self.class_value(&class, name) {
                        if slot.0 == 16 {
                            if let Ok(value) = self.slot_read(&subject, &slot.1) { return Ok(value); }
                        }
                    }
                }
            }
        }
        if let Value::Object(code) = &subject {
            if self.code_class.as_ref().is_some_and(|kind| Rc::ptr_eq(kind, &code.class_now()))
                && self.lang.class_details.get("code.replace").and_then(|words| words.first()).is_some_and(|word| word == name) {
                return Ok(Value::ValueMethod(Rc::new((subject.clone(), "code_replace".to_owned()))));
            }
        }
        if !self.lang.weak_refused.is_empty() {
            if let Value::Object(proxy) = subject.contents() {
                if matches!(proxy.class_now().name.as_str(), "ProxyType" | "CallableProxyType") {
                    let weak = proxy.fields.borrow().iter().find(|(key, _)| key == "\0weak").map(|(_, v)| v.clone());
                    if let Some(Value::Faint(weak)) = weak {
                        let target = weak.revive().ok_or_else(|| Fault::Note("ReferenceError: weakly-referenced object no longer exists".to_string()))?;
                        return self.class_get(target, name, plain);
                    }
                }
            }
        }

        if let Value::Adapter(entry) = &subject {
            if entry.0 == 64 {
                if name == self.class_word("module") { return self.class_get(entry.1[1].clone(), name, plain); }
                if self.lang.class_special.get(79).is_some_and(|word| word == name) { return Ok(Self::adapter(132, vec![entry.1[0].clone()])); }
            }
            if matches!(entry.0, 63 | 64) && matches!(name, "__name__" | "__qualname__" | "__doc__") {
                return self.class_get(entry.1[1].clone(), name, plain);
            }
        }
        if name.starts_with('\0') && matches!(&subject, Value::Object(object) if object.fields.borrow().iter().any(|(key, _)| key == "\0immutable")) {
            return Err(self.missing_member(&subject, name));
        }
        if let Value::Adapter(proxy) = &subject {
            if let (9, [Value::Class(owner), receiver]) = (proxy.0, proxy.1.as_slice()) {
                let dynamic = match receiver {
                    Value::Object(o) => o.class_now().clone(),
                    Value::Class(c) => {
                        if std::iter::once(c).chain(c.lineage.borrow().iter()).any(|base| Rc::ptr_eq(base, owner)) { c.clone() }
                        else { Self::maker_beneath(c).unwrap_or_else(|| c.clone()) }
                    }
                    _ => return Err(self.class_refusal()),
                };
                if let Some(found) = self.super_walk(owner, receiver, dynamic, name)? { return Ok(found); }
                return Err(self.missing_member(&subject, name));
            }
        }
        // A thing of the parent class reads a member past the class it
        // was made against: the same walk the parent proxy takes, over
        // the classes the thing it stands on answers as. What the walk
        // never finds the plain reading of the thing answers, the very
        // way the reference has a name the classes past know nothing of
        // fall to the parent thing's own members.
        if let Value::Object(o) = &subject {
            if self.lang.fuller_classes && Self::super_descended(&o.class_now()) && name != self.class_word("kind") {
                let (owner, receiver, objtype) = {
                    let fields = o.fields.borrow();
                    (fields.iter().find(|(key, _)| key == "__thisclass__").map(|(_, v)| v.clone()),
                     fields.iter().find(|(key, _)| key == "__self__").map(|(_, v)| v.clone()),
                     fields.iter().find(|(key, _)| key == "\0objtype").map(|(_, v)| v.clone()))
                };
                if name == "__self_class__" { return Ok(objtype.unwrap_or(Value::Null)); }
                if matches!(name, "__thisclass__" | "__self__") && owner.is_none() { return Ok(Value::Null); }

                if let (Some(owner), Some(receiver), Some(dynamic)) = (self.super_class_of(owner), receiver, self.super_class_of(objtype)) {
                    if !matches!(receiver, Value::Null) {
                        if let Some(found) = self.super_walk(&owner, &receiver, dynamic, name)? { return Ok(found); }
                    }
                }
            }
        }
        let answer = self.class_read(subject.clone(), name, plain);
        if plain { return answer; }
        let Err(fault) = &answer else { return answer };
        if !self.attribute_fault(fault) { return answer; }
        if let Value::Class(class) = &subject {
            if let Some(maker) = Self::maker_beneath(class) {
                if let Some(reader) = self.lang.reader.as_deref().and_then(|key| self.class_value(&maker, key)) {
                    let bound = self.bind_class_value(reader, Some(subject.clone()), maker)?;
                    return self.class_apply(bound, vec![Value::text(name)]).map_err(|failure| self.attribute_from_hook(failure, &subject, name));
                }
            }
            return answer;
        }
        let Value::Object(o) = &subject else { return answer };
        let Some(reader) = self.lang.reader.as_deref().and_then(|n| self.class_value(&o.class_now(), n)) else { return answer };
        self.absent_member = None;
        let bound = self.bind_class_value(reader, Some(subject.clone()), o.class_now().clone())?;
        self.class_apply(bound, vec![Value::text(name)]).map_err(|failure| self.attribute_from_hook(failure, &subject, name))
    }
    fn class_read(&mut self, subject: Value, name: &str, plain: bool) -> Flow<Value> {
        if self.lang.clock_parts && matches!(&subject, Value::Native(Builtin::Clock, word) if word.as_ref() == "ctime") && name == self.class_word("module") {
            return Ok(Value::text("time"));
        }
        let allocator_owner = match &subject {
            Value::Adapter(entry) if entry.0 == 14 => self.spelled_kind(&entry.1[0].plain()),
            Value::Adapter(entry) if entry.0 == 1 && entry.1.is_empty() => Some(Value::Class(self.root_class())),
            Value::Adapter(entry) if entry.0 == 40 => Some(self.kind_maker_word()),
            _ => None,
        };
        if let Some(owner) = allocator_owner {
            if name == self.class_word("receiver") { return Ok(owner); }
            if name == self.class_word("name") { return Ok(Value::text("__new__")); }
            if name == self.class_word("qualified") { return Ok(Value::text(&format!("{}.__new__", self.class_get(owner.clone(), "__name__", false)?.plain()))); }
            if name == self.class_word("module") { return Ok(Value::Null); }
            if name == "__reduce__" {
                let getter = Value::Native(Builtin::GetAttr, Rc::from("getattr"));
                return Ok(Self::adapter(132, vec![Value::tuple(vec![getter, Value::tuple(vec![owner, Value::text("__new__")])])]));
            }
            if name == "__reduce_ex__" { return Ok(Value::ValueMethod(Rc::new((subject, "callable_reduce_ex".into())))); }
        }
        if name == self.class_word("allocate") && matches!(subject.contents(), Value::Small(_) | Value::Huge(_) | Value::Real(_) | Value::Complex(_) | Value::Text(_) | Value::Codepoints(_) | Value::Array(_) | Value::Map(_) | Value::Tuple(_) | Value::Bytes(..) | Value::Set(_)) {
            let owner = self.named_kind(&subject.contents());
            return self.class_get(owner, name, plain);
        }
        if name == "__reduce__" {
            let python_binding = match &subject {
                Value::Method(owner, code, _) => Some((Value::Object(owner.clone()), Value::Routine(code.clone()))),
                Value::Adapter(entry) if entry.0 == 3 && matches!(entry.1.first(), Some(Value::Routine(_) | Value::Native(Builtin::Text(_), _))) => Some((entry.1[1].clone(), entry.1[0].clone())),
                _ => None,
            };
            if let Some((owner, code)) = python_binding {
                let title = self.class_get(code, "__name__", false)?;
                let lookup = Value::Native(Builtin::GetAttr, Rc::from("getattr"));
                return Ok(Self::adapter(132, vec![Value::tuple(vec![lookup, Value::tuple(vec![owner, title])])]));
            }
        }
        if name == self.class_word("allocate") && matches!(&subject, Value::Native(operation, _) if !Self::kind_builtin(operation)) { return Ok(self.root_allocator()); }
        if name == "__reduce_ex__" && (matches!(&subject, Value::Native(operation, _) if !Self::kind_builtin(operation)) || (matches!(&subject, Value::ValueMethod(_) | Value::TextMethod(..) | Value::Method(..)) || matches!(&subject, Value::Adapter(entry) if entry.0 == 3))) {
            return Ok(Value::ValueMethod(Rc::new((subject, String::from("callable_reduce_ex")))));
        }
        if matches!(&subject, Value::Adapter(parts) if parts.0 == 4 || parts.0 == 5 || parts.0 == 29 && parts.1.len() == 3) {
            if let Some(index) = self.lang.class_special.iter().position(|word| word == name).filter(|index| *index == 79 || *index == 81) {
                let operation = if index == 81 { "descriptor_reduce_ex" } else { "descriptor_reduce" };
                return Ok(Value::ValueMethod(Rc::new((subject.clone(), operation.to_owned()))));
            }
        }
        if self.lang.constructor.as_deref() == Some(name) && matches!(&subject, Value::Class(class) if self.class_root.as_ref().is_some_and(|root| Rc::ptr_eq(root, class))) { return Ok(self.root_initialiser()); }
        if name == "__getformat__" {
            let owner = match subject.contents() {
                Value::Native(Builtin::AsReal, _) => Some(subject.clone()),
                Value::Real(_) => self.spelled_kind("float"),
                Value::Class(class) if Self::own_kind(&class).or_else(|| Self::kind_beneath(&class)).as_deref() == Some("float") => Some(subject.clone()),
                Value::Object(instance) if Self::kind_beneath(&instance.class_now()).as_deref() == Some("float") => Some(Value::Class(instance.class_now())),
                _ => None,
            };
            if let Some(owner) = owner { return Ok(Value::ValueMethod(Rc::new((owner, String::from("float_getformat"))))); }
        }
        if let Value::Native(op, word) = &subject {
            if !Self::kind_builtin(op) && !self.class_word("name").is_empty() {
                if let Some((family, method)) = word.rsplit_once('.') {
                    if let Some(owner) = self.spelled_kind(family) {
                        let classmethod = matches!(method, "fromkeys" | "fromhex" | "from_bytes" | "from_number" | "__getformat__");
                        let staticmethod = method == "maketrans";
                        if name == self.class_word("receiver") && (classmethod || staticmethod) { return Ok(if staticmethod { Value::Null } else { owner.clone() }); }
                        if name == "__objclass__" && !classmethod && !staticmethod { return Ok(owner.clone()); }
                        if name == self.class_word("module") && (classmethod || staticmethod) { return Ok(Value::Null); }
                        if self.lang.class_special.get(79).is_some_and(|label| label == name) {
                            let getter = Value::Native(Builtin::GetAttr, Rc::from("getattr"));
                            return Ok(Self::adapter(132, vec![Value::tuple(vec![getter, Value::tuple(vec![owner, Value::text(method)])])]));
                        }
                    }
                }
                if name == self.class_word("name") { return Ok(Value::text(word.rsplit('.').next().unwrap_or(word))); }
                if name == self.class_word("qualified") { return Ok(Value::text(word)); }
                if name == self.class_word("module") { return Ok(Value::text(self.home_module_word())); }
                if self.lang.class_special.get(79).is_some_and(|label| label == name) {
                    return Ok(Self::adapter(132, vec![Value::text(word)]));
                }
            }
        }
        if let Value::Native(Builtin::SortOf, word) = &subject {
            if name == self.class_word("name") || name == self.class_word("qualified") { return Ok(Value::text(word)); }
            if name == self.class_word("module") { return Ok(Value::text(self.home_module_word())); }
        }
        if let Some(word) = self.kind_spelled(&subject) {
            if self.lang.constructor.as_deref() == Some(name) {
                if let Some(native) = self.spelled_kind(&word) { return self.class_read(native, name, plain); }
            }
            if name == self.class_word("name") || name == self.class_word("qualified") { return Ok(Value::text(&word)); }
            if name == self.class_word("module") { return Ok(Value::text(self.home_module_word())); }
        }
        let raw = subject.contents();
        if self.lang.bind_names && !name.starts_with("__") {
            if let Value::Object(instance) = &raw {
                let owner = instance.class_now();
                if Self::own_kind(&owner).is_none() && Self::kind_beneath(&owner).map_or(true, |kind| kind == self.class_word("root")) {
                    let own = {
                        let fields = instance.fields.borrow();
                        if fields.iter().any(|(key, _)| key.starts_with('\0')) { None }
                        else { fields.iter().find(|(key, _)| key == name).map(|(_, value)| value.clone()) }
                    };
                    if let Some(held) = own {
                        let default_reader = plain || self.class_value(&owner, self.class_word("get")).is_none();
                        let descriptor = self.class_value(&owner, name);
                        if default_reader && !descriptor.as_ref().is_some_and(|value| self.takes_writes(value)) {
                            return Ok(held);
                        }
                    }
                }
            }
        }
        if let Value::Adapter(function) = &raw {
            if function.0 == 180 {
                match name {
                    "__code__" => return Ok(function.1[0].clone()),
                    "__globals__" => return Ok(function.1[1].clone()),
                    "__name__" => return Ok(function.1[2].clone()),
                    "__qualname__" => return Ok(Value::text("<module>")),
                    "__defaults__" => return Ok(function.1[3].clone()),
                    "__closure__" => return Ok(function.1[4].clone()),
                    "__kwdefaults__" => return Ok(Value::Null),
                    _ => {},
                }
            }
        }
        if name == self.class_word("kind") && !name.is_empty()
            && matches!(raw, Value::Small(_) | Value::Huge(_) | Value::Flag(_) | Value::Real(_) | Value::Text(_) | Value::Map(_) | Value::Array(_) | Value::Tuple(_) | Value::Complex(_) | Value::Bytes(..) | Value::Null | Value::Set(_)) {
            return Ok(self.named_kind(&raw));
        }
        if let Value::Object(object) = &subject {
            if let Some(value) = self.frame_member(object, name) { return Ok(value); }
        }
        if let Value::Generator(generator) = &subject {
            if self.is_async_generator(&subject) {
                if let Some(index) = self.lang.async_generator_fields.iter().position(|word| word == name) {
                    if index == 2 { return Ok(Value::Flag(self.async_generator_running(&subject))); }
                    let state = generator.try_borrow().map_err(|_| self.class_refusal())?;
                    return Ok(match index {
                        0 => state.program.as_ref().map_or(Value::Null, |body| self.routine_code(body)),
                        1 => if state.closed { Value::Null } else { state.trace_frame.clone().map_or(Value::Null, Value::Object) },
                        2 => Value::Flag(false),
                        _ => match state.delegate.clone().unwrap_or(Value::Null) { Value::Adapter(parts) if parts.0 == 33 => parts.1[0].clone(), other => other },
                    });
                }
                if self.lang.trace_fields.iter().position(|key| key == name).is_some_and(|at| matches!(at, 14 | 15 | 19..=24)) {
                    return Err(self.missing_member(&subject, name));
                }
            }
            let index = self.lang.trace_fields.iter().position(|key| key == name);
            if matches!(index, Some(14 | 15 | 19 | 20 | 21 | 22 | 23 | 24)) {
                if index == Some(25) && generator.try_borrow().is_err() { return Ok(Value::text("GEN_RUNNING")); }
                let kept = generator.try_borrow().map_err(|_| self.class_refusal())?;
                match index {
                    Some(14 | 19 | 20) => return Ok(if kept.closed { Value::Null } else { kept.trace_frame.clone().map_or(Value::Null, Value::Object) }),
                    Some(21 | 22) => return Ok(Value::Flag(kept.started && !kept.closed)),
                    Some(23) => return Ok(Value::Flag(false)),
                    Some(24) => return Ok(kept.delegate.clone().unwrap_or(Value::Null)),
                    _ => if let Some(program) = &kept.program { return Ok(self.routine_code(program)); },
                }
            }
        }
        if matches!(&subject, Value::Trace(_)) {
            if let Some(method) = self.builtin_directory_method(&subject, name) { return Ok(method); }
        }
        if let Value::Trace(trace) = &subject {
            return match self.lang.trace_fields.iter().position(|key| key == name) {
                Some(1) => Ok(Value::Small(trace.line)),
                Some(2) => Ok(trace.next.borrow().clone()),
                Some(3) => Ok(Value::Object(trace.frame.clone())),
                Some(26) => Ok(Value::Small(trace.instruction)),
                Some(16) => Ok(Value::Small(trace.location.map_or(trace.line, |p| p.2 as i64))),
                Some(17) => Ok(trace.location.map_or(Value::Null, |p| Value::Small(p.1 as i64))),
                Some(18) => Ok(trace.location.map_or(Value::Null, |p| Value::Small(p.3 as i64))),
                _ => Err(self.missing_member(&subject, name)),
            };
        }
        if let Value::Adapter(entry) = &subject {
            let w = entry;
            if entry.0 == 44 && entry.1.len() == 1 && self.lang.annotation_adapter.first().is_some_and(|word| word == name) {
                return Ok(match &entry.1[0] {
                    Value::Class(owner) => owner.shared.borrow().iter().find(|(key, _)| key == crate::code::ANNOTATE_WORD)
                        .map(|(_, rows)| rows.contents()).unwrap_or_else(|| Value::array(Vec::new())),
                    rows => rows.clone(),
                });
            }
            if w.0 == 16 && w.1.get(2).is_some_and(|part| part.plain() == "\0instance-namespace") {
                if name == self.class_word("name") { return Ok(w.1[0].clone()); }
                if name == "__objclass__" { return Ok(w.1[1].clone()); }
                if name == self.class_word("qualified") {
                    let Value::Class(owner) = &w.1[1] else { return Err(self.class_refusal()); };
                    let qualified = self.class_get(Value::Class(owner.clone()), name, false)?;
                    return Ok(Value::text(&format!("{}.{}", qualified.plain(), w.1[0].plain())));
                }
                if name == self.class_word("doc") { return Ok(Value::text("dictionary for instance variables")); }
            }
            if entry.0 == 128 {
                if ["descriptor.get", "descriptor.set", "descriptor.delete"].iter().any(|part| name == self.class_word(part)) {
                    return Ok(Self::adapter(129, vec![subject.clone(), Value::text(name)]));
                }
                if name == self.class_word("name") { return Ok(entry.1[0].clone()); }
            }
        }
        if let Value::Adapter(property) = &subject {
            if property.0 == 6 && Lang::spells(&self.lang.property_setter, name) { return Ok(Self::adapter(13, vec![subject])); }
        }
        if name == "__class_getitem__" && Lang::spells(&self.lang.builtin_bases, "tuple") {
            let kind = match subject.contents() {
                Value::Class(class) if self.class_value(&class, name).is_none() => Self::own_kind(&class).or_else(|| Self::kind_beneath(&class)),
                Value::Native(operation, word) if Self::kind_builtin(&operation) => Some(word.to_string()),
                _ => None,
            };
            if matches!(kind.as_deref(), Some("tuple" | "list" | "dict" | "set" | "frozenset")) {
                return Ok(Value::ValueMethod(Rc::new((subject.clone(), name.to_string()))));
            }
        }
        if ["__buffer__", "__release_buffer__"].contains(&name) && Lang::spells(&self.lang.builtin_bases, "bytes") {
            let bytes = match subject.contents() {
                Value::ByteKind(mutable, _) | Value::Bytes(_, mutable, _) => name == "__buffer__" || mutable,
                _ => false,
            };
            if bytes {
                if matches!(subject.contents(), Value::Class(_) | Value::ByteKind(..)) {
                    let word = match subject.contents() {
                        Value::ByteKind(mutable, _) => self.byte_kind_word(mutable).to_string(),
                        Value::Class(class) => Self::own_kind(&class).or_else(|| Self::kind_beneath(&class)).unwrap_or_default(),
                        _ => unreachable!(),
                    };
                    return Ok(self.held_kind_descriptor(&word, name));
                }
                return Ok(Value::ValueMethod(Rc::new((subject.clone(), name.to_owned()))));
            }
        }
        // The kind's word, or the kind read as a class, answers for its
        // type flags from the class the kind stands for.
        let bound = match &subject {
            Value::ValueMethod(method) => Some((method.0.clone(), match method.1.as_str() {
                "float_getformat" => "__getformat__", "integer_from_bytes" => "from_bytes",
                "integer_bytes" => "to_bytes", "integer_size" => "__sizeof__",
                "float_fromhex" | "bytes_fromhex" | "bytearray_fromhex" => "fromhex",
                "float_from_number" | "complex_from_number" => "from_number",
                "callable_reduce_ex" => "__reduce_ex__", "classmethod_bind" => "__get__", word => word,
            }.to_owned())),
            Value::TextMethod(text, _, word) => Some((Value::Text(text.clone()), word.to_string())),
            Value::Adapter(binding) if binding.0 == 3 => {
                if let Some(Value::Adapter(descriptor)) = binding.1.first() {
                    if descriptor.0 == 29 { Some((binding.1[1].clone(), descriptor.1[1].plain())) } else { None }
                } else if let Some(Value::Native(_, word)) = binding.1.first() { Some((binding.1[1].clone(), word.rsplit('.').next().unwrap_or(word).to_owned())) }
                else { None }
            }
            _ => None,
        };
        if let Some((owner, member)) = bound {
            if name == self.class_word("receiver") { return Ok(owner); }
            if name == self.class_word("name") { return Ok(Value::text(&member)); }
            if name == self.class_word("qualified") {
                let title = match &owner { Value::Class(_) => self.class_get(owner.clone(), name, false)?.plain(), Value::Native(_, spelling) => spelling.to_string(), _ => owner.core_kind() };
                return Ok(Value::text(&format!("{title}.{member}")));
            }
            if name == self.class_word("module") { return Ok(Value::Null); }
            if self.lang.class_special.get(79).is_some_and(|word| word == name) {
                let getter = Value::Native(Builtin::GetAttr, Rc::from("getattr"));
                let reduction = Value::tuple(vec![getter, Value::tuple(vec![owner, Value::text(&member)])]);
                return Ok(Self::adapter(132, vec![reduction]));
            }
        }
        if let Value::Adapter(allocator) = &subject {
            if allocator.0 == 14 && name == self.class_word("receiver") {
                if let Some(class) = self.spelled_kind(&allocator.1[0].plain()) { return Ok(class); }
            }
        }
        if let Value::Adapter(descriptor) = &subject {
            if descriptor.0 == 29 && self.lang.class_special.get(79).is_some_and(|word| word == name) {
                let owner = self.spelled_kind(&descriptor.1[0].plain()).ok_or_else(|| self.class_refusal())?;
                let getter = Value::Native(Builtin::GetAttr, Rc::from("getattr"));
                return Ok(Self::adapter(132, vec![Value::tuple(vec![getter, Value::tuple(vec![owner, descriptor.1[1].clone()])])]));
            }
        }
        if name==self.class_word("flags") && !name.is_empty() {
            let held=subject.contents();
            let builtin=match &held {
                Value::Native(op,word) if Self::kind_builtin(op)=>Some(word.clone()),
                Value::ByteKind(mutable, _) => Some(Rc::from(self.byte_kind_word(*mutable))),
                other=>self.kind_spelled(other),
            };
            if let Some(word)=builtin {
                let kind=self.kind_class(&word);
                return Ok(Value::Small(self.flags_of(&kind)));
            }
        }
        if let Value::ByteKind(mutable, _) = subject.contents() {
            if name == "fromhex" {
                let word = self.byte_kind_word(mutable);
                return Ok(Value::Native(Builtin::Bytes(if mutable { 49 } else { 5 }), Rc::from(format!("{word}.fromhex"))));
            }
            {
                let word = self.byte_kind_word(mutable).to_string();
                let kind = self.kind_class(&word);
                return self.class_get(Value::Class(kind), name, plain);
            }
        }
        if name==self.class_word("namespace") {
            let held=subject.contents();
            let builtin=match &held {
                Value::Native(op,word) if Self::kind_builtin(op)=>Some(word.clone()),
                other=>self.kind_spelled(other),
            };
            if let Some(word)=builtin {
                if word.as_ref() == "type" {
                    let owner = self.metaclass_root();
                    return Ok(Value::View(Rc::new((Value::Class(owner), "mapping".into()))));
                }
                let mut listed = self.kind_sample(&word).map_or_else(Vec::new, |sample| self.kind_member_names(&sample));
                if word.as_ref() == "type" { listed.extend([8, 17].iter().filter_map(|at| self.lang.class_special.get(*at).cloned())); }
                if word.as_ref() == "dict" { listed.extend(self.lang.value_methods.iter().filter(|(_, op)| op.as_str() == "fromkeys").map(|(key, _)| key.clone())); }
                let mut pairs: Vec<(Value, Value)> = listed.iter().filter_map(|key| self.loose_kind_member(&subject, key).map(|held| {
                    let raw = if word.as_ref() == "dict" && key == "fromkeys" { Self::adapter(29, vec![Value::text("dict"), Value::text(key), Value::Flag(true)]) } else { held };
                    (Value::text(key), raw)
                })).collect();
                if matches!(word.as_ref(), "int" | "float" | "str" | "tuple" | "bytes" | "bytearray" | "dict" | "set" | "frozenset" | "complex" | "list") {
                    pairs.push((Value::text(self.class_word("allocate")), self.native_allocator(&word)));
                }
                // Public native class dictionaries include their stored Python slots.
                let kind = self.kind_class(&word);
                for (key, member) in kind.shared.borrow().iter() {
                    if !key.starts_with('\0') && !pairs.iter().any(|(name, _)| name.plain() == *key) {
                        pairs.push((Value::text(key), member.clone()));
                    }
                }
                return Ok(Value::View(Rc::new((Value::Map(Rc::new(pairs.into())),"mapping".to_string()))));
            }
        }
        // A routine, a wrapped routine and a slot each read as a member
        // that binds; the slot writes and removes as well.
        if name == self.class_word("descriptor.get") && !name.is_empty()
            && (matches!(&subject, Value::Routine(_)) || matches!(&subject, Value::Native(Builtin::Text(_), word) if word.contains('.')) || matches!(&subject, Value::Adapter(w) if (matches!(w.0, 4 | 5 | 10 | 11 | 12 | 16 | 236) || w.0 == 29 && w.1.len() == 2))) {
            return Ok(Self::adapter(15, vec![subject]));
        }
        if let Value::Adapter(w) = &subject {
            if w.0 == 16 && !name.is_empty() {
                if name == self.class_word("descriptor.set") { return Ok(Self::adapter(17, w.1.clone())); }
                if name == self.class_word("descriptor.delete") { return Ok(Self::adapter(18, w.1.clone())); }
            }
        }
        if !self.class_word("base").is_empty() && (name == self.class_word("base") || name == self.class_word("bases")) {
            let class = match &subject {
                Value::Class(c) => Some(c.clone()),
                Value::Native(Builtin::SortOf, _) => Some(self.metaclass_root()),
                Value::Native(op, word) if Self::kind_builtin(op) => Some(self.kind_class(word)),
                _ => None,
            };
            if let Some(c) = class {
                if let Some(maker) = Self::maker_beneath(&c) {
                    if let Some(member) = self.class_value(&maker, name) {
                        if !self.takes_writes(&member) {
                            if let Some(own) = self.class_value(&c, name) { return self.bind_class_value(own, None, c); }
                        }
                        return self.bind_class_value(member, Some(subject.clone()), maker);
                    }
                }
                return Ok(if name == self.class_word("base") { c.base.clone().map_or(Value::Null, |base| self.public_class(base)) }
                    else { Value::tuple(c.direct.iter().cloned().map(|base| self.public_class(base)).collect()) });
            }
        }
        if matches!(&subject, Value::Native(Builtin::SortOf, _))
            || (!self.lang.class_builder.is_empty() && (matches!(&subject, Value::Class(c) if self.is_metaclass_root(c) || Self::own_kind(c).is_some_and(|word| self.lang.builtins.get(&word) == Some(&Builtin::SortOf)))
                || self.kind_spelled(&subject).is_some_and(|word| self.lang.builtins.get(word.as_ref()) == Some(&Builtin::SortOf)))) {
            let tag = if self.lang.constructor.as_deref() == Some(name) { Some(78) }
                else if name == self.class_word("call") { Some(41) }
                else if name == self.class_word("allocate") { Some(40) }
                else if name == self.class_word("prepare") { Some(42) }
                else if name == self.class_word("set") { Some(11) }
                else if name == self.class_word("get") { Some(10) }
                else if name == self.class_word("remove") { Some(12) }
                else { None };
            if let Some(tag) = tag { return Ok(Self::adapter(tag, if name == self.class_word("subclass") { vec![Value::text(name)] } else { vec![] })); }
        }
        if matches!(&subject, Value::ByteKind(..)) {
            if let Some(member) = self.loose_kind_member(&subject, name) { return Ok(member); }
        }
        // A row of bytes answers to the methods its kind holds, whose
        // words are the ones text goes by, bound to the row they were
        // read from.
        if let Value::Bytes(_, changeable, _) = &subject {
            if let Some(working) = self.byte_member(name, *changeable) {
                return Ok(Value::ValueMethod(Rc::new((subject.clone(), working.to_string()))));
            }
        }
        if self.lang.bind_names && name == self.class_word("call")
            && (matches!(&subject, Value::Native(op, _) if !Self::kind_builtin(op))
                || matches!(&subject, Value::ValueMethod(_) | Value::TextMethod(..))
                || matches!(&subject, Value::Adapter(w) if w.0 == 29)) {
            return Ok(subject);
        }
        if matches!(&subject,Value::Native(op,_) if !Self::kind_builtin(op)) {
            if let Some(member)=self.builtin_member(&subject,name)? { return Ok(member); }
        }
        match &subject {
            // A builtin kind's word, read as a class: its maker, and its name.
            Value::Native(op, word) if Self::kind_builtin(op) => {
                if *op == Builtin::AsReal && self.lang.float_from_number.iter().any(|spelling| spelling.rsplit('.').next() == Some(name)) {
                    return Ok(Value::ValueMethod(Rc::new((subject.clone(), "float_from_number".to_string()))));
                }
                if *op == Builtin::Complex && self.lang.float_from_number.iter().any(|spelling| spelling.rsplit('.').next() == Some(name)) {
                    return Ok(Value::ValueMethod(Rc::new((subject.clone(), "complex_from_number".to_string()))));
                }
                if name == self.class_word("bases") && !name.is_empty() {
                    let parent = if *op == Builtin::Bool { self.spelled_kind("int").unwrap_or(Value::Class(self.root_class())) }
                        else { Value::Class(self.root_class()) };
                    return Ok(Value::tuple(vec![parent]));
                }
                if name==self.class_word("module") { return Ok(Value::text(self.home_module_word())); }
                if name==self.class_word("qualified") { return Ok(Value::text(word)); }
                if let Some(size) = self.integer_member(&subject, name) { return Ok(size); }
                if name==self.class_word("allocate") {
                    // Making a property outright allocates the subclass it
                    // is handed, the way the root's making does.
                    if *op == Builtin::ClassTool(11) { return Ok(Self::adapter(1, Vec::new())); }
                    return Ok(self.native_allocator(word));
                }
                if name==self.class_word("name") || self.lang.class_name.as_deref()==Some(name) { return Ok(Value::text(word)); }
                if name==self.class_word("doc") {
                    if let Some(doc) = Self::builtin_kind_doc(word) { return Ok(Value::text(doc)); }
                }
                if name==self.class_word("namespace") {

                    let mut names=self.kind_special_names(word);
                    names.push(name.to_string());
                    let rows=names.into_iter().filter_map(|key| self.loose_kind_member(&subject, &key).map(|value| (Value::text(&key), value))).collect();
                    let book=Value::Map(Rc::new(rows));
                    return Ok(Value::View(Rc::new((book,"mapping".to_string()))));
                }
                // The kind read as a class stands on the root and on
                // nothing else, so that is the whole of its line.
                if name == self.class_word("bases") { return Ok(Value::tuple(vec![Value::Class(self.root_class())])); }
                if name == self.class_word("order") && !self.lang.class_builder.is_empty() {
                    let method = self.held_kind_descriptor("type", name);
                    if *op == Builtin::SortOf { return Ok(method); }
                    let maker = self.metaclass_root();
                    return self.bind_class_value(method, Some(subject.clone()), maker);
                }
                if name==self.class_word("mro") || name==self.class_word("order") {
                    let listed=name==self.class_word("order");
                    let word=word.to_string();
                    let kind=if *op == Builtin::SortOf { self.metaclass_root() } else { self.kind_class(&word) };
                    let order = Self::class_order(&kind).into_iter().map(|base| self.public_class(base)).collect();
                    let line=Value::tuple(order);
                    return Ok(if listed {Self::adapter(0,vec![line])} else {line});
                }
                // Whatever a value of the kind answers to is carried by
                // the kind itself, standing loose: the value it works
                // upon is the first it is called with.
                if let Some(loose)=self.loose_kind_member(&subject,name) { return Ok(loose); }
                if self.lang.constructor.as_deref() == Some(name) { return Ok(self.root_initialiser()); }
                if let Some(inherited) = self.root_member(name, None) { return Ok(inherited); }
            }
            Value::Class(c) => {
                if name == "__weakref__" && self.weak_layout(c) {
                    return Ok(Self::adapter(16, vec![Value::text(name), Value::Class(c.clone())]));
                }
                if self.is_metaclass_root(c) {
                    if name == self.class_word("allocate") { return Ok(Self::adapter(40, Vec::new())); }
                    if name == self.class_word("call") { return Ok(Self::adapter(41, Vec::new())); }
                }
                if !self.class_word("name").is_empty() {
                    if let Some(maker) = Self::maker_beneath(c) {
                        if let Some(member) = self.class_value(&maker, name).filter(|member| self.takes_writes(member) && (matches!(member, Value::Adapter(w) if matches!(w.0, 6 | 16 | 28)) || self.descriptor_hook(member, "descriptor.get").is_some())) {
                            return self.bind_class_value(member, Some(subject.clone()), maker);
                        }
                    }
                }
                if self.lang.bind_names && name == self.class_word("kind") {
                    return self.class_type(vec![subject.clone()]);
                }
                if !self.class_word("module").is_empty() && name == self.class_word("module") {
                    if let Some(stored) = Self::own_class_value(c, name) { return Ok(stored); }
                    let is_root = self.class_root.as_ref().is_some_and(|root| Rc::ptr_eq(root, c));
                    if is_root || self.is_metaclass_root(c) || Self::own_kind(c).is_some() {
                        return Ok(Value::text(self.home_module_word()));
                    }
                }
                if let Some(word) = Self::own_kind(c) {
                    if name == self.class_word("allocate") && self.lang.builtins.get(&word).is_some_and(Self::kind_builtin) {
                        return Ok(self.native_allocator(&word));
                    }
                    if name==self.class_word("module") { return Ok(Value::text(match word.as_str() { "SimpleNamespace" | "GenericAlias" => "types", "Union" => "typing", _ => self.home_module_word() })); }
                    if name==self.class_word("qualified") { return Ok(Value::text(&word)); }
                }
                if name == self.class_word("flags") {
                    return Ok(Value::Small(self.flags_of(c)));
                }
                if (Self::own_kind(c).as_deref() == Some("float") || Self::kind_beneath(c).as_deref() == Some("float")) && self.lang.float_from_number.iter().any(|spelling| spelling.rsplit('.').next() == Some(name)) {
                    return Ok(Value::ValueMethod(Rc::new((subject.clone(), "float_from_number".to_string()))));
                }
                if (Self::own_kind(c).as_deref() == Some("complex") || Self::kind_beneath(c).as_deref() == Some("complex")) && self.lang.float_from_number.iter().any(|spelling| spelling.rsplit('.').next() == Some(name)) {
                    return Ok(Value::ValueMethod(Rc::new((subject.clone(), "complex_from_number".to_string()))));
                }
                if Self::kind_beneath(c).as_deref() == Some("float") && name == "__getformat__" {
                    return Ok(Value::ValueMethod(Rc::new((subject.clone(), String::from("float_getformat")))));
                }
                if name == "fromhex" {
                    if let Some(kind) = Self::kind_beneath(c).filter(|kind| matches!(kind.as_str(), "bytes" | "bytearray")) {
                        return Ok(Value::ValueMethod(Rc::new((subject.clone(), format!("{kind}_fromhex")))));
                    }
                }
                if Self::kind_beneath(c).as_deref() == Some("float") && name == "fromhex" {
                    return Ok(Value::ValueMethod(Rc::new((subject.clone(), "float_fromhex".to_string()))));
                }
                // Byte class methods participate in the same MRO as stored
                // attributes. Stop at the native owner: a later mixin must
                // not hide it. Metaclass data descriptors were read above.
                if matches!(name, "fromhex" | "maketrans")
                    && matches!(Self::kind_beneath(c).as_deref(), Some("bytes" | "bytearray")) {
                    for base in std::iter::once(c.as_ref()).chain(c.lineage.borrow().iter().map(Rc::as_ref)) {
                        if let Some(kind) = Self::own_kind(base).filter(|kind| matches!(kind.as_str(), "bytes" | "bytearray")) {
                            if name == "maketrans" {
                                return Ok(Value::Native(Builtin::Bytes(40), Rc::from("bytes.maketrans")));
                            }
                            let method = format!("{kind}_fromhex");
                            return Ok(Value::ValueMethod(Rc::new((subject.clone(), method))));
                        }
                        if let Some(entry) = Self::own_class_value(base, name) {
                            return self.bind_class_value(entry, None, c.clone());
                        }
                    }
                }
                if name==self.class_word("name") { return Ok(c.python_names.borrow().as_ref().map_or_else(|| Value::text(&c.name), |names| names.0.clone())); }
                if name==self.class_word("qualified") { return Ok(c.python_names.borrow().as_ref().map(|names| names.1.clone()).unwrap_or_else(|| self.class_value(c,name).unwrap_or_else(|| Value::text(&c.name)))); }
                if name==self.class_word("bases") { return Ok(Value::tuple(c.direct.iter().cloned().map(|base| self.public_class(base)).collect())); }
                if name==self.class_word("namespace") {
                    let annotate = self.lang.class_details.get("code.fields").and_then(|row| row.get(10)).cloned().unwrap_or_default();
                    if !annotate.is_empty() && c.shared.borrow().iter().any(|(key, _)| key == crate::code::ANNOTATE_WORD)
                        && !c.shared.borrow().iter().any(|(key, _)| key == &annotate || key == "__annotate_func__") {
                        self.class_get(subject.clone(), &annotate, true)?;
                    }

                    if let Some(maker)=Self::maker_beneath(c) {
                        if let Some(descriptor)=self.class_value(&maker,name).filter(|entry| self.takes_writes(entry)) {
                            return self.bind_class_value(descriptor,Some(subject.clone()),maker);
                        }
                    }
                    // A class standing for a builtin kind holds no
                    // members of its own; what it names are the ones a
                    // value of the kind answers to.
                    return Ok(Value::View(Rc::new((Value::Class(c.clone()), "mapping".into()))));
                }
                if name == self.class_word("order") && !self.lang.class_builder.is_empty() {
                    if let Some(own) = self.class_value(c, name) { return self.bind_class_value(own, None, c.clone()); }
                    if let Some(maker) = Self::maker_beneath(c) {
                        if let Some(method) = self.class_value(&maker, name) { return self.bind_class_value(method, Some(subject.clone()), maker); }
                    }
                    let method = self.held_kind_descriptor("type", name);
                    let maker = self.metaclass_root();
                    return self.bind_class_value(method, Some(subject.clone()), maker);
                }
                if name==self.class_word("mro") || name==self.class_word("order") {
                    let order = Self::class_order(c).into_iter().map(|base| self.public_class(base)).collect();
                    let tuple=Value::tuple(order);
                    return Ok(if name==self.class_word("order") {Self::adapter(0,vec![tuple])} else {tuple});
                }
                if self.lang.annotation_adapter.first().is_some_and(|word| word == name) {
                    let replaced = self.lang.class_details.get("code.fields").and_then(|fields| fields.get(10))
                        .is_some_and(|public| c.shared.borrow().iter().any(|(key, _)| key == public));
                    if replaced { return Ok(Value::Null); }
                    return Ok(c.shared.borrow().iter().find(|(key, _)| key == crate::code::ANNOTATE_WORD)
                        .map(|(_, value)| value.contents()).unwrap_or_else(|| Value::array(Vec::new())));
                }
                let annotate_word = self.lang.class_details.get("code.fields").and_then(|row| row.get(10));
                if annotate_word.is_some_and(|word| word == name) {
                    let public = { let members = c.shared.borrow(); members.iter().find(|(key, _)| key == name).or_else(|| members.iter().find(|(key, _)| key == "__annotate_func__")).map(|(_, own)| own.contents()) };
                    if let Some(own) = public { return Ok(own); }
                    let has_values = c.shared.borrow().iter().find(|(key, _)| key == crate::code::ANNOTATE_WORD)
                        .is_some_and(|(_, value)| matches!(value.contents(), Value::Array(row) if row.chunks(2).any(|entry| matches!(entry.get(1), Some(Value::Routine(code)) if !code.postponed_annotation))));
                    let has_values = c.shared.borrow().iter().find(|(key, _)| key == "\0annotation_strings").map_or(has_values, |(_, mode)| !mode.is_true());
                    if !has_values { return Ok(Value::Null); }
                    let callable = Self::adapter(44, vec![subject.clone()]);
                    c.shared.borrow_mut().push(("__annotate_func__".to_owned(), callable.clone()));
                    return Ok(callable);
                }
                if self.lang.class_annotations.first().map_or(false,|word|word==name) { return self.class_annotations(c); }
                if let Some(v)=self.class_value(c,name) { return self.bind_class_value(v,None,c.clone()); }
                // The classes written beneath this one, the live ones,
                // as the reference's own type.__subclasses__ tells them.
                if !self.class_word("subclasses").is_empty() && name==self.class_word("subclasses") { return Ok(Self::adapter(204, vec![subject.clone()])); }
                if let Some(size) = self.integer_member(&subject, name) { return Ok(size); }
                if let Some(member)=self.loose_kind_member(&subject,name) { return self.bind_class_value(member,None,c.clone()); }
                if Self::own_kind(c).is_none() {
                    if let Some(base) = c.lineage.borrow().iter().find(|base| Self::own_kind(base).is_some()) {
                        if self.lang.value_methods.get(name).map(String::as_str) != Some("fromkeys") {
                            if let Some(inherited) = self.loose_kind_member(&Value::Class(base.clone()), name) { return Ok(inherited); }
                        }
                    }
                }
                // A class standing on a builtin kind reads that kind's
                // own class method too, bound to the class itself, so
                // that `dictlike.fromkeys` reaches `dict.fromkeys` and
                // hands back a dictlike.
                if self.lang.value_methods.get(name).map(String::as_str) == Some("fromkeys") {
                    if let Some(base) = c.lineage.borrow().iter().find(|base| Self::own_kind(base).is_some()) {
                        if self.loose_kind_member(&Value::Class(base.clone()), name).is_some() {
                            return Ok(Value::ValueMethod(Rc::new((subject.clone(), name.to_string()))));
                        }
                    }
                }
                if name == self.class_word("allocate") {
                    if let Some(native) = Self::kind_beneath(c).filter(|word| self.lang.builtins.get(word).is_some_and(Self::kind_builtin)) {
                        return Ok(self.native_allocator(&native));
                    }
                    return Ok(self.root_allocator());
                }
                // A class also reads what the metaclass that made it
                // holds, each member bound to the class itself, the way
                // a thing's method is bound to the thing.
                if let Some(maker)=Self::maker_beneath(c) {
                    if let Some(v)=self.class_value(&maker,name) { return self.bind_class_value(v,Some(subject.clone()),maker); }
                }
                if self.lang.bind_names && name == self.class_word("call") { return Ok(Self::adapter(3, vec![Self::adapter(41, Vec::new()), subject.clone()])); }
                // The formatting every class has from the root: a thing
                // and a specification, answered as the format builtin would.
                if self.lang.class_special.get(72).map_or(false,|word|word==name) {return Ok(Self::adapter(19,vec![]));}
                {
                    let index = if name==self.class_word("allocate") && (self.is_metaclass_root(c) || c.lineage.borrow().iter().any(|b| self.is_metaclass_root(b))) {Some(40)}
                        else if name==self.class_word("allocate") {Some(1)}
                        else if self.lang.constructor.as_deref()==Some(name) && (self.is_metaclass_root(c) || c.lineage.borrow().iter().any(|b| self.is_metaclass_root(b))) {Some(78)}
                        else if self.lang.constructor.as_deref()==Some(name) || name==self.class_word("subclass") {Some(2)}
                        else if name==self.class_word("get") {Some(10)} else if name==self.class_word("set") {Some(11)}
                        else if name==self.class_word("remove") {Some(12)} else {None};
                    if let Some(i)=index {
                        let state = if (10..=12).contains(&i) && !self.lang.class_builder.is_empty() { vec![Value::text(self.class_word("root"))] }
                            else if name == self.class_word("subclass") { vec![Value::text(name)] } else { Vec::new() };
                        return Ok(Self::adapter(i,state));
                    }
                }
                if let Some(root)=self.root_member(name,Some(c)) {return Ok(root);}
            }
            Value::Object(o) => {
                let module = self.module_holding(&subject).is_some() || Self::kind_beneath(&o.class_now()).as_deref() == Some("module");
                if module {
                    let annotate = self.lang.class_details.get("code.fields").and_then(|row| row.get(10)).cloned().unwrap_or_default();
                    let book = o.fields.borrow().iter().find(|(key, _)| key == "\0namespace").map(|(_, value)| value.clone());
                    let lookup = |key: &str| {
                        book.as_ref().and_then(|book| match book.contents() {
                            Value::Map(entries) => entries.iter().find(|(word, _)| word.plain() == key).map(|(_, value)| value.contents()), _ => None,
                        }).or_else(|| o.fields.borrow().iter().find(|(word, _)| word == key).map(|(_, value)| value.contents()))
                    };
                    let own = lookup(name);
                    if name == annotate { return Ok(own.unwrap_or(Value::Null)); }
                    if self.lang.class_annotations.first().is_some_and(|key| key == name) {
                        if let Some(value) = own.filter(|value| !matches!(value, Value::Blank)) { return Ok(value); }
                        let source = lookup(&annotate);
                        let value = if let Some(source) = source.filter(|value| !matches!(value, Value::Null | Value::Blank)) {
                            self.class_apply(source, vec![Value::Small(1)])?
                        } else { self.keep_collection(Value::Map(Rc::new(Vec::new().into()))) };
                        if let Some(book) = book { Self::book_write(&book, name, Some(value.clone())); }
                        else { Self::write_members(&mut o.fields.borrow_mut(), name, Some(value.clone()), true).map_err(|_| self.class_refusal())?; }
                        return Ok(value);
                    }
                }

                if Self::own_kind(&o.class_now()).as_deref() == Some("Union") {
                    if name == "__origin__" { return Ok(Value::Class(self.kind_class("Union"))); }
                    if name == "__parameters__" {
                        let items = o.fields.borrow().iter().find(|(key, _)| key == "__args__").unwrap().1.clone();
                        let module = self.import_module("typing")?;
                        let collect = self.class_get(module, "_collect_type_parameters", false)?.contents();
                        return self.class_apply(collect, vec![items]);
                    }
                }
                if Self::own_kind(&o.class_now()).is_some_and(|kind| matches!(kind.as_str(), "TypeVar" | "ParamSpec" | "TypeVarTuple" | "TypeAliasType")) {
                if matches!(name, "args" | "kwargs") && o.class_now().name == "ParamSpec" {
                    let class = self.kind_class(if name == "args" { "ParamSpecArgs" } else { "ParamSpecKwargs" });
                    let title = o.fields.borrow().iter().find(|(key, _)| key == "__name__").map(|(_, value)| value.plain()).unwrap_or_default();
                    self.made += 1;
                    return Ok(Value::Object(Rc::new(Instance { replacement_class: RefCell::new(None), class, mark: self.made,
                        fields: RefCell::new(vec![("__origin__".into(), subject.clone()), ("\0typing_repr".into(), Value::text(&format!("{title}.{name}")))]) })));
                }
                if name == "__parameters__" && o.class_now().name == "TypeAliasType" {
                    return Ok(o.fields.borrow().iter().find(|(key, _)| key == "__type_params__").map(|(_, value)| value.clone()).unwrap_or_else(|| Value::tuple(Vec::new())));
                }
                let substitution = match (o.class_now().name.as_str(), name) {
                    ("TypeVar", "__typing_subst__") => Some("_typevar_subst"),
                    ("ParamSpec", "__typing_subst__") => Some("_paramspec_subst"),
                    ("ParamSpec", "__typing_prepare_subst__") => Some("_paramspec_prepare_subst"),
                    ("TypeVarTuple", "__typing_prepare_subst__") => Some("_typevartuple_prepare_subst"),
                    _ => None,
                };
                if let Some(function) = substitution { return Ok(Self::adapter(154, vec![Value::text(function), subject.clone()])); }
                let evaluation = match name {
                    "evaluate_value" => Some("__value__"), "evaluate_bound" if o.class_now().name == "TypeVar" => Some("__bound__"),
                    "evaluate_constraints" if o.class_now().name == "TypeVar" => Some("__constraints__"), "evaluate_default" => Some("__default__"), _ => None,
                };
                if let Some(attribute) = evaluation {
                    let fields = o.fields.borrow();
                    let source = fields.iter().find(|(key, _)| key == &format!("\0lazy:{attribute}")).map(|(_, source)| source.clone())
                        .or_else(|| if matches!(attribute, "__bound__" | "__constraints__") {
                            let constrained = fields.iter().any(|(key, value)| key == "\0type_constraints" && value.is_true());
                            if (attribute == "__constraints__") == constrained { fields.iter().find(|(key, _)| key == "\0type_bound").map(|(_, source)| source.clone()) } else { None }
                        } else { None });
                    if let Some(source) = source { return Ok(Self::adapter(47, vec![source])); }
                    if let Some((_, held)) = fields.iter().find(|(key, _)| key == attribute) {
                        if matches!(attribute, "__bound__" | "__constraints__") && (matches!(held, Value::Null) || matches!(held, Value::Tuple(items) if items.is_empty())) { return Ok(Value::Null); }
                        return Ok(Self::adapter(48, vec![held.clone()]));
                    }
                }
                if matches!(name, "__bound__" | "__constraints__") {
                    let constrained = o.fields.borrow().iter().any(|(key, value)| key == "\0type_constraints" && value.is_true());
                    let bound = if (name == "__constraints__") == constrained {
                        o.fields.borrow().iter().find(|(key, _)| key == "\0type_bound").map(|(_, evaluator)| evaluator.clone())
                    } else { None };
                    if let Some(evaluator) = bound.filter(|_| !o.fields.borrow().iter().any(|(key, _)| key == "\0bound_ready")) {
                        let result = self.class_apply(evaluator, Vec::new())?;
                        let constraints = constrained;
                        let mut fields = o.fields.borrow_mut();
                        for (key, value) in fields.iter_mut() {
                            if key == "__bound__" { *value = if constraints { Value::Null } else { result.clone() }; }
                            if key == "__constraints__" { *value = if constraints { result.clone() } else { Value::tuple(Vec::new()) }; }
                        }
                        fields.push(("\0bound_ready".into(), Value::Flag(true)));
                    }
                }
                let deferred = o.fields.borrow().iter().find(|(key, _)| key == &format!("\0lazy:{name}")).map(|(_, value)| value.clone());
                if let Some(evaluator) = deferred {
                    if let Some((_, value)) = o.fields.borrow().iter().find(|(key, _)| key == name) { return Ok(value.clone()); }
                    let result = self.class_apply(evaluator, Vec::new())?;
                    o.fields.borrow_mut().push((name.into(), result.clone()));
                    return Ok(result);
                }
                if name == "has_default" && matches!(o.class_now().name.as_str(), "TypeVar" | "ParamSpec" | "TypeVarTuple") {
                    let no_default = self.no_type_default();
                    let answer = o.fields.borrow().iter().any(|(key, _)| key == "\0lazy:__default__") || o.fields.borrow().iter().find(|(key, _)| key == "__default__").is_some_and(|(_, value)| !value.same_place(&no_default));
                    return Ok(Self::adapter(0, vec![Value::Flag(answer)]));
                }

                }
                if !plain { if let Some(f)=self.class_value(&o.class_now(),self.class_word("get")) {
                    return self.class_apply(f,vec![subject.clone(),Value::text(name)]);
                } }
                if !self.generic_instance_access(&o.class_now()) && self.lang.reader.as_deref()
                    .and_then(|reader| self.class_value(&o.class_now(), reader)).is_none() {
                    return Err(self.missing_member(&subject, name));
                }
                if name == self.class_word("kind") {
                    if self.module_holding(&subject).is_some() { return Ok(self.named_kind(&subject)); }
                    let actual = o.class_now().clone();
                    if let Some(overridden) = self.class_value(&actual, name) {
                        return self.bind_class_value(overridden, Some(subject.clone()), actual);
                    }
                    if !self.instance_root_visible(&actual) { return Err(self.missing_member(&subject,name)); }
                    return Ok(Value::Class(actual));
                }
                if name==self.class_word("namespace") {
                    if let Some(descriptor)=self.class_value(&o.class_now(),name) {
                        if Self::kind_beneath(&o.class_now()).as_deref() == Some("module") && !self.takes_writes(&descriptor) {
                            if let Some(held) = o.fields.borrow().iter().find(|(key, _)| key == name).map(|(_, v)| v.clone()) { return Ok(held); }
                        }
                        return self.bind_class_value(descriptor,Some(subject.clone()),o.class_now().clone());
                    }
                    // A class that names its slots and leaves the namespace out of them has things without one.
                    if o.class_now().mro_adopted.get() || o.class_now().python_names.borrow().is_some() || !self.slots_allow(&o.class_now(),name) {return Err(self.missing_member(&subject,name));}
                    if let Some((_, dictionary)) = o.fields.borrow().iter().find(|(key, _)| key == "\0namespace") { return Ok(dictionary.clone()); }
                    return Ok(Value::Fields(o.clone()));
                }
                let member=self.class_value(&o.class_now(),name);
                if member.is_none() && self.exception_class(&o.class_now()) && self.exception_method_named(name) {
                    return Ok(Value::ValueMethod(Rc::new((subject.clone(), name.to_string()))));
                }
                // A member that takes writes speaks before the thing's own
                // fields; any other member speaks after them.
                if member.as_ref().map_or(false,|m|self.takes_writes(m)) {return self.bind_class_value(member.unwrap(),Some(subject.clone()),o.class_now().clone());}
                let replaced = o.fields.borrow().iter().find(|(key, _)| key == "\0namespace").map(|(_, value)| value.clone());
                if let Some(dictionary) = replaced {
                    let raw = Self::worth_of(&dictionary).unwrap_or(dictionary).contents();
                    // A namespace may be another thing's own fields,
                    // shared by handing one `__dict__` to another; the
                    // name is then read from those very fields.
                    match raw {
                        Value::Map(entries) => {
                            if let Some((_, value)) = entries.iter().find(|(key, _)| matches!(key, Value::Text(text) if text.as_ref() == name)) { return Ok(value.clone()); }
                        }
                        Value::Fields(view) => {
                            if let Some((_, value)) = view.fields.borrow().iter().find(|(key, held)| key.as_str() == name && !matches!(held.contents(), Value::Blank)) { return Ok(value.clone()); }
                        }
                        _ => {}
                    }
                }
                if let Some((_,v))=o.fields.borrow().iter().find(|(n,v)| n==name && !matches!(v.contents(), Value::Blank)) {return Ok(v.clone());}
                if let Some(v)=member {return self.bind_class_value(v,Some(subject.clone()),o.class_now().clone());}
                if self.lang.class_details.get("root.members").and_then(|words| words.get(9)).map_or(false, |word| word == name) {
                    let root = Self::adapter(30, vec![Value::text(name)]);
                    return Ok(Self::adapter(3, vec![root, subject.clone()]));
                }
                if let Some(size) = self.integer_member(&subject, name) { return Ok(size); }
                if ["__reduce__", "__reduce_ex__", "__getstate__"].contains(&name) {
                    if let Some(root) = self.root_member(name, Some(&o.class_now())) {
                        return Ok(Self::adapter(3, vec![root, subject.clone()]));
                    }
                }
                if name == "__getnewargs__" {
                    if let Some(native) = Self::worth_of(&subject) {
                        if matches!(native.contents(), Value::Small(_) | Value::Huge(_) | Value::Real(_) | Value::Tuple(_) | Value::Text(_) | Value::Codepoints(_) | Value::Bytes(_, false, _)) {
                            return Ok(Value::ValueMethod(Rc::new((subject.clone(), String::from("__getnewargs__")))));
                        }
                    }
                }

                if Self::worth_of(&subject).is_some_and(|value| matches!(value.contents(), Value::Set(_))) {
                    if let Some(slot) = self.lang.class_special.iter().position(|word| word == name).filter(|at| *at == 79 || *at == 81) {
                        let operation = if slot == 81 { "set_reduce_ex" } else { "set_reduce" };
                        return Ok(Value::ValueMethod(Rc::new((subject.clone(), operation.to_owned()))));
                    }
                }
                // The worth a thing keeps answers for the methods of its kind.
                if let Some(worth)=Self::worth_of(&subject).filter(|v|matches!(v.contents(),Value::Set(_))) {
                    if [79, 81].iter().any(|slot| self.lang.class_special.get(*slot).is_some_and(|word| word == name)) {
                        return Ok(Value::ValueMethod(Rc::new((subject.clone(), name.to_string()))));
                    }
                    if let Some(member)=self.builtin_member(&worth,name)? { return Ok(member); }
                }
                if let Some(word @ Value::Text(_)) = Self::worth_of(&subject) {
                    if let Some(method) = self.builtin_member(&word, name)? { return Ok(method); }
                }
                if let Some(worth)=Self::worth_of(&subject).filter(|v|!matches!(v.contents(),Value::Set(_))) {
                    let collection_slot = self.lang.class_special.iter().position(|slot| slot == name).map_or(false, |slot| (10..=14).contains(&slot));
                    if (matches!(worth.contents(), Value::Complex(_)) || collection_slot) && self.native_special(&worth, name) {
                        if matches!(worth.contents(), Value::Map(_)) && self.lang.class_special.get(11).map_or(false, |key| key == name) {
                            let getter = Self::adapter(29, vec![Value::text("dict"), Value::text(name)]);
                            return Ok(Self::adapter(3, vec![getter, subject.clone()]));
                        }
                        return Ok(Value::ValueMethod(Rc::new((subject.clone(), name.to_string()))));
                    }
                    if let Some(op)=self.lang.value_methods.get(name).cloned() {
                        // The parts of a complex number are read rather
                        // than called, as they are on the number itself.
                        if matches!(op.as_str(),"real"|"imag") {
                            if let Value::Complex(z)=worth.contents() {
                                return Ok(crate::complex::real(if op=="real" {z.real} else {z.imag}));
                            }
                        }
                        // `fromkeys` belongs to the class, so a thing of
                        // a mapping kind is handed over itself and not
                        // its worth, that its own class may make it.
                        if op == "fromkeys" {
                            return Ok(Value::ValueMethod(Rc::new((subject.clone(), op))));
                        }
                        return Ok(Value::ValueMethod(Rc::new((subject.clone(),op))));
                    }
                    // A row of bytes answers to the methods its kind
                    // keeps, which the worth beneath the thing works.
                    if let Value::Bytes(_,changeable,_)=worth.contents() {
                        if let Some(working)=self.byte_member(name,changeable) {
                            return Ok(Value::ValueMethod(Rc::new((worth,working.to_string()))));
                        }
                    }
                    // A set answers some of its methods through builtins
                    // that take the receiver first, so reading one binds
                    // it to the worth.
                    if let Some(operation) = self.lang.builtins.get(name).copied().filter(|op| op.set_method()) {
                        let callable = if self.lang.class_builder.is_empty() { Value::text(name) }
                            else { Value::Native(operation, Rc::from(name)) };
                        return Ok(Self::adapter(3,vec![callable,worth]));
                    }
                }
                if self.module_holding(&subject).is_some() {
                    let getter = o.fields.borrow().iter().find(|(key, _)| key == "__getattr__")
                        .map(|(_, value)| value.contents()).filter(|value| !matches!(value, Value::Blank));
                    if let Some(getter) = getter { return self.class_apply(getter, vec![Value::text(name)]).map(|value| value.contents()); }
                }
                // A native base's reduction slots must not be replaced by
                // the root's empty-argument reconstruction of an ordinary object.
                let native_reduction = [79, 81].iter().any(|at| self.lang.class_special.get(*at).map_or(false, |word| word == name))
                    && Self::worth_of(&subject).map_or(false, |worth| self.native_special(&worth, name));
                if let Some(root)=self.instance_root_visible(&o.class_now()).then(|| self.root_member(name,Some(&o.class_now()))).flatten().filter(|_| !native_reduction) {
                    // The root's own maker takes the class rather than a
                    // thing, and the hook for a class stood on hears
                    // from the class; any other is bound to the thing.
                    if name==self.class_word("allocate") {return Ok(root);}
                    let receiver=if name==self.class_word("subclass") {Value::Class(o.class_now().clone())} else {subject.clone()};
                    return Ok(Self::adapter(3,vec![root,receiver]));
                }
            }
            Value::Method(o, f, _) => {
                if name==self.class_word("receiver") {return Ok(Value::Object(o.clone()));}
                if name==self.class_word("function") {return Ok(Value::Routine(f.clone()));}
                return self.class_get(Value::Routine(f.clone()),name,true);
            }
            Value::Routine(f) => {
                let f=&Self::routine_now(f);
                if let Some(v)=self.routine_member(&subject,name) {return Ok(v);}
                // The row of type parameters the declaration wrote, made
                // the first asking and kept, so every asking answers the
                // selfsame row.
                if name==self.class_word("type_params") {
                    let at=self.function_storage(&subject);
                    let own=format!("\0{name}");
                    if let Some(held)=self.function_members[at].1.fields.borrow().iter().find(|(n,_)|*n==own).map(|(_,v)|v.clone()) {return Ok(held);}
                    let held=self.routine_type_values(f)?;
                    self.function_members[at].1.fields.borrow_mut().push((own,held.clone()));
                    return Ok(held);
                }
                let annotation_name = self.lang.class_annotations.first().map(String::as_str).unwrap_or("");
                let annotate_name = self.lang.class_details.get("code.fields").and_then(|v| v.get(10)).map(String::as_str).unwrap_or("");
                if !annotation_name.is_empty() && name == annotation_name {
                    let evaluator = self.routine_member(&subject, annotate_name)
                        .or_else(|| f.annotation.clone().map(Value::Routine));
                    let result = match evaluator {
                        Some(a) if !matches!(a.contents(), Value::Null) => self.class_apply(a, vec![Value::Small(1)])?,
                        _ => self.keep_collection(Value::Map(Rc::new(Vec::new().into()))),
                    };
                    let at = self.function_storage(&subject);
                    self.function_members[at].1.fields.borrow_mut().push((format!("\0{name}"), result.clone()));
                    return Ok(result);
                }
                if !annotate_name.is_empty() && name == annotate_name { return Ok(f.annotation.clone().map_or(Value::Null, Value::Routine)); }
                // A routine built by hand keeps the dictionary it was
                // handed and answers with that, however it came to be.
                if name==self.class_word("globals") {
                    if let Some(globe)=&f.globe { return Ok(globe.clone()); }
                }
                if name==self.class_word("globals") && f.written_in.is_none() { return Ok(Value::Bond(self.constructor_book(f).unwrap_or_else(|| self.outer_book_made()))); }
                // The builtins a routine reads its unbound names from.
                if self.lang.module_builtins.iter().any(|word| word == name) {
                    return self.routine_builtins(f);
                }
                if name==self.class_word("name") {return Ok(self.routine_held(&subject,name,Value::text(&f.ident)));}
                if name==self.class_word("qualified") {return Ok(self.routine_held(&subject,name,Value::text(&f.qualified)));}
                if name==self.class_word("doc") {let fresh=f.doc.clone().map_or(Value::Null,|s|Value::text(&s));return Ok(self.routine_held(&subject,name,fresh));}
                if name==self.class_word("module") {let home=self.routine_module(f);return Ok(self.routine_held(&subject,name,home));}
                if name==self.class_word("defaults") {
                    let values=f.carried.iter().zip(&f.held).filter(|(i,_)| **i<f.formals.len() && f.parameter_rules.as_ref().map_or(true,|rules|rules[**i]<2)).map(|(_,v)|v.clone()).collect::<Vec<_>>();
                    let positional = f.parameter_rules.as_ref().map_or(f.formals.len(), |rules| rules.iter().filter(|rule| **rule < 2).count());
                    if values.is_empty() && f.least < positional && f.within.is_some(){return Err(self.class_refusal());}
                    return Ok(if values.is_empty(){Value::Null}else{Value::tuple(values)});
                }
                if name==self.class_word("keywords") {
                    let pairs=Self::spare_arguments(f,true);
                    return Ok(if pairs.is_empty(){Value::Null}else{Value::Map(Rc::new(pairs.into_iter().map(|(i,v)|(Value::text(&f.formals[i]),v)).collect()))});
                }
                if name==self.class_word("code") {return Ok(self.routine_code(f));}
                if name==self.class_word("namespace") { let at=self.function_storage(&subject); return Ok(Value::Fields(self.function_members[at].1.clone())); }
                // The cells a routine closes over, in the order of the
                // names they stand for, or nothing where it closes over
                // none.
                if name==self.class_word("closure") {
                    if f.enclosed.is_empty() {return Ok(Value::Null);}
                    let mut named:Vec<(&str,&Value)>=f.enclosed.iter().map(|(at,cell)|(f.idents.get(*at).map_or("", |name| Self::closure_title(name)),cell)).collect();
                    named.sort_by(|x,y|x.0.cmp(y.0));
                    return Ok(Value::tuple(named.into_iter().map(|(_,cell)|Self::adapter(31,vec![cell.clone()])).collect()));
                }
                // A routine answers its own call, so reading it back and
                // calling it does what calling the routine does.
                if name==self.class_word("call") { return Ok(Value::Routine(f.clone())); }
            }
            Value::Generator(held) if name == self.class_word("name") || name == self.class_word("qualified") => {
                let state = held.try_borrow().map_err(|_| self.lang.yield_busy[0].clone())?;
                return Ok(Value::text(if name == self.class_word("name") { &state.name } else { &state.qualified }));
            }
            Value::Generator(held) if !self.is_async_generator(&subject) && !self.lang.yield_running.is_empty() && name==self.lang.yield_running[0] => {
                // Running exactly while its own frame is on the way
                // through the machine, which is exactly when the cell
                // that holds it cannot be borrowed a second time.
                return Ok(Value::Flag(held.try_borrow().is_err()));
            }
            Value::Adapter(w) if w.0==7 => {
                if let Value::Routine(f)=&w.1[0] {
                    if self.lang.class_details.get("code.replace").and_then(|words| words.first()).map_or(false, |word| word == name) {
                        return Ok(Value::ValueMethod(Rc::new((subject.clone(), "code_replace".to_string()))));
                    }
                    if let Some(field) = self.lang.class_details.get("code.fields").and_then(|v| v.iter().position(|s| s == name)) {
                        let number = |n: usize| Value::Small(n as i64);
                        let words = |v: &[String]| Value::tuple(v.iter().map(|s| Value::text(s)).collect());
                        let value = match field {
                            0 => Value::text(if f.ident.starts_with("#generator") { "<genexpr>" } else if f.ident == "<program>" { self.lang.trace_fields.get(10).map_or("<module>", String::as_str) } else { &f.ident }),
                            1 => Value::text(&f.qualified),
                            2 => number(f.parameter_rules.as_ref().map_or(0, |v| v.iter().filter(|r| **r == 1).count())),
                            3 => number(f.parameter_rules.as_ref().map_or(0, |v| v.iter().filter(|r| **r == 2).count())),
                            4 => number(f.local_names.len()),
                            5 => words(&f.code_names),
                            6 => Value::tuple(f.code_constants.iter().map(|v| if let Value::Routine(r) = v { self.routine_code(r) } else { v.clone() }).collect()),
                            7 => Value::Small(f.code_flags),
                            8 => Value::Text(f.written_in.clone().unwrap_or_else(|| self.root_source.clone())),
                            9 => number(f.declared_on.max(1) as usize),
                            11 => {
                                let mut names: Vec<String> = f.enclosing.iter().map(|(slot, _)| Self::closure_title(&f.idents[*slot]).to_owned()).collect();
                                names.extend(f.enclosed.iter().map(|(slot, _)| Self::closure_title(&f.idents[*slot]).to_owned()));
                                names.sort();
                                names.dedup();
                                words(&names)
                            }
                            _ => return Err(self.missing_member(&subject, name)),
                        };
                        return Ok(value);
                    }
                    match self.lang.trace_fields.iter().position(|key| key == name) {
                        Some(6) => return Ok(Value::text(if f.ident == "<program>" { &self.lang.trace_fields[10] } else { &f.ident })),
                        Some(7) => return Ok(Value::Text(f.written_in.clone().unwrap_or_else(|| self.root_source.clone()))),
                        Some(8) => return Ok(Value::Small(f.declared_on.max(1) as i64)),
                        _ => {}
                    }
                    if name==self.class_word("argcount") {return Ok(Value::Small(f.parameter_rules.as_ref().map_or(f.formals.len(),|rules|rules.iter().filter(|r|**r<2).count()) as i64));}
                    if name==self.class_word("varnames") {return Ok(Value::tuple(f.local_names.iter().map(|n|Value::text(n)).collect()));}
                }
            }
            Value::Adapter(w) if w.0==4 || w.0==5 => {
                if name==self.class_word("function"){return Ok(w.1[0].clone());}
                // CPython 3.10 and after hand the wrapper the routine
                // it holds under `__wrapped__`, and read the routine's
                // own metadata through the wrapper unchanged.
                if name=="__wrapped__" { return Ok(w.1[0].clone()); }
                // The abstract mark of the routine within is read through
                // the wrapper as well; a routine with no mark answers no.
                if name==self.class_word("abstractmethod") && !self.class_word("abstractmethod").is_empty() {
                    return match self.class_get(w.1[0].clone(),name,true) {
                        Ok(held) => Ok(held),
                        Err(fault) if self.attribute_fault(&fault) => Ok(Value::Flag(false)),
                        Err(fault) => Err(fault),
                    };
                }
                let inner=w.1[0].clone();
                let carried=[self.class_word("module"),self.class_word("qualified"),self.class_word("name"),self.class_word("doc")];
                let copied=carried.iter().any(|word|!word.is_empty()&&*word==name)
                    || self.lang.class_annotations.first().map_or(false,|word|word==name)
                    || self.lang.class_details.get("code.fields").and_then(|fields| fields.get(10)).is_some_and(|word| word == name);
                if copied { return self.class_get(inner,name,true); }
            }
            Value::Adapter(w) if w.0 == 143 => {
                if name == self.class_word("code") { return Ok(w.1[0].clone()); }
                if name == self.class_word("globals") { return Ok(w.1[1].clone()); }
                if name == self.class_word("name") || name == self.class_word("qualified") { return Ok(w.1[2].clone()); }
                if name == self.class_word("call") { return Ok(subject.clone()); }
                if name == self.class_word("defaults") || name == self.class_word("keywords") || name == self.class_word("closure") { return Ok(Value::Null); }
                if name == self.class_word("kind") { return Ok(Value::Class(self.kind_class("function"))); }
            }
            Value::Adapter(w) if w.0==32 => {
                if ["send", "throw", "close"].contains(&name)
                    || self.lang.class_special.get(15).is_some_and(|word| word == name)
                    || self.lang.class_special.get(16).is_some_and(|word| word == name)
                    || self.lang.async_generator_methods.get(3).is_some_and(|word| word == name) {
                    return Ok(Value::ValueMethod(Rc::new((subject.clone(), name.to_string()))));
                }
            }
            Value::Adapter(w) if w.0==31 && name==self.class_word("cell.contents") => {
                return Self::cell_held(w).ok_or_else(||self.class_word("cell.empty").to_string().into());
            }
            Value::Adapter(w) if w.0 == 235 => {
                if name == self.class_word("name") { return Ok(Value::text("__get__")); }
                if name == self.class_word("qualified") { return Ok(Value::text("classmethod_descriptor.__get__")); }
                if name == "__objclass__" { return Ok(Value::Class(self.kind_class("classmethod_descriptor"))); }
            }
            Value::Adapter(w) if w.0==29 => {
                if w.1.len() == 3 && name == self.class_word("descriptor.get") { return Ok(Value::ValueMethod(Rc::new((subject.clone(), "classmethod_bind".to_owned())))); }
                if name==self.class_word("qualified") { return Ok(Value::text(&format!("{}.{}", w.1[0].plain(), w.1[1].plain()))); }
                if name==self.class_word("name") { return Ok(w.1[1].clone()); }
                if name=="__objclass__" {
                    if let Some(kind)=self.spelled_kind(&w.1[0].plain()) { return Ok(kind); }
                }
            }
            Value::Adapter(w) if matches!(w.0, 3 | 131) => {
                if w.0 == 3 && name == self.class_word("kind") { return self.class_type(vec![subject.clone()]); }
                if name==self.class_word("receiver") {return Ok(w.1[1].clone());}
                if name==self.class_word("function") {return Ok(w.1[0].clone());}
                if w.0 == 131 {
                    if name == self.class_word("kind") { return self.class_type(vec![subject.clone()]); }
                    if name == self.class_word("name") || name == self.class_word("qualified") {
                        if let Value::Native(_, word) = &w.1[0] { return Ok(Value::text(word)); }
                    }
                    return self.class_get(w.1[0].clone(), name, true);
                }
                return self.class_get(w.1[0].clone(), name, plain);
            }
            _ => {}
        }
        if name == self.class_word("allocate") && matches!(subject, Value::Null | Value::Ellipsis | Value::Declined(_)) {
            let kind = self.named_kind(&subject);
            return self.class_get(kind, name, plain);
        }
        // Whatever is not a thing is of the kind the kind builtin names
        // for it, where it names one.
        if name==self.class_word("kind") && !matches!(subject,Value::Object(_)) {
            if let Ok(kind)=self.class_type(vec![subject.clone()]) {return Ok(kind);}
        }
        self.absent_member = Some((name.to_string(), subject.clone()));
        Err(self.missing_member(&subject,name))
    }
    /// A routine as it now stands, its spare arguments or code written
    /// over by the program or not.
    /// The name of the module a routine was written in: the module the
    /// file it came from was read as, or the run's own name where the
    /// routine is the program itself.
    pub(super) fn routine_home(&self, f: &Routine) -> String {
        f.written_in.as_ref()
            .and_then(|place| self.module_slots.get(place))
            .map_or_else(|| self.class_word("main").to_string(), |(_, path)| path.clone())
    }
    /// The module a routine answers as its own before anything the
    /// program set on it: the name of the namespace a routine made by
    /// hand was made in, caught when it was made (None where that
    /// namespace named nothing), else the module its file was read as.
    /// Whether a name the compiler gave a routine stands for a function
    /// of its own to carry a namespace name: a real name does, the word
    /// the language writes an anonymous routine under does, and a mark
    /// standing for a piece of the program around it does not.
    pub(super) fn takes_a_title(&self, ident: &str) -> bool {
        if ident.starts_with('#') { return false; }
        if !ident.starts_with('<') { return true; }
        self.lang.lambda_name.iter().any(|word| word == ident)
    }
    /// The name the namespace a routine is made beside goes by, caught
    /// as the routine is made: the name it answers to now, or that it
    /// names nothing at all. Nothing at all here means the routine was
    /// made beside no namespace worth asking.
    pub(super) fn captured_title(&self, globe: &Value) -> Option<Option<Rc<str>>> {
        let held = match globe {
            Value::Bond(cell) | Value::Binding(cell) | Value::Collection(cell, _) => cell.borrow().clone(),
            other => other.clone(),
        };
        let named_in = match &held {
            Value::Map(pairs) => pairs,
            Value::Fields(_) => return self.titled_fields(&held),
            _ => return None,
        };
        for word in &self.lang.module_names {
            if let Some(worth) = named_in.iter().find(|(key, _)| matches!(key, Value::Text(name) if name.as_ref() == word.as_str())).map(|(_, worth)| worth.contents()) {
                return Some(match worth { Value::Text(named) => Some(named.clone()), _ => None });
            }
        }
        Some(None)
    }
    /// Capture a function's name from its globals, including the main
    /// program's live dictionary when no imported namespace was attached.
    pub(super) fn creation_title(&mut self, routine: &Routine) -> Option<Option<Rc<str>>> {
        if !self.takes_a_title(&routine.ident) || self.lang.module_names.is_empty() { return None; }
        match &routine.globe {
            Some(globals) => self.captured_title(globals),
            None => {
                let globals = Value::Bond(self.book_here(true));
                self.captured_title(&globals)
            }
        }
    }
    /// The same question asked of the face a thing holds: its entries
    /// stand as rows of its own rather than of a map.
    fn titled_fields(&self, held: &Value) -> Option<Option<Rc<str>>> {
        let Value::Fields(o) = held else { return None };
        let fields = o.fields.borrow();
        for word in &self.lang.module_names {
            if let Some(worth) = fields.iter().find(|(n, _)| n == word).map(|(_, worth)| worth.contents()) {
                return Some(match worth { Value::Text(named) => Some(named.clone()), _ => None });
            }
        }
        Some(None)
    }
    pub(super) fn routine_module(&self, f: &Routine) -> Value {
        match &f.home {
            Some(named) => named.as_ref().map_or(Value::Null, |word| Value::text(word)),
            None => Value::text(&self.routine_home(f)),
        }
    }
    /// The word the definition gives the module the builtin names live in.
    pub(super) fn home_module_word(&self) -> &str {
        self.lang.names_module.first().map_or("builtins", String::as_str)
    }

    /// A builtin kind read by the word that spells it, where that word
    /// names a kind: the value `int`, not the class standing for it.
    pub(super) fn spelled_kind(&self, word: &str) -> Option<Value> {
        self.lang.builtins.get(word).copied().filter(|op| Self::kind_builtin(op)).map(|op| match op {
            Builtin::Bytes(tag @ 0..=1) if Lang::spells(&self.lang.builtin_bases, word) => self.byte_kind(tag == 1),
            _ => Value::Native(op, Rc::from(word)),
        })
    }
    fn routine_type_values(&mut self, f: &Routine) -> Flow<Value> {
        let mut items = Vec::new();
        if !f.type_params.is_empty() {
            let maker = self.type_holder()?;
            for name in &f.type_params {
                items.push(self.class_apply(maker.clone(), vec![Value::text(name)])?);
            }
        }
        Ok(Value::tuple(items))
    }
    /// The maker of type-parameter names: the hinting module's own
    /// maker, from the module the program already holds where it holds
    /// one, and read in from the library where it does not.
    fn type_holder(&mut self) -> Flow<Value> {
        self.typing_module();
        Ok(Value::Class(self.kind_class("TypeVar")))
    }
    fn routine_now(f: &Rc<Routine>) -> Rc<Routine> {
        f.revised.borrow().clone().unwrap_or_else(|| f.clone())
    }
    /// The spare arguments a routine carries, slot by slot in the order
    /// they were written: those taken by position, or those taken by
    /// name alone.
    fn spare_arguments(f: &Routine, named: bool) -> Vec<(usize, Value)> {
        f.carried.iter().zip(&f.held)
            .filter(|(i,_)| **i<f.formals.len() && (f.parameter_rules.as_ref().map_or(0,|rules|rules[**i])==2)==named)
            .map(|(i,v)|(*i,v.clone())).collect()
    }
    /// A copy of a routine carrying other spare arguments: those taken
    /// by position laid over the last such parameters, and those taken
    /// by name alone over the parameters so named. Nothing given keeps
    /// what it carried.
    fn with_spare_arguments(base: &Routine, by_place: Option<Vec<Value>>, by_name: Option<Vec<(String,Value)>>) -> Routine {
        let mut made = base.clone();
        made.revised = RefCell::new(None);
        let rules = base.parameter_rules.clone().unwrap_or_else(|| vec![0; base.formals.len()]);
        let placed: Vec<usize> = (0..base.formals.len()).filter(|i| rules[*i] < 2).collect();
        let mut kept: Vec<(usize, Value)> = base.carried.iter().zip(&base.held)
            .filter(|(i,_)| **i >= base.formals.len() || if rules[**i] == 2 { by_name.is_none() } else { by_place.is_none() })
            .map(|(i,v)| (*i, v.clone())).collect();
        let mut wanted = placed.len();
        if let Some(values) = by_place {
            let fitted = values.len().min(placed.len());
            wanted = placed.len() - fitted;
            kept.extend(placed[wanted..].iter().copied().zip(values[values.len()-fitted..].iter().cloned()));
        } else {
            wanted -= kept.iter().filter(|(i,_)| placed.contains(i)).count();
        }
        for (key, value) in by_name.unwrap_or_default() {
            if let Some(i) = (0..base.formals.len()).find(|i| rules[*i] == 2 && base.formals[*i] == key) { kept.push((i, value)); }
        }
        kept.sort_by_key(|(i,_)| *i);
        made.carried = kept.iter().map(|(i,_)| *i).collect();
        made.held = kept.into_iter().map(|(_,v)| v).collect();
        made.least = wanted;
        made.parameter_rules = Some(rules);
        made
    }
    /// Whether a code object written over a routine's own was made of
    /// another generator/coroutine category. Ordinary flags do not change
    /// that category and must not emit the deprecation warning.
    fn code_kind_differs(f: &Rc<Routine>, value: Option<&Value>) -> bool {
        let Some(Value::Adapter(code)) = value.map(Value::contents) else { return false };
        let (7, Some(Value::Routine(source))) = (code.0, code.1.first()) else { return false };
        (Self::routine_now(source).code_flags ^ Self::routine_now(f).code_flags) & (0x20 | 0x80 | 0x200) != 0
    }
    /// The routine as it stands once the program writes its spare
    /// arguments, its keyword-only spare arguments or its code over, or
    /// takes them away; a value of the wrong kind is refused with
    /// CPython's words, as is code closing over another count of cells.
    fn routine_rewritten(&self, f: &Rc<Routine>, name: &str, value: Option<Value>) -> Flow<Routine> {
        let now = Self::routine_now(f);
        let refused = |part: &str| -> Fault { self.class_word(part).to_string().into() };
        if name == self.class_word("code") {
            let Some(Value::Adapter(code)) = value.as_ref().map(Value::contents) else { return Err(refused("code.amiss")) };
            let (7, Some(Value::Routine(source))) = (code.0, code.1.first()) else { return Err(refused("code.amiss")) };
            let source = Self::routine_now(source);
            if source.enclosing.len() != now.enclosing.len() {
                let pieces = self.lang.class_details.get("code.free").cloned().unwrap_or_default();
                if pieces.len() != 3 { return Err(self.class_refusal()); }
                return Err(format!("{}{}{}{}{}{}", pieces[0], now.ident, pieces[1], now.enclosing.len(), pieces[2], source.enclosing.len()).into());
            }
            let mut made = (*source).clone();
            made.annotation = now.annotation.clone();
            made.ident = now.ident.clone();
            made.qualified = now.qualified.clone();
            made.doc = now.doc.clone();
            made.enclosed = source.enclosing.iter().zip(&now.enclosed).map(|((at,_),(_,cell))| (*at, cell.clone())).collect();
            let by_place = Self::spare_arguments(&now, false).into_iter().map(|(_,v)| v).collect();
            let by_name = Self::spare_arguments(&now, true).into_iter().map(|(i,v)| (now.formals[i].clone(), v)).collect();
            return Ok(Self::with_spare_arguments(&made, Some(by_place), Some(by_name)));
        }
        if name == self.class_word("keywords") {
            let pairs = match value.as_ref().map(Value::contents) {
                None | Some(Value::Null) => Vec::new(),
                Some(Value::Map(pairs)) => pairs.iter().map(|(k,v)| (k.plain(), v.clone())).collect(),
                Some(_) => return Err(refused("keywords.amiss")),
            };
            return Ok(Self::with_spare_arguments(&now, None, Some(pairs)));
        }
        let values = match value.as_ref().map(Value::contents) {
            None | Some(Value::Null) => Vec::new(),
            Some(Value::Tuple(items)) => items.to_vec(),
            Some(_) => return Err(refused("defaults.amiss")),
        };
        Ok(Self::with_spare_arguments(&now, Some(values), None))
    }
    /// Where a routine keeps the namespace the program handed it in
    /// place of the one it was made with.
    const HANDED_BOOK: &'static str = "\0 namespace";
    /// The builtins a routine answers with: what its own dictionary
    /// keeps under that name where it holds one, else the builtins in
    /// force where it was built, else the module's own dictionary. A
    /// module handed there stands for the dictionary it keeps.
    fn routine_builtins(&mut self, program: &Rc<Routine>) -> Flow<Value> {
        let word = match self.lang.module_builtins.first() { Some(word) => word.clone(), None => return Ok(self.native_dict()) };
        if let Some(globe) = &program.globe {
            if let Some(held) = self.dyn_lookup(globe, &word)? { return self.as_builtins_dictionary(held); }
        }
        if let Some(born) = &program.born { return self.as_builtins_dictionary(born.clone()); }
        Ok(self.native_dict())
    }
    /// A module handed over where a dictionary of builtins is wanted
    /// stands for the dictionary it keeps, as the reference reads it.
    pub(super) fn as_builtins_dictionary(&mut self, held: Value) -> Flow<Value> {
        let value = held.contents();
        let module_kind = self.lang.module_kind.last();
        let module = self.module_holding(&value).is_some()
            || matches!(&value, Value::Object(o) if module_kind.is_some_and(|name| o.class_now().named(name, false)));
        if module {
            let word = self.class_word("namespace").to_string();
            return self.class_get(held, &word, false);
        }
        Ok(held)
    }
    /// A member a routine holds of its own: its name, full name or
    /// account of itself as the program wrote them over, the namespace
    /// handed to it, or an entry of whichever namespace it keeps.
    fn routine_member(&self, subject: &Value, name: &str) -> Option<Value> {
        let (_,members)=self.function_members.iter().find(|(v,_)| v.revive().map_or(false, |key| key.equals(subject)))?;
        let fields=members.fields.borrow();
        {
            let own=format!("\0{name}");
            if let Some((_,v))=fields.iter().find(|(n,_)| *n==own) {return Some(v.clone());}
        }
        match fields.iter().find(|(n,_)| n==Self::HANDED_BOOK) {
            Some((_,book)) if name==self.class_word("namespace") => Some(book.clone()),
            Some((_,book)) => match book.contents() {
                Value::Map(pairs) => pairs.iter().find(|(k,_)| matches!(k,Value::Text(t) if t.as_ref()==name)).map(|(_,v)|v.clone()),
                _ => None,
            },
            None => fields.iter().find(|(n,_)| n==name).map(|(_,v)|v.clone()),
        }
    }
    /// The names a routine's own namespace holds, for its directory.
    fn routine_member_names(&self, subject: &Value) -> Vec<String> {
        let Some((_,members))=self.function_members.iter().find(|(v,_)| v.revive().map_or(false, |key| key.equals(subject))) else {return Vec::new()};
        let fields=members.fields.borrow();
        if let Some((_,book))=fields.iter().find(|(n,_)| n==Self::HANDED_BOOK) {
            let Value::Map(pairs)=book.contents() else {return Vec::new()};
            return pairs.iter().filter_map(|(k,_)| match k {Value::Text(t)=>Some(t.to_string()),_=>None}).collect();
        }
        fields.iter().filter(|(n,_)|!n.starts_with(['\0', '#'])).map(|(n,_)|n.clone()).collect()
    }
    /// A namespace handed to a routine: a dictionary it keeps from then
    /// on, the very one handed over; anything else, or taking it away,
    /// is refused as CPython refuses it.
    pub(super) fn routine_namespace_write(&mut self, at: usize, value: Option<Value>) -> Flow<Value> {
        let Some(book)=value else {return Err(self.class_word("namespace.kept").to_string().into())};
        if !matches!(book.contents(),Value::Map(_)) {
            let pieces=self.lang.class_details.get("namespace.amiss").cloned().unwrap_or_default();
            if pieces.len()!=2 {return Err(self.class_refusal());}
            return Err(format!("{}{}{}",pieces[0],Self::shown_kind(&book.contents()),pieces[1]).into());
        }
        let mut fields=self.function_members[at].1.fields.borrow_mut();
        fields.retain(|(n,_)| n.starts_with('\0'));
        let _=Self::write_members(&mut fields,Self::HANDED_BOOK,Some(book),false);
        Ok(Value::Null)
    }
    /// An entry written into, or taken out of, a namespace handed to a
    /// routine, through the cell it is held in so every name for it
    /// sees the change. False where there was nothing to take away.
    fn book_write(book: &Value, name: &str, value: Option<Value>) -> bool {
        let (Value::Collection(cell,_)|Value::Bond(cell)|Value::Binding(cell))=book else {return false};
        let mut held=cell.borrow_mut();
        let Value::Map(pairs)=&mut *held else {return false};
        let rows=Rc::make_mut(pairs);
        let at=rows.iter().position(|(k,_)| matches!(k,Value::Text(t) if t.as_ref()==name));
        match (at,value) {
            (Some(i),Some(v)) => rows[i].1=v,
            (None,Some(v)) => rows.push((Value::text(name),v)),
            (Some(i),None) => {rows.remove(i);}
            (None,None) => return false,
        }
        true
    }
    /// One of a routine's own readings that must answer the selfsame
    /// object on every asking -- its name, full name and module -- kept
    /// where the program's own writes to them are kept, so a later read
    /// finds it and a later write writes over it.
    fn routine_held(&mut self, subject: &Value, name: &str, fresh: Value) -> Value {
        let at=self.function_storage(subject);
        let own=format!("\0{name}");
        if let Some(held)=self.function_members[at].1.fields.borrow().iter().find(|(n,_)|*n==own).map(|(_,v)|v.clone()){return held;}
        self.function_members[at].1.fields.borrow_mut().push((own,fresh.clone()));
        fresh
    }
    pub(super) fn function_storage(&mut self, function: &Value) -> usize {
        self.constructor_indices.borrow_mut().clear();
        if let Some(at) = self.function_members.iter().position(|(v, _)| v.revive().map_or(false, |key| key.equals(function))) { return at; }
        let class = self.root_class();
        self.made += 1;
        let fields = Rc::new(Instance {replacement_class: RefCell::new(None),  class, fields: RefCell::new(Vec::new()), mark: self.made });
        let entry = (crate::faint::hold_of(function).expect("function has a weak identity"), fields);
        // Reclaim dead metadata without moving live indices held across
        // annotation callbacks. Repeated short-lived functions must not
        // leave an ever-growing lookup table behind.
        if let Some(vacant) = self.function_members.iter().position(|(key, _)| key.gone()) {
            self.function_members[vacant] = entry;
            vacant
        } else {
            self.function_members.push(entry);
            self.function_members.len() - 1
        }
    }
    /// The annotations a class carries: its own, worked out the first
    /// time they are asked for from the routines its body kept and held
    /// from then on. A class whose body annotated nothing has an empty
    /// map of its own; a parent's annotations are never handed down.
    pub(super) fn class_annotations(&mut self,c:&Rc<Class>) -> Flow<Value> {
        let word=self.lang.class_annotations[0].clone();
        if let Some((_,held))=c.shared.borrow().iter().find(|(n,_)|*n==word || n == "__annotations_cache__") { return Ok(held.clone()); }
        let annotate = self.lang.class_details.get("code.fields").and_then(|row| row.get(10)).cloned().unwrap_or_default();
        let overwritten = { let members = c.shared.borrow(); members.iter().find(|(key, _)| key == &annotate).or_else(|| members.iter().find(|(key, _)| key == "__annotate_func__")).map(|(_, value)| value.contents()) };
        let made = match overwritten {
            Some(Value::Null) => self.keep_collection(Value::Map(Rc::new(Vec::new().into()))),
            Some(callable) => self.class_apply(callable, vec![Value::Small(1)])?,
            None => self.evaluate_class_annotations(c)?,
        };
        c.shared.borrow_mut().push(("__annotations_cache__".into(),made.clone()));
        Ok(made)
    }
    fn evaluate_class_annotations(&mut self, c: &Rc<Class>) -> Flow<Value> {
        // The entry stands in a cell, as a class's members do.
        let kept=c.shared.borrow().iter().find(|(n,_)|n==crate::code::ANNOTATE_WORD).map(|(_,v)|v.contents());
        let mut pairs=Vec::new();
        if let Some(Value::Array(row))=kept {
            for pair in row.chunks(2) {
                let [key,routine]=pair else {break};
                let routine = routine.contents();
                if matches!(routine, Value::Blank) { continue; }
                let value=match routine {Value::Routine(_)|Value::Method(..)=>self.class_apply(routine,Vec::new())?,other=>other};
                pairs.push((key.clone(),value));
            }
        }
        let made=Value::Map(Rc::new(pairs.into()));
        Ok(self.keep_collection(made))
    }
    pub(super) fn class_write(&mut self, subject:Value, name:&str, value:Option<Value>, plain:bool) -> Flow<Value> {
        if matches!(&subject,Value::Object(instance) if Self::own_kind(&instance.class_now()).as_deref()==Some("struct_time")) {
            if ["tm_year","tm_mon","tm_mday","tm_hour","tm_min","tm_sec","tm_wday","tm_yday","tm_isdst","tm_zone","tm_gmtoff"].contains(&name) {return Err("AttributeError: readonly attribute".into())}
            return Err(format!("AttributeError: 'time.struct_time' object has no attribute '{name}'").into());
        }
        // A changed abstract marker must be read again on the next construction.
        if self.lang.class_details.get("abstract").and_then(|words| words.first()).is_some_and(|word| word == name) {
            if let Value::Class(class) = subject.contents() { self.concrete_kinds.borrow_mut().remove(&(Rc::as_ptr(&class) as usize)); }
        }
        if let Value::Trace(trace) = &subject {
            if self.lang.trace_fields.get(2).map(String::as_str) == Some(name) {
                return match value {
                    None => Err(format!("TypeError: can't delete {name} attribute").into()),
                    Some(Value::Null) => { *trace.next.borrow_mut() = Value::Null; Ok(Value::Null) }
                    Some(fresh @ Value::Trace(_)) => {
                        if Self::traceback_reaches(&fresh, trace) { return Err("ValueError: traceback loop detected".into()); }
                        *trace.next.borrow_mut() = fresh;
                        Ok(Value::Null)
                    }
                    Some(other) => Err(format!("TypeError: expected traceback object, got '{}'", other.core_kind()).into()),
                };
            }
            return Err("AttributeError: readonly attribute".into());
        }
        // A late module binding also replaces a compiled builtin in its code.
        if self.lang.shadow_builtins && self.lang.builtins.contains_key(name) {
            if let Some(owner) = self.module_holding(&subject) {
                if self.lang.names_module.first().is_some_and(|word| word == &owner) {
                    let native = self.lang.builtins.get(name);
                    let restored = value.as_ref().is_some_and(|held| match (held.contents(), native) {
                        (Value::Native(operation, _), Some(expected)) => operation == *expected,
                        (Value::ByteKind(mutable, _), Some(Builtin::Bytes(mode @ 0..=1))) => mutable == (*mode == 1),
                        _ => false,
                    });
                    if restored { self.changed_builtins.remove(name); }
                    else { self.changed_builtins.insert(name.to_string()); }
                }
                for (source, (_, module)) in &self.module_slots {
                    if module == &owner { self.changed_builtin_scopes.insert(source.clone()); }
                }
                self.wildcard_file = None;
            }
        }
        if let Value::Object(instance) = &subject {
            if let Some(kind) = Self::own_kind(&instance.class_now()).filter(|kind| matches!(kind.as_str(), "TypeVar" | "ParamSpec" | "TypeVarTuple" | "TypeAliasType" | "NoDefaultType")) {
                if name == "__name__" || (kind == "ParamSpec" && name == "__bound__") { return Err("AttributeError: readonly attribute".into()); }
                let protected = matches!(name, "__default__" | "__constraints__" | "__bound__" | "__covariant__" | "__contravariant__" | "__infer_variance__") && kind != "TypeVarTuple" || name == "__default__";
                if protected || kind == "TypeAliasType" || kind == "NoDefaultType" { return Err(format!("AttributeError: attribute '{name}' of 'typing.{kind}' objects is not writable").into()); }
                Self::write_members(&mut instance.fields.borrow_mut(), name, value, false).map_err(|_| self.missing_member(&subject, name))?;
                return Ok(Value::Null);
            }
        }
        if !self.lang.weak_refused.is_empty() {
            if let Value::Object(proxy) = subject.contents() {
                if matches!(proxy.class_now().name.as_str(), "ProxyType" | "CallableProxyType") {
                    let weak = proxy.fields.borrow().iter().find(|(key, _)| key == "\0weak").map(|(_, v)| v.clone());
                    if let Some(Value::Faint(weak)) = weak {
                        let target = weak.revive().ok_or_else(|| Fault::Note("ReferenceError: weakly-referenced object no longer exists".to_string()))?;
                        return self.class_write(target, name, value, plain);
                    }
                }
            }
        }

        if !self.class_word("name").is_empty() && !matches!(&subject, Value::Class(_)) {
            if let Some(kind) = self.kind_word_of(&subject.contents()) {
                return Err(format!("TypeError: cannot set '{name}' attribute of immutable type '{kind}'").into());
            }
        }
        if matches!(&subject, Value::Object(object) if object.fields.borrow().iter().any(|(key, _)| key == "\0immutable")) {
            return Err(format!("AttributeError: '{}' object is immutable", subject.core_kind()).into());
        }
        if self.lang.traceback_member.as_deref() == Some(name) && matches!(&subject, Value::Object(o) if self.exception_class(&o.class_now())) {
            match &value {
                Some(Value::Null | Value::Trace(_)) => {},
                None => return Err("TypeError: __traceback__ may not be deleted".into()),
                _ => return Err("TypeError: __traceback__ must be a traceback or None".into()),
            }
        }
        if let Value::Generator(cell) = &subject {
            if self.lang.yield_running.first().is_some_and(|word| word == name) {
                let parts = &self.lang.class_details["method.fixed"];
                return Err(format!("{}{name}{}{}{}", parts[0], parts[1], subject.core_kind(), parts[2]).into());
            }
            if name == self.class_word("name") || name == self.class_word("qualified") {
                let Some(Value::Text(text)) = value.as_ref().map(Value::contents) else {
                    let words = self.lang.class_details.get("text.amiss").cloned().unwrap_or_default();
                    return Err(format!("{}{name}{}", words[0], words[1]).into());
                };
                let mut state = cell.try_borrow_mut().map_err(|_| self.lang.yield_busy[0].clone())?;
                if name == self.class_word("name") { state.name = text.to_string(); } else { state.qualified = text.to_string(); }
                return Ok(Value::Null);
            }
        }
        let mut value = value;
        if let Value::Object(o) = &subject {
            if self.exception_class(&o.class_now()) {
                if (self.lang.group_message.as_deref() == Some(name) || self.lang.group_members.as_deref() == Some(name))
                    && o.class_now().has_public_field("\0group") {
                    return Err(format!("AttributeError: attribute '{name}' of '{}' objects is not writable", o.class_now().name).into());
                }
                let cause = self.lang.exception_cause.as_deref() == Some(name);
                let context = self.lang.exception_context.as_deref() == Some(name);
                if cause || context {
                    match &value {
                        None => return Err(format!("TypeError: {name} may not be deleted").into()),
                        Some(v) if matches!(v.contents(), Value::Null) || matches!(v.contents(), Value::Object(e) if self.exception_class(&e.class_now())) => {},
                        _ => return Err(format!("TypeError: exception {} must be None or derive from BaseException", if cause { "cause" } else { "context" }).into()),
                    }
                }
                if self.lang.exception_suppress.as_deref() == Some(name) {
                    match &value {
                        None => return Err("TypeError: can't delete numeric/char attribute".into()),
                        Some(v) if matches!(v.contents(), Value::Flag(_)) => {},
                        _ => return Err("TypeError: attribute value type must be bool".into()),
                    }
                }
                if value.is_none() {
                    if self.lang.exception_args.as_deref() == Some(name) {
                        return Err(format!("TypeError: {name} may not be deleted").into());
                    }
                    let member = (self.stands_on(&o.class_now(), 36) && (self.lang.syntax_members.iter().any(|n| n == name) || name == "_metadata"))
                        || (self.stands_on(&o.class_now(), 19) && ["msg", "name", "path", "name_from"].contains(&name))
                        || (self.stands_on(&o.class_now(), 17) && name == "code")
                        || (self.stop_class(&o.class_now()) && self.lang.stop_value_member.as_deref() == Some(name))
                        || ((self.stands_on(&o.class_now(), 10) || self.stands_on(&o.class_now(), 12)) && self.lang.absent_name_member.as_deref() == Some(name))
                        || (self.stands_on(&o.class_now(), 12) && self.lang.absent_object_member.as_deref() == Some(name))
                        || (self.stands_on(&o.class_now(), 20) && (self.lang.os_members.iter().any(|n| n == name) || name == "filename2"))
                        || ((self.stands_on(&o.class_now(), 43) || self.stands_on(&o.class_now(), 44) || self.stands_on(&o.class_now(), 45)) && ["encoding", "object", "reason"].contains(&name));
                    if member { value = Some(Value::Null); }
                }
            }
        }
        let absent=self.missing_member(&subject,name);
        match &subject {
            Value::Object(o) => {
                if o.fields.borrow().iter().any(|(key, _)| key == "\0structseq") {
                    return Err(if name.starts_with("st_") { "AttributeError: readonly attribute".to_string().into() } else { format!("AttributeError: 'os.stat_result' object has no attribute '{}' and no __dict__ for setting new attributes", name).into() });
                }
                if self.module_holding(&subject).is_some() || Self::kind_beneath(&o.class_now()).as_deref() == Some("module") {
                    let annotate = self.lang.class_details.get("code.fields").and_then(|row| row.get(10)).cloned().unwrap_or_default();
                    if name == annotate {
                        let Some(incoming) = value.as_ref() else { return Err("TypeError: cannot delete __annotate__ attribute".into()); };
                        if !matches!(incoming.contents(), Value::Null) {
                            if !self.class_work(2, vec![incoming.clone()])?.is_true() { return Err("TypeError: __annotate__ must be callable or None".into()); }
                            if let Some(annotation) = self.lang.class_annotations.first() {
                                o.fields.borrow_mut().retain(|(key, _)| key != annotation);
                                let book = o.fields.borrow().iter().find(|(key, _)| key == "\0namespace").map(|(_, value)| value.clone());
                                if let Some(book) = book { Self::book_write(&book, annotation, None); }
                            }
                        }
                    } else if self.lang.class_annotations.first().is_some_and(|key| key == name) {
                        let book = o.fields.borrow().iter().find(|(key, _)| key == "\0namespace").map(|(_, value)| value.clone());
                        if let Some(book) = book { Self::book_write(&book, &annotate, Some(Value::Null)); }
                        else { Self::write_members(&mut o.fields.borrow_mut(), &annotate, Some(Value::Null), true).map_err(|_| self.class_refusal())?; }
                    }
                }

                if self.exception_class(&o.class_now()) && self.lang.exception_args.as_deref() == Some(name) {
                    if let Some(v) = value.as_ref() {
                        let items = match v.contents() {
                            Value::Array(items) | Value::Tuple(items) => Value::Tuple(items),
                            other => return Err(format!("TypeError: '{}' object is not iterable", Self::shown_kind(&other)).into()),
                        };
                        let mut fields = o.fields.borrow_mut();
                        for key in [name, "\0arguments"] { let _ = Self::write_members(&mut fields, key, Some(items.clone()), false); }
                        return Ok(Value::Null);
                    }
                }
                if !plain {
                    let hook=if value.is_some(){"set"}else{"remove"};
                    if let Some(f)=self.class_value(&o.class_now(),self.class_word(hook)) {
                        let mut args=vec![subject.clone(),Value::text(name)];args.extend(value);return self.class_apply(f,args);
                    }
                    if let Some(Value::Adapter(w))=self.class_value(&o.class_now(),name) {
                        if w.0==6 {
                            if let (Some(set),Some(v))=(w.1.get(1),value.clone()) {return self.class_apply(set.clone(),vec![subject.clone(),v]);}
                            return Err(absent);
                        }
                    }
                }
                if !self.generic_instance_access(&o.class_now()) {
                    let availability = if self.lang.reader.as_deref().and_then(|key| self.class_value(&o.class_now(),key)).is_some() { "only read-only attributes" } else { "no attributes" };
                    return Err(format!("TypeError: '{}' object has {} ({} .{})", o.class_now().name, availability, if value.is_some() { "assign to" } else { "del" }, name).into());
                }
                // A member that takes writes takes this one: a slot keeps
                // the value, a property's kept accessor refuses, and any
                // other asks its class's writer or remover.
                if let Some(member)=self.class_value(&o.class_now(),name) {
                    if let Value::Adapter(w)=&member {
                        if w.0==16 {return self.slot_write(&subject,&w.1,value);}
                        // A property keeps its docstring and assigned name
                        // in private fields that take writes; its accessors do
                        // not, as CPython keeps them read-only.
                        if w.0==28 {
                            let place = w.1[0].plain();
                            if !matches!(place.as_str(), "\0doc" | "\0name") { return Err(self.class_word("property.readonly").to_string().into()); }
                            let mut fields = o.fields.borrow_mut();
                            let _ = Self::write_members(&mut fields, &place, value, false);
                            return Ok(Value::Null);
                        }
                    }
                    if self.takes_writes(&member) {
                        let part=if value.is_some(){"descriptor.set"}else{"descriptor.delete"};
                        let hook=self.descriptor_hook(&member,part).ok_or_else(||self.missing_member(&subject,name))?;
                        let mut args=vec![subject.clone()];args.extend(value);
                        self.call_descriptor(&member,hook,args)?;
                        return Ok(Value::Null);
                    }
                }
                if name == self.class_word("namespace") && self.module_holding(&subject).is_some()
                    && self.class_value(&o.class_now(), name).is_none() {
                    return Err(self.class_word("property.readonly").to_string().into());
                }
                if name == self.class_word("namespace") && Self::kind_beneath(&o.class_now()).as_deref() == Some("module")
                    && self.class_value(&o.class_now(), name).is_some() {
                    Self::write_members(&mut o.fields.borrow_mut(), name, value, true).map_err(|_| absent)?;
                    return Ok(Value::Null);
                }
                // A thing's own namespace, written back to it after an
                // entry was put in, is where it was: nothing to do.
                if name==self.class_word("namespace") {
                    if let Some(Value::Fields(view))=&value {if Rc::ptr_eq(view,o){return Ok(Value::Null);}}
                }
                // A thing's namespace taken away leaves it with an empty
                // one, and a dictionary handed to it becomes its entries.
                if name==self.class_word("namespace") {
                    return self.replace_instance_namespace(o, value);
                }
                if name == self.class_word("kind") {
                    let Some(Value::Class(next)) = value else { return Err("TypeError: __class__ must be set to a class".to_string().into()); };
                    let old = o.class_now();
                    if Self::own_class_value(&old, self.class_word("module")).is_none() || Self::own_class_value(&next, self.class_word("module")).is_none()
                        || Self::kind_beneath(&old) != Self::kind_beneath(&next) || self.instance_layout(&old) != self.instance_layout(&next) {
                        return Err("TypeError: __class__ assignment: object layout differs".to_string().into());
                    }
                    let old_slots = self.layout_slots(&old);
                    let new_slots = self.layout_slots(&next);
                    for (field, _) in o.fields.borrow_mut().iter_mut() {
                        if let Some(at) = old_slots.iter().position(|(_, place)| place == field) { *field = new_slots[at].1.clone(); }
                    }
                    *o.replacement_class.borrow_mut() = Some(next);
                    return Ok(Value::Null);
                }
                // A class naming the members its things hold, and holding
                // a value of its own under a name not among them, has
                // that name read-only: the class's own holding is not
                // what a write to a thing reaches, and the thing keeps
                // no namespace to put one of its own in.
                if self.slots_named(&o.class_now()) && !self.slots_allow(&o.class_now(),name) && self.class_value(&o.class_now(),name).is_some() {
                    return Err(self.readonly_member(&subject,name));
                }
                if value.is_some() && !self.slots_allow(&o.class_now(),name) {return Err(self.unwritable_member(&subject,name));}
                if value.is_some() && Self::kind_beneath(&o.class_now()).is_none() {
                    let mut fields = o.fields.borrow_mut();
                    if !fields.iter().any(|(key, _)| key == "\0namespace" || key == "\0expanded") && fields.iter().filter(|(key, _)| !key.starts_with('\0')).count() >= 30 {
                        // Keep the attribute owner stable for existing dictionary views.
                        // The expanded layout can grow past the compact capacity.
                        fields.push(("\0expanded".to_string(), Value::Flag(true)));
                    }
                }
                let dictionary = o.fields.borrow().iter().find(|(key, _)| key == "\0namespace").map(|(_, value)| value.clone());
                if let Some(dictionary) = dictionary {
                    if matches!(dictionary, Value::Bond(_) | Value::Binding(_) | Value::Collection(..))
                        && matches!(dictionary.contents(), Value::Map(_)) {
                        if !Self::book_write(&dictionary, name, value) { return Err(absent); }
                        return Ok(Value::Null);
                    }
                    let raw = Self::worth_of(&dictionary).unwrap_or_else(|| dictionary.held(false));
                    match value {
                        Some(value) => { self.value_method(&raw, "update", vec![Value::Map(Rc::new(vec![(Value::text(name), value)].into()))], vec![])?; }
                        None => { self.value_method(&raw, "pop", vec![Value::text(name)], vec![])?; }
                    }
                    return Ok(Value::Null);
                }

                // A module's members are its own bindings, written through
                // so that its routines see the new value; a thing's member
                // is simply written over.
                let module=self.module_holding(&subject).is_some();
                if module { self.module_member_mirror(o, name, &value); }
                // A name a module takes from outside that it never bound
                // for itself becomes one of its own globals, standing in
                // the place its routines read it from, as the reference's
                // module.__dict__ write is a global write.
                if module && value.is_some() && !o.fields.borrow().iter().any(|(n,_)| n == name) {
                    let suffix = format!(":{}:{}", o.class_now().name, name);
                    // The newest incarnation of the module answers for
                    // the name: a module read in again owns fresh cells.
                    if let Some(slot) = self.registry.idents.iter().rposition(|word| word.starts_with("\0module:") && word.ends_with(&suffix)) {
                        let shared = Value::Bond(Rc::new(RefCell::new(value.clone().unwrap())));
                        self.world.resize(self.registry.idents.len(), Value::Blank);
                        self.world[slot] = shared.clone();
                        o.fields.borrow_mut().push((name.to_string(), shared));
                        return Ok(Value::Null);
                    }
                }
                Self::write_members(&mut o.fields.borrow_mut(),name,value,module).map_err(|_|absent)?;
            }
            Value::Class(c) => {
                if !plain && !self.class_word("name").is_empty() {
                    let operation = if value.is_some() { "set" } else { "remove" };
                    if let Some(metaclass) = Self::maker_beneath(c) {
                        if let Some(writer) = self.class_value(&metaclass, self.class_word(operation)) {
                            let bound = self.bind_class_value(writer, Some(subject.clone()), metaclass)?;
                            let mut given = vec![Value::text(name)];
                            given.extend(value);
                            return self.class_apply(bound, given);
                        }
                    }
                }
                if !self.class_word("name").is_empty() && !Self::class_sealed(c) {
                    if let Some(maker) = Self::maker_beneath(c) {
                        if let Some(member) = self.class_value(&maker, name).filter(|member| self.takes_writes(member)) {
                            let part = if value.is_some() { "descriptor.set" } else { "descriptor.delete" };
                            let hook = self.descriptor_hook(&member, part).ok_or_else(|| Fault::Note(format!("AttributeError: {}", self.class_word(part))))?;
                            let mut args = vec![subject.clone()]; args.extend(value);
                            self.call_descriptor(&member, hook, args)?;
                            return Ok(Value::Null);
                        }
                    }
                }
                if Self::class_sealed(c) || c.constants.iter().any(|(key, _)| key == "\0native-name") || (c.python_names.borrow().is_none() && ["name", "qualified", "doc"].iter().any(|key| !self.class_word(key).is_empty() && name == self.class_word(key))) {
                    return Err(format!("TypeError: cannot set '{name}' attribute of immutable type '{}'", c.name).into());
                }
                // The kinds of the two named singletons are fixed the
                // way the reference fixes them: nothing is written onto
                // the kind itself or taken off it.
                if matches!(Self::own_kind(c).as_deref(), Some("NotImplementedType") | Some("ellipsis")) {
                    return Err(format!("TypeError: cannot set '{name}' attribute of immutable type '{}'", c.name).into());
                }
                if c.python_names.borrow().is_some() && name == self.class_word("doc") && value.is_none() {
                    return Err(format!("TypeError: cannot delete '{name}' attribute of immutable type '{}'", c.python_names.borrow().as_ref().unwrap().0.type_text().plain()).into());
                }
                if c.python_names.borrow().is_some() && (name == self.class_word("name") || name == self.class_word("qualified")) {
                    let Some(candidate) = value.as_ref() else { return Err(format!("TypeError: cannot delete '{name}' attribute of immutable type '{}'", c.python_names.borrow().as_ref().unwrap().0.type_text().plain()).into()); };
                    let raw = candidate.type_text();
                    if !matches!(raw, Value::Text(_) | Value::Codepoints(_)) { return Err(format!("TypeError: can only assign string to {}.{name}, not '{}'", c.python_names.borrow().as_ref().unwrap().0.type_text().plain(), Self::type_argument_kind(candidate)).into()); }
                    if name == self.class_word("name") {
                        self.type_title(&raw)?;
                        c.python_names.borrow_mut().as_mut().unwrap().0 = candidate.contents();
                    } else { c.python_names.borrow_mut().as_mut().unwrap().1 = candidate.contents(); }
                    return Ok(Value::Null);
                }
                if !self.class_word("base").is_empty() && name == self.class_word("base") {
                    if let Some(member) = Self::maker_beneath(c).and_then(|maker| self.class_value(&maker, name)) {
                        if self.takes_writes(&member) {
                            let part = if value.is_some() { "descriptor.set" } else { "descriptor.delete" };
                            let hook = self.descriptor_hook(&member, part).ok_or_else(|| self.missing_member(&subject, name))?;
                            let mut given = vec![subject.clone()]; given.extend(value);
                            return self.call_descriptor(&member, hook, given);
                        }
                        Self::write_members(&mut c.shared.borrow_mut(), name, value, false).map_err(|_| absent)?;
                        return Ok(Value::Null);
                    }
                    return Err("AttributeError: readonly attribute".into());
                }
                if name == self.class_word("bases") && c.python_names.borrow().is_some() {
                    if let Some(Value::Tuple(bases)) = value.as_ref().map(Value::contents) {
                        let actual = bases.iter().map(|base| self.type_base(base)).collect::<Flow<Vec<_>>>()?;
                        if actual.len() == c.direct.len() && actual.iter().zip(&c.direct).all(|(a, b)| Rc::ptr_eq(a, b)) { return Ok(Value::Null); }
                    }
                }
                if name == self.class_word("qualified") {
                    match value.as_ref().map(Value::contents) {
                        Some(Value::Text(_)) => {},
                        Some(other) => return Err(format!("TypeError: can only assign string to {}.__qualname__, not '{}'", c.name, other.core_kind()).into()),
                        None => return Err(self.class_refusal()),
                    }
                } else if ["name","kind","base","bases","mro","namespace","order"].iter().any(|key|name==self.class_word(key)){return Err(self.class_refusal());}
                let annotate = self.lang.class_details.get("code.fields").and_then(|row| row.get(10)).cloned().unwrap_or_default();
                if !annotate.is_empty() && name == annotate {
                    let Some(incoming) = value.as_ref() else { return Err("TypeError: cannot delete __annotate__ attribute".into()); };
                    if !matches!(incoming.contents(), Value::Null) {
                        if !self.class_work(2, vec![incoming.clone()])?.is_true() { return Err("TypeError: __annotate__ must be callable or None".into()); }
                        if let Some(annotation) = self.lang.class_annotations.first() { c.shared.borrow_mut().retain(|(key, _)| key != annotation && key != "__annotations_cache__"); }
                    }
                }
                if name == self.class_word("type_params") && value.is_none() {
                    return Err("TypeError: cannot delete '__type_params__' attribute of immutable type".into());
                }
                if name == self.class_word("module") {
                    c.shared.borrow_mut().retain(|(member, _)| member != "__firstlineno__");
                }
                if self.lang.class_annotations.first().is_some_and(|key| key == name) {
                    c.shared.borrow_mut().retain(|(key, _)| key != "__annotate_func__");
                    let annotate = self.lang.class_details.get("code.fields").and_then(|row| row.get(10)).cloned().unwrap_or_default();
                    Self::write_members(&mut c.shared.borrow_mut(), &annotate, Some(Value::Null), false).map_err(|_| self.class_refusal())?;
                    if !c.shared.borrow().iter().any(|(key, _)| key == name) {
                        Self::write_members(&mut c.shared.borrow_mut(), "__annotations_cache__", value, false).map_err(|_| absent)?;
                        return Ok(Value::Null);
                    }
                }
                Self::write_members(&mut c.shared.borrow_mut(),name,value,false).map_err(|_|absent)?;
            }
            Value::Routine(_) => {
                if name == self.class_word("namespace") {
                    let at = self.function_storage(&subject);
                    if matches!(&value, Some(Value::Fields(fields)) if Rc::ptr_eq(fields, &self.function_members[at].1)) { return Ok(Value::Null); }
                    return self.routine_namespace_write(at, value);
                }
                if ["globals","closure"].iter().any(|k|name==self.class_word(k)) {return Err(self.class_word("property.readonly").to_string().into());}
                // The builtins a routine reaches its unbound names
                // through are read off it, never written over.
                if self.lang.module_builtins.iter().any(|word| word == name) {return Err(self.class_word("property.readonly").to_string().into());}
                if name==self.class_word("defaults") || name==self.class_word("keywords") || name==self.class_word("code") {
                    let Value::Routine(f)=&subject else {return Err(self.class_refusal())};
                    if name==self.class_word("code") && Self::code_kind_differs(f, value.as_ref()) {
                        let told=self.lang.class_details.get("code.mismatch").and_then(|w| w.first()).cloned().unwrap_or_default();
                        self.warn_like(26,&told)?;
                    }
                    let now=self.routine_rewritten(f,name,value)?;
                    *f.revised.borrow_mut()=Some(Rc::new(now));
                    return Ok(Value::Null);
                }
                let annotate = self.lang.class_details.get("code.fields").and_then(|fields| fields.get(10)).cloned().unwrap_or_default();
                if !annotate.is_empty() && name == annotate {
                    let Some(given) = value else { return Err("TypeError: __annotate__ cannot be deleted".into()); };
                    if !matches!(given.contents(), Value::Null) && !self.class_work(2, vec![given.clone()])?.is_true() {
                        return Err("TypeError: __annotate__ must be callable or None".into());
                    }
                    let at = self.function_storage(&subject);
                    let mut members = self.function_members[at].1.fields.borrow_mut();
                    if !matches!(given.contents(), Value::Null) {
                        if let Some(annotation) = self.lang.class_annotations.first() { members.retain(|(key, _)| key != &format!("\0{annotation}")); }
                    }
                    Self::write_members(&mut members, &format!("\0{annotate}"), Some(given), false).map_err(|_| absent)?;
                    return Ok(Value::Null);
                }
                if self.lang.class_annotations.first().map_or(false, |s| s == name) {
                    let at = self.function_storage(&subject);
                    let given = match value.as_ref().map(Value::contents) {
                        None | Some(Value::Null) => self.keep_collection(Value::Map(Rc::new(Vec::new().into()))),
                        Some(Value::Map(_)) => value.unwrap(),
                        _ => return Err(format!("TypeError: {name} must be set to a dict object").into()),
                    };
                    let mut members = self.function_members[at].1.fields.borrow_mut();
                    Self::write_members(&mut members, &format!("\0{name}"), Some(given), false).map_err(|_| self.class_refusal())?;
                    Self::write_members(&mut members, &format!("\0{annotate}"), Some(Value::Null), false).map_err(|_| absent)?;
                    return Ok(Value::Null);
                }
                if name==self.class_word("type_params") {
                    // The row of type parameters takes a row and nothing
                    // else, and cannot be taken away at all.
                    if !matches!(value.as_ref().map(Value::contents), Some(Value::Tuple(_))) {
                        return Err(self.class_word("defaults.amiss").to_string().into());
                    }
                    let at=self.function_storage(&subject);
                    Self::write_members(&mut self.function_members[at].1.fields.borrow_mut(), &format!("\0{name}"), value, false).map_err(|_| absent)?;
                    return Ok(Value::Null);
                }
                let at=self.function_storage(&subject);
                // Its name and full name take text alone, and its account
                // of itself anything at all; the three stand apart from
                // the namespace, and taking the account away leaves none.
                if name==self.class_word("name") || name==self.class_word("qualified") {
                    let Some(Value::Text(_))=value.as_ref().map(Value::contents) else {
                        let pieces=self.lang.class_details.get("text.amiss").cloned().unwrap_or_default();
                        return Err(if pieces.len()==2 {format!("{}{name}{}",pieces[0],pieces[1]).into()} else {self.class_refusal()});
                    };
                }
                if name==self.class_word("name") || name==self.class_word("qualified") || name==self.class_word("doc") || name==self.class_word("module") || self.lang.class_annotations.first().map_or(false, |s| s == name) {
                    let own=format!("\0{name}");
                    Self::write_members(&mut self.function_members[at].1.fields.borrow_mut(),&own,Some(value.unwrap_or(Value::Null)),false).map_err(|_|absent)?;
                    return Ok(Value::Null);
                }
                let book=self.function_members[at].1.fields.borrow().iter().find(|(n,_)|n==Self::HANDED_BOOK).map(|(_,v)|v.clone());
                match book {
                    Some(book) => if !Self::book_write(&book,name,value) {return Err(absent);},
                    None => Self::write_members(&mut self.function_members[at].1.fields.borrow_mut(),name,value,false).map_err(|_|absent)?,
                }
            }
            // A cell takes what it holds, and taking that away leaves it
            // empty; it keeps nothing else.
            Value::Adapter(w) if w.0==32 => {
                let known = ["send", "throw", "close", "__class__", "__doc__"].contains(&name)
                    || self.lang.class_special.get(15).is_some_and(|word| word == name)
                    || self.lang.class_special.get(16).is_some_and(|word| word == name)
                    || self.lang.async_generator_methods.get(3).is_some_and(|word| word == name);
                let message = if known { format!("AttributeError: '{}' object attribute '{}' is read-only", subject.core_kind(), name) }
                    else { format!("AttributeError: '{}' object has no attribute '{}' and no __dict__ for setting new attributes", subject.core_kind(), name) };
                return Err(message.into());
            }
            Value::Adapter(w) if w.0==31 && name==self.class_word("cell.contents") => {
                let (Some(Value::Binding(held)) | Some(Value::Bond(held)))=w.1.first() else {return Err(absent)};
                *held.borrow_mut()=value.unwrap_or(Value::Blank);
                return Ok(Value::Null);
            }
            // A method keeps nothing of its own: the thing and routine
            // it binds are fixed, its account of itself is the
            // routine's, and nothing else can be written or taken away.
            Value::Method(..) | Value::Adapter(_) if matches!(&subject, Value::Method(..)) || matches!(&subject, Value::Adapter(w) if w.0 == 131) => {
                if name==self.class_word("receiver") || name==self.class_word("function") {return Err(self.class_word("property.readonly").to_string().into());}
                if name==self.class_word("kind") {
                    let part=if value.is_some(){"kind.fixed"}else{"kind.kept"};
                    return Err(self.class_word(part).to_string().into());
                }
                if name==self.class_word("doc") {
                    let pieces=self.lang.class_details.get("method.fixed").cloned().unwrap_or_default();
                    if pieces.len()==3 {return Err(format!("{}{name}{}{}{}",pieces[0],pieces[1],subject.core_kind(),pieces[2]).into());}
                }
                return Err(if value.is_some() {self.unwritable_member(&subject,name)} else {absent});
            }
            // A singleton's class is as fixed as the singleton itself:
            // answered for, never written over nor taken away.
            Value::Declined(_) | Value::Ellipsis if name == self.class_word("kind") => {
                return Err(self.class_word(if value.is_some() { "kind.fixed" } else { "kind.kept" }).to_string().into());
            }
            // A value of a builtin kind keeps no namespace: a write
            // says so outright, while a taking-away only reports the
            // member that was never there.
            _ if value.is_some() => return Err(self.unwritable_member(&subject,name)),
            _ => return Err(self.missing_member(&subject,name)),
        }
        Ok(Value::Null)
    }
    fn write_members(members:&mut Vec<(String,Value)>,name:&str,value:Option<Value>,through:bool)->Result<(),()> {
        let at=members.iter().position(|(n,_)|n==name);
        // A member held in a shared cell is written through the cell where
        // the holder asks it -- a module's own binding, which its routines
        // read -- unless the cell itself is what was handed back.
        match (at,value) {
            (Some(i),Some(v))=>match (&members[i].1,&v) {
                (Value::Bond(cell),Value::Bond(given)) if Rc::ptr_eq(cell,given)=>{},
                (Value::Bond(cell),_) if through=>{*cell.borrow_mut()=v;},
                _=>members[i].1=v },
            (None,Some(v))=>{
                // A binding of the module's own put back after it was
                // taken away goes into the very cell its code reads,
                // where the name had one of its own.
                if through {
                    if let Some((_, Value::Map(links))) = members.iter().find(|(n,_)| n == "\0bindings") {
                        if let Some(link) = links.iter().find(|(key,_)| matches!(key, Value::Text(word) if word.as_ref() == name)).map(|(_, held)| held.clone()) {
                            if let Value::Bond(cell)|Value::Binding(cell)|Value::Collection(cell,_) = &link { *cell.borrow_mut() = v; }
                            members.push((name.into(), link));
                            return Ok(());
                        }
                    }
                }
                members.push((name.into(),v));
            },
            (Some(i),None)=>{
                // A binding taken away leaves its cell empty, so the
                // name is gone from the very slot the module's own code
                // reads through and not only from the face it shows.
                if through {
                    if let Value::Bond(cell)|Value::Binding(cell)|Value::Collection(cell,_) = &members[i].1 {
                        *cell.borrow_mut() = Value::Blank;
                    }
                }
                members.remove(i);
            },
            _=>return Err(())} Ok(())
    }
    /// Whether the class, or one it stands on, names the members its
    /// things may hold. Such a thing keeps no namespace of its own.
    fn layout_slots(&self, class: &Class) -> Vec<(String, String)> {
        let mut places = Vec::new();
        for base in std::iter::once(class).chain(class.lineage.borrow().iter().map(Rc::as_ref)) {
            for (name, entry) in base.shared.borrow().iter() {
                if let Value::Adapter(slot) = entry {
                    if slot.0 == 16 && !matches!(name.as_str(), "__dict__" | "__weakref__") {
                        if let Some(Value::Class(owner)) = slot.1.get(1) {
                            places.push((name.clone(), format!("\0slot:{name}:{:p}", Rc::as_ptr(owner))));
                        }
                    }
                }
            }
        }
        places.sort_by(|one, two| one.0.cmp(&two.0));
        places
    }
    fn instance_layout(&self, class: &Class) -> (Vec<String>, bool, bool) {
        let mut names = Vec::new();
        let mut dictionary = !self.slots_named(class);
        for base in std::iter::once(class).chain(class.lineage.borrow().iter().map(Rc::as_ref)) {
            for (name, entry) in base.shared.borrow().iter() {
                if matches!(entry, Value::Adapter(slot) if slot.0 == 16) {
                    if name == "__dict__" { dictionary = true; }
                    else if name != "__weakref__" { names.push(name.clone()); }
                }
            }
        }
        names.sort();
        (names, dictionary, self.weak_layout(class))
    }
    fn slots_named(&self,c:&Class)->bool {
        Self::own_class_value(c,self.class_word("slots")).is_some()
            || c.direct.iter().filter(|b|b.name!=self.class_word("root")).any(|b|self.slots_named(b))
    }
    /// Weak storage belongs to the instance layout, independently of its
    /// dictionary and of the native value kept in its fields.
    pub(super) fn weak_layout(&self, c: &Class) -> bool {
        if let Some(layout) = c.weak_storage.get() { return layout; }
        if matches!(Self::kind_beneath(c).as_deref(), Some("int" | "tuple" | "bytes")) { return false; }
        if let Some(kind) = Self::own_kind(c) { return matches!(kind.as_str(), "set" | "frozenset"); }
        if c.direct.is_empty() { return false; }
        match Self::own_class_value(c, self.class_word("slots")) {
            None => true,
            Some(slots) => {
                let names = slots.contents();
                let weak = |v: &Value| matches!(v, Value::Text(s) if s.as_ref() == "__weakref__");
                let declared = match &names { Value::Tuple(v) | Value::Array(v) => v.iter().any(weak), v => weak(v) };
                declared || c.direct.iter().any(|base| self.weak_layout(base))
            }
        }
    }
    fn replace_instance_namespace(&self, o: &Rc<Instance>, value: Option<Value>) -> Flow<Value> {
        if matches!(&value, Some(Value::Fields(view)) if Rc::ptr_eq(view, o)) { return Ok(Value::Null); }
        let entries=match value.as_ref().map(|v| Self::worth_of(v).unwrap_or_else(|| v.clone()).contents()) {
            None => Vec::new(),
            Some(Value::Map(pairs)) => pairs.iter().filter_map(|(k,v)| match k {Value::Text(t)=>Some((t.to_string(),v.clone())),_=>None}).collect(),
            Some(Value::Fields(view)) => view.fields.borrow().iter().filter(|(key,_)| !key.starts_with('\0')).cloned().collect(),
            Some(other) => {
                let pieces=self.lang.class_details.get("namespace.amiss").cloned().unwrap_or_default();
                if pieces.len()!=2 {return Err(self.class_refusal());}
                return Err(format!("{}{}{}",pieces[0],Self::shown_kind(&other),pieces[1]).into());
            }
        };
        let mut fields=o.fields.borrow_mut();
        fields.retain(|(n,_)| n.starts_with('\0') && n != "\0namespace");
        if let Some(dictionary) = value { fields.push(("\0namespace".to_string(), dictionary.held(true))); }
        else { fields.push(("\0namespace".to_string(), Value::Collection(Rc::new(RefCell::new(Value::Map(Rc::new(Vec::new().into())))), true))); }
        let _ = entries;
        return Ok(Value::Null);
    }
    fn slots_allow(&self,c:&Class,name:&str)->bool {
        if !self.lang.class_builder.is_empty() && self.class_root.as_ref().is_some_and(|root| std::ptr::eq(root.as_ref(),c)) { return false; }
        if matches!(Self::own_kind(c).as_deref(), Some("SimpleNamespace" | "module")) { return true; }
        let own=Self::own_class_value(c,self.class_word("slots"));
        let Some(own)=own else{return Self::own_kind(c).map_or(true, |word| word == "module");};
        let slots=own.contents();
        let allows=|v:&Value|match v {Value::Text(s)=>s.as_ref()==name||s.as_ref()==self.class_word("namespace"),_=>false};
        let fits=match &slots {Value::Array(v)|Value::Tuple(v)=>v.iter().any(allows),v=>allows(v)};
        fits||c.direct.iter().filter(|b|b.name!=self.class_word("root")).any(|b|self.slots_allow(b,name))
    }
    /// The type flags the reference reports for a class: a heap type,
    /// standing as a base unless sealed (a sealed class reads as
    /// immutable instead), with a namespace dictionary and an inline
    /// layout where they apply, and tracked by the cyclic collector
    /// unless its things keep a worth of an atomic builtin kind, which
    /// can take no part in a cycle.
    pub(super) fn flags_of(&self, c: &Class) -> i64 {
        let dictionary = self.slots_allow(c, self.class_word("namespace"));
        let inline = dictionary && Self::kind_beneath(c).is_none();
        let tracked = Self::own_kind(c).is_none()
            && !matches!(Self::kind_beneath(c).as_deref(), Some("tuple" | "int" | "float" | "complex" | "str" | "bytes" | "bytearray"));
        let protocol = self.class_value(c, "__abc_tpflags__").map_or(0, |v| match v.contents() { Value::Small(n) => n, _ => 0 }) & 96;
        protocol + if c.constants.iter().any(|(key, _)| key == "\0native-name") && !Self::class_sealed(c) { 256 } else { 0 } + if Self::own_kind(c).is_some() || c.base.is_none() && c.name == self.class_word("root") { 0 } else { 512 } + if Self::class_sealed(c) { 256 } else { 1024 } + if dictionary { 16 } else { 0 } + if inline { 4 } else { 0 } + if tracked { 16384 } else { 0 }
    }
    pub(super) fn type_base(&mut self, value: &Value) -> Flow<Rc<Class>> {
        if !self.lang.class_builder.is_empty() && self.names_property_class(&value.contents()) { return Ok(self.property_class()); }
        match value.contents() {
            Value::Class(class) if Self::class_sealed(&class) => {
                let name = if class.shared.borrow().iter().any(|(key, flag)| key == "\0buffer_allocator" && flag.is_true()) { class.python_title().unwrap_or_else(|| class.name.clone()) } else if !self.lang.weak_refused.is_empty() && matches!(class.name.as_str(), "ProxyType" | "CallableProxyType") { format!("weakref.{}", class.name) } else { class.name.clone() };
                Err(format!("TypeError: type '{name}' is not an acceptable base type").into())
            },
            Value::Class(class) => Ok(class),
            Value::ByteKind(mutable, _) if Lang::spells(&self.lang.builtin_bases, self.byte_kind_word(mutable)) => {
                let word = self.byte_kind_word(mutable).to_string();
                Ok(self.kind_class(&word))
            }
            Value::Native(Builtin::SortOf, _) => Ok(self.metaclass_root()),
            Value::Native(Builtin::Bool, _) => Err(self.lang.bool_base.clone().unwrap_or_default().into()),
            Value::Native(operation, name) if Lang::spells(&self.lang.builtin_bases, &name)
                || (!self.lang.class_builder.is_empty() && matches!(operation, Builtin::ClassTool(9..=11))) => Ok(self.kind_class(&name)),
            Value::ByteKind(_, _) | Value::SortOf(_) | Value::Adapter(_) if !self.lang.class_builder.is_empty() => {
                let name = value.kind_it_names().map(Rc::from).or_else(|| self.kind_spelled(value));
                if let Some(name) = name {
                    if Lang::spells(&self.lang.builtin_bases, &name) || matches!(self.lang.builtins.get(name.as_ref()), Some(Builtin::ClassTool(9..=11))) { return Ok(self.kind_class(&name)); }
                }
                Err("TypeError: bases must be types".into())
            },
            other if self.names_property_class(&other) => Ok(self.property_class()),
            _ => Err("TypeError: bases must be types".to_string().into()),
        }
    }
    pub(super) fn class_type(&mut self,args:Vec<Value>)->Flow<Value> {
        let mut named = Vec::new(); let mut plain = Vec::new();
        for (word, value) in self.call_items(args)? {
            if let Some(word) = word { named.push(Value::Tie(Rc::new((Value::text(&word), value)))); }
            else { plain.push(value.contents()); }
        }
        let mut args = plain;
        if args.len() != 3 && !named.is_empty() { return Err("TypeError: type() takes 1 or 3 arguments".into()); }
        if args.len() == 3 {
            self.type_title(&args[0])?;
            let original = args[2].clone();
            if let Some(worth) = Self::worth_of(&original) {
                if matches!(worth.contents(), Value::Map(_)) {
                    let method = self.class_get(original.clone(), "keys", false)?;
                    let view = self.class_apply(method, Vec::new())?;
                    let mut copied = Vec::new();
                    for key in self.comprehension_items(&view)? {
                        let value = self.special_dyad(&crate::code::Action::At, &original, &key)?;
                        copied.push((key, value));
                    }
                    args[2] = Value::Map(Rc::new(copied.into()));
                } else { args[2] = worth.contents(); }
            }
            else if let Value::Object(object) = &original {
                if object.class_now().name == "frozendict" {
                    if let Some((_, rows)) = object.fields.borrow().iter().find(|(key, _)| key == "_rows") { args[2] = rows.contents(); }
                }
            }
        }
        match args.as_slice() {
            // A module read in is a thing like any other, but the
            // reference knows it by the one word for every module
            // rather than by the name that module goes by.
            [Value::Object(_)] if self.module_holding(&args[0]).is_some()=>Ok(self.named_kind(&args[0])),
            [Value::Object(o)] if self.property_class.as_ref().is_some_and(|kind|Rc::ptr_eq(kind,&o.class_now()))=>Ok(Value::Native(Builtin::ClassTool(11),Rc::from(o.class_now().name.as_str()))),
            [Value::Object(o)]=>Ok(Value::Class(o.class_now().clone())),
            [Value::Generator(_)] if self.is_async_generator(&args[0]) => Ok(Value::Class(self.kind_class("async_generator"))),
            // A class is of the kind that made it: the metaclass named
            // for it or for a class it stands on, and otherwise the kind
            // builtin itself, under whatever word spells it.
            [Value::Class(c)]=>Ok(match Self::maker_beneath(c) {
                Some(maker)=>Value::Class(maker),
                None=>self.kind_maker_word(),
            }),
            // A builtin word read as a class is of the kind builtin's
            // kind as well; a routine and a method are of kinds the
            // definition does not name, so they take the words for them.
            [Value::Adapter(_)] if self.kind_spelled(&args[0]).is_some()=>Ok(self.kind_maker_word()),
            // A method or a data member read off a builtin kind's own
            // word is of the descriptor kind CPython gives it.
            [Value::Adapter(w)] if matches!(w.0,29|63|64)=>Ok(self.named_kind(&args[0])),
            [Value::Adapter(w)] if w.0==3 && matches!(w.1.first(),Some(Value::Adapter(draw)) if draw.0==63)=>Ok(self.named_kind(&args[0])),
            [Value::Routine(_)]|[Value::Method(..)]=>Ok(self.named_kind(&args[0])),
            [Value::Adapter(_)]=>Ok(self.named_kind(&args[0])),
            [_, _, _] => {
                let mut parts = vec![self.kind_maker_word()]; parts.extend(args.clone()); parts.extend(named.clone());
                let made = self.class_from_parts(parts)?;
                let root = self.metaclass_root();
                args.extend(named);
                self.initialise_made_class(&made, &root, args)?;
                Ok(made)
            }
            [one] => {
                let word = self.lang.builtin_words.iter().find(|(op, _)| *op == Builtin::SortOf).map(|(_, word)| word.clone()).unwrap_or_default();
                Ok(self.builtin(Builtin::SortOf, &word, &mut vec![one.clone()])?)
            }
            _=>Err("TypeError: type() takes 1 or 3 arguments".into()),
        }
    }
    /// A class whose metaclass holds the member for the question may
    /// answer it itself: the member is read from the metaclass bound to
    /// the class, called with the value, and its word taken. A class no
    /// metaclass of its own made says nothing, and the plain check
    /// stands, so the common question costs one look at the constants.
    pub(super) fn maker_answers(&mut self,wanted:&Value,given:&Value,subclass:bool)->Flow<Option<bool>> {
        let Value::Class(class)=wanted else{return Ok(None);};
        let Some(maker)=Self::maker_beneath(class) else{return Ok(None);};
        let Some(name)=self.lang.class_special.get(if subclass{77}else{76}).cloned() else{return Ok(None);};
        let Some(member)=self.class_value(&maker,&name) else{return Ok(None);};
        let bound=self.bind_class_value(member,Some(wanted.clone()),maker)?;
        // A kind word asked about stands before the hook as the kind's
        // own class, the way the reference hands the type itself over.
        let given=match given {
            Value::Adapter(w) if w.0==8 => match &w.1[0] { Value::Text(word) => Value::Class(self.kind_class(word)), _ => given.clone() },
            Value::Native(_, word) if self.lang.builtins.get(word.as_ref()).copied().map_or(false,|op| Self::kind_builtin(&op)) => Value::Class(self.kind_class(word)),
            Value::ByteKind(mutable, _) => { let word=self.byte_kind_word(*mutable).to_string(); Value::Class(self.kind_class(&word)) }
            other => other.clone(),
        };
        let told=self.class_apply(bound,vec![given])?;
        Ok(Some(self.truth(&told)))
    }
    /// The builtin words that name a kind of value rather than a piece
    /// of work. Only one of these stands as a class where `issubclass`
    /// and `isinstance` ask for one; every other builtin word is as
    /// much a refusal there as a number would be.
    pub(super) fn kind_builtin(op:&Builtin)->bool { op.names_kind() }
    /// The word a value names a builtin kind by, where it names one.
    pub(super) fn kind_spelled(&self,value:&Value)->Option<Rc<str>> {
        let Value::Adapter(w)=value else{return None};
        if w.0!=8 {return None;}
        let Value::Text(word)=&w.1[0] else{return None};
        self.lang.builtins.get(word.as_ref()).filter(|op|Self::kind_builtin(op)).map(|_|word.clone())
    }
    /// Whether a value stands for a kind rather than being one of a
    /// kind: a class, a builtin word naming a kind, one of the two
    /// bytes kinds, or the value a plain sort goes by. The kind of any
    /// of these is the kind builtin itself.
    pub(super) fn stands_for_kind(&self,value:&Value)->bool {
        matches!(value,Value::Class(_)|Value::ByteKind(..)|Value::SortOf(_))
            ||matches!(value,Value::Native(op,_) if Self::kind_builtin(op))
            ||self.kind_spelled(value).is_some()
    }
    /// The operands accepted by the native union constructor: classes,
    /// existing union values and None, which denotes NoneType.
    pub(super) fn union_member(&self,value:&Value)->bool {
        matches!(value,Value::Null)
            ||self.stands_for_kind(value)
            ||matches!(value, Value::Object(o) if Self::own_kind(&o.class_now()).as_deref() == Some("Union"))
    }
    /// At least one operand must provide the union operator. None alone
    /// supplies no such operator.
    pub(super) fn union_anchor(&self,value:&Value)->bool {
        self.stands_for_kind(value)
            ||matches!(value, Value::Object(o) if Self::own_kind(&o.class_now()).as_deref() == Some("Union"))
    }
    /// The kind builtin read as a value: what the kind of a kind is.
    pub(super) fn kind_maker_word(&self)->Value {
        match self.lang.builtins.iter().find(|(_,b)|**b==Builtin::SortOf) {
            Some((word,_))=>Value::Native(Builtin::SortOf,Rc::from(word.as_str())),
            None=>Value::Null,
        }
    }
    /// The class standing for a kind the definition has no word of its
    /// own for, named as the reference names that kind. It is made once
    /// and kept, so that two askings answer with the very same class.
    pub(super) fn named_kind(&mut self,value:&Value)->Value {
        if let Value::Object(instance) = value.contents() {
            let declared = instance.class_now();
            if Self::kind_beneath(&declared).as_deref() == Some("module") { return Value::Class(declared); }
        }
        let word=if matches!(value, Value::Native(Builtin::Text(_), name) if name.contains('.')) { String::from("method_descriptor") } else if self.is_async_generator(value) { String::from("async_generator") } else { match self.module_holding(value) {Some(_)=>String::from("module"),None=>value.core_kind()} };
        // Where the definition spells that very kind, its builtin word
        // is the answer, so that a kind asked for and a kind answered
        // with are the one value: `type(enumerate(r)) is enumerate`.
        if let Some(op)=self.lang.builtins.get(&word).copied().filter(Self::kind_builtin) {
            return Value::Native(op,Rc::from(word.as_str()));
        }
        Value::Class(self.kind_class(&word))
    }
    /// One builtin kind stands beneath another only where it is that
    /// very kind, or where it is the flag kind, which stands under the
    /// whole-number kind as the language counts a flag a number.
    fn kinds_beneath(&self,under:&str,over:&str)->bool {
        under==over
            || (self.lang.builtins.get(under)==Some(&Builtin::Bool) && self.lang.builtins.get(over)==Some(&Builtin::ToInt))
    }
    /// Whether a value stands as a class at all: one the program wrote,
    /// or a builtin word that names a kind.
    fn stands_as_class(&self,value:&Value)->bool {
        matches!(value,Value::Class(_))||self.kind_spelled(value).is_some()
    }
    /// Whether a value is of a builtin kind. A walk is known by the name
    /// its kind is told by, which is the word that made it.
    pub(super) fn kind_holds(&self,op:&Builtin,word:&str,value:&Value)->bool {
        match op {
            Builtin::ToInt=>matches!(value,Value::Small(_)|Value::Huge(_)|Value::Flag(_)),
            Builtin::ToText=>matches!(value,Value::Text(_) | Value::Codepoints(_)),
            Builtin::AsReal=>matches!(value,Value::Real(_)),
            Builtin::List=>matches!(value,Value::Array(_)),
            Builtin::SortOf=>self.stands_for_kind(value),
            Builtin::Dict=>matches!(value,Value::Map(_)|Value::Fields(_)),
            Builtin::Tuple=>matches!(value,Value::Tuple(_)),
            Builtin::Set=>matches!(value,Value::Set(_))&&!value.set_fixed(),
            Builtin::Frozen=>value.set_fixed(),
            Builtin::Bool=>matches!(value,Value::Flag(_)),
            Builtin::Complex=>matches!(value,Value::Complex(_)),
            Builtin::Span=>matches!(value,Value::Counted(_)),
            Builtin::MakeSlice=>matches!(value,Value::Slice(_)),
            Builtin::Bytes(m)=>matches!(value,Value::Bytes(_,mutable,_) if *mutable==(*m==1)),
            Builtin::ClassTool(9) => matches!(value, Value::Adapter(v) if v.0 == 4),
            Builtin::ClassTool(10) => matches!(value, Value::Adapter(v) if v.0 == 5),
            Builtin::Enumerate|Builtin::Zip|Builtin::Map|Builtin::Filter|Builtin::Reversed=>value.core_kind()==word,
            _=>false,
        }
    }
    /// The refusal for a subject or a kind that is no class. Where a
    /// language has no words of its own for it, the class refusal
    /// stands, as it did before any of these were spelled.
    pub(super) fn unclassed(&self,label:&str)->Fault {
        let told=self.core_fault(label,"");
        if told.is_empty(){self.class_refusal()}else{told.into()}
    }
    /// The member a value holds under a name where it holds one: a
    /// member that is absent is the plain absence the reference reads,
    /// while any other fault is passed on as it stands. Both the
    /// class questions ask values this way.
    fn asked_member(&mut self, value:&Value, name:&str) -> Flow<Option<Value>> {
        if name.is_empty() { return Ok(None); }
        match self.class_get(value.clone(), name, false) {
            Ok(found) => Ok(Some(found.contents())),
            Err(fault) if self.attribute_fault(&fault) => Ok(None),
            Err(other) => Err(other),
        }
    }
    /// The `__bases__` a value stands under where it keeps a tuple of
    /// them, the reference's own reading of a class a value may stand
    /// for without being a class itself. A member that is absent, or
    /// is no tuple, leaves the walk with nothing.
    fn class_bases(&mut self, value:&Value) -> Flow<Option<Vec<Value>>> {
        let name = self.class_word("bases").to_string();
        Ok(match self.asked_member(value, &name)? {
            Some(Value::Tuple(items)) => Some(items.as_ref().clone()),
            _ => None,
        })
    }
    /// The class a value names through its own `__class__`, whether or
    /// not that is the class it was made by.
    fn named_class_of(&mut self, value:&Value) -> Flow<Option<Value>> {
        let name = self.class_word("kind").to_string();
        self.asked_member(value, &name)
    }
    /// Whether two values are the very one thing, which is how the
    /// reference follows a `__bases__` line: the same class, the same
    /// object, the same builtin kind under whatever spelling, or the
    /// same placed value.
    fn one_place(&self, a:&Value, b:&Value) -> bool {
        match (a, b) {
            (Value::Class(x), Value::Class(y)) => Rc::ptr_eq(x, y),
            (Value::Object(x), Value::Object(y)) => Rc::ptr_eq(x, y),
            (Value::Native(x, xw), Value::Native(y, yw)) => x == y && xw == yw,
            (Value::Null, Value::Null) => true,
            _ => match (self.kind_word_of(a), self.kind_word_of(b)) {
                (Some(one), Some(two)) => one == two,
                _ => a.same_place(b),
            },
        }
    }
    /// A value that is no class of its own but holds the question's own
    /// member answers it, as the reference asks before falling back on
    /// class lines; this is how a stub kind carries its own checking.
    fn value_answers(&mut self,wanted:&Value,given:&Value,subclass:bool)->Flow<Option<bool>> {
        let Value::Object(object)=wanted else{return Ok(None);};
        let holder=object.class_now();
        let Some(name)=self.lang.class_special.get(if subclass{77}else{76}).cloned() else{return Ok(None);};
        let Some(member)=self.class_value(&holder,&name) else{return Ok(None);};
        let bound=self.bind_class_value(member,Some(wanted.clone()),holder)?;
        let told=self.class_apply(bound,vec![given.clone()])?;
        Ok(Some(self.truth(&told)))
    }
    /// The reference's walk along a `__bases__` line from `derived`
    /// towards the very `wanted`: a single base is stepped along and
    /// does not grow the stack, two or more are each walked in turn,
    /// and the whole is guarded so an endless line raises as the
    /// reference's own does.
    fn bases_walk(&mut self, mut derived:Value, wanted:&Value) -> Flow<bool> {
        loop {
            if self.one_place(&derived, wanted) { return Ok(true); }
            let Some(bases) = self.class_bases(&derived)? else { return Ok(false); };
            if bases.is_empty() { return Ok(false); }
            if bases.len() == 1 { derived = bases.into_iter().next().unwrap(); continue; }
            self.reaching_further()?;
            let mut answer = Ok(false);
            for base in bases.iter() {
                match self.bases_walk(base.clone(), wanted) {
                    Ok(true) => { answer = Ok(true); break; }
                    Ok(false) => {}
                    Err(fault) => { answer = Err(fault); break; }
                }
            }
            self.answered();
            return answer;
        }
    }
    /// The kind word a value reports, where it reports one: a class the
    /// program wrote and the builtin kind beneath it, a builtin word, a
    /// bytes kind, or the NoneType a bare `None` stands for.
    fn reported_kind_word(&self, value:&Value) -> Option<String> {
        match value {
            Value::Class(c) => Self::own_kind(c).or_else(|| Self::kind_beneath(c)),
            Value::Native(op, word) if Self::kind_builtin(op) => Some(word.to_string()),
            Value::Adapter(parts) if parts.0 == 8 => match &parts.1[0] { Value::Text(word) => Some(word.to_string()), _ => None },
            Value::ByteKind(mutable, _) => Some(self.byte_kind_word(*mutable).to_string()),
            Value::Null => Some("NoneType".to_string()),
            _ => None,
        }
    }
    /// Whether a class a value reports through `__class__` stands
    /// beneath the kind asked after, in whatever shape each came: a
    /// class beneath another class, a builtin word, or a bytes kind.
    fn reported_stands_beneath(&self, reported:&Value, wanted:&Value) -> bool {
        if let (Value::Class(a), Value::Class(b)) = (reported, wanted) {
            if Self::contains_class(a, b) || Self::exception_beneath(a, b) { return true; }
        }
        match (self.reported_kind_word(reported), self.reported_kind_word(wanted)) {
            (Some(under), Some(over)) => self.kinds_beneath(&under, &over),
            _ => false,
        }
    }
    pub(super) fn module_of_kind(&self, value: &Value, expected: &Rc<Class>) -> bool {
        if self.module_holding(value).is_none() { return false; }
        let [path, member] = self.lang.module_kind.as_slice() else { return false };
        let Some(Value::Object(space)) = self.modules.get(path) else { return false };
        space.fields.borrow().iter().any(|(key, held)| key == member && matches!(held.contents(), Value::Class(kind) if Rc::ptr_eq(&kind, expected)))
    }
    pub(super) fn beneath(&mut self,value:&Value,wanted:&Value,subclass:bool)->Flow<bool> {
        if let Value::Collection(cell, _) | Value::Bond(cell) = value { let held = cell.borrow().clone(); return self.beneath(&held, wanted, subclass); }
        if let Value::Collection(cell, _) | Value::Bond(cell) = wanted { let held = cell.borrow().clone(); return self.beneath(value, &held, subclass); }
        // Plain numeric values cannot override __class__ or type hooks.
        // Keep the native membership rule without a repeated class walk.
        if !subclass && self.lang.bind_names {
            if let (Value::Small(_) | Value::Huge(_) | Value::Flag(_) | Value::Real(_) | Value::Frac(_),
                Value::Native(operation @ (Builtin::ToInt | Builtin::AsReal | Builtin::Bool), word)) = (value, wanted) {
                return Ok(self.kind_holds(operation, word, value));
            }
        }
        // Legacy method wrappers share the canonical public wrapper kinds.
        if !subclass && self.lang.bind_names {
            if matches!(value, Value::Adapter(slot) if slot.0 == 4) && self.names_staticmethod_class(wanted)
                || matches!(value, Value::Adapter(slot) if slot.0 == 5) && self.names_classmethod_class(wanted) {
                return Ok(true);
            }
        }
        if let Value::Object(object) = wanted {
            if Self::own_kind(&object.class_now()).as_deref() == Some("Union") {
                let members = object.fields.borrow().iter().find(|(key, _)| key == "__args__").map(|(_, row)| row.clone()).ok_or_else(|| self.class_refusal())?;
                return self.beneath(value, &members, subclass);
            }
        }
        if matches!(wanted, Value::Object(o) if o.class_now().name == "GenericAlias") {
            return Err("TypeError: isinstance() argument 2 cannot be a parameterized generic".into());
        }
        if let Some(told)=self.maker_answers(wanted,value,subclass)? { return Ok(told); }
        // A side still standing behind a cell is asked about as the
        // value the cell keeps.
        if matches!(value, Value::Bond(_) | Value::Binding(_) | Value::Collection(..)) { return self.beneath(&value.contents(), wanted, subclass); }
        if matches!(wanted, Value::Bond(_) | Value::Binding(_) | Value::Collection(..)) { return self.beneath(value, &wanted.contents(), subclass); }
        if let Some(told)=self.value_answers(wanted,value,subclass)? { return Ok(told); }
        // The bytes kinds stand as values of their own rather than as
        // builtin words, so each is asked about under its own word.
        if let Value::ByteKind(mutable, _) = value { let word=self.byte_kind_word(*mutable).to_string(); return self.beneath(&Self::adapter(8, vec![Value::text(&word)]), wanted, subclass); }
        if let Value::ByteKind(mutable, _) = wanted { let word=self.byte_kind_word(*mutable).to_string(); return self.beneath(value, &Self::adapter(8, vec![Value::text(&word)]), subclass); }
        if let Value::Native(operation, word) = value {
            if Self::kind_builtin(operation) { return self.beneath(&Self::adapter(8, vec![Value::text(word)]), wanted, subclass); }
            if self.lang.bind_names {
                if subclass { return Err(self.unclassed("core.issubclass.subject")); }
                let actual = self.named_kind(value);
                return self.beneath(&actual, wanted, true);
            }
            return self.beneath(&Self::adapter(8, vec![Value::text(word)]), wanted, subclass);
        }
        // The property and the two method wrappers name a class of
        // their own where they are asked after as one: a wrapper of
        // their kind, or a class written upon them, answers to them.
        if self.names_property_class(wanted) { let c=self.property_class(); return self.beneath(value, &Value::Class(c), subclass); }
        if self.names_classmethod_class(wanted) { let c=self.classmethod_class(); return self.beneath(value, &Value::Class(c), subclass); }
        if self.names_staticmethod_class(wanted) { let c=self.staticmethod_class(); return self.beneath(value, &Value::Class(c), subclass); }
        if let Value::Native(_, word) = wanted { return self.beneath(value, &Self::adapter(8, vec![Value::text(word)]), subclass); }
        if let Value::Tuple(v)=wanted {
            // A tuple of kinds is walked member by member, and the walk
            // is guarded so a tuple nested without end raises as the
            // reference's own recursion does rather than running on.
            let members: Vec<Value> = v.iter().cloned().collect();
            self.reaching_further()?;
            let mut answer = Ok(false);
            for member in members.iter() {
                match self.beneath(value, member, subclass) {
                    Ok(true) => { answer = Ok(true); break; }
                    Ok(false) => {}
                    Err(fault) => { answer = Err(fault); break; }
                }
            }
            self.answered();
            return answer;
        }
        // A union built by `|` carries a bare `None` for the `NoneType`
        // member, the very value `None` itself is, so a chained union
        // reads it back this way rather than needing `type(None)`.
        if matches!(wanted,Value::Null) {
            return Ok(if subclass{matches!(value,Value::SortOf(Sort::Null))}else{matches!(value,Value::Null)});
        }
        // Whatever is asked about must be a class where a class is asked
        // about, and the kind asked after must be one wherever it is
        // asked: each has its own words, as the reference has.
        let amiss=if subclass{"core.issubclass.amiss"}else{"core.isinstance.amiss"};
        if let Value::Class(c)=wanted {
            if !subclass && self.module_of_kind(value, c) { return Ok(true); }
            if subclass && !self.stands_as_class(value){
                if self.class_bases(value)?.is_some() { return self.bases_walk(value.clone(), wanted); }
                return Err(self.unclassed("core.issubclass.subject"));
            }
            // Everything stands beneath the class every other one does.
            if Rc::ptr_eq(c, &self.root_class()) {
                if self.lang.class_builder.is_empty() { return Ok(true); }
                match value {
                    Value::Class(actual) if subclass => return Ok(Self::contains_class(actual, c)),
                    Value::Object(object) if !subclass => return Ok(Self::contains_class(&object.class_now(), c)),
                    _ => return Ok(true),
                }
            }
            // The run's own module objects answer to the native module
            // kind, whatever class a module keeps for its members.
            if Self::own_kind(c).as_deref()==Some("module") && self.module_holding(value).is_some(){return Ok(true);}
            if !subclass && !matches!(value, Value::Object(_)) {
                if let Some(word)=Self::own_kind(c) { return Ok(value.core_kind()==word); }
            }
            if !subclass {
                if let Value::Class(held) = value {
                    let maker = Self::maker_beneath(held).unwrap_or_else(|| self.metaclass_root());
                    return Ok(Self::contains_class(&maker, c));
                }
            }
            let kind=match value {Value::Object(o) if !subclass=>Some(&o.class_now()),Value::Class(c) if subclass=>Some(c),_=>None};
            if kind.is_some_and(|k| (!subclass && Rc::ptr_eq(k, c)) || Self::exception_beneath(k,c)||Self::contains_class(k,c)){return Ok(true);}
            // A value not of the kind may still name a class beneath it
            // through its own `__class__`, which is read here and may
            // raise, as the reference asks it. It is the class the value
            // was made by that this second look must differ from, so a
            // proxy naming the very kind asked after is of it.
            if !subclass {
                // A module already answers by the kind it was made as;
                // asking it for its own `__class__` would walk the
                // module's reader, so only a laid-out thing is asked.
                if self.module_holding(value).is_none() {
                    if let Value::Object(thing) = value {
                        if let Some(Value::Class(held))=self.named_class_of(value)? {
                            if !Rc::ptr_eq(&held, &thing.class_now()) {
                                return Ok(Self::exception_beneath(&held,c)||Self::contains_class(&held,c));
                            }
                        }
                    }
                }
            }
            return Ok(false);
        }
        if let Value::Adapter(w)=wanted {
            if w.0==8 {if let Value::Text(word)=&w.1[0] {
                let Some(builtin)=self.lang.builtins.get(word.as_ref()).copied().filter(Self::kind_builtin) else{return Err(self.unclassed(amiss));};
                if subclass{
                    if let Value::Class(c)=value{
                        if builtin==Builtin::SortOf { return Ok(self.is_metaclass_root(c)||c.lineage.borrow().iter().any(|base|self.is_metaclass_root(base))); }
                        return Ok(Self::kind_among(c,word));
                    }
                    if let Some(under)=self.kind_spelled(value) { return Ok(self.kinds_beneath(&under,word)); }
                    // A subject that stands as no class still may stand
                    // as one through the `__bases__` it keeps.
                    if self.class_bases(value)?.is_some() { return self.bases_walk(value.clone(), wanted); }
                    return Err(self.unclassed("core.issubclass.subject"));
                }
                // A thing of a class standing on the kind is of the
                // kind; a value not of it may still name a kind through
                // its own `__class__`.
                if let Value::Object(o)=value{
                    if Self::kind_among(&o.class_now(),word) { return Ok(true); }
                    // A module already answers by the kind it was made
                    // as; its own reader is not walked for `__class__`.
                    if self.module_holding(value).is_some() { return Ok(false); }
                    if let Some(reported)=self.named_class_of(value)? {
                        if !self.one_place(&reported, &Value::Class(o.class_now())) {
                            return Ok(self.reported_stands_beneath(&reported, wanted));
                        }
                    }
                    return Ok(false);
                }
                return Ok(self.kind_holds(&builtin,word,value));
            }}
        }
        // The kind may name no class of its own and still stand as one
        // through its `__bases__`, as the reference asks before saying
        // no; and the subject may stand as one the same way.
        if self.class_bases(wanted)?.is_some() {
            if subclass {
                if self.class_bases(value)?.is_none() { return Err(self.unclassed("core.issubclass.subject")); }
                return self.bases_walk(value.clone(), wanted);
            }
            let Some(named)=self.named_class_of(value)? else { return Ok(false); };
            return self.bases_walk(named, wanted);
        }
        if subclass && !self.stands_as_class(value){return Err(self.unclassed("core.issubclass.subject"));}
        Err(self.unclassed(amiss))
    }
    /// The words for a question handed the wrong number of arguments,
    /// which name the question, the number it wants and the number it
    /// was given.
    fn arity_told(&self,name:&str,wanted:usize,given:usize)->Fault {
        let Some(words)=self.lang.core_words.get("core.arity.exact").filter(|w|w.len()>=3) else{return self.class_refusal();};
        format!("{}{}{}{}{}{}",words[0],name,words[1],wanted,words[2],given).into()
    }
    /// The word this language spells one of the class tools with, the
    /// one a question handed the wrong number of arguments names itself
    /// by.
    fn class_tool_word(&self,which:u8)->String {
        let target=match which {
            0=>Builtin::InstanceOf, 2=>Builtin::Callable, 3=>Builtin::GetAttr,
            4=>Builtin::SetAttr, 5=>Builtin::DelAttr, 6=>Builtin::HasAttr, 7=>Builtin::Vars,
            other=>Builtin::ClassTool(other),
        };
        self.lang.builtins.iter().find(|(_,b)|**b==target).map(|(n,_)|n.clone()).unwrap_or_default()
    }
    pub(super) fn class_work(&mut self,which:u8,mut args:Vec<Value>)->Flow<Value> {

        if (3..=6).contains(&which) && args.len() >= 2 {
            if let Some(Value::Text(word)) = Self::worth_of(&args[1]) { args[1] = Value::Text(word); }
        }
        if which == 13 && self.lang.class_builder.is_empty() {
            if let Value::Tuple(header) = args[4].contents() {
                let opened = self.call_items(header.to_vec())?;
                let mut bases = Vec::new(); let mut keywords = Vec::new();
                for (key, value) in opened {
                    match key {
                        None => bases.push(value),
                        Some(key) if Lang::spells(&self.lang.metaclass_word, &key) => args[2] = value,
                        Some(key) => keywords.push(Value::tuple(vec![Value::text(&key), value])),
                    }
                }
                args[1] = Value::tuple(bases); args[3] = Value::tuple(keywords);
            }
            let Value::Tuple(listed) = args[1].contents() else { return Err(self.class_refusal()); };
            let mut bases = Vec::new();
            for value in listed.iter() { if let Ok(base) = self.type_base(value) { bases.push(base); } }
            let asked = (!matches!(args[2], Value::Null)).then(|| args[2].clone());
            if let Some(maker) = self.maker_in_force(asked, &bases).ok().flatten() {
                let target = Value::Class(maker);
                let preparation = self.class_word("prepare").to_string();
                match self.class_get(target, &preparation, false) {
                    Ok(hook) => {
                        let mut supplied = vec![args[0].clone(), args[1].clone()];
                        if let Value::Tuple(options) = args[3].contents() {
                            for option in options.iter() {
                                if let Value::Tuple(pair) = option.contents() { supplied.push(Value::Tie(Rc::new((pair[0].clone(), pair[1].clone())))); }
                            }
                        }
                        return self.class_apply(hook, supplied);
                    },
                    Err(fault) if self.attribute_fault(&fault) => {},
                    Err(fault) => return Err(fault),
                }
            }
            return Ok(Value::Collection(Rc::new(RefCell::new(Value::Map(Rc::new(Vec::new().into())))), true));
        }
        let one=args.first().cloned().unwrap_or(Value::Null);
        if matches!(which,3..=6) && args.len()>=2 {
            if let (Value::Object(instance),Value::Codepoints(_))=(&one,&args[1]) {
                let mut entries=self.fields_entries(instance);
                let at=entries.iter().position(|(key,_)|key.equals(&args[1]));
                if which==3 || which==6 {
                    if which==6{return Ok(Value::Flag(at.is_some()))}
                    if let Some(index)=at{return Ok(entries[index].1.clone())}
                    if let Some(default)=args.get(2){return Ok(default.clone())}
                } else if which==4 && args.len()==3 {
                    if let Some(index)=at{entries[index].1=args[2].clone()}else{entries.push((args[1].clone(),args[2].clone()))}
                    self.fields_restore(instance,entries);return Ok(Value::Null);
                } else if which==5 {if let Some(index)=at{entries.remove(index);self.fields_restore(instance,entries);return Ok(Value::Null)}}
            }
        }
        match which {
            13 => self.build_body_class(args),
            14 => self.require_class_builder(),
            15 => self.class_body_book(one),
            16 => self.dispatch_class_builder(args),
            18 => self.class_namespace_read(args),
            20 => self.class_namespace_remove(args),
            26 => {
                let entries = self.call_items(args)?;
                if entries.iter().any(|(name, _)| name.is_some()) { return Err("TypeError: sys._clear_type_descriptors() takes no keyword arguments".into()); }
                let args: Vec<Value> = entries.into_iter().map(|(_, value)| value).collect();
                if args.len() != 1 { return Err(format!("TypeError: sys._clear_type_descriptors() takes exactly one argument ({} given)", args.len()).into()); }
                let class = match args[0].contents() {
                    Value::Class(class) => class,
                    value if self.stands_for_kind(&value) => return Err("TypeError: argument is immutable".into()),
                    value => return Err(format!("TypeError: _clear_type_descriptors() argument must be type, not {}", value.core_kind()).into()),
                };
                if class.python_names.borrow().is_none() || class.sealed.get() { return Err("TypeError: argument is immutable".into()); }
                class.shared.borrow_mut().retain(|(key, _)| key != "__dict__" && key != "__weakref__");
                // The kind no longer carries a weak reference where it
                // stands, so a class built over it may name one itself.
                class.weak_storage.set(Some(false));
                Ok(Value::Null)
            }
            24 | 25 if args.len() == 1 => {
                let evaluator = Self::adapter(44, vec![args.remove(0).contents()]);
                if which == 25 { self.class_apply(evaluator, vec![Value::Small(1)]) } else { Ok(evaluator) }
            }
            23 => {
                let entries = self.call_items(args)?;
                if entries.iter().any(|(key, _)| key.is_some()) { return Err("TypeError: _typing._idfunc() takes no keyword arguments".into()); }
                if entries.len() != 1 { return Err(format!("TypeError: _typing._idfunc() takes exactly one argument ({} given)", entries.len()).into()); }
                Ok(entries.into_iter().next().unwrap().1)
            }
            21 if args.is_empty() => Ok(Value::Class(self.kind_class("SimpleNamespace"))),
            12 if args.len() == 1 => Ok(Value::Flag(match &one {
                Value::Object(object) => self.slots_allow(&object.class_now(), self.class_word("namespace"))
                    && Self::kind_beneath(&object.class_now()).is_none()
                    && !object.fields.borrow().iter().any(|(key, _)| key == "\0namespace" || key == "\0expanded"),
                _ => false,
            })),
            0|1 if args.len()==2=>Ok(Value::Flag(self.beneath(&one,&args[1],which==1)?)),
            // Both questions want two arguments and name themselves
            // where they are handed another number of them.
            0|1=>Err(self.arity_told(&self.class_tool_word(which),2,args.len())),
            2 if args.len()==1=>Ok(Value::Flag(matches!(one,Value::Class(_)|Value::Routine(_)|Value::Method(..)|Value::Native(..)|Value::ByteKind(..)|Value::ValueMethod(_)|Value::TextMethod(..))||matches!(&one,Value::Adapter(w) if matches!(w.0,0..=4|8..=12|14|15|17..=27|29|30|40..=48|77..=80|131|63|64|180|132|133|236))||matches!(&one,Value::Object(o) if self.class_value(&o.class_now(),self.class_word("call")).is_some()))),
            // getattr and hasattr want the receiver and a name, and take
            // a name of any kind but a string only to say so.
            3|6 if args.len()>=2=>{
                // A name standing on text is asked after as the text it keeps.
                let asked=match &args[1] {Value::Object(_)=>Self::worth_of(&args[1]).map(|w|w.contents()).filter(|w|matches!(w,Value::Text(_))),_=>None}.unwrap_or_else(||args[1].clone());
// A text keeping a lone surrogate names a member too;
                // no member's name keeps one, so it is asked after as
                // the stand-in text such a row reads as elsewhere.
                let asked=match &asked {Value::Codepoints(row)=>Value::text(&Value::predicate_text(row)),other=>other.clone()};
                let Value::Text(name)=&asked else{return Err(self.core_fault("core.attribute.name",&args[1].core_kind()).into());};match self.class_get(one.clone(),name,false){Ok(v)=>Ok(if which==6{Value::Flag(true)}else if self.lang.syntax_members.is_empty(){match v{Value::Bond(cell)=>cell.borrow().clone(),held=>held}}else{let keep=match &v{Value::Bond(cell)=>matches!(&*cell.borrow(),Value::Array(_)|Value::Set(_)|Value::SetWalk(..)|Value::Map(_)|Value::Bytes(..)),_=>false};if keep{v}else{match v{Value::Bond(cell)=>cell.borrow().clone(),held=>held}}}),Err(fault) if self.attribute_fault(&fault)=>if which==6{self.absent_member = None; Ok(Value::Flag(false))}else if args.len()==3{self.absent_member = None; Ok(args[2].clone())}else{
                // A module asked by name for a member it has not may answer through its own routine, as it does for a member read in the program.
                if let Value::Object(o)=&args[0]{if let Some(routine)=self.module_reader(o){self.invoke(&routine,vec![Value::text(name)]).map_err(|failure| self.attribute_from_hook(failure, &one, name))?;return self.drop_top().map_err(Fault::Note);}}
                Err(self.attribute_from_hook(fault, &one, name))},Err(e)=>Err(self.attribute_from_hook(e, &one, name))}},
            3=>Err(self.arity_told(&self.class_tool_word(3),2,args.len())),
            6=>Err(self.arity_told(&self.class_tool_word(6),2,args.len())),
            4|5 if args.len()==if which==4{3}else{2}=>{let written=match &args[1]{Value::Codepoints(row)=>Value::text(&Value::predicate_text(row)),other=>other.clone()};let Value::Text(n)=&written else{return Err(self.core_fault("core.attribute.name",&args[1].core_kind()).into());};self.class_write(one,n,args.get(2).cloned(),false)},
            4|5=>Err(self.arity_told(&self.class_tool_word(which),if which==4{3}else{2},args.len())),
            7 if args.len()==1=>{
                let word=self.class_word("namespace").to_string();
                match self.class_get(one,&word,false) {
                    Err(fault) if self.attribute_fault(&fault) => Err(self.core_fault("core.vars", "").into()),
                    result => result,
                }
            },
            8 if args.len()==1=>{
                if let Value::Class(class) = &one {
                    if let Some(maker)=Self::maker_beneath(class) {
                        if let Some(word)=self.lang.class_special.get(75).cloned() {
                            if let Some(method)=self.class_value(&maker,&word) {
                                let bound=self.bind_class_value(method,Some(one.clone()),maker)?;
                                let answer=self.class_apply(bound,Vec::new())?;
                                let names=self.special_items(&answer).map_err(Fault::Note)?;
                                let ordered=self.steady_order(names,&Value::Null,false).map_err(Fault::Note)?;
                                return Ok(Value::array(ordered));
                            }
                        }
                    }
                }
                if let Value::Object(module) = &one {
                    let class=module.class_now();
                    let module_kind=Self::kind_beneath(&class).as_deref() == Some("module") || class.name == "ModuleType" || class.lineage.borrow().iter().any(|base| base.name == "ModuleType");
                    let class_directory=self.lang.class_special.get(75)
                        .and_then(|word| std::iter::once(class.as_ref()).chain(class.lineage.borrow().iter().map(Rc::as_ref))
                            .find_map(|base| Self::own_class_value(base, word))).is_some();
                    if (module_kind || self.module_holding(&one).is_some()) && !class_directory {
                        let namespace_name = self.class_word("namespace").to_string();
                        let namespace = self.class_get(one.clone(), &namespace_name, false)?;
                        let stored = Self::worth_of(&namespace).unwrap_or_else(|| namespace.clone()).contents();
                        let entries = match stored {
                            Value::Map(rows) => rows.iter().cloned().collect::<Vec<_>>(),
                            Value::Fields(owner) => self.fields_entries(&owner),
                            _ => return Err("TypeError: <module>.__dict__ is not a dictionary".into()),
                        };
                        if let Some(word)=self.lang.class_special.get(75) {
                            if let Some((_,method))=entries.iter().find(|(key,_)| key.plain()==*word) {
                                let answer=self.class_apply(method.clone(),Vec::new())?;
                                let names=self.special_items(&answer).map_err(Fault::Note)?;
                                let ordered=self.steady_order(names,&Value::Null,false).map_err(Fault::Note)?;
                                return Ok(Value::array(ordered));
                            }
                        }
                        let names = entries.into_iter().map(|(key, _)| key).collect();
                        let ordered = self.steady_order(names, &Value::Null, false).map_err(Fault::Note)?;
                        return Ok(Value::array(ordered));
                    }
                }
                // A thing with a directory method of its own answers with
                // it, and the names it gives are put in order.
                if matches!(&one,Value::Object(_)) {
                    if let Some(answer)=self.special_call(&one,75,vec![]).map_err(Fault::Note)? {
                        let names=self.special_items(&answer).map_err(Fault::Note)?;
                        let ordered=self.steady_order(names,&Value::Null,false).map_err(Fault::Note)?;
                        return Ok(Value::array(ordered));
                    }
                }
                self.default_directory(&one)
            }
            8=>Err(self.arity_told(&self.class_tool_word(8),1,args.len())),
            // A property is a thing of the property class, where the
            // definition spells the protocol; else the older wrapper.
            11 if !self.class_word("descriptor.get").is_empty()=>{let class=self.property_class();self.class_make(class,args)},
            9..=11 if !args.is_empty()=>Ok(Self::adapter(which-5,args)),
            _=>Err(self.class_refusal()),
        }
    }
    /// The default directory is shared by the builtin and the root
    /// method. Custom directory answers are handled before this path.
    pub(super) fn default_directory(&mut self, one: &Value) -> Flow<Value> {
        // A routine lists the members it can honestly answer
        // for, alongside any it was given of its own.
        if let Value::Adapter(bound) = one {
            if bound.0 == 3 && one.core_kind() == "method" { return self.default_directory(&bound.1[0]); }
        }
        if let Value::Routine(_)|Value::Method(..)=one {
            let mut names:Vec<String>=vec![];
            for part in ["name","qualified","doc","module","defaults","call"] {
                let word=self.class_word(part);
                if !word.is_empty() { names.push(word.to_string()); }
            }
            let routine=match one {Value::Method(_, f, _)=>Value::Routine(f.clone()),other=>other.clone()};
            names.extend(self.routine_member_names(&routine));
            names.sort();names.dedup();
            return Ok(Value::array(names.iter().map(|n|Value::text(n)).collect()));
        }
                if matches!(one, Value::Adapter(parts) if parts.0 == 32) {
                    let mut names = Vec::new();
                    names.extend(self.lang.async_generator_methods.iter().skip(3).cloned());
                    names.extend([15, 16].iter().filter_map(|index| self.lang.class_special.get(*index).cloned()));
                    for words in [&self.lang.yield_send, &self.lang.yield_throw, &self.lang.yield_close] {
                        names.extend(words.iter().cloned());
                    }
                    names.sort(); names.dedup();
                    return Ok(Value::array(names.iter().map(|name| Value::text(name)).collect()));
                }
        // A walk over a routine's own body lists the walking
        // pair it answers to (through the family below) and the
        // few names a language gives it for stepping it by hand.
        if let Value::Generator(_)=one {
            let mut names=self.kind_member_names(one);
                    if self.is_async_generator(one) {
                        names.extend(self.lang.async_generator_methods.iter().take(3).cloned());
                        names.extend(self.lang.async_generator_fields.iter().cloned());
                        names.sort(); names.dedup();
                        return Ok(Value::array(names.iter().map(|n|Value::text(n)).collect()));
                    }
            for words in [&self.lang.yield_close,&self.lang.yield_send,&self.lang.yield_throw,&self.lang.yield_running] {
                if let Some(w)=words.first() { names.push(w.clone()); }
            }
            for word in [14, 15, 21, 24].iter().filter_map(|i| self.lang.trace_fields.get(*i).cloned()).filter(|word| !word.is_empty()) {
                if !word.is_empty() { names.push(word); }
            }
            names.sort();names.dedup();
            return Ok(Value::array(names.iter().map(|n|Value::text(n)).collect()));
        }
        // A builtin kind, named as the kind itself or held as a
        // value of one, answers the members a value of that kind
        // has: the methods of the kind and the special names its
        // family answers to.
        if let Some(sample)=self.dir_sample(one) {
            let names=self.kind_member_names(&sample);
            return Ok(Value::array(names.iter().map(|n|Value::text(n)).collect()));
        }
        let mut names = Vec::new();
        let class = match one {
            Value::Class(class) => Some(class.clone()),
            Value::Object(object) => {
                names.extend(object.fields.borrow().iter().filter(|(key, _)| !key.starts_with(['\0', '#'])).map(|(key, _)| key.clone()));
                if self.lang.bool_result.is_none() { Some(object.class_now()) } else {
                let spelling = self.class_word("kind").to_string();
                match self.class_get(one.clone(), &spelling, false) {
                    Ok(reported) => self.super_class_of(Some(reported)),
                    Err(error) if self.attribute_fault(&error) => None,
                    Err(error) => return Err(error),
                }
                }
            }
            _ => None,
        };
        if let Some(c)=&class {
            for b in Self::class_order(c).iter() {
                names.extend(b.shared.borrow().iter().filter(|(n,_)|!n.starts_with(['\0', '#'])).map(|(n,_)|n.clone()));
                if let Some(sample) = Self::own_kind(b).and_then(|word| self.kind_sample(&word)) { names.extend(self.kind_member_names(&sample)); }
            }
        }
        else {names.extend(self.routine_member_names(one));}
        if let Some(class) = &class {
            for part in ["kind", "get", "set", "remove", "allocate", "subclass"] {
                let name = self.class_word(part); if !name.is_empty() { names.push(name.to_string()); }
            }
            if let Some(name) = &self.lang.constructor { names.push(name.clone()); }
            if self.slots_allow(class, self.class_word("namespace")) { names.push(self.class_word("namespace").to_string()); }
            if !self.lang.weak_refused.is_empty() && self.weak_layout(class) { names.push("__weakref__".into()); }
            names.extend(self.lang.class_details.get("root.members").into_iter().flatten().cloned());
            names.extend(self.lang.class_special.get(72).cloned());
        }
        names.sort();names.dedup();Ok(Value::array(names.iter().map(|n|Value::text(n)).collect()))
    }

    /// The value whose kind a directory should describe: an empty value
    /// of the kind a builtin kind word names, or the value itself where
    /// it is one of a builtin kind. Nothing for a class or a thing of
    /// one, which answer with their own members instead.
    fn dir_sample(&self,value:&Value)->Option<Value> {
        if let Value::Native(_,word)=value { return self.kind_sample(word); }
        if let Value::ByteKind(mutable,_)=value { let word=self.byte_kind_word(*mutable).to_string(); return self.kind_sample(&word); }
        if let Value::Class(c)=value { return Self::own_kind(c).and_then(|word|self.kind_sample(&word)); }
        if matches!(value,Value::Object(_)) { return None; }
        if self.kind_member_names(value).is_empty() { return None; }
        Some(value.clone())
    }

    pub(super) fn class_super(&mut self,subject:Value,owner:&str,name:&str,args:Vec<Value>)->Flow<Value> {
        let receiver=match &subject{Value::Object(o)=>o.class_now().clone(),Value::Class(c)=>c.clone(),_=>return Err(self.class_refusal())};
        let owned=|a:&Self,c:&Rc<Class>|c.name==owner || c.python_names.borrow().as_ref().map(|names| names.3.clone()).or_else(|| Self::own_class_value(c,a.class_word("qualified"))).map_or(false,|v| (if c.python_names.borrow().is_some() { v.type_text() } else { Self::worth_of(&v).unwrap_or(v) }).plain()==owner);
        let mut sequence=Self::class_order(&receiver);
        let mut at=sequence.iter().position(|c|owned(self,c));
        // A method of a metaclass is written in the metaclass, not in the
        // class it was given, so its forebears are the metaclass's own.
        if at.is_none() {
            if let Some(maker)=Self::maker_beneath(&receiver) {
                sequence=Self::class_order(&maker);
                at=sequence.iter().position(|c|owned(self,c));
            }
        }
        let at=at.ok_or_else(||self.class_refusal())?;
        for c in sequence.iter().skip(at+1) {
            // The class every metaclass stands on: it lays a class out
            // and makes a thing of one, as the kind builtin plainly does.
            if self.is_metaclass_root(c) {
                // The kind builtin's own making: a name, the bases and
                // a namespace become a class, remembering the metaclass
                // it was handed as the one that made it.
                if self.lang.constructor.as_deref() == Some(name) {
                    let mut values = vec![subject.clone()]; values.extend(args);
                    return self.type_initialiser(values);
                }
                if name==self.class_word("allocate") { return self.class_from_parts(args); }
                if name==self.class_word("call") {
                    let Value::Class(made)=&subject else{return Err(self.class_refusal())};
                    // What was spread is opened first, so that a class
                    // taking nothing is not handed an empty spread.
                    let opened=self.call_items(args)?.into_iter()
                        .map(|(key,v)|match key{Some(k)=>Value::Tie(Rc::new((Value::text(&k),v))),None=>v}).collect();
                    return self.class_construct(made.clone(),opened);
                }
                continue;
            }
            // The kind a class stands on makes the thing, takes its
            // constructing in silence, and answers its kind's methods
            // through the worth the thing keeps.
            if let Some(word)=Self::own_kind(c) {
                if name==self.class_word("allocate"){return self.class_apply(Self::adapter(14,vec![Value::text(&word)]),args);}
                if word == "module" && self.lang.class_special.get(1).is_some_and(|key| key == name) { return self.module_repr_value(subject); }
                if self.lang.constructor.as_deref()==Some(name){
                    if word == "module" { return self.initialise_module(subject, args); }
                    if let Some(worth) = Self::worth_of(&subject).filter(|held| matches!(held.contents(), Value::Set(_) | Value::Array(_) | Value::Map(_))) {
                        let mut given = Vec::new(); let mut named = Vec::new();
                        for (key, value) in self.call_items(args)? { match key { Some(key) => named.push((key, value)), None => given.push(value) } }
                        return Ok(self.value_method(&worth, name, given, named)?);
                    }
                    return Ok(Value::Null);
                }
                // A value working is taken only when the worth the
                // thing keeps answers to it: `__hash__`, for one, names
                // a working a slice answers and a text does not, so a
                // text asked through its base falls to the kind's own
                // member below rather than to a working it refuses.
                if let (Some(worth),Some(op))=(Self::worth_of(&subject),self.lang.value_methods.get(name).cloned()) {
                    if crate::methods::answered(&worth,&op) {
                        let mut positional=Vec::new();let mut named=Vec::new();
                        for (key,v) in self.call_items(args)? {match key{Some(k)=>named.push((k,v)),None=>positional.push(v)}}
                        return Ok(self.value_method(&worth,&op,positional,named)?);
                    }
                }
                // A member the kind carries that no value working goes
                // by, such as `__hash__` or `__eq__`, is the kind's own
                // and is worked upon the worth the thing keeps -- the
                // very reading `super().__hash__()` asks of the base.
                if let Some(worth)=Self::worth_of(&subject) {
                    if let Some(Value::ValueMethod(method))=self.builtin_member(&worth,name)? {
                        let mut positional=Vec::new();let mut named=Vec::new();
                        for (key,v) in self.call_items(args)? {match key{Some(k)=>named.push((k,v)),None=>positional.push(v)}}
                        return Ok(self.value_method(&method.0,&method.1,positional,named)?);
                    }
                }
                if let Some(member) = Self::own_class_value(c, name) {
                    let bound = self.bind_class_value(member, Some(subject.clone()), receiver.clone())?;
                    return self.class_apply(bound, args);
                }
                continue;
            }
            if let Some(member) = Self::own_class_value(c, name) {
                let bound = if name == self.class_word("allocate") { member }
                    else { self.bind_class_value(member, Some(subject.clone()), receiver.clone())? };
                return self.class_apply(bound, args);
            }
            if self.exception_class(c) && self.class_word("allocate") == name {
                let Some((Value::Class(cls), rest)) = args.split_first() else { return Err(self.class_refusal()) };
                return self.native_exception_new(c.clone(), cls.clone(), rest.to_vec());
            }
            if self.exception_class(c) && self.lang.constructor.as_deref() == Some(name) {
                if let Value::Object(o) = &subject {
                    let opened = self.call_items(args)?.into_iter().map(|(key, value)| match key { Some(key) => Value::Tie(Rc::new((Value::text(&key), value))), None => value }).collect::<Vec<_>>();
                    return self.exception_method(o.clone(), name, &opened);
                }
            }
            // An exception's making, asked of a base, is the root's making.
            if self.exception_class(c) && name == self.class_word("allocate") {
                return self.class_apply(Self::adapter(1, Vec::new()), args);
            }
            if c.name==self.class_word("root") && name==self.class_word("allocate"){let allocator=self.class_get(Value::Class(c.clone()),name,true)?;return self.class_apply(allocator,args);}
            if c.name==self.class_word("root") {
                let f=self.class_get(Value::Class(c.clone()),name,true)?;
                let mut all=vec![subject.clone()];all.extend(args);return self.class_apply(f,all);
            }
        }
        Err(self.missing_member(&subject,name))
    }
}

impl<'a> Engine<'a> {
    fn namespace_work(&mut self, mode: i64, args: Vec<Value>) -> Flow<Value> {
        let mut opened = self.call_items(args)?;
        if opened.is_empty() { return Err(self.class_refusal()); }
        let (_, receiver) = opened.remove(0);
        if mode == 0 {
            let Value::Class(class) = receiver else { return Err(self.class_refusal()); };
            if Self::kind_beneath(&class).as_deref() != Some("SimpleNamespace") { return Err(self.class_refusal()); }
            self.made += 1;
            return Ok(Value::Object(Rc::new(Instance { replacement_class: RefCell::new(None), class,
                fields: RefCell::new(Vec::new()), mark: self.made })));
        }
        let Value::Object(object) = &receiver else { return Err(self.class_refusal()); };
        if Self::kind_beneath(&object.class_now()).as_deref() != Some("SimpleNamespace") { return Err(self.class_refusal()); }
        if mode == 1 {
            let positional: Vec<_> = opened.iter().filter(|(key, _)| key.is_none()).map(|(_, v)| v.clone()).collect();
            if positional.len() > 1 { return Err(format!("TypeError: {} expected at most 1 argument, got {}", object.class_now().name, positional.len()).into()); }
            let mut entries = Vec::new();
            if let Some(mapping) = positional.first() {
                let mut given = vec![mapping.clone()];
                let dict = self.builtin(Builtin::Dict, "dict", &mut given)?;
                let Value::Map(pairs) = dict.contents() else { return Err(self.class_refusal()); };
                for (key, held) in pairs.iter() {
                    let Value::Text(key) = key else { return Err("TypeError: keywords must be strings".into()); };
                    entries.push((key.to_string(), held.clone()));
                }
            }
            entries.extend(opened.into_iter().filter_map(|(key, held)| key.map(|key| (key, held))));
            for (key, held) in entries { self.class_write(receiver.clone(), &key, Some(held), true)?; }
            return Ok(Value::Null);
        }
        let dictionary = self.class_get(receiver.clone(), self.class_word("namespace").to_string().as_str(), true)?;
        if mode == 2 {
            if !opened.is_empty() { return Err(self.class_refusal()); }
            let shown = if Self::own_kind(&object.class_now()).is_some() { "namespace".to_string() } else { object.class_now().name.clone() };
            // Repr's recursion guard belongs to this object and is hidden from its namespace.
            if object.fields.borrow().iter().any(|(key, _)| key == "\0namespace-repr") { return Ok(Value::text(&format!("{shown}(...)"))); }
            object.fields.borrow_mut().push(("\0namespace-repr".to_string(), Value::Null));
            let result = (|| {
                let mut fragments = Vec::new();
                for key in self.comprehension_items(&dictionary)? {
                    if let Value::Text(name) = key {
                        if !name.is_empty() && !name.starts_with('\0') {
                            let held = self.class_get(receiver.clone(), &name, true)?;
                            let mut values = vec![held];
                            let rendered = self.builtin(Builtin::Repr, "repr", &mut values)?;
                            fragments.push(format!("{name}={}", rendered.plain()));
                        }
                    }
                }
                Ok(Value::text(&format!("{shown}({})", fragments.join(", "))))
            })();
            object.fields.borrow_mut().retain(|(key, _)| key != "\0namespace-repr");
            return result;
        }
        if mode == 3 || mode == 4 {
            if opened.len() != 1 { return Err(self.class_refusal()); }
            let other = &opened[0].1;
            let Value::Object(right) = other.contents() else { return Ok(Value::Declined(Rc::from("NotImplemented"))); };
            if Self::kind_beneath(&right.class_now()).as_deref() != Some("SimpleNamespace") { return Ok(Value::Declined(Rc::from("NotImplemented"))); }
            let right_dict = self.class_get(Value::Object(right), &self.class_word("namespace").to_string(), true)?;
            let left_map = dictionary.contents(); let right_map = right_dict.contents();
            let compared = self.special_dyad(&Action::Eq, &left_map, &right_map)?;
            return Ok(Value::Flag(self.truth(&compared) == (mode == 3)));
        }
        if mode == 5 {
            if !opened.is_empty() { return Err(self.class_refusal()); }
            return Ok(Value::tuple(vec![Value::Class(object.class_now().clone()), Value::tuple(Vec::new()), dictionary]));
        }
        if mode == 6 {
            if opened.iter().any(|(key, _)| key.is_none()) { return Err("TypeError: __replace__() takes no positional arguments".into()); }
            let made = self.class_make(object.class_now().clone(), Vec::new())?;
            let Value::Object(result) = &made else { return Err(self.class_refusal()); };
            if Self::kind_beneath(&result.class_now()).as_deref() != Some("SimpleNamespace") { return Err(self.class_refusal()); }
            for key in self.comprehension_items(&dictionary)? {
                if let Value::Text(key) = key { let held = self.class_get(receiver.clone(), &key, true)?; self.class_write(made.clone(), &key, Some(held), true)?; }
            }
            for (key, held) in opened { self.class_write(made.clone(), &key.unwrap(), Some(held), true)?; }
            return Ok(made);
        }
        Err(self.class_refusal())
    }
}

impl<'a> Engine<'a> {
    fn alias_work(&mut self, operation: i64, mut given: Vec<Value>) -> Flow<Value> {
        if given.is_empty() { return Err(self.class_refusal()); }
        let receiver = given.remove(0);
        if operation == 0 {
            let Value::Class(class) = receiver else { return Err(self.class_refusal()); };
            if given.len() != 2 { return Err("TypeError: GenericAlias expected 2 arguments".into()); }
            let args = match given[1].contents() { Value::Tuple(_) => given[1].clone(), value => Value::tuple(vec![value]) };
            let mut parameters: Vec<Value> = Vec::new();
            for argument in self.comprehension_items(&args)? {
                let found = if matches!(argument.contents(), Value::Object(ref object) if matches!(Self::own_kind(&object.class_now()).as_deref(), Some("TypeVar" | "ParamSpec" | "TypeVarTuple"))) {
                    vec![argument]
                } else {
                    match self.class_get(argument, "__parameters__", false) {
                        Ok(found) => self.comprehension_items(&found)?,
                        Err(fault) if self.attribute_fault(&fault) => Vec::new(),
                        Err(fault) => return Err(fault),
                    }
                };
                for parameter in found { if !parameters.iter().any(|kept| kept.identical(&parameter)) { parameters.push(parameter); } }
            }
            self.made += 1;
            return Ok(Value::Object(Rc::new(Instance { replacement_class: RefCell::new(None), class,
                fields: RefCell::new(vec![("__origin__".into(), given[0].clone()), ("__args__".into(), args),
                    ("__unpacked__".into(), Value::Flag(false)), ("__parameters__".into(), Value::tuple(parameters))]), mark: self.made })));
        }
        let origin = self.class_get(receiver.clone(), "__origin__", true)?;
        let args = self.class_get(receiver.clone(), "__args__", true)?;
        match operation {
            1 => {
                let mut parts = Vec::new();
                for arg in self.comprehension_items(&args)? {
                    parts.push(if matches!(arg, Value::Null) { "None".into() }
                        else if let Some(word) = arg.kind_it_names() { word }
                        else { self.builtin(Builtin::Repr, "repr", &mut vec![arg])?.plain() });
                }
                let star = self.class_get(receiver.clone(), "__unpacked__", true)?;
                Ok(Value::text(&format!("{}{}[{}]", if matches!(star, Value::Flag(true)) { "*" } else { "" }, origin.kind_it_names().unwrap_or_else(|| origin.plain()), parts.join(", "))))
            }
            5 if given.is_empty() => {
                let class = self.kind_class("GenericAlias");
                let alias = self.class_make(class, vec![origin, args])?;
                if let Value::Object(object) = &alias {
                    for (key, value) in object.fields.borrow_mut().iter_mut() { if key == "__unpacked__" { *value = Value::Flag(true); } }
                }
                Ok(Self::core_cursor_walked(crate::value::CursorSource::Items(Rc::new(vec![alias]).into(), 0), Some(Rc::from("generic_alias_iterator"))))
            }
            2 => self.class_apply(origin, given),
            3 if given.len() == 1 => Ok(Value::tuple(vec![origin])),
            4 if given.len() == 1 => {
                if !matches!(given[0].contents(), Value::Object(o) if Self::kind_beneath(&o.class_now()).as_deref() == Some("GenericAlias")) { return Ok(Value::Declined(Rc::from("NotImplemented"))); }
                let other_origin = self.class_get(given[0].clone(), "__origin__", true)?;
                let other_args = self.class_get(given[0].clone(), "__args__", true)?;
                let same_origin = self.special_dyad(&Action::Eq, &origin, &other_origin)?;
                let same_args = self.special_dyad(&Action::Eq, &args, &other_args)?;
                Ok(Value::Flag(self.truth(&same_origin) && self.truth(&same_args)))
            }
            _ => Err(self.class_refusal()),
        }
    }
}

impl<'a> Engine<'a> {
    pub(super) fn join_types(&mut self, left: &Value, right: &Value) -> Value {
        let mut members = Vec::new();
        for given in [left, right] {
            let entries = match given {
                Value::Object(o) if Self::own_kind(&o.class_now()).as_deref() == Some("Union") => o.fields.borrow().iter()
                    .find(|(key, _)| key == "__args__").and_then(|(_, v)| match v { Value::Tuple(row) => Some(row.to_vec()), _ => None }).unwrap_or_default(),
                Value::Null => vec![Value::Class(self.kind_class("NoneType"))],
                other => vec![other.clone()],
            };
            for entry in entries { if !members.iter().any(|held: &Value| self.one_place(held, &entry)) { members.push(entry); } }
        }
        if members.len() == 1 { return members.remove(0); }
        let class = self.kind_class("Union");
        self.made += 1;
        Value::Object(Rc::new(Instance { replacement_class: RefCell::new(None), class, mark: self.made,
            fields: RefCell::new(vec![("__args__".into(), Value::tuple(members))]) }))
    }
}

impl<'a> Engine<'a> {
    /// Whether the class is the parent class or stands beneath it: the
    /// kind word it or one of its forebears was made with is the parent
    /// class's own.
    pub(super) fn super_descended(c: &Rc<Class>) -> bool {
        std::iter::once(c).chain(c.lineage.borrow().iter()).any(|base| Self::own_kind(base).as_deref() == Some("super"))
    }

    /// The member a parent reading finds: the lineage of the class the
    /// thing answers as, walked from just after the class given, each
    /// member found bound to the thing the reading stands on. Nothing
    /// where the walk finds no member, so the caller's own reading of
    /// the parent thing answers instead.
    fn super_walk(&mut self, owner: &Rc<Class>, receiver: &Value, dynamic: Rc<Class>, name: &str) -> Flow<Option<Value>> {
        // A thing standing on the very class it answers as reads the
        // member loose, the reading that class itself takes; anything
        // else -- a class its maker answers for among them -- reads it
        // bound to the thing.
        let loose = matches!(&receiver, Value::Class(c) if Rc::ptr_eq(c, &dynamic));
        let order = Self::class_order(&dynamic);
        let Some(start) = order.iter().position(|class| Self::super_same_class(class, owner)) else { return Ok(None) };
        let root_class = self.root_class();
        for class in &order[start + 1..] {
            if name == self.class_word("allocate") && Self::own_class_value(class, name).is_none() {
                if let Some(word) = Self::own_kind(class) {
                    if word != self.class_word("root") { return Ok(Some(Self::adapter(14, vec![Value::text(&word)]))); }
                }
            }
            if let Some(value) = Self::own_class_value(class, name) {
                // The subclass hook is a class method: it binds to the
                // class the thing answers as, loose or not.
                let subject = if name == self.class_word("subclass") && loose { Some(Value::Class(dynamic.clone())) }
                    else if name == self.class_word("allocate") || loose { None } else { Some(receiver.clone()) };
                return Ok(Some(self.bind_class_value(value, subject, dynamic)?));
            }
            // Root descriptors belong here only if object is in this suffix.
            if Rc::ptr_eq(class, &root_class) {
                if let Some(root) = self.root_member(name, Some(&dynamic)) {
                    let bound = if name == self.class_word("subclass") { Some(Value::Class(dynamic.clone())) }
                        else if name == self.class_word("allocate") || loose { None }
                        else { Some(receiver.clone()) };
                    return Ok(Some(match bound {
                        Some(subject) => Self::adapter(3, vec![root, subject]),
                        None => root,
                    }));
                }
                continue;
            }
            if Self::own_kind(class).is_some() || self.exception_class(class) || self.is_metaclass_root(class) {
                let constructing = self.lang.constructor.as_deref() == Some(name) || name == self.class_word("allocate");
                let native_method = match Self::worth_of(receiver) {
                    Some(worth) => matches!(self.builtin_member(&worth, name)?, Some(Value::ValueMethod(_))),
                    None => false,
                };
                if constructing || native_method {
                    return Ok(Some(Self::adapter(44, vec![receiver.clone(), Value::text(&owner.name), Value::text(name)])));
                }
            }
            if Self::own_class_value(class, self.class_word("module")).is_none() {
                let base_value = Self::own_kind(class).and_then(|word| self.spelled_kind(&word)).unwrap_or_else(|| Value::Class(class.clone()));
                match self.class_get(base_value, name, false) {
                    Ok(value) => {
                        // A read on the class itself binds by what the
                        // member is: the root's hooks bind to the class
                        // the thing answers as, everything else stands
                        // as the class's own reading of it stands.
                        if loose {
                            if matches!(&value, Value::Adapter(payload) if payload.0 == 2) {
                                return Ok(Some(Self::adapter(3, vec![value, Value::Class(dynamic)])));
                            }
                            return Ok(Some(self.bind_class_value(value, None, dynamic)?));
                        }
                        return Ok(Some(if matches!(&value, Value::Adapter(payload) if [2, 10, 11, 12, 19, 29, 30, 41].contains(&payload.0)) { Self::adapter(3, vec![value, receiver.clone()]) } else { value }));
                    }
                    Err(fault) if self.attribute_fault(&fault) => (),
                    Err(fault) => return Err(fault),
                }
            }
            if Self::own_kind(class).is_some() {
                if let Some(descriptor) = self.loose_kind_member(&Value::Class(class.clone()), name) {
                    return Ok(Some(if loose { descriptor } else { Self::adapter(3, vec![descriptor, receiver.clone()]) }));
                }
            }
        }
        Ok(None)
    }

    /// The name the parent class's refusals give a thing's kind: the
    /// class's own name for a thing of one, the kind word for a plain
    /// value, and the cell's name for a cell, as the reference names
    /// them in this very complaint.
    pub(super) fn super_tp_name(&self, value: &Value) -> String {
        if let Some(word) = self.kind_spelled(value) { return word.to_string(); }
        match value {
            Value::Object(o) => o.class_now().name.clone(),
            Value::Class(c) => c.name.clone(),
            Value::Adapter(w) if w.0 == 31 => "cell".to_string(),
            other => other.core_kind(),
        }
    }

    /// The class a kept form names: a class as it stands, or the class
    /// of a builtin kind word, the two forms the parent thing keeps its
    /// classes in.
    fn super_class_of(&mut self, held: Option<Value>) -> Option<Rc<Class>> {
        match held {
            Some(Value::Class(c)) => Some(c),
            Some(Value::Native(op, word)) if Self::kind_builtin(&op) => Some(self.kind_class(&word)),
            _ => None,
        }
    }

    /// Whether two classes are one for the parent walk's purposes:
    /// the very class, or two standings of one builtin kind, whose
    /// class is made in more than one place.
    pub(super) fn super_same_class(a: &Rc<Class>, b: &Rc<Class>) -> bool {
        Rc::ptr_eq(a, b) || matches!((Self::own_kind(a), Self::own_kind(b)), (Some(x), Some(y)) if x == y)
    }

    /// The class the first of the parent class's arguments names: a
    /// class as it stands, the class of a builtin kind word, and
    /// otherwise the refusal the reference words the same way.
    fn super_type_arg(&mut self, given: &Value) -> Flow<Rc<Class>> {
        match given {
            Value::Class(c) => Ok(c.clone()),
            Value::Native(op, name) if Self::kind_builtin(op) => Ok(self.kind_class(name.as_ref())),
            other => Err(format!("TypeError: super() argument 1 must be a type, not {}", other.core_kind()).into()),
        }
    }

    /// The class a parent thing answers as: the thing given itself for
    /// a class beneath the type, the thing's own class for a thing of
    /// one, and otherwise the refusal the reference words the same way.
    fn super_check(&mut self, owner: &Rc<Class>, receiver: &Value) -> Flow<Rc<Class>> {
        if let Value::Class(given) = receiver {
            if Self::class_order(given).iter().any(|base| Self::super_same_class(base, owner)) { return Ok(given.clone()); }
            // A class the type given makes is one of its things: the
            // maker answers for it, as its own kind does in the reference.
            if let Some(maker) = Self::maker_beneath(given) {
                if Self::class_order(&maker).iter().any(|base| Self::super_same_class(base, owner)) { return Ok(maker); }
            }
        }
        if let Value::Object(o) = receiver {
            let actual = o.class_now().clone();
            if Self::class_order(&actual).iter().any(|base| Self::super_same_class(base, owner)) { return Ok(actual); }
        }
        if let Value::Native(op, name) = receiver {
            if Self::kind_builtin(op) {
                let actual = self.kind_class(name.as_ref());
                if Self::class_order(&actual).iter().any(|base| Self::super_same_class(base, owner)) { return Ok(actual); }
            }
        }
        let spelled = Self::worth_of(receiver).and_then(|worth| self.kind_spelled(&worth).map(|word| word.to_string()))
            .unwrap_or_else(|| receiver.core_kind());
        if !spelled.is_empty() {
            let actual = self.kind_class(&spelled);
            if Self::class_order(&actual).iter().any(|base| Self::super_same_class(base, owner)) { return Ok(actual); }
        }
        // A proxy's own `__class__` may speak where its own class
        // cannot: read through the ordinary reading, and where the
        // answer is a type beneath the one given, it is the class the
        // thing answers as.
        let kind_word = self.class_word("kind").to_owned();
        match self.class_get(receiver.clone(), &kind_word, false) {
            Ok(exposed) => {
                if let Ok(shown) = self.super_type_arg(&exposed) {
                    if Self::class_order(&shown).iter().any(|base| Self::super_same_class(base, owner)) { return Ok(shown); }
                }
            }
            Err(fault) if self.attribute_fault(&fault) => {}
            Err(fault) => return Err(fault),
        }
        let (which, spelled) = match receiver {
            Value::Class(given) => ("type", given.name.clone()),
            Value::Native(op, name) if Self::kind_builtin(op) => ("type", name.to_string()),
            other => ("instance of", self.super_tp_name(other)),
        };
        Err(format!("TypeError: super(type, obj): obj ({} {}) is not an instance or subtype of type ({}).", which, spelled, owner.name).into())
    }

    /// A thing of the parent class made of the class it looks past and
    /// the thing it stands on, the reference's own checks worded as it
    /// words them: too many arguments, a first that is no type, and a
    /// second the type has no hold on.
    pub(super) fn super_made(&mut self, c: Rc<Class>, args: Vec<Value>) -> Flow<Value> {
        // A thing of the parent class is built the way any call of a
        // class builds a thing: the allocation the class names, then
        // the constructing it answers with, handed the arguments given.
        let made = self.super_allocated(&c, args.clone())?;
        let Value::Object(o) = &made else { return Ok(made) };
        if !Self::super_descended(&o.class_now()) { return Ok(made); }
        let constructor = self.lang.constructor.clone().unwrap_or_default();
        if let Some(init) = self.class_value(&o.class_now(), &constructor) {
            if matches!(&init, Value::Routine(_)) {
                let bound = self.bind_class_value(init, Some(made.clone()), o.class_now().clone())?;
                let answer = self.class_apply(bound, args)?;
                if !matches!(answer, Value::Null) { return Err(format!("TypeError: __init__() should return None, not '{}'", answer.core_kind()).into()); }
                return Ok(made);
            }
            let mut given = vec![made.clone()];
            given.extend(args);
            self.class_apply(init, given)?;
        }
        Ok(made)
    }

    /// The allocation a call of the parent class or a class beneath it
    /// runs: a `__new__` written out, called with the class and the
    /// arguments given, or a bare thing of the class otherwise.
    fn super_allocated(&mut self, c: &Rc<Class>, args: Vec<Value>) -> Flow<Value> {
        let allocate = self.class_word("allocate").to_string();
        if let Some(newer @ Value::Routine(_)) = self.class_value(c, &allocate) {
            let bound = self.bind_class_value(newer, None, c.clone())?;
            let mut given = vec![Value::Class(c.clone())];
            given.extend(args);
            return self.class_apply(bound, given);
        }
        self.made += 1;
        Ok(Value::Object(Rc::new(Instance { replacement_class: RefCell::new(None), class: c.clone(), fields: RefCell::new(Vec::new()), mark: self.made })))
    }

    /// A parent call with no argument written: the frame gave the class
    /// and the thing, and only the parent class itself is filled from
    /// them at once; a class beneath it is built as any call of it
    /// builds a thing, its own constructing, where one is written out,
    /// running with no argument at all.
    pub(super) fn super_framed(&mut self, c: Rc<Class>, owner: Rc<Class>, receiver: Value) -> Flow<Value> {
        let made = self.super_allocated(&c, Vec::new())?;
        let Value::Object(o) = &made else { return Ok(made) };
        if !Self::super_descended(&o.class_now()) { return Ok(made); }
        if Self::own_kind(&c).as_deref() == Some("super") {
            self.super_fill(o, owner, receiver)?;
            return Ok(made);
        }
        // A class beneath the parent class is built as any call of it
        // builds a thing, its own constructing running with no
        // argument at all; the parent class's own init, where it is
        // the one answered, reads the frame the call stands in.
        let constructor = self.lang.constructor.clone().unwrap_or_default();
        if let Some(init) = self.class_value(&o.class_now(), &constructor) {
            if matches!(&init, Value::Routine(_)) {
                let bound = self.bind_class_value(init, Some(made.clone()), o.class_now().clone())?;
                let answer = self.class_apply(bound, Vec::new())?;
                if !matches!(answer, Value::Null) { return Err(format!("TypeError: __init__() should return None, not '{}'", answer.core_kind()).into()); }
                return Ok(made);
            }
            self.class_apply(init, vec![made.clone()])?;
        }
        Ok(made)
    }

    /// The three names a parent thing keeps, filled from the class it
    /// looks past and the thing it stands on, each class kept in the
    /// form the program reads it.
    pub(super) fn super_fill(&mut self, o: &Rc<Instance>, owner: Rc<Class>, receiver: Value) -> Flow<()> {
        let mut objtype = Value::Null;
        if !matches!(receiver, Value::Null) {
            let checked = self.super_check(&owner, &receiver)?;
            objtype = self.public_class(checked);
        }
        let shown_owner = self.public_class(owner);
        let mut fields = o.fields.borrow_mut();
        for (key, value) in [("__thisclass__", shown_owner), ("__self__", receiver), ("\0objtype", objtype)] {
            match fields.iter_mut().find(|(held, _)| held == key) {
                Some((_, place)) => *place = value,
                None => fields.push((key.to_string(), value)),
            }
        }
        Ok(())
    }

    /// The parent class's own constructing called by name: the thing is
    /// measured against the arguments once more and made over, the way
    /// the reference lets its constructing run again on a thing made.
    fn super_initialised(&mut self, args: Vec<Value>) -> Flow<Value> {
        let items = self.call_items(args)?;
        let plain: Vec<Value> = items.iter().filter(|(key, _)| key.is_none()).map(|(_, value)| value.contents()).collect();
        let Some(subject) = plain.first() else { return Err("TypeError: descriptor '__init__' of 'super' object needs an argument".into()); };
        let o = match subject {
            Value::Object(o) if Self::super_descended(&o.class_now()) => o,
            _ => {
                let received = self.class_type(vec![subject.clone()])?.kind_it_names().ok_or_else(|| self.class_refusal())?;
                return Err(format!("TypeError: descriptor '__init__' requires a 'super' object but received a '{received}'").into());
            },
        };
        if items.iter().any(|(key, _)| key.is_some()) { return Err("TypeError: super() takes no keyword arguments".into()); }
        if plain.len() > 3 { return Err(format!("TypeError: super() expected at most 2 arguments, got {}", plain.len() - 1).into()); }
        // Reached with nothing but the thing, the constructing reads
        // its class and first argument from the frame it was called in.
        let (owner, receiver) = match plain.get(1) {
            Some(given) => (self.super_type_arg(given)?, plain.get(2).cloned().unwrap_or(Value::Null)),
            None => self.super_frame_args()?,
        };
        let o = o.clone();
        self.super_fill(&o, owner, receiver)?;
        return Ok(Value::Null);
    }

    /// The class and the first argument a no-argument parent init reads
    /// from the frame the call stands in: the routine running now, the
    /// worth its first name holds, and the class cell it closes over,
    /// each absence said the way the reference says it.
    fn super_frame_args(&mut self) -> Flow<(Rc<Class>, Value)> {
        let Some(program) = self.running_routine.clone() else { return Err("RuntimeError: super(): no current frame".into()); };
        if program.formals.is_empty() { return Err("RuntimeError: super(): no arguments".into()); }
        let mut first = Value::Blank;
        if let Some(frame) = &self.trace_frame {
            let held = frame.fields.borrow();
            if let Some((_, Value::Tuple(slots))) = held.iter().find(|(name, _)| name == "\0slots") {
                if let Some(value) = slots.first() {
                    first = match value { Value::Binding(cell) | Value::Bond(cell) => cell.borrow().clone(), other => other.clone() };
                }
            }
        }
        if matches!(first, Value::Blank) { return Err("RuntimeError: super(): arg[0] deleted".into()); }
        for (at, cell_value) in program.enclosed.iter() {
            if !program.idents[*at].starts_with("#class_cell") { continue; }
            let (Value::Binding(cell) | Value::Bond(cell)) = cell_value else { return Err("RuntimeError: super(): bad __class__ cell".into()); };
            let held = cell.borrow().contents();
            if matches!(held, Value::Blank) { return Err("RuntimeError: super(): empty __class__ cell".into()); }
            let Value::Class(owner) = held else {
                return Err(format!("RuntimeError: super(): __class__ is not a type ({})", self.super_tp_name(&held)).into());
            };
            return Ok((owner, first));
        }
        Err("RuntimeError: super(): __class__ cell not found".into())
    }
}
