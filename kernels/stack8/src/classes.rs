// Classes whose ancestry is an ordered company, and the members which
// belong to classes and routines rather than to their callers.
use super::*;

impl<'a> Engine<'a> {
    pub(super) fn class_word(&self, part: &str) -> &str {
        self.lang.class_details.get(part).and_then(|v| v.first()).map_or("", String::as_str)
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
            direct: vec![], lineage: vec![], base: None, answers: vec![], fields: vec![], reaches: vec![],
            methods: vec![], constants: vec![], shared: RefCell::new(vec![]), weak_storage: std::cell::Cell::new(None), sealed: std::cell::Cell::new(false), python_names: std::cell::RefCell::new(None) });
        self.class_root = Some(c.clone());
        c
    }
    /// The class standing for a builtin kind, made once for each word
    /// the definition names: a thing of a class beneath it keeps a worth
    /// of that kind among its members, under a name no program can spell.
    pub(super) fn kind_class(&mut self, word: &str) -> Rc<Class> {
        if let Some((_, c)) = self.kind_classes.iter().find(|(w, _)| w == word) { return c.clone(); }
        let root = self.root_class();
        let mut hooks = Vec::new();
        if matches!(self.lang.builtins.get(word), Some(Builtin::Zip | Builtin::Map | Builtin::Filter)) {
            for (place, mode) in [(15, 4), (16, 3)] {
                if let Some(name) = self.lang.class_special.get(place) { hooks.push((name.clone(), Self::adapter(119, vec![Value::Small(mode)]))); }
            }
        }
        let c = Rc::new(Class { outline: Some(format!("<class '{word}'>")), name: word.to_string(),
            direct: vec![root.clone()], lineage: vec![root.clone()], base: Some(root), answers: vec![], fields: vec![], reaches: vec![],
            methods: vec![], constants: vec![("\0kind".to_string(), Value::text(word))], shared: RefCell::new(hooks), weak_storage: std::cell::Cell::new(None), sealed: std::cell::Cell::new(false), python_names: std::cell::RefCell::new(None) });
        self.kind_classes.push((word.to_string(), c.clone()));
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
            direct: vec![root.clone()], lineage: vec![root.clone()], base: Some(root), answers: vec![], fields: vec![], reaches: vec![],
            methods: vec![], constants: vec![], shared: RefCell::new(vec![]), weak_storage: std::cell::Cell::new(None), sealed: std::cell::Cell::new(false), python_names: std::cell::RefCell::new(None) });
        self.class_maker = Some(c.clone());
        c
    }
    fn is_metaclass_root(&self, c: &Rc<Class>) -> bool {
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
        std::iter::once(c).chain(c.lineage.iter().map(Rc::as_ref)).find_map(Self::own_maker)
    }
    /// The metaclass that will make a class: the one its header named,
    /// else the one its forebears were made by. Where both speak, the
    /// one standing on the other is taken, as the deeper answer.
    fn maker_in_force(&mut self, asked: Option<Value>, bases: &[Rc<Class>]) -> Flow<Option<Rc<Class>>> {
        let named = match asked.map(|v| v.contents()) {
            None => None,
            // The kind builtin names the plainest maker there is, which
            // is no metaclass of its own; so does the class it stands for.
            Some(Value::Native(Builtin::SortOf, _)) => None,
            Some(Value::Class(c)) if self.is_metaclass_root(&c) => None,
            Some(Value::Class(c)) => Some(c),
            Some(other) => return Err(self.core_fault("core.uncallable", &other.core_kind()).into()),
        };
        let inherited = bases.iter().find_map(|b| Self::maker_beneath(b));
        Ok(match (named, inherited) {
            (Some(a), Some(b)) => Some(if b.lineage.iter().any(|c| Rc::ptr_eq(c, &a)) { b } else { a }),
            (a, b) => a.or(b),
        })
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
        std::iter::once(c).chain(c.lineage.iter().map(Rc::as_ref)).find_map(Self::own_kind)
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
        // A routine built by hand out of a code value and a
        // dictionary of names: the reference's way of making a
        // function, which keeps the dictionary it was handed.
        if word == "function" {
            let (Some(Value::Adapter(code)), Some(globe)) = (args.first(), args.get(1)) else { return Err(self.class_refusal()); };
            if code.0 != 7 { return Err(self.class_refusal()); }
            let Some(Value::Routine(template)) = code.1.first() else { return Err(self.class_refusal()); };
            let born = self.ambient_builtins();
            let mut made = (**template).clone();
            made.globe = Some(globe.clone());
            made.born = Some(born);
            made.revised = RefCell::new(None);
            self.made += 1;
            return Ok(Value::Routine(Rc::new(made)));
        }
        let Some(op) = self.lang.builtins.get(word).copied() else { return Err(self.class_refusal()); };
        let items = self.call_items(args)?;
        let made = self.builtin_call(op, word, items)?;
        let made = if word == "str" { Self::worth_of(&made).unwrap_or(made) } else { made };
        let kept = match made.contents() {
            held @ (Value::Array(_) | Value::Map(_)) => Value::Collection(Rc::new(RefCell::new(held)), true),
            other => other,
        };
        self.made += 1;
        Ok(Value::Object(Rc::new(Instance {replacement_class: RefCell::new(None),  class: c, fields: RefCell::new(vec![("\0worth".to_string(), kept)]), mark: self.made })))
    }
    pub(super) fn form_class(&mut self, name: String, bases: Vec<Rc<Class>>, members: Vec<(String, Value)>) -> Flow<Value> {
        self.form_named_class(Value::text(&name), bases, members)
    }
    fn form_named_class(&mut self, title_value: Value, mut bases: Vec<Rc<Class>>, mut members: Vec<(String, Value)>) -> Flow<Value> {
        let name = self.type_title(&title_value)?;
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
            if let Some(factory) = maker.filter(|v| !matches!(v, Value::Native(Builtin::SortOf, _))) {
                let entries = members.iter().filter(|(_, value)| !matches!(value, Value::Blank))
                    .map(|(word, value)| (Value::text(word), value.clone())).collect::<Vec<_>>();
                let namespace = Value::Collection(Rc::new(RefCell::new(Value::Map(Rc::new(entries.into())))), true);
                let mut given = vec![Value::text(&name), Value::tuple(raw_bases), namespace];
                given.extend(keywords.into_iter().map(|(word, value)| Value::Tie(Rc::new((Value::text(&word), value)))));
                return self.class_apply(factory, given);
            }
            bases.clear();
            for value in raw_bases { bases.push(self.type_base(&value)?); }
            members.extend(keywords.into_iter().map(|(word, value)| (format!("\0keyword:{word}"), value)));
        }
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
                let listed = Value::tuple(bases.iter().cloned().map(Value::Class).collect());
                let pairs: Vec<(Value, Value)> = members.iter().filter(|(_, v)| !matches!(v, Value::Blank))
                    .map(|(n, v)| (Value::text(n), v.clone())).collect();
                let namespace = Value::Collection(Rc::new(RefCell::new(Value::Map(Rc::new(pairs.into())))), true);
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
        if plain.len() < 4 { return Err(self.class_refusal()); }
        let by = match &plain[0] { Value::Class(m) if !self.is_metaclass_root(m) => Some(m.clone()), _ => None };
        let title = self.type_title(&plain[1])?;
        let mut parents = Vec::new();
        let (Value::Tuple(listed) | Value::Array(listed)) = plain[2].contents() else { return Err(self.class_refusal()) };
        for b in listed.iter() { parents.push(self.type_base(b)?); }
        if parents.is_empty() { parents.push(self.root_class()); }
        let Value::Map(entries) = plain[3].contents() else { return Err(self.class_refusal()) };
        let members = entries.iter().map(|(k, v)| (k.plain(), v.clone())).collect();
        self.forge_class(title, parents, members, by, named, plain[1].contents())
    }
    fn type_argument_kind(value: &Value) -> String {
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
    fn type_utf8(&mut self, value: &Value) -> Flow<()> {
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
    /// The class itself, laid out from its name, its bases, its members
    /// and the metaclass it is to remember. This is the making the kind
    /// builtin does, and what a metaclass reaches for through its
    /// forebears when it has made a namespace of its own.
    pub(super) fn forge_class(&mut self, name: String, bases: Vec<Rc<Class>>, mut members: Vec<(String, Value)>,
        maker: Option<Rc<Class>>, carried: Vec<Value>, title_value: Value) -> Flow<Value> {
        // A place only an arm of a conditional writes to may never have
        // been written. Nothing stands there, and the class keeps no
        // member for it: a name a conditional never bound is no member.
        members.retain(|(_, held)| !matches!(held, Value::Blank));
        let mut lines: Vec<Vec<Rc<Class>>> = bases.iter().map(|b| {
            let mut line = vec![b.clone()]; line.extend(b.lineage.iter().cloned()); line
        }).collect();
        lines.push(bases.clone());
        let mut lineage = Vec::new();
        while lines.iter().any(|s| !s.is_empty()) {
            let head = lines.iter().filter_map(|s| s.first()).find(|head|
                !lines.iter().any(|s| s.iter().skip(1).any(|tail| Rc::ptr_eq(head, tail)))).cloned()
                .ok_or_else(|| self.class_word("mro.amiss").to_string())?;
            if lineage.iter().any(|c| Rc::ptr_eq(c, &head)) { return Err(self.class_word("mro.amiss").to_string().into()); }
            for line in &mut lines { if line.first().map_or(false, |c| Rc::ptr_eq(c, &head)) { line.remove(0); } }
            lineage.push(head);
        }
        // Two builtin kinds cannot both be kept in one thing.
        let mut kinds: Vec<String> = lineage.iter().filter_map(|b| Self::own_kind(b)).collect();
        kinds.dedup();
        if kinds.len() > 1 { return Err(self.lang.layout_amiss.clone().unwrap_or_else(|| self.class_word("unready").to_string()).into()); }
        // The module named is the one the class statement runs in: the
        // module the file being read was loaded as, or the run's own name
        // where the class is written in the program itself.
        let module = if self.class_word("module").is_empty() {
            self.class_word("main").to_string()
        } else {
            self.module_slots.get(&self.source).map(|(_, path)| path.clone())
                .unwrap_or_else(|| self.class_word("main").to_string())
        };
        if !members.iter().any(|(n,_)| n == self.class_word("module")) {
            members.push((self.class_word("module").to_string(), Value::text(&module)));
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
            } else { members.push((self.class_word("doc").to_string(), Value::Null)); }
            members.retain(|(key, _)| key != self.class_word("qualified"));
            Some((title_value, qualified.clone(), self.class_word("module").to_string(), qualified))
        } else { None };
        let display=members.iter().find(|(n,_)|n==self.class_word("qualified")).map(|(_,v)|v.plain()).unwrap_or_else(||name.clone());
        let c = Rc::new(Class { name: name.clone(), outline: Some(format!("<class '{module}.{display}'>")),
            base: bases.first().cloned(), direct: bases, lineage, answers: vec![], fields: vec![], reaches: vec![],
            methods: vec![], constants: maker.map(|m| vec![(MAKER_MEMBER.to_string(), Value::Class(m))]).unwrap_or_default(),
            shared: RefCell::new(members), weak_storage: std::cell::Cell::new(None), sealed: std::cell::Cell::new(false), python_names: RefCell::new(python_names) });
        self.furnish_slots(&c)?;
        // Each member that asks to be told its name is told it, once the
        // class stands, before any forebear hears of the new class.
        if !self.class_word("descriptor.name").is_empty() {
            let declared = c.shared.borrow().clone();
            for (member, held) in declared {
                let Some(told) = self.descriptor_hook(&held, "descriptor.name") else { continue };
                self.call_descriptor(&held, told, vec![Value::Class(c.clone()), Value::text(&member)])?;
            }
        }
        if let Some(hook) = c.lineage.iter().find_map(|b| Self::own_class_value(b, self.class_word("subclass"))) {
            if matches!(&hook,Value::Adapter(w) if w.0==5){let bound=self.bind_class_value(hook,None,c.clone())?;self.class_apply(bound,carried)?;}
            else{let mut given=vec![Value::Class(c.clone())];given.extend(carried);self.class_apply(hook, given)?;}
        }
        // Keywords with no hook to receive them are refused.
        else if !carried.is_empty() { return Err(self.class_refusal()); }
        Ok(Value::Class(c))
    }
    /// The slots a class names become members of it, each a descriptor
    /// that keeps the slot's value in the thing under a name of its own,
    /// so that two classes of one line naming the same slot keep two.
    fn furnish_slots(&mut self, c: &Rc<Class>) -> Flow<()> {
        if self.class_word("descriptor.get").is_empty() { return Ok(()); }
        c.weak_storage.set(Some(self.weak_layout(c)));
        let Some(slots) = Self::own_class_value(c, self.class_word("slots")) else { return Ok(()) };
        let named: Vec<Value> = match slots.contents() { Value::Tuple(v) | Value::Array(v) => v.as_ref().clone(), single => vec![single] };
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
                c.lineage.iter().any(|b| !Rc::ptr_eq(b, &root) && Self::own_kind(b).is_none() && Self::own_class_value(b, &slots_word).is_none())
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
    fn slot_place(&self, thing: &Value, parts: &[Value]) -> Flow<String> {
        let (Value::Object(o), Some(Value::Class(owner))) = (thing, parts.get(1)) else { return Err(self.class_refusal()) };
        let word = parts[0].plain();
        if !Rc::ptr_eq(&o.class_now(), owner) && !o.class_now().lineage.iter().any(|b| Rc::ptr_eq(b, owner)) {
            let pieces = self.lang.class_details.get("descriptor.foreign").cloned().unwrap_or_default();
            if pieces.len() != 4 { return Err(self.class_refusal()); }
            return Err(format!("{}{word}{}{}{}{}{}", pieces[0], pieces[1], owner.name, pieces[2], o.class_now().name, pieces[3]).into());
        }
        Ok(format!("\0slot:{word}:{:p}", Rc::as_ptr(owner)))
    }
    fn slot_read(&self, thing: &Value, parts: &[Value]) -> Flow<Value> {
        let place = self.slot_place(thing, parts)?;
        let Value::Object(o) = thing else { return Err(self.class_refusal()) };
        let kept = o.fields.borrow().iter().find(|(n, _)| *n == place).map(|(_, v)| v.clone());
        kept.ok_or_else(|| self.missing_member(thing, &parts[0].plain()))
    }
    fn slot_write(&self, thing: &Value, parts: &[Value], value: Option<Value>) -> Flow<Value> {
        let place = self.slot_place(thing, parts)?;
        let Value::Object(o) = thing else { return Err(self.class_refusal()) };
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
        for part in ["property.fget", "property.fset", "property.fdel", "doc"] {
            members.push((self.class_word(part).to_string(), Self::adapter(28, vec![Value::text(Self::accessor_place(part))])));
        }
        let c = Rc::new(Class { outline: Some(format!("<class '{name}'>")), name,
            direct: vec![root.clone()], lineage: vec![root.clone()], base: Some(root), answers: vec![], fields: vec![], reaches: vec![],
            methods: vec![], constants: vec![], shared: RefCell::new(members), weak_storage: std::cell::Cell::new(None), sealed: std::cell::Cell::new(false), python_names: std::cell::RefCell::new(None) });
        self.property_class = Some(c.clone());
        c
    }
    /// Where a property keeps each accessor among its own members, under
    /// names no program can spell.
    fn accessor_place(part: &str) -> &'static str {
        match part { "property.fget" => "\0fget", "property.fset" => "\0fset", "property.fdel" => "\0fdel", "doc" => "\0doc", _ => "\0name" }
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
        let of = match thing { Value::Object(o) => o.class_now().name.clone(), Value::Class(c) => if self.class_word("name").is_empty() { c.name.clone() } else { Self::maker_beneath(c).map_or_else(|| c.name.clone(), |maker| maker.python_names.borrow().as_ref().map_or_else(|| maker.name.clone(), |names| names.0.type_text().plain())) }, other => other.plain() };
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
            // its name is left for the class that takes it to give.
            23..=25 => {
                let [accessor] = given else { return Err(self.class_refusal()) };
                let place = ["\0fget", "\0fset", "\0fdel"][(tag - 23) as usize];
                let mut fields: Vec<(String, Value)> = property.fields.borrow().iter().filter(|(n, _)| n != place && n != "\0name").cloned().collect();
                fields.push((place.to_string(), accessor.clone()));
                self.made += 1;
                Ok(Value::Object(Rc::new(Instance {replacement_class: RefCell::new(None),  class: property.class_now().clone(), fields: RefCell::new(fields), mark: self.made })))
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
                let mut fields = property.fields.borrow_mut();
                for (place, value) in ["\0fget", "\0fset", "\0fdel", "\0doc"].iter().zip(kept) { fields.push((place.to_string(), value)); }
                Ok(Value::Null)
            }
            27 => {
                let [_, name] = given else { return Err(self.class_refusal()) };
                let _ = Self::write_members(&mut property.fields.borrow_mut(), "\0name", Some(name.clone()), false);
                Ok(Value::Null)
            }
            _ => Err(self.class_refusal()),
        }
    }
    /// What a property's kept accessor reads as: the accessor itself, or
    /// for its first string, the one it was given, else its getter's.
    fn property_reading(&self, property: &Instance, place: &str) -> Value {
        if let Some(v) = Self::property_accessor(property, place) { return v; }
        if place == "\0doc" {
            if let Some(Value::Routine(getter)) = Self::property_accessor(property, "\0fget") {
                return getter.doc.clone().map_or(Value::Null, |s| Value::text(&s));
            }
        }
        Value::Null
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
        let bound = self.bind_class_value(hook, Some(member.clone()), o.class_now().clone())?;
        self.class_apply(bound, args)
    }
    /// A member that takes writes as well as reads: it is asked before a
    /// thing's own fields, where one that only reads gives way to them.
    fn takes_writes(&self, member: &Value) -> bool {
        if let Value::Adapter(w) = member { return matches!(w.0, 6 | 16 | 28); }
        self.descriptor_hook(member, "descriptor.set").is_some() || self.descriptor_hook(member, "descriptor.delete").is_some()
    }
    /// Whether a fault is a missing member's, however it was raised.
    pub(super) fn attribute_fault(&self, fault: &Fault) -> bool {
        let kind = self.class_word("attribute.amiss").split(':').next().unwrap_or_default();
        if kind.is_empty() { return false; }
        match fault {
            Fault::Note(told) => told.split(':').next() == Some(kind),
            Fault::Thrown(Value::Object(o)) => std::iter::once(&o.class_now()).chain(o.class_now().lineage.iter()).any(|c| c.name == kind),
            _ => false,
        }
    }
    fn own_class_value(c: &Class, name: &str) -> Option<Value> {
        if let Some((_,v)) = c.shared.borrow().iter().find(|(n,_)| n == name) { return Some(v.clone()); }
        c.methods.iter().find(|(n,_)| n == name).map(|(_,p)| Value::Routine(p.clone()))
            .or_else(|| c.constants.iter().find(|(n,_)| n == name).map(|(_,v)| v.clone()))
    }
    pub(super) fn class_value(&self, c: &Class, name: &str) -> Option<Value> {
        Self::own_class_value(c,name).or_else(|| c.lineage.iter().find_map(|b| Self::own_class_value(b,name)))
    }
    pub(super) fn adapter(kind: u8, values: Vec<Value>) -> Value { Value::Adapter(Rc::new((kind,values))) }
    pub(super) fn class_apply(&mut self, callable: Value, mut args: Vec<Value>) -> Flow<Value> {
        match callable {
            Value::Routine(p) => { self.invoke(&p,args)?; Ok(self.drop_top()?) }
            // A method bound to a value of a builtin kind, reached as a
            // value in its own right and then called.
            Value::ValueMethod(bound) => Ok(self.value_method(&bound.0,&bound.1,args,Vec::new())?),
            Value::Method(o,p) => { args.insert(0,Value::Object(o)); self.invoke(&p,args)?; Ok(self.drop_top()?) }
            // A thing called stands on its own call member, which may be
            // a thing again: each such step is counted with the calls
            // standing, so a thing whose call member is a thing of its
            // own kind is refused at the depth any endless call is.
            Value::Object(o) => {let f=self.class_value(&o.class_now(),self.class_word("call")).ok_or_else(||self.class_refusal())?;self.reaching_further()?;args.insert(0,Value::Object(o));let answer=self.class_apply(f,args);self.answered();answer},
            Value::Class(c) => self.class_make(c,args),
            Value::Adapter(w) => match w.0 {
                9 if w.1.is_empty() && args.len() == 2 && matches!(args[0], Value::Class(_)) => Ok(Self::adapter(9, args)),
                0 => Ok(w.1[0].clone()),
                1 => {
                    let Some(Value::Class(c)) = args.first() else { return Err(self.class_refusal()); };
                    if self.exception_class(c) { return Ok(self.exception_instance(c.clone(), args[1..].to_vec(), Value::Null)); }
                    if args.len()!=1 { self.root_refuses_arguments(c,true)?; }
                    self.made += 1;
                    Ok(Value::Object(Rc::new(Instance {replacement_class: RefCell::new(None), class:c.clone(),fields:RefCell::new(vec![]),mark:self.made})))
                }
                2 if matches!(args.first(), Some(Value::Object(o)) if self.exception_class(&o.class_now())) => {
                    let Value::Object(o) = args.remove(0) else { unreachable!() };
                    let name = self.lang.constructor.clone().unwrap_or_default();
                    self.exception_method(o, &name, &args)
                }
                2 if args.len()==1 => Ok(Value::Null),
                2 => {
                    let Some(Value::Object(o)) = args.first() else { return Err(self.class_refusal()); };
                    self.root_refuses_arguments(&o.class_now(),false)?;
                    Ok(Value::Null)
                }
                40 => self.class_from_parts(args),
                41 => {
                    let opened = self.call_items(args)?;
                    let mut values: Vec<Value> = opened.into_iter().map(|(key, value)| match key {
                        Some(key) => Value::Tie(Rc::new((Value::text(&key), value))), None => value,
                    }).collect();
                    if values.is_empty() { return Err(self.class_refusal()); }
                    let Value::Class(class) = values.remove(0) else { return Err(self.class_refusal()); };
                    self.class_construct(class, values)
                }
                43 => {
                    if let Some(first) = args.first() { self.iterator(first.clone())?; }
                    self.class_apply(w.1[0].clone(), args)
                }
                42 => Ok(Value::Collection(Rc::new(RefCell::new(Value::Map(Rc::new(Vec::new().into())))), true)),
                30 => self.root_work(&w.1[0].plain(),args),
                // The maker of a builtin kind: given the class to make a
                // thing of and what the kind's builtin takes.
                14 if !args.is_empty() => {
                    let kind = args.remove(0);
                    let word = w.1[0].plain();
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
                    let Value::Class(c) = kind else { return Err(self.class_refusal()); };
                    let given = if self.lang.builtins.get(&word) == Some(&Builtin::Set) { Vec::new() } else { args };
                    self.thing_of_kind(c, &word, given)
                }
                119 if args.len() == 1 => {
                    let Value::Small(mode) = w.1[0] else { return Err(self.class_refusal()); };
                    self.iterator_recipe_next(mode, &args[0])?.ok_or_else(|| self.core_fault("core.exhausted", "").into())
                }
                3 => { args.insert(0,w.1[1].clone()); self.class_apply(w.1[0].clone(),args) }
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
                        let pieces = self.lang.class_details.get("descriptor.unbound").cloned().unwrap_or_default();
                        return if pieces.len() == 3 {
                            Err(format!("{}{word}{}{member}{}", pieces[0], pieces[1], pieces[2]).into())
                        } else {
                            Err(self.class_refusal())
                        };
                    }
                    let subject = args.remove(0);
                    // A thing of a class standing on the very kind this
                    // word names answers as its worth would, since the
                    // loose member is the kind's own and not the
                    // class's: `set.union(s, ...)` for `s` a subclass of
                    // `set` works upon what `s` keeps of a set.
                    let receiver = match &subject {
                        // The worth is kept as it stands, cell and all,
                        // where it is one that a method writes into (a
                        // row or a map, behind a cell of its own): the
                        // writing must reach the very thing the subclass
                        // instance keeps, not a copy taken out of it.
                        Value::Object(o) if Self::kind_beneath(&o.class_now()).as_deref() == Some(word.as_ref()) => {
                            Self::worth_of(&subject).unwrap_or_else(|| subject.clone())
                        }
                        _ => subject.clone(),
                    };
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
                    let found = if of_own_kind { self.builtin_member(&receiver,&member)? } else { None };
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
                4 | 8 => self.class_apply(w.1[0].clone(),args),
                5 => Err(self.core_fault("core.uncallable", "classmethod").into()),
                13 if args.len() == 1 => {
                    let Value::Adapter(property) = &w.1[0] else { return Err(self.class_refusal()); };
                    let mut members = property.1.clone();
                    members.resize(2, Value::Null); members[1] = args.remove(0);
                    Ok(Self::adapter(6, members))
                }
                10..=12 => {
                    let Some(subject) = args.first().cloned() else { return Err(self.class_refusal()); };
                    let Some(Value::Text(name)) = args.get(1) else { return Err("TypeError: attribute name must be string".to_string().into()); };
                    if w.0 == 10 { self.class_get(subject,name,true) }
                    else { self.class_write(subject,name,if w.0 == 11 {args.get(2).cloned()} else {None},true) }
                }
                // The reader of a member that binds: given the thing, or
                // nothing and the class, it answers what a read through
                // that thing or class would.
                15 if !args.is_empty() && args.len() <= 2 => {
                    let thing = match &args[0] { Value::Null => None, other => Some(other.clone()) };
                    let owner = match (args.get(1), &thing) {
                        (Some(Value::Class(c)), _) => c.clone(),
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
                else { Ok(self.builtin(op,&word,&mut args)?) }
            }
            other => Err(self.core_fault("core.uncallable", &other.core_kind()).into()),
        }
    }
    /// Making a thing of a class. A class made by a metaclass is called
    /// through that metaclass's own call, which decides what comes of it.
    pub(super) fn class_make(&mut self, c: Rc<Class>, args: Vec<Value>) -> Flow<Value> {
        if matches!(Self::own_kind(&c).as_deref(), Some("range_iterator" | "longrange_iterator")) {
            return Err(format!("TypeError: cannot create '{}' instances", c.name).into());
        }
        if c.name == "FunctionType" && self.lang.trace_fields.len() > 18 && matches!(args.first().map(Value::contents), Some(Value::Adapter(code)) if code.0 == 7) {
            let mut parts = vec![None; 5];
            let mut next = 0;
            for (named, value) in self.call_items(args)? {
                let at = match named {
                    Some(word) => ["code", "globals", "name", "argdefs", "closure"].iter().position(|part| *part == word)
                        .ok_or_else(|| format!("TypeError: function() got an unexpected keyword argument '{word}'"))?,
                    None => { let at = next; next += 1; at }
                };
                if at >= parts.len() || parts[at].replace(value).is_some() { return Err("TypeError: invalid function arguments".into()); }
            }
            let Some(Value::Adapter(code)) = parts[0].as_ref().map(Value::contents) else { return Err("TypeError: function() argument 'code' must be code".into()); };
            let (7, Some(Value::Routine(origin))) = (code.0, code.1.first()) else { return Err("TypeError: function() argument 'code' must be code".into()); };
            let Some(globals) = parts[1].clone() else { return Err("TypeError: function() missing required argument 'globals'".into()); };
            if !matches!(globals.contents(), Value::Map(_)) { return Err("TypeError: function() argument 'globals' must be dict".into()); }
            let mut made = (**origin).clone();
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
            let created = Value::Routine(Rc::new(made));
            let at = self.function_storage(&created);
            self.routine_namespace_write(at, Some(globals))?;
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
    /// The making itself, as the kind builtin does it: the class
    /// allocates a thing and constructs it.
    pub(super) fn class_construct(&mut self, c: Rc<Class>, args: Vec<Value>) -> Flow<Value> {
        let allocation = self.class_value(&c,self.class_word("allocate"));
        // A metaclass called outright makes a class, the way the kind
        // builtin does, from a name, bases and a namespace.
        if allocation.is_none() && c.lineage.iter().any(|b| self.is_metaclass_root(b)) {
            let mut given = vec![Value::Class(c.clone())];
            given.extend(args);
            return self.class_from_parts(given);
        }
        let kind = Self::kind_beneath(&c);
        let object = if let Some(f) = allocation {
            let mut given = vec![Value::Class(c.clone())]; given.extend(args.clone());
            self.class_apply(f,given)?
        } else if let Some(word) = &kind {
            if matches!(word.as_str(), "str_iterator" | "str_ascii_iterator") {
                return Err(format!("TypeError: cannot create '{}' instances", word).into());
            }
            let mut initial = args.clone();
            match self.lang.builtins.get(word) {
                Some(Builtin::Set) => initial.clear(),
                Some(Builtin::AsReal) if self.lang.constructor.as_deref().and_then(|name| self.class_value(&c, name)).is_some() => {
                    initial = self.call_items(initial)?.into_iter().filter_map(|(key, value)| key.is_none().then_some(value)).take(1).collect();
                }
                Some(Builtin::List) if self.lang.constructor.as_deref().and_then(|name| self.class_value(&c, name)).is_some() => {
                    initial = self.call_items(initial)?.into_iter().filter_map(|(key, value)| key.is_none().then_some(value)).collect();
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
        if let Value::Object(o) = &object {
            // One whose class has last words to say is remembered, so that
            // a round holding it may be found when the program asks.
            if self.lang.finaliser.is_some() && crate::faint::last_word_of(&o.class).is_some() {
                crate::faint::remember(crate::faint::Hold::Object(Rc::downgrade(o)));
            }
            if self.lang.destructor.is_some() || self.lang.finaliser.is_some() {
                self.things_made.borrow_mut().push(Rc::downgrade(o));
            }
            if Rc::ptr_eq(&o.class_now(),&c) || o.class_now().lineage.iter().any(|b| Rc::ptr_eq(b,&c)) {
                let init = self.lang.constructor.as_deref().and_then(|n| self.class_value(&o.class_now(),n));
                if let Some(f) = init {
                    let bound=self.bind_class_value(f,Some(object.clone()),o.class_now().clone())?;
                    let answer = self.class_apply(bound,args)?;
                    if !matches!(answer,Value::Null) { return Err(self.class_refusal()); }
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
    /// A member every thing and every class has from the root, where
    /// the name is one: the members the language names for it, and the
    /// hooks for making, constructing, reading, writing, removing and
    /// formatting. Given the class of a thing, the hooks come bare, to
    /// be bound to the thing.
    fn root_member(&self, name: &str, of: Option<&Rc<Class>>) -> Option<Value> {
        if name.is_empty() { return None; }
        if self.lang.class_details.get("root.members").map_or(false,|names| names.iter().any(|n| n==name)) {
            return Some(Self::adapter(30,vec![Value::text(name)]));
        }
        of?;
        let hook = if name==self.class_word("allocate") {1}
            else if self.lang.constructor.as_deref()==Some(name) || name==self.class_word("subclass") {2}
            else if name==self.class_word("get") {10} else if name==self.class_word("set") {11}
            else if name==self.class_word("remove") {12}
            else if self.lang.class_special.get(72).map_or(false,|word| word==name) {19}
            else {return None};
        Some(Self::adapter(hook,vec![]))
    }
    /// The root's own working of one of its members, handed the value
    /// it works upon first.
    fn root_work(&mut self, name: &str, args: Vec<Value>) -> Flow<Value> {
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
            9 => self.default_directory(&subject),
            10 => self.root_state(&subject),
            11 | 12 => {
                if place == 12 {
                    if let Some(answer) = self.special_call(&subject, 79, Vec::new()).map_err(Fault::Note)? {
                        return Ok(answer);
                    }
                }
                let class = match &subject { Value::Object(o) => Value::Class(o.class_now().clone()), other => self.class_type(vec![other.clone()])? };
                Value::tuple(vec![class, Value::tuple(Vec::new()), self.root_state(&subject)])
            }
            13 => Value::Small(match &subject { Value::Object(o) if o.class_now().name != self.class_word("root") => 24, _ => 16 }),
            _ => return Err(self.class_refusal()),
        })
    }
    /// What a thing holds of its own, as a dictionary, or nothing where
    /// it holds nothing.
    fn root_state(&self, subject: &Value) -> Value {
        let Value::Object(o) = subject else { return Value::Null };
        let entries: Vec<(Value, Value)> = o.fields.borrow().iter().filter(|(n,v)| !n.starts_with(['\0', '#']) && !matches!(v, Value::Blank)).map(|(n,v)| (Value::text(n), v.clone())).collect();
        if entries.is_empty() { Value::Null } else { Value::Map(Rc::new(entries.into())) }
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
            Value::Object(instance) if instance.class_now().name == "ModuleType" => {
                let fields = instance.fields.borrow();
                let label = fields.iter().find(|(key, _)| key == "__name__").and_then(|(_, value)| match value.contents() { Value::Text(word) => Some(word.to_string()), _ => None });
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
        let class = match subject { Value::Object(o)=>o.class_now().name.clone(), Value::Class(c)=>c.name.clone(), other=>other.contents().core_kind() };
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
    pub(super) fn bind_class_value(&mut self, value: Value, subject: Option<Value>, class: Rc<Class>) -> Flow<Value> {
        if let Value::Adapter(w) = &value {
            return match w.0 {
                119 if subject.is_some() => Ok(Self::adapter(3, vec![value.clone(), subject.unwrap()])),
                4 => Ok(w.1[0].clone()),
                5 => Ok(Self::adapter(3,vec![w.1[0].clone(),Value::Class(class)])),
                6 if subject.is_some() => self.class_apply(w.1[0].clone(),vec![subject.unwrap()]),
                16 if subject.is_some() => self.slot_read(&subject.unwrap(), &w.1),
                // A working of the property class, read through a
                // property: bound to it. Its kept accessors read plainly.
                20..=27 if subject.is_some() => Ok(Self::adapter(3, vec![value.clone(), subject.unwrap()])),
                28 => match subject { Some(Value::Object(o)) => Ok(self.property_reading(&o, &w.1[0].plain())), _ => Ok(value) },
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
            (Value::Routine(f),Some(Value::Object(o))) => Ok(Value::Method(o,f)),
            (Value::Routine(f),Some(other)) => Ok(Self::adapter(3, vec![Value::Routine(f), other])),
            (v,_) => Ok(v),
        }
    }
    pub(super) fn integer_member(&self, subject: &Value, name: &str) -> Option<Value> {
        if let Value::Native(Builtin::Bool, word) = subject {
            if name == self.class_word("allocate") {
                return Some(Self::adapter(14, vec![Value::text(word)]));
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
        if let Value::Adapter(proxy) = &subject {
            if let (9, [Value::Class(owner), receiver]) = (proxy.0, proxy.1.as_slice()) {
                let dynamic = match receiver { Value::Object(o) => o.class_now().clone(), Value::Class(c) => c.clone(), _ => return Err(self.class_refusal()) };
                let order: Vec<_> = std::iter::once(dynamic.clone()).chain(dynamic.lineage.iter().cloned()).collect();
                if let Some(start) = order.iter().position(|class| Rc::ptr_eq(class, owner)) {
                    for class in &order[start + 1..] {
                        if let Some(value) = Self::own_class_value(class, name) {
                            return self.bind_class_value(value, Some(receiver.clone()), dynamic);
                        }
                    }
                }
                return Err(self.missing_member(&subject, name));
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
        let bound = self.bind_class_value(reader, Some(subject.clone()), o.class_now().clone())?;
        self.class_apply(bound, vec![Value::text(name)]).map_err(|failure| self.attribute_from_hook(failure, &subject, name))
    }
    fn class_read(&mut self, subject: Value, name: &str, plain: bool) -> Flow<Value> {
        let raw = subject.contents();
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
                    Some(25) => return Ok(Value::text(if kept.closed { "GEN_CLOSED" } else if kept.started { "GEN_SUSPENDED" } else { "GEN_CREATED" })),
                    _ => if let Some(program) = &kept.program { return Ok(self.routine_code(program)); },
                }
            }
        }
        if matches!(&subject, Value::Trace(_)) {
            if let Some(method) = self.builtin_directory_method(&subject, name) { return Ok(method); }
        }
        if let Value::Trace(trace) = &subject {
            return match self.lang.trace_fields.iter().position(|key| key == name) {
                Some(1) => Ok(Value::Small(trace.line as i64)),
                Some(2) => Ok(trace.next.clone()),
                Some(3) => Ok(Value::Object(trace.frame.clone())),
                Some(26) => Ok(Value::Small(trace.instruction)),
                Some(16) => Ok(Value::Small(trace.location.map_or(trace.line, |p| p.2) as i64)),
                Some(17) => Ok(trace.location.map_or(Value::Null, |p| Value::Small(p.1 as i64))),
                Some(18) => Ok(trace.location.map_or(Value::Null, |p| Value::Small(p.3 as i64))),
                _ => Err(self.missing_member(&subject, name)),
            };
        }
        if let Value::Adapter(property) = &subject {
            if property.0 == 6 && Lang::spells(&self.lang.property_setter, name) { return Ok(Self::adapter(13, vec![subject])); }
        }
        // The kind's word, or the kind read as a class, answers for its
        // type flags from the class the kind stands for.
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
        if name==self.class_word("namespace") {
            let held=subject.contents();
            let builtin=match &held {
                Value::Native(op,word) if Self::kind_builtin(op)=>Some(word.clone()),
                other=>self.kind_spelled(other),
            };
            if let Some(word)=builtin {
                let mut listed=self.kind_special_names(&word);
                listed.push(name.to_string());
                let pairs:Vec<(Value,Value)>=listed.into_iter().map(|entry| {
                    (Value::text(&entry),Value::text(&format!("<attribute '{entry}' of '{word}' objects>")))
                }).collect();
                return Ok(Value::View(Rc::new((Value::Map(Rc::new(pairs.into())),"mapping".to_string()))));
            }
        }
        // A routine, a wrapped routine and a slot each read as a member
        // that binds; the slot writes and removes as well.
        if name == self.class_word("descriptor.get") && !name.is_empty()
            && (matches!(&subject, Value::Routine(_)) || matches!(&subject, Value::Adapter(w) if matches!(w.0, 4 | 5 | 16))) {
            return Ok(Self::adapter(15, vec![subject]));
        }
        if let Value::Adapter(w) = &subject {
            if w.0 == 16 && !name.is_empty() {
                if name == self.class_word("descriptor.set") { return Ok(Self::adapter(17, w.1.clone())); }
                if name == self.class_word("descriptor.delete") { return Ok(Self::adapter(18, w.1.clone())); }
            }
        }
        if matches!(&subject, Value::Native(Builtin::SortOf, _)) {
            let tag = if name == self.class_word("call") { Some(41) }
                else if name == self.class_word("allocate") { Some(40) }
                else if name == self.class_word("prepare") { Some(42) }
                else if name == self.class_word("set") { Some(11) }
                else if name == self.class_word("get") { Some(10) }
                else if name == self.class_word("remove") { Some(12) }
                else { None };
            if let Some(tag) = tag { return Ok(Self::adapter(tag, vec![])); }
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
                if *op == Builtin::AsReal && name == "__getformat__" {
                    return Ok(Value::Native(Builtin::ValueMethod, Rc::from("float.__getformat__")));
                }
                if name == self.class_word("bases") && !name.is_empty() {
                    let parent = if *op == Builtin::Bool { self.spelled_kind("int").unwrap_or(Value::Class(self.root_class())) }
                        else { Value::Class(self.root_class()) };
                    return Ok(Value::tuple(vec![parent]));
                }
                if name==self.class_word("module") { return Ok(Value::text(self.home_module_word())); }
                if name==self.class_word("qualified") { return Ok(Value::text(word)); }
                if let Some(size) = self.integer_member(&subject, name) { return Ok(size); }
                if name==self.class_word("allocate") { return Ok(Self::adapter(14, vec![Value::text(word)])); }
                if name==self.class_word("name") || self.lang.class_name.as_deref()==Some(name) { return Ok(Value::text(word)); }
                if name==self.class_word("doc") {
                    if let Some(doc) = Self::builtin_kind_doc(word) { return Ok(Value::text(doc)); }
                }
                if name==self.class_word("namespace") {
                    let mut names=self.kind_special_names(word);
                    names.push(name.to_string());
                    let rows=names.into_iter().map(|key| (Value::text(&key),Value::text(&format!("<attribute '{key}' of '{word}' objects>")))).collect();
                    let book=Value::Map(Rc::new(rows));
                    return Ok(Value::View(Rc::new((book,"mapping".to_string()))));
                }
                // The kind read as a class stands on the root and on
                // nothing else, so that is the whole of its line.
                if name==self.class_word("mro") || name==self.class_word("order") {
                    let listed=name==self.class_word("order");
                    let word=word.to_string();
                    let kind=self.kind_class(&word);
                    let root=self.root_class();
                    let line=Value::tuple(vec![Value::Class(kind),Value::Class(root)]);
                    return Ok(if listed {Self::adapter(0,vec![line])} else {line});
                }
                // Whatever a value of the kind answers to is carried by
                // the kind itself, standing loose: the value it works
                // upon is the first it is called with.
                if let Some(loose)=self.loose_kind_member(&subject,name) { return Ok(loose); }
            }
            Value::Class(c) => {
                if !self.class_word("name").is_empty() {
                    if let Some(maker) = Self::maker_beneath(c) {
                        if let Some(member) = self.class_value(&maker, name).filter(|member| self.takes_writes(member) && (matches!(member, Value::Adapter(w) if matches!(w.0, 6 | 16 | 28)) || self.descriptor_hook(member, "descriptor.get").is_some())) {
                            return self.bind_class_value(member, Some(subject.clone()), maker);
                        }
                    }
                }
                if let Some(word) = Self::own_kind(c) {
                    if name==self.class_word("module") { return Ok(Value::text(self.home_module_word())); }
                    if name==self.class_word("qualified") { return Ok(Value::text(&word)); }
                    if name == "__getformat__" && word == "float" {
                        return Ok(Value::Native(Builtin::ValueMethod, Rc::from("float.__getformat__")));
                    }
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
                if Self::kind_beneath(c).as_deref() == Some("float") && name == "fromhex" {
                    return Ok(Value::ValueMethod(Rc::new((subject.clone(), "float_fromhex".to_string()))));
                }
                if name==self.class_word("name") { return Ok(c.python_names.borrow().as_ref().map_or_else(|| Value::text(&c.name), |names| names.0.clone())); }
                if name==self.class_word("qualified") { return Ok(c.python_names.borrow().as_ref().map(|names| names.1.clone()).unwrap_or_else(|| self.class_value(c,name).unwrap_or_else(|| Value::text(&c.name)))); }
                if name==self.class_word("bases") { return Ok(Value::tuple(c.direct.iter().cloned().map(Value::Class).collect())); }
                if name==self.class_word("namespace") {
                    if let Some(maker)=Self::maker_beneath(c) {
                        if let Some(descriptor)=self.class_value(&maker,name).filter(|entry| self.takes_writes(entry)) {
                            return self.bind_class_value(descriptor,Some(subject.clone()),maker);
                        }
                    }
                    // A class standing for a builtin kind holds no
                    // members of its own; what it names are the ones a
                    // value of the kind answers to.
                    if let Some(word)=Self::own_kind(c) {
                        let named=self.kind_special_names(&word);
                        return Ok(Value::Map(Rc::new(named.iter().map(|n|(Value::text(n),Value::text(&format!("<slot wrapper '{n}' of '{word}' objects>")))).collect())));
                    }
                    return Ok(Self::namespace(&c.shared.borrow()));
                }
                if name==self.class_word("mro") || name==self.class_word("order") {
                    let mut order=vec![subject.clone()]; order.extend(c.lineage.iter().cloned().map(Value::Class));
                    let tuple=Value::tuple(order);
                    return Ok(if name==self.class_word("order") {Self::adapter(0,vec![tuple])} else {tuple});
                }
                if self.lang.class_annotations.first().map_or(false,|word|word==name) { return self.class_annotations(c); }
                if let Some(v)=self.class_value(c,name) { return self.bind_class_value(v,None,c.clone()); }
                if let Some(size) = self.integer_member(&subject, name) { return Ok(size); }
                if let Some(member)=self.loose_kind_member(&subject,name) { return Ok(member); }
                // A class standing on a builtin kind reads that kind's
                // own class method too, bound to the class itself, so
                // that `dictlike.fromkeys` reaches `dict.fromkeys` and
                // hands back a dictlike.
                if self.lang.value_methods.get(name).map(String::as_str) == Some("fromkeys") {
                    if let Some(base) = c.lineage.iter().find(|base| Self::own_kind(base).is_some()) {
                        if self.loose_kind_member(&Value::Class(base.clone()), name).is_some() {
                            return Ok(Value::ValueMethod(Rc::new((subject.clone(), name.to_string()))));
                        }
                    }
                }
                // A class also reads what the metaclass that made it
                // holds, each member bound to the class itself, the way
                // a thing's method is bound to the thing.
                if let Some(maker)=Self::maker_beneath(c) {
                    if let Some(v)=self.class_value(&maker,name) { return self.bind_class_value(v,Some(subject.clone()),maker); }
                }
                // The formatting every class has from the root: a thing
                // and a specification, answered as the format builtin would.
                if self.lang.class_special.get(72).map_or(false,|word|word==name) {return Ok(Self::adapter(19,vec![]));}
                {
                    let index = if name==self.class_word("allocate") {Some(1)}
                        else if self.lang.constructor.as_deref()==Some(name) || name==self.class_word("subclass") {Some(2)}
                        else if name==self.class_word("get") {Some(10)} else if name==self.class_word("set") {Some(11)}
                        else if name==self.class_word("remove") {Some(12)} else {None};
                    if let Some(i)=index {return Ok(Self::adapter(i,vec![]));}
                }
                if let Some(root)=self.root_member(name,None) {return Ok(root);}
            }
            Value::Object(o) => {
                if !plain { if let Some(f)=self.class_value(&o.class_now(),self.class_word("get")) {
                    return self.class_apply(f,vec![subject.clone(),Value::text(name)]);
                } }
                if name == self.class_word("kind") {
                    let actual = o.class_now().clone();
                    if let Some(overridden) = self.class_value(&actual, name) {
                        return self.bind_class_value(overridden, Some(subject.clone()), actual);
                    }
                    return Ok(Value::Class(actual));
                }
                if name==self.class_word("namespace") {
                    if let Some(descriptor)=self.class_value(&o.class_now(),name) {
                        return self.bind_class_value(descriptor,Some(subject.clone()),o.class_now().clone());
                    }
                    // A class that names its slots and leaves the namespace out of them has things without one.
                    if !self.slots_allow(&o.class_now(),name) {return Err(self.missing_member(&subject,name));}
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
                            if let Some((_, value)) = view.fields.borrow().iter().find(|(key, _)| key.as_str() == name) { return Ok(value.clone()); }
                        }
                        _ => {}
                    }
                }
                if let Some((_,v))=o.fields.borrow().iter().find(|(n,_)| n==name) {return Ok(v.clone());}
                if let Some(v)=member {return self.bind_class_value(v,Some(subject.clone()),o.class_now().clone());}
                if self.lang.class_details.get("root.members").and_then(|words| words.get(9)).map_or(false, |word| word == name) {
                    let root = Self::adapter(30, vec![Value::text(name)]);
                    return Ok(Self::adapter(3, vec![root, subject.clone()]));
                }
                if let Some(size) = self.integer_member(&subject, name) { return Ok(size); }
                // The worth a thing keeps answers for the methods of its kind.
                if let Some(worth)=Self::worth_of(&subject).filter(|v|matches!(v.contents(),Value::Set(_))) {
                    if let Some(member)=self.builtin_member(&worth,name)? { return Ok(member); }
                }
                if let Some(word @ Value::Text(_)) = Self::worth_of(&subject) {
                    if let Some(method) = self.builtin_member(&word, name)? { return Ok(method); }
                }
                if let Some(worth)=Self::worth_of(&subject).filter(|v|!matches!(v.contents(),Value::Set(_))) {
                    if matches!(worth.contents(), Value::Complex(_)) && self.native_special(&worth, name) {
                        return Ok(Value::ValueMethod(Rc::new((worth, name.to_string()))));
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
                        return Ok(Value::ValueMethod(Rc::new((worth,op))));
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
                    if matches!(self.lang.builtins.get(name),Some(b) if b.set_method()) {
                        return Ok(Self::adapter(3,vec![Value::text(name),worth]));
                    }
                }
                // A native base's reduction slots must not be replaced by
                // the root's empty-argument reconstruction of an ordinary object.
                let native_reduction = [79, 81].iter().any(|at| self.lang.class_special.get(*at).map_or(false, |word| word == name))
                    && Self::worth_of(&subject).map_or(false, |worth| self.native_special(&worth, name));
                if let Some(root)=self.root_member(name,Some(&o.class_now())).filter(|_| !native_reduction) {
                    // The root's own maker takes the class rather than a
                    // thing, and the hook for a class stood on hears
                    // from the class; any other is bound to the thing.
                    if name==self.class_word("allocate") {return Ok(root);}
                    let receiver=if name==self.class_word("subclass") {Value::Class(o.class_now().clone())} else {subject.clone()};
                    return Ok(Self::adapter(3,vec![root,receiver]));
                }
            }
            Value::Method(o,f) => {
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
                if name==self.class_word("doc") {return Ok(f.doc.clone().map_or(Value::Null,|s|Value::text(&s)));}
                if name==self.class_word("module") {let home=self.routine_module(f);return Ok(self.routine_held(&subject,name,home));}
                if name==self.class_word("defaults") {
                    let values=f.carried.iter().zip(&f.held).filter(|(i,_)| **i<f.formals.len() && f.parameter_rules.as_ref().map_or(true,|rules|rules[**i]<2)).map(|(_,v)|v.clone()).collect::<Vec<_>>();
                    if values.is_empty() && f.least<f.formals.len() && f.within.is_some(){return Err(self.class_refusal());}
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
                    let mut named:Vec<(&str,&Value)>=f.enclosed.iter().map(|(at,cell)|(f.idents.get(*at).map_or("",String::as_str),cell)).collect();
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
                let inner=w.1[0].clone();
                let carried=[self.class_word("module"),self.class_word("qualified"),self.class_word("name"),self.class_word("doc")];
                let copied=carried.iter().any(|word|!word.is_empty()&&*word==name)
                    || self.lang.class_annotations.first().map_or(false,|word|word==name);
                if copied { return self.class_get(inner,name,true); }
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
            Value::Adapter(w) if w.0==29 => {
                if name==self.class_word("qualified") { return Ok(Value::text(&format!("{}.{}", w.1[0].plain(), w.1[1].plain()))); }
                if name==self.class_word("name") { return Ok(w.1[1].clone()); }
                if name=="__objclass__" {
                    if let Some(kind)=self.spelled_kind(&w.1[0].plain()) { return Ok(kind); }
                }
            }
            Value::Adapter(w) if w.0==3 => {
                if name==self.class_word("receiver") {return Ok(w.1[1].clone());}
                if name==self.class_word("function") {return Ok(w.1[0].clone());}
            }
            _ => {}
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
        self.lang.builtins.get(word).copied().filter(|op| Self::kind_builtin(op)).map(|op| Value::Native(op, Rc::from(word)))
    }
    /// The row of type parameters a routine was declared with: for each
    /// name written between the brackets, a holder made by the hinting
    /// module's own maker, in the order the names were written. No
    /// names written, an empty row.
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
        let module = match self.modules.get("typing") {
            Some(held) => held.clone(),
            None => self.import_module("typing")?,
        };
        let maker = match &module {
            Value::Object(space) => space.fields.borrow().iter().find(|(n,_)| n=="TypeVar").map(|(_,v)| match v { Value::Bond(cell)=>cell.borrow().clone(), other=>other.clone() }),
            _ => None,
        };
        maker.ok_or_else(|| self.class_refusal())
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
    /// another kind of program: the flags the two carry differ, which
    /// CPython now says is going away.
    fn code_kind_differs(f: &Rc<Routine>, value: Option<&Value>) -> bool {
        let Some(Value::Adapter(code)) = value.map(Value::contents) else { return false };
        let (7, Some(Value::Routine(source))) = (code.0, code.1.first()) else { return false };
        Self::routine_now(source).code_flags != Self::routine_now(f).code_flags
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
        if self.module_holding(&held).is_some() {
            let word = self.class_word("namespace").to_string();
            return self.class_get(held, &word, false);
        }
        Ok(held)
    }
    /// A member a routine holds of its own: its name, full name or
    /// account of itself as the program wrote them over, the namespace
    /// handed to it, or an entry of whichever namespace it keeps.
    fn routine_member(&self, subject: &Value, name: &str) -> Option<Value> {
        let (_,members)=self.function_members.iter().find(|(v,_)| v.equals(subject))?;
        let fields=members.fields.borrow();
        if name==self.class_word("name") || name==self.class_word("qualified") || name==self.class_word("doc") || self.lang.class_annotations.first().map_or(false, |s| s == name) {
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
        let Some((_,members))=self.function_members.iter().find(|(v,_)| v.equals(subject)) else {return Vec::new()};
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
    fn routine_namespace_write(&mut self, at: usize, value: Option<Value>) -> Flow<Value> {
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
    fn function_storage(&mut self, function: &Value) -> usize {
        if let Some(at) = self.function_members.iter().position(|(v, _)| v.equals(function)) { return at; }
        let class = self.root_class();
        self.made += 1;
        let fields = Rc::new(Instance {replacement_class: RefCell::new(None),  class, fields: RefCell::new(Vec::new()), mark: self.made });
        self.function_members.push((function.clone(), fields));
        self.function_members.len() - 1
    }
    fn namespace(members:&[(String,Value)]) -> Value {Value::Map(Rc::new(members.iter().filter(|(n,_)|!n.starts_with(['\0', '#'])).map(|(n,v)|(Value::text(n),v.clone())).collect()))}
    /// The annotations a class carries: its own, worked out the first
    /// time they are asked for from the routines its body kept and held
    /// from then on. A class whose body annotated nothing has an empty
    /// map of its own; a parent's annotations are never handed down.
    pub(super) fn class_annotations(&mut self,c:&Rc<Class>) -> Flow<Value> {
        let word=self.lang.class_annotations[0].clone();
        if let Some((_,held))=c.shared.borrow().iter().find(|(n,_)|*n==word) { return Ok(held.clone()); }
        // The entry stands in a cell, as a class's members do.
        let kept=c.shared.borrow().iter().find(|(n,_)|n==crate::code::ANNOTATE_WORD).map(|(_,v)|v.contents());
        let mut pairs=Vec::new();
        if let Some(Value::Array(row))=kept {
            for pair in row.chunks(2) {
                let [key,routine]=pair else {break};
                let value=match routine {Value::Routine(_)|Value::Method(..)=>self.class_apply(routine.clone(),Vec::new())?,other=>other.clone()};
                pairs.push((key.clone(),value));
            }
        }
        let made=Value::Map(Rc::new(pairs.into()));
        c.shared.borrow_mut().push((word,made.clone()));
        Ok(made)
    }
    pub(super) fn class_write(&mut self, subject:Value, name:&str, value:Option<Value>, plain:bool) -> Flow<Value> {
        if !self.class_word("name").is_empty() && !matches!(&subject, Value::Class(_)) {
            if let Some(kind) = self.kind_word_of(&subject.contents()) {
                return Err(format!("TypeError: cannot set '{name}' attribute of immutable type '{kind}'").into());
            }
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
                // A member that takes writes takes this one: a slot keeps
                // the value, a property's kept accessor refuses, and any
                // other asks its class's writer or remover.
                if let Some(member)=self.class_value(&o.class_now(),name) {
                    if let Value::Adapter(w)=&member {
                        if w.0==16 {return self.slot_write(&subject,&w.1,value);}
                        if w.0==28 {return Err(self.class_word("property.readonly").to_string().into());}
                    }
                    if self.takes_writes(&member) {
                        let part=if value.is_some(){"descriptor.set"}else{"descriptor.delete"};
                        let hook=self.descriptor_hook(&member,part).ok_or_else(||self.missing_member(&subject,name))?;
                        let mut args=vec![subject.clone()];args.extend(value);
                        self.call_descriptor(&member,hook,args)?;
                        return Ok(Value::Null);
                    }
                }
                // A thing's own namespace, written back to it after an
                // entry was put in, is where it was: nothing to do.
                if name==self.class_word("namespace") {
                    if let Some(Value::Fields(view))=&value {if Rc::ptr_eq(view,o){return Ok(Value::Null);}}
                }
                // A thing's namespace taken away leaves it with an empty
                // one, and a dictionary handed to it becomes its entries.
                if name==self.class_word("namespace") {
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
                if name == self.class_word("kind") {
                    let Some(Value::Class(next)) = value else { return Err("TypeError: __class__ must be set to a class".to_string().into()); };
                    let old = o.class_now();
                    if Self::own_class_value(&old, self.class_word("module")).is_none() || Self::own_class_value(&next, self.class_word("module")).is_none()
                        || Self::kind_beneath(&old) != Self::kind_beneath(&next) || self.slots_named(&old) || self.slots_named(&next) {
                        return Err("TypeError: __class__ assignment: object layout differs".to_string().into());
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
                let module=self.modules.values().any(|held|matches!(held,Value::Object(space) if Rc::ptr_eq(space,o)));
                Self::write_members(&mut o.fields.borrow_mut(),name,value,module).map_err(|_|absent)?;
            }
            Value::Class(c) => {
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
                if Self::class_sealed(c) || (c.python_names.borrow().is_none() && ["name", "qualified", "doc"].iter().any(|key| !self.class_word(key).is_empty() && name == self.class_word(key))) {
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
                if name == self.class_word("qualified") {
                    match value.as_ref().map(Value::contents) {
                        Some(Value::Text(_)) => {},
                        Some(other) => return Err(format!("TypeError: can only assign string to {}.__qualname__, not '{}'", c.name, other.core_kind()).into()),
                        None => return Err(self.class_refusal()),
                    }
                } else if ["name","kind","bases","mro","namespace","order"].iter().any(|key|name==self.class_word(key)){return Err(self.class_refusal());}
                if name == self.class_word("module") {
                    c.shared.borrow_mut().retain(|(member, _)| member != "__firstlineno__");
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
                if self.lang.class_annotations.first().map_or(false, |s| s == name) {
                    let given = match value.as_ref().map(Value::contents) {
                        None | Some(Value::Null) => self.keep_collection(Value::Map(Rc::new(Vec::new().into()))),
                        Some(Value::Map(_)) => value.unwrap(),
                        _ => return Err(format!("TypeError: {name} must be set to a dict object").into()),
                    };
                    let at = self.function_storage(&subject);
                    Self::write_members(&mut self.function_members[at].1.fields.borrow_mut(), &format!("\0{name}"), Some(given), false).map_err(|_| absent)?;
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
            Value::Method(..) => {
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
            (None,Some(v))=>members.push((name.into(),v)),(Some(i),None)=>{members.remove(i);},_=>return Err(())} Ok(())
    }
    /// Whether the class, or one it stands on, names the members its
    /// things may hold. Such a thing keeps no namespace of its own.
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
    fn slots_allow(&self,c:&Class,name:&str)->bool {
        let own=Self::own_class_value(c,self.class_word("slots"));
        let Some(own)=own else{return Self::own_kind(c).is_none();};
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
        512 + if Self::class_sealed(c) { 256 } else { 1024 } + if dictionary { 16 } else { 0 } + if inline { 4 } else { 0 } + if tracked { 16384 } else { 0 }
    }
    fn type_base(&mut self, value: &Value) -> Flow<Rc<Class>> {
        match value.contents() {
            Value::Class(class) if Self::class_sealed(&class) => Err(format!("TypeError: type '{}' is not an acceptable base type", class.name).into()),
            Value::Class(class) => Ok(class),
            Value::ByteKind(mutable, _) if Lang::spells(&self.lang.builtin_bases, self.byte_kind_word(mutable)) => {
                let word = self.byte_kind_word(mutable).to_string();
                Ok(self.kind_class(&word))
            }
            Value::Native(Builtin::SortOf, _) => Ok(self.metaclass_root()),
            Value::Native(Builtin::Bool, _) => Err(self.lang.bool_base.clone().unwrap_or_default().into()),
            Value::Native(_, name) if Lang::spells(&self.lang.builtin_bases, &name) => Ok(self.kind_class(&name)),
            _ => Err("TypeError: bases must be types".to_string().into()),
        }
    }
    pub(super) fn class_type(&mut self,args:Vec<Value>)->Flow<Value> {
        let original_title = args.first().map(Value::contents);
        let mut args: Vec<Value> = args.iter().map(Value::contents).collect();
        if args.len() == 3 {
            args[0] = Value::text(&self.type_title(&args[0])?);
            let original = args[2].clone();
            if let Some(worth) = Self::worth_of(&original) { args[2] = worth.contents(); }
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
            [Value::Adapter(w)] if w.0==29=>Ok(self.named_kind(&args[0])),
            [Value::Routine(_)]|[Value::Method(..)]=>Ok(self.named_kind(&args[0])),
            [Value::Adapter(w)] if matches!(w.0,7|14|31|32|119)=>Ok(self.named_kind(&args[0])),
            [Value::Text(name),Value::Tuple(bases),Value::Map(members)] => {
                let mut parents=vec![];for b in bases.iter(){parents.push(self.type_base(b)?);}
                let mut own=vec![];for (k,v) in members.iter(){if let Value::Text(n)=k{own.push((n.to_string(),v.clone()));}else{return Err(self.class_refusal());}}
                self.form_named_class(original_title.unwrap_or_else(|| Value::text(name)),parents,own)
            }
            _=>Err("TypeError: type() requires a name, a tuple of bases, and a dict".to_string().into()),
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
        let told=self.class_apply(bound,vec![given.clone()])?;
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
    /// Whether a value may stand as one member of a union built by
    /// `|`: a kind itself, `None` (which the sign reads as the kind
    /// `None` alone is of), or a tuple of further such members, the
    /// shape a union already takes once built. A tuple built some
    /// other way, holding a value that is none of these, is not one.
    pub(super) fn union_member(&self,value:&Value)->bool {
        matches!(value,Value::Null)
            ||self.stands_for_kind(value)
            ||matches!(value,Value::Tuple(items) if items.iter().all(|v|self.union_member(v)))
    }
    /// Whether a value is enough on its own to tell `|` a union is
    /// meant rather than some other working: a kind by itself, or a
    /// tuple already built as one (non-empty, every member a union
    /// member). `None` alone answers to neither, the way CPython's
    /// own `NoneType` defines no `__or__`/`__ror__` of its own.
    pub(super) fn union_anchor(&self,value:&Value)->bool {
        self.stands_for_kind(value)
            ||matches!(value,Value::Tuple(items) if !items.is_empty()&&items.iter().all(|v|self.union_member(v)))
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
        let word=if self.is_async_generator(value) { String::from("async_generator") } else { match self.module_holding(value) {Some(_)=>String::from("module"),None=>value.core_kind()} };
        // Where the definition spells that very kind, its builtin word
        // is the answer, so that a kind asked for and a kind answered
        // with are the one value: `type(enumerate(r)) is enumerate`.
        if let Some(op)=self.lang.builtins.get(&word).copied().filter(Self::kind_builtin) {
            return Value::Native(op,Rc::from(word.as_str()));
        }
        Value::Class(self.kind_class(&word))
    }
    /// Whether a value stands as a class at all: one the program wrote,
    /// or a builtin word that names a kind.
    fn stands_as_class(&self,value:&Value)->bool {
        matches!(value,Value::Class(_))||self.kind_spelled(value).is_some()
    }
    /// One builtin kind stands beneath another only where it is that
    /// very kind, or where it is the flag kind, which stands under the
    /// whole-number kind as the language counts a flag a number.
    fn kinds_beneath(&self,under:&str,over:&str)->bool {
        under==over
            || (self.lang.builtins.get(under)==Some(&Builtin::Bool) && self.lang.builtins.get(over)==Some(&Builtin::ToInt))
    }
    /// Whether a value is of a builtin kind. A walk is known by the name
    /// its kind is told by, which is the word that made it.
    pub(super) fn kind_holds(&self,op:&Builtin,word:&str,value:&Value)->bool {
        match op {
            Builtin::ToInt=>matches!(value,Value::Small(_)|Value::Huge(_)|Value::Flag(_)),
            Builtin::ToText=>matches!(value,Value::Text(_) | Value::Codepoints(_)),
            Builtin::AsReal=>matches!(value,Value::Real(_)),
            Builtin::List=>matches!(value,Value::Array(_)),
            Builtin::SortOf=>matches!(value,Value::Class(_)|Value::Native(..)|Value::ByteKind(..)|Value::SortOf(_))||self.kind_spelled(value).is_some(),
            Builtin::Dict=>matches!(value,Value::Map(_)|Value::Fields(_)),
            Builtin::Tuple=>matches!(value,Value::Tuple(_)),
            Builtin::Set=>matches!(value,Value::Set(_))&&!value.set_fixed(),
            Builtin::Frozen=>value.set_fixed(),
            Builtin::Bool=>matches!(value,Value::Flag(_)),
            Builtin::Complex=>matches!(value,Value::Complex(_)),
            Builtin::Span=>matches!(value,Value::Counted(_)),
            Builtin::MakeSlice=>matches!(value,Value::Slice(_)),
            Builtin::Bytes(m)=>matches!(value,Value::Bytes(_,mutable,_) if *mutable==(*m==1)),
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
    fn beneath(&mut self,value:&Value,wanted:&Value,subclass:bool)->Flow<bool> {
        if matches!(wanted, Value::Object(o) if o.class_now().name == "GenericAlias") {
            return Err("TypeError: isinstance() argument 2 cannot be a parameterized generic".into());
        }
        if let Some(told)=self.maker_answers(wanted,value,subclass)? { return Ok(told); }
        // The bytes kinds stand as values of their own rather than as
        // builtin words, so each is asked about under its own word.
        if let Value::ByteKind(mutable, _) = value { let word=self.byte_kind_word(*mutable).to_string(); return self.beneath(&Self::adapter(8, vec![Value::text(&word)]), wanted, subclass); }
        if let Value::ByteKind(mutable, _) = wanted { let word=self.byte_kind_word(*mutable).to_string(); return self.beneath(value, &Self::adapter(8, vec![Value::text(&word)]), subclass); }
        if let Value::Native(_, word) = value { return self.beneath(&Self::adapter(8, vec![Value::text(word)]), wanted, subclass); }
        if let Value::Native(_, word) = wanted { return self.beneath(value, &Self::adapter(8, vec![Value::text(word)]), subclass); }
        if let Value::Array(v)|Value::Tuple(v)=wanted {for c in v.iter(){if self.beneath(value,c,subclass)?{return Ok(true);}}return Ok(false);}
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
            if subclass && !self.stands_as_class(value){return Err(self.unclassed("core.issubclass.subject"));}
            // Everything stands beneath the class every other one does.
            if c.name==self.class_word("root"){return Ok(true);}
            if !subclass && !matches!(value, Value::Object(_)) {
                if let Some(word)=Self::own_kind(c) { return Ok(value.core_kind()==word); }
            }
            let kind=match value {Value::Object(o) if !subclass=>Some(&o.class_now()),Value::Class(c) if subclass=>Some(c),_=>None};
            return Ok(kind.map_or(false,|k|Rc::ptr_eq(k,c)||k.lineage.iter().any(|b|Rc::ptr_eq(b,c))));
        }
        if let Value::Adapter(w)=wanted {
            if w.0==8 {if let Value::Text(word)=&w.1[0] {
                let Some(builtin)=self.lang.builtins.get(word.as_ref()).copied().filter(Self::kind_builtin) else{return Err(self.unclassed(amiss));};
                if subclass{
                    if let Value::Class(c)=value{return Ok(Self::kind_beneath(c).as_deref()==Some(word.as_ref()));}
                    let Some(under)=self.kind_spelled(value) else{return Err(self.unclassed("core.issubclass.subject"));};
                    return Ok(self.kinds_beneath(&under,word));
                }
                // A thing of a class standing on the kind is of the kind.
                if let Value::Object(o)=value{return Ok(Self::kind_beneath(&o.class_now()).as_deref()==Some(word.as_ref()));}
                return Ok(self.kind_holds(&builtin,word,value));
            }}
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
        let one=args.first().cloned().unwrap_or(Value::Null);
        match which {
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
            2 if args.len()==1=>Ok(Value::Flag(matches!(one,Value::Class(_)|Value::Routine(_)|Value::Method(..))||matches!(&one,Value::Adapter(w) if matches!(w.0,0..=4|8..=12|15|17..=27|29|30|40..=42))||matches!(&one,Value::Object(o) if self.class_value(&o.class_now(),self.class_word("call")).is_some()))),
            // getattr and hasattr want the receiver and a name, and take
            // a name of any kind but a string only to say so.
            3|6 if args.len()>=2=>{
                // A name standing on text is asked after as the text it keeps.
                let asked=match &args[1] {Value::Object(_)=>Self::worth_of(&args[1]).map(|w|w.contents()).filter(|w|matches!(w,Value::Text(_))),_=>None}.unwrap_or_else(||args[1].clone());
// A text keeping a lone surrogate names a member too;
                // no member's name keeps one, so it is asked after as
                // the stand-in text such a row reads as elsewhere.
                let asked=match &asked {Value::Codepoints(row)=>Value::text(&Value::predicate_text(row)),other=>other.clone()};
                let Value::Text(name)=&asked else{return Err(self.core_fault("core.attribute.name",&args[1].core_kind()).into());};match self.class_get(one.clone(),name,false){Ok(v)=>Ok(if which==6{Value::Flag(true)}else if self.lang.syntax_members.is_empty(){match v{Value::Bond(cell)=>cell.borrow().clone(),held=>held}}else{let keep=match &v{Value::Bond(cell)=>matches!(&*cell.borrow(),Value::Array(_)|Value::Set(_)|Value::SetWalk(..)|Value::Map(_)|Value::Object(_)|Value::Fields(_)|Value::Bytes(..)),_=>false};if keep{v}else{match v{Value::Bond(cell)=>cell.borrow().clone(),held=>held}}}),Err(fault) if self.attribute_fault(&fault)=>if which==6{Ok(Value::Flag(false))}else if args.len()==3{Ok(args[2].clone())}else{
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
                if let Value::Object(thing) = &one {
                    let class = thing.class_now();
                    if class.lineage.iter().any(|base| base.name == "ModuleType") {
                        if let Some(value) = Self::own_class_value(&class, self.class_word("namespace")) {
                            if !matches!(value.contents(), Value::Map(_)) { return Err("TypeError: <module>.__dict__ is not a dictionary".into()); }
                        }
                    }
                }
                if let Value::Object(module) = &one {
                    let class=module.class_now();
                    let module_kind=class.name == "ModuleType" || class.lineage.iter().any(|base| base.name == "ModuleType");
                    let class_directory=self.lang.class_special.get(75)
                        .and_then(|word| self.class_value(&class,word)).is_some();
                    if (module_kind || self.module_holding(&one).is_some()) && !class_directory {
                        let entries=Self::fields_entries(module);
                        if let Some(word)=self.lang.class_special.get(75) {
                            if let Some((_,method))=entries.iter().find(|(key,_)| key.plain()==*word) {
                                let answer=self.class_apply(method.clone(),Vec::new())?;
                                let names=self.special_items(&answer).map_err(Fault::Note)?;
                                let ordered=self.steady_order(names,&Value::Null,false).map_err(Fault::Note)?;
                                return Ok(Value::array(ordered));
                            }
                        }
                        let mut names: Vec<_> = entries.into_iter()
                            .map(|(key, _)| key.plain()).collect();
                        names.sort();
                        return Ok(Value::array(names.iter().map(|name| Value::text(name)).collect()));
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
                Ok(self.default_directory(&one))
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
    pub(super) fn default_directory(&self, one: &Value) -> Value {
        // A routine lists the members it can honestly answer
        // for, alongside any it was given of its own.
        if let Value::Routine(_)|Value::Method(..)=one {
            let mut names:Vec<String>=vec![];
            for part in ["name","qualified","doc","module","defaults","call"] {
                let word=self.class_word(part);
                if !word.is_empty() { names.push(word.to_string()); }
            }
            let routine=match one {Value::Method(_,f)=>Value::Routine(f.clone()),other=>other.clone()};
            names.extend(self.routine_member_names(&routine));
            names.sort();names.dedup();
            return Value::array(names.iter().map(|n|Value::text(n)).collect());
        }
                if matches!(one, Value::Adapter(parts) if parts.0 == 32) {
                    let mut names = Vec::new();
                    names.extend(self.lang.async_generator_methods.iter().skip(3).cloned());
                    names.extend([15, 16].iter().filter_map(|index| self.lang.class_special.get(*index).cloned()));
                    for words in [&self.lang.yield_send, &self.lang.yield_throw, &self.lang.yield_close] {
                        names.extend(words.iter().cloned());
                    }
                    names.sort(); names.dedup();
                    return Value::array(names.iter().map(|name| Value::text(name)).collect());
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
                        return Value::array(names.iter().map(|n|Value::text(n)).collect());
                    }
            for words in [&self.lang.yield_close,&self.lang.yield_send,&self.lang.yield_throw,&self.lang.yield_running] {
                if let Some(w)=words.first() { names.push(w.clone()); }
            }
            for word in [14, 15, 21, 24].iter().filter_map(|i| self.lang.trace_fields.get(*i).cloned()).filter(|word| !word.is_empty()) {
                if !word.is_empty() { names.push(word); }
            }
            names.sort();names.dedup();
            return Value::array(names.iter().map(|n|Value::text(n)).collect());
        }
        // A builtin kind, named as the kind itself or held as a
        // value of one, answers the members a value of that kind
        // has: the methods of the kind and the special names its
        // family answers to.
        if let Some(sample)=self.dir_sample(one) {
            let names=self.kind_member_names(&sample);
            return Value::array(names.iter().map(|n|Value::text(n)).collect());
        }
        let mut names=vec![];let class=match one{Value::Class(c)=>Some(c),Value::Object(o)=>{names.extend(o.fields.borrow().iter().filter(|(n,_)|!n.starts_with(['\0', '#'])).map(|(n,_)|n.clone()));Some(&o.class_now())},_=>None};
        if let Some(c)=class {
            for b in std::iter::once(c).chain(c.lineage.iter()) {
                names.extend(b.shared.borrow().iter().filter(|(n,_)|!n.starts_with(['\0', '#'])).map(|(n,_)|n.clone()));
                if let Some(sample) = Self::own_kind(b).and_then(|word| self.kind_sample(&word)) { names.extend(self.kind_member_names(&sample)); }
            }
        }
        else {names.extend(self.routine_member_names(one));}
        if matches!(one, Value::Object(_)) && !names.iter().any(|n| n == self.class_word("kind")) {
            names.extend(self.lang.class_details.get("root.members").into_iter().flatten().cloned());
        }
        names.sort();names.dedup();Value::array(names.iter().map(|n|Value::text(n)).collect())
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
        let mut sequence=vec![receiver.clone()];sequence.extend(receiver.lineage.iter().cloned());
        let mut at=sequence.iter().position(|c|owned(self,c));
        // A method of a metaclass is written in the metaclass, not in the
        // class it was given, so its forebears are the metaclass's own.
        if at.is_none() {
            if let Some(maker)=Self::maker_beneath(&receiver) {
                sequence=vec![maker.clone()];sequence.extend(maker.lineage.iter().cloned());
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
                if self.lang.constructor.as_deref()==Some(name){
                    if let Some(worth) = Self::worth_of(&subject).filter(|held| matches!(held.contents(), Value::Set(_) | Value::Array(_))) {
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
                continue;
            }
            if let Some(f)=Self::own_class_value(c,name){let mut all=if name==self.class_word("allocate"){vec![]}else{vec![subject.clone()]};all.extend(args);return self.class_apply(f,all);}
            if self.exception_class(c) && self.lang.constructor.as_deref() == Some(name) {
                if let Value::Object(o) = &subject { return self.exception_method(o.clone(), name, &args); }
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
