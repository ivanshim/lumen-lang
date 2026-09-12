// Classes whose ancestry is an ordered company, and the members which
// belong to classes and routines rather than to their callers.
use super::*;

impl<'a> Engine<'a> {
    pub(super) fn class_word(&self, part: &str) -> &str {
        self.lang.class_details.get(part).and_then(|v| v.first()).map_or("", String::as_str)
    }
    pub(super) fn fuller_classes(&self) -> bool { !self.class_word("root").is_empty() }
    fn class_refusal(&self) -> Fault { self.class_word("unready").to_string().into() }
    pub(super) fn root_class(&mut self) -> Rc<Class> {
        if let Some(c) = &self.class_root { return c.clone(); }
        let name = self.class_word("root").to_string();
        let c = Rc::new(Class { outline: Some(format!("<class '{name}'>")), name,
            direct: vec![], lineage: vec![], base: None, answers: vec![], fields: vec![], reaches: vec![],
            methods: vec![], constants: vec![], shared: RefCell::new(vec![]) });
        self.class_root = Some(c.clone());
        c
    }
    /// The class standing for a builtin kind, made once for each word
    /// the definition names: a thing of a class beneath it keeps a worth
    /// of that kind among its members, under a name no program can spell.
    pub(super) fn kind_class(&mut self, word: &str) -> Rc<Class> {
        if let Some((_, c)) = self.kind_classes.iter().find(|(w, _)| w == word) { return c.clone(); }
        let root = self.root_class();
        let c = Rc::new(Class { outline: Some(format!("<class '{word}'>")), name: word.to_string(),
            direct: vec![root.clone()], lineage: vec![root.clone()], base: Some(root), answers: vec![], fields: vec![], reaches: vec![],
            methods: vec![], constants: vec![("\0kind".to_string(), Value::text(word))], shared: RefCell::new(vec![]) });
        self.kind_classes.push((word.to_string(), c.clone()));
        c
    }
    /// The builtin kind a class itself stands for, if it is one.
    fn own_kind(c: &Class) -> Option<String> {
        c.constants.iter().find(|(n, _)| n == "\0kind").map(|(_, v)| v.plain())
    }
    /// The builtin kind a class stands on, through any of its line.
    pub(super) fn kind_beneath(c: &Class) -> Option<String> {
        std::iter::once(c).chain(c.lineage.iter().map(Rc::as_ref)).find_map(Self::own_kind)
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
        let Some(op) = self.lang.builtins.get(word).copied() else { return Err(self.class_refusal()); };
        let items = self.call_items(args)?;
        let made = self.builtin_call(op, word, items)?;
        let kept = match made.contents() {
            held @ (Value::Array(_) | Value::Map(_)) => Value::Collection(Rc::new(RefCell::new(held)), true),
            other => other,
        };
        self.made += 1;
        Ok(Value::Object(Rc::new(Instance { class: c, fields: RefCell::new(vec![("\0worth".to_string(), kept)]), mark: self.made })))
    }
    pub(super) fn form_class(&mut self, name: String, mut bases: Vec<Rc<Class>>, mut members: Vec<(String, Value)>) -> Flow<Value> {
        if bases.is_empty() { bases.push(self.root_class()); }
        // The keywords the header carried are no members: they go by
        // name to the forebear's subclass hook.
        let mut carried = Vec::new();
        members.retain(|(n, v)| match n.strip_prefix("\0keyword:") {
            Some(word) => { carried.push(Value::Tie(Rc::new((Value::text(word), v.clone())))); false }
            None => true,
        });
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
        let module = self.class_word("main").to_string();
        if !members.iter().any(|(n,_)| n == self.class_word("module")) {
            members.push((self.class_word("module").to_string(), Value::text(&module)));
        }
        let display=members.iter().find(|(n,_)|n==self.class_word("qualified")).map(|(_,v)|v.plain()).unwrap_or_else(||name.clone());
        let c = Rc::new(Class { name: name.clone(), outline: Some(format!("<class '{module}.{display}'>")),
            base: bases.first().cloned(), direct: bases, lineage, answers: vec![], fields: vec![], reaches: vec![],
            methods: vec![], constants: vec![], shared: RefCell::new(members) });
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
        let Some(slots) = Self::own_class_value(c, self.class_word("slots")) else { return Ok(()) };
        let named: Vec<Value> = match slots.contents() { Value::Tuple(v) | Value::Array(v) => v.as_ref().clone(), single => vec![single] };
        for slot in named {
            let Value::Text(word) = slot else { return Err(self.class_refusal()) };
            if word.as_ref() == self.class_word("namespace") { continue; }
            if Self::own_class_value(c, &word).is_some() { return Err(self.class_refusal()); }
            c.shared.borrow_mut().push((word.to_string(), Self::adapter(16, vec![Value::Text(word.clone()), Value::Class(c.clone())])));
        }
        Ok(())
    }
    /// Where a slot's value is kept in a thing: under the slot's name and
    /// the class that declared it. A thing not of that class has no such
    /// place, and the descriptor says so.
    fn slot_place(&self, thing: &Value, parts: &[Value]) -> Flow<String> {
        let (Value::Object(o), Some(Value::Class(owner))) = (thing, parts.get(1)) else { return Err(self.class_refusal()) };
        let word = parts[0].plain();
        if !Rc::ptr_eq(&o.class, owner) && !o.class.lineage.iter().any(|b| Rc::ptr_eq(b, owner)) {
            let pieces = self.lang.class_details.get("descriptor.foreign").cloned().unwrap_or_default();
            if pieces.len() != 4 { return Err(self.class_refusal()); }
            return Err(format!("{}{word}{}{}{}{}{}", pieces[0], pieces[1], owner.name, pieces[2], o.class.name, pieces[3]).into());
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
            methods: vec![], constants: vec![], shared: RefCell::new(members) });
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
        let of = match thing { Value::Object(o) => o.class.name.clone(), Value::Class(c) => c.name.clone(), other => other.plain() };
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
                Ok(Value::Object(Rc::new(Instance { class: property.class.clone(), fields: RefCell::new(fields), mark: self.made })))
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
        self.class_value(&o.class, word)
    }
    fn call_descriptor(&mut self, member: &Value, hook: Value, args: Vec<Value>) -> Flow<Value> {
        let Value::Object(o) = member else { return Err(self.class_refusal()) };
        let bound = self.bind_class_value(hook, Some(member.clone()), o.class.clone())?;
        self.class_apply(bound, args)
    }
    /// A member that takes writes as well as reads: it is asked before a
    /// thing's own fields, where one that only reads gives way to them.
    fn takes_writes(&self, member: &Value) -> bool {
        if let Value::Adapter(w) = member { return matches!(w.0, 6 | 16 | 28); }
        self.descriptor_hook(member, "descriptor.set").is_some() || self.descriptor_hook(member, "descriptor.delete").is_some()
    }
    /// Whether a fault is a missing member's, however it was raised.
    fn attribute_fault(&self, fault: &Fault) -> bool {
        let kind = self.class_word("attribute.amiss").split(':').next().unwrap_or_default();
        if kind.is_empty() { return false; }
        match fault {
            Fault::Note(told) => told.split(':').next() == Some(kind),
            Fault::Thrown(Value::Object(o)) => std::iter::once(&o.class).chain(o.class.lineage.iter()).any(|c| c.name == kind),
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
    fn adapter(kind: u8, values: Vec<Value>) -> Value { Value::Adapter(Rc::new((kind,values))) }
    pub(super) fn class_apply(&mut self, callable: Value, mut args: Vec<Value>) -> Flow<Value> {
        match callable {
            Value::Routine(p) => { self.invoke(&p,args)?; Ok(self.drop_top()?) }
            Value::Method(o,p) => { args.insert(0,Value::Object(o)); self.invoke(&p,args)?; Ok(self.drop_top()?) }
            Value::Object(o) => {let f=self.class_value(&o.class,self.class_word("call")).ok_or_else(||self.class_refusal())?;args.insert(0,Value::Object(o));self.class_apply(f,args)},
            Value::Class(c) => self.class_make(c,args),
            Value::Adapter(w) => match w.0 {
                0 => Ok(w.1[0].clone()),
                1 => {
                    if args.len()!=1{return Err(self.class_refusal());}
                    let Some(Value::Class(c)) = args.first() else { return Err(self.class_refusal()); };
                    self.made += 1;
                    Ok(Value::Object(Rc::new(Instance {class:c.clone(),fields:RefCell::new(vec![]),mark:self.made})))
                }
                2 if args.len()==1 => Ok(Value::Null),
                // The maker of a builtin kind: given the class to make a
                // thing of and what the kind's builtin takes.
                14 if !args.is_empty() => {
                    let Value::Class(c) = args.remove(0) else { return Err(self.class_refusal()); };
                    let word = w.1[0].plain();
                    self.thing_of_kind(c, &word, args)
                }
                3 => { args.insert(0,w.1[1].clone()); self.class_apply(w.1[0].clone(),args) }
                4 | 8 => self.class_apply(w.1[0].clone(),args),
                13 if args.len() == 1 => {
                    let Value::Adapter(property) = &w.1[0] else { return Err(self.class_refusal()); };
                    let mut members = property.1.clone();
                    members.resize(2, Value::Null); members[1] = args.remove(0);
                    Ok(Self::adapter(6, members))
                }
                10..=12 => {
                    let Some(subject) = args.first().cloned() else { return Err(self.class_refusal()); };
                    let Some(Value::Text(name)) = args.get(1) else { return Err(self.class_refusal()); };
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
                        (_, Some(Value::Object(o))) => o.class.clone(),
                        (_, Some(_)) => self.root_class(),
                        (_, None) => return Err(self.class_refusal()),
                    };
                    self.bind_class_value(w.1[0].clone(), thing, owner)
                }
                17 if args.len() == 2 => self.slot_write(&args[0], &w.1, Some(args[1].clone())),
                18 if args.len() == 1 => self.slot_write(&args[0], &w.1, None),
                19 if args.len() == 2 => {
                    let Value::Text(spec) = &args[1] else { return Err(self.class_refusal()) };
                    let spec = spec.to_string();
                    self.special_format(&args[0], &spec).map(|shown| Value::text(&shown)).map_err(Fault::Note)
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
            _ => Err(self.class_refusal()),
        }
    }
    pub(super) fn class_make(&mut self, c: Rc<Class>, args: Vec<Value>) -> Flow<Value> {
        let allocation = self.class_value(&c,self.class_word("allocate"));
        let kind = Self::kind_beneath(&c);
        let object = if let Some(f) = allocation {
            let mut given = vec![Value::Class(c.clone())]; given.extend(args.clone());
            self.class_apply(f,given)?
        } else if let Some(word) = &kind {
            self.thing_of_kind(c.clone(), word, args.clone())?
        } else {
            self.made += 1;
            Value::Object(Rc::new(Instance {class:c.clone(),fields:RefCell::new(vec![]),mark:self.made}))
        };
        if let Value::Object(o) = &object {
            if Rc::ptr_eq(&o.class,&c) || o.class.lineage.iter().any(|b| Rc::ptr_eq(b,&c)) {
                let init = self.lang.constructor.as_deref().and_then(|n| self.class_value(&o.class,n));
                if let Some(f) = init {
                    let bound=self.bind_class_value(f,Some(object.clone()),o.class.clone())?;
                    let answer = self.class_apply(bound,args)?;
                    if !matches!(answer,Value::Null) { return Err(self.class_refusal()); }
                } else if !args.is_empty() && kind.is_none() { return Err(self.class_refusal()); }
            }
        }
        Ok(object)
    }
    fn missing_member(&self, subject: &Value, name: &str) -> Fault {
        let class = match subject { Value::Object(o)=>o.class.name.as_str(), Value::Class(c)=>c.name.as_str(), _=>"function" };
        let pieces = self.lang.class_details.get("attribute.amiss").cloned().unwrap_or_default();
        if pieces.len()!=3 { return self.class_refusal(); }
        format!("{}{class}{}{name}{}",pieces[0],pieces[1],pieces[2]).into()
    }
    pub(super) fn bind_class_value(&mut self, value: Value, subject: Option<Value>, class: Rc<Class>) -> Flow<Value> {
        if let Value::Adapter(w) = &value {
            return match w.0 {
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
    /// A member read that ends in a missing member -- whether the class's
    /// own reading hook said so, or a property's getter, or nothing was
    /// found -- is offered to the class's fallback reader before it is
    /// reported. A plain read, the root's own, has no fallback.
    pub(super) fn class_get(&mut self, subject: Value, name: &str, plain: bool) -> Flow<Value> {
        let answer = self.class_read(subject.clone(), name, plain);
        if plain { return answer; }
        let Err(fault) = &answer else { return answer };
        let Value::Object(o) = &subject else { return answer };
        if !self.attribute_fault(fault) { return answer; }
        let Some(reader) = self.lang.reader.as_deref().and_then(|n| self.class_value(&o.class, n)) else { return answer };
        let bound = self.bind_class_value(reader, Some(subject.clone()), o.class.clone())?;
        self.class_apply(bound, vec![Value::text(name)])
    }
    fn class_read(&mut self, subject: Value, name: &str, plain: bool) -> Flow<Value> {
        if let Value::Adapter(property) = &subject {
            if property.0 == 6 && Lang::spells(&self.lang.property_setter, name) { return Ok(Self::adapter(13, vec![subject])); }
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
        match &subject {
            // A builtin kind's word, read as a class: its maker, and its name.
            Value::Native(_, word) if Lang::spells(&self.lang.builtin_bases, word) => {
                if name==self.class_word("allocate") { return Ok(Self::adapter(14, vec![Value::text(word)])); }
                if name==self.class_word("name") || self.lang.class_name.as_deref()==Some(name) { return Ok(Value::text(word)); }
            }
            Value::Class(c) => {
                if name==self.class_word("name") { return Ok(Value::text(&c.name)); }
                if name==self.class_word("qualified") { return Ok(self.class_value(c,name).unwrap_or_else(|| Value::text(&c.name))); }
                if name==self.class_word("bases") { return Ok(Value::Tuple(Rc::new(c.direct.iter().cloned().map(Value::Class).collect()))); }
                if name==self.class_word("namespace") { return Ok(Self::namespace(&c.shared.borrow())); }
                if name==self.class_word("mro") || name==self.class_word("order") {
                    let mut order=vec![subject.clone()]; order.extend(c.lineage.iter().cloned().map(Value::Class));
                    let tuple=Value::Tuple(Rc::new(order));
                    return Ok(if name==self.class_word("order") {Self::adapter(0,vec![tuple])} else {tuple});
                }
                if let Some(v)=self.class_value(c,name) { return self.bind_class_value(v,None,c.clone()); }
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
            }
            Value::Object(o) => {
                if !plain { if let Some(f)=self.class_value(&o.class,self.class_word("get")) {
                    return self.class_apply(f,vec![subject.clone(),Value::text(name)]);
                } }
                if name==self.class_word("kind") {return Ok(Value::Class(o.class.clone()));}
                if name==self.class_word("namespace") {
                    // A class that names its slots and leaves the namespace out of them has things without one.
                    if !self.slots_allow(&o.class,name) {return Err(self.missing_member(&subject,name));}
                    return Ok(Value::Fields(o.clone()));
                }
                let member=self.class_value(&o.class,name);
                // A member that takes writes speaks before the thing's own
                // fields; any other member speaks after them.
                if member.as_ref().map_or(false,|m|self.takes_writes(m)) {return self.bind_class_value(member.unwrap(),Some(subject.clone()),o.class.clone());}
                if let Some((_,v))=o.fields.borrow().iter().find(|(n,_)| n==name) {return Ok(v.clone());}
                if let Some(v)=member {return self.bind_class_value(v,Some(subject.clone()),o.class.clone());}
                // The worth a thing keeps answers for the methods of its kind.
                if let (Some(worth),Some(op))=(Self::worth_of(&subject),self.lang.value_methods.get(name).cloned()) {
                    return Ok(Value::ValueMethod(Rc::new((worth,op))));
                }
            }
            Value::Method(o,f) => {
                if name==self.class_word("receiver") {return Ok(Value::Object(o.clone()));}
                if name==self.class_word("function") {return Ok(Value::Routine(f.clone()));}
                return self.class_get(Value::Routine(f.clone()),name,true);
            }
            Value::Routine(f) => {
                if let Some((_,members))=self.function_members.iter().find(|(v,_)| v.equals(&subject)) {
                    if let Some((_,v))=members.fields.borrow().iter().find(|(n,_)| n==name) {return Ok(v.clone());}
                }
                if name==self.class_word("name") {return Ok(Value::text(&f.ident));}
                if name==self.class_word("qualified") {return Ok(Value::text(&f.qualified));}
                if name==self.class_word("doc") {return Ok(f.doc.clone().map_or(Value::Null,|s|Value::text(&s)));}
                if name==self.class_word("module") {return Ok(Value::text(self.class_word("main")));}
                if name==self.class_word("defaults") {
                    let values=f.carried.iter().zip(&f.held).filter(|(i,_)| **i<f.formals.len() && f.parameter_rules.as_ref().map_or(true,|rules|rules[**i]<2)).map(|(_,v)|v.clone()).collect::<Vec<_>>();
                    if values.is_empty() && f.least<f.formals.len() && f.within.is_some(){return Err(self.class_refusal());}
                    return Ok(if values.is_empty(){Value::Null}else{Value::Tuple(Rc::new(values))});
                }
                if name==self.class_word("code") {return Ok(Self::adapter(7,vec![subject.clone()]));}
                if name==self.class_word("namespace") { let at=self.function_storage(&subject); return Ok(Value::Fields(self.function_members[at].1.clone())); }
            }
            Value::Adapter(w) if w.0==7 => {
                if let Value::Routine(f)=&w.1[0] {
                    if name==self.class_word("argcount") {return Ok(Value::Small(f.parameter_rules.as_ref().map_or(f.formals.len(),|rules|rules.iter().filter(|r|**r<2).count()) as i64));}
                    if name==self.class_word("varnames") {return Ok(Value::Tuple(Rc::new(f.idents.iter().filter(|n|!n.starts_with('#')).map(|n|Value::text(n)).collect())));}
                }
            }
            Value::Adapter(w) if w.0==4 || w.0==5 => {
                if name==self.class_word("function"){return Ok(w.1[0].clone());}
            }
            Value::Adapter(w) if w.0==3 => {
                if name==self.class_word("receiver") {return Ok(w.1[1].clone());}
                if name==self.class_word("function") {return Ok(w.1[0].clone());}
            }
            _ => {}
        }
        Err(self.missing_member(&subject,name))
    }
    fn function_storage(&mut self, function: &Value) -> usize {
        if let Some(at) = self.function_members.iter().position(|(v, _)| v.equals(function)) { return at; }
        let class = self.root_class();
        self.made += 1;
        let fields = Rc::new(Instance { class, fields: RefCell::new(Vec::new()), mark: self.made });
        self.function_members.push((function.clone(), fields));
        self.function_members.len() - 1
    }
    fn namespace(members:&[(String,Value)]) -> Value {Value::Map(Rc::new(members.iter().map(|(n,v)|(Value::text(n),v.clone())).collect()))}
    pub(super) fn class_write(&mut self, subject:Value, name:&str, value:Option<Value>, plain:bool) -> Flow<Value> {
        let absent=self.missing_member(&subject,name);
        match &subject {
            Value::Object(o) => {
                if self.exception_class(&o.class) && self.lang.exception_args.as_deref() == Some(name) {
                    if let Some(v) = value.as_ref() {
                        let items = match v.contents() {
                            Value::Array(items) | Value::Tuple(items) => Value::Tuple(items),
                            _ => return Err(self.lang.exception_unready.clone().unwrap_or_default().into()),
                        };
                        let mut fields = o.fields.borrow_mut();
                        for key in [name, "\0arguments"] { let _ = Self::write_members(&mut fields, key, Some(items.clone()), false); }
                        return Ok(Value::Null);
                    }
                }
                if !plain {
                    let hook=if value.is_some(){"set"}else{"remove"};
                    if let Some(f)=self.class_value(&o.class,self.class_word(hook)) {
                        let mut args=vec![subject.clone(),Value::text(name)];args.extend(value);return self.class_apply(f,args);
                    }
                    if let Some(Value::Adapter(w))=self.class_value(&o.class,name) {
                        if w.0==6 {
                            if let (Some(set),Some(v))=(w.1.get(1),value.clone()) {return self.class_apply(set.clone(),vec![subject.clone(),v]);}
                            return Err(absent);
                        }
                    }
                }
                // A member that takes writes takes this one: a slot keeps
                // the value, a property's kept accessor refuses, and any
                // other asks its class's writer or remover.
                if let Some(member)=self.class_value(&o.class,name) {
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
                if name==self.class_word("kind") || name==self.class_word("namespace"){return Err(self.class_refusal());}
                if value.is_some() && !self.slots_allow(&o.class,name) {return Err(absent);}
                // A module's members are its own bindings, written through
                // so that its routines see the new value; a thing's member
                // is simply written over.
                let module=self.modules.values().any(|held|matches!(held,Value::Object(space) if Rc::ptr_eq(space,o)));
                Self::write_members(&mut o.fields.borrow_mut(),name,value,module).map_err(|_|absent)?;
            }
            Value::Class(c) => {
                if ["name","qualified","kind","bases","mro","namespace","order"].iter().any(|key|name==self.class_word(key)){return Err(self.class_refusal());}
                Self::write_members(&mut c.shared.borrow_mut(),name,value,false).map_err(|_|absent)?;
            }
            Value::Routine(_) => {
                if name == self.class_word("namespace") {
                    let at = self.function_storage(&subject);
                    if matches!(&value, Some(Value::Fields(fields)) if Rc::ptr_eq(fields, &self.function_members[at].1)) { return Ok(Value::Null); }
                }
                if ["defaults","code","namespace"].iter().any(|k|name==self.class_word(k)) {return Err(self.class_refusal());}
                let at=self.function_storage(&subject);
                Self::write_members(&mut self.function_members[at].1.fields.borrow_mut(),name,value,false).map_err(|_|absent)?;
            }
            _ => return Err(absent),
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
    fn slots_allow(&self,c:&Class,name:&str)->bool {
        let own=Self::own_class_value(c,self.class_word("slots"));
        let Some(slots)=own else{return Self::own_kind(c).is_none();};
        let allows=|v:&Value|match v {Value::Text(s)=>s.as_ref()==name||s.as_ref()==self.class_word("namespace"),_=>false};
        let fits=match slots {Value::Array(v)|Value::Tuple(v)=>v.iter().any(allows),v=>allows(&v)};
        fits||c.direct.iter().filter(|b|b.name!=self.class_word("root")).any(|b|self.slots_allow(b,name))
    }
    pub(super) fn class_type(&mut self,args:Vec<Value>)->Flow<Value> {
        let args: Vec<Value> = args.iter().map(Value::contents).collect();
        match args.as_slice() {
            [Value::Object(o)]=>Ok(Value::Class(o.class.clone())),
            // A class is of the kind that makes classes, which is the
            // kind builtin itself, under whatever word spells it.
            [Value::Class(_)]=>Ok(self.lang.builtins.iter().find(|(_,b)|**b==crate::code::Builtin::SortOf).map_or(Value::Null,|(word,_)|Value::Native(crate::code::Builtin::SortOf,std::rc::Rc::from(word.as_str())))),
            [Value::Text(name),Value::Array(bases),Value::Map(members)] | [Value::Text(name),Value::Tuple(bases),Value::Map(members)] => {
                let mut parents=vec![];for b in bases.iter(){if let Value::Class(c)=b{parents.push(c.clone());}else{return Err(self.class_refusal());}}
                let mut own=vec![];for (k,v) in members.iter(){if let Value::Text(n)=k{own.push((n.to_string(),v.clone()));}else{return Err(self.class_refusal());}}
                self.form_class(name.to_string(),parents,own)
            }
            _=>Err(self.class_refusal()),
        }
    }
    fn beneath(&self,value:&Value,wanted:&Value,subclass:bool)->Flow<bool> {
        if let Value::Native(_, word) = value { return self.beneath(&Self::adapter(8, vec![Value::text(word)]), wanted, subclass); }
        if let Value::Native(_, word) = wanted { return self.beneath(value, &Self::adapter(8, vec![Value::text(word)]), subclass); }
        if let Value::Array(v)|Value::Tuple(v)=wanted {for c in v.iter(){if self.beneath(value,c,subclass)?{return Ok(true);}}return Ok(false);}
        if let Value::Class(c)=wanted {
            if c.name==self.class_word("root"){
                if !subclass||matches!(value,Value::Class(_)){return Ok(true);}
                if let Value::Adapter(w)=value{if w.0==8{if let Value::Text(n)=&w.1[0]{return Ok(matches!(self.lang.builtins.get(n.as_ref()),Some(Builtin::ToInt|Builtin::ToText|Builtin::AsReal|Builtin::List|Builtin::SortOf)));}}}
                return Err(self.class_refusal());
            }
            let kind=match value {Value::Object(o) if !subclass=>Some(&o.class),Value::Class(c) if subclass=>Some(c),_=>None};
            return Ok(kind.map_or(false,|k|Rc::ptr_eq(k,c)||k.lineage.iter().any(|b|Rc::ptr_eq(b,c))));
        }
        if let Value::Adapter(w)=wanted {
            if w.0==8 {if let Value::Text(word)=&w.1[0] {
                let Some(builtin)=self.lang.builtins.get(word.as_ref()) else{return Err(self.class_refusal());};
                if subclass{
                    if !matches!(builtin,Builtin::ToInt|Builtin::ToText|Builtin::AsReal|Builtin::List|Builtin::SortOf|Builtin::Dict|Builtin::Tuple|Builtin::Set|Builtin::Bool){return Err(self.class_refusal());}
                    if let Value::Class(c)=value{return Ok(Self::kind_beneath(c).as_deref()==Some(word.as_ref()));}
                    if let Value::Adapter(other)=value {if other.0==8{return Ok(other.1[0].equals(&w.1[0]));}}
                    return Err(self.class_refusal());
                }
                // A thing of a class standing on the kind is of the kind.
                if let Value::Object(o)=value{return Ok(Self::kind_beneath(&o.class).as_deref()==Some(word.as_ref()));}
                return Ok(match builtin{Builtin::ToInt=>matches!(value,Value::Small(_)|Value::Huge(_)|Value::Flag(_)),Builtin::ToText=>matches!(value,Value::Text(_)),Builtin::AsReal=>matches!(value,Value::Real(_)),Builtin::List=>matches!(value,Value::Array(_)),Builtin::SortOf=>matches!(value,Value::Class(_)),Builtin::Dict=>matches!(value,Value::Map(_)),Builtin::Tuple=>matches!(value,Value::Tuple(_)),Builtin::Set=>matches!(value,Value::Set(_)),Builtin::Bool=>matches!(value,Value::Flag(_)),_=>return Err(self.class_refusal())});
            }}
        }
        Err(self.class_refusal())
    }
    pub(super) fn class_work(&mut self,which:u8,args:Vec<Value>)->Flow<Value> {
        let one=args.first().cloned().unwrap_or(Value::Null);
        match which {
            0|1 if args.len()==2=>Ok(Value::Flag(self.beneath(&one,&args[1],which==1)?)),
            2 if args.len()==1=>Ok(Value::Flag(matches!(one,Value::Class(_)|Value::Routine(_)|Value::Method(..))||matches!(&one,Value::Adapter(w) if matches!(w.0,0..=4|8..=12|15|17..=27))||matches!(&one,Value::Object(o) if self.class_value(&o.class,self.class_word("call")).is_some()))),
            3|6 if args.len()>=2=>{let Value::Text(name)=&args[1]else{return Err(self.class_refusal());};match self.class_get(one,name,false){Ok(v)=>Ok(if which==6{Value::Flag(true)}else{match v{Value::Bond(cell)=>cell.borrow().clone(),held=>held}}),Err(fault) if self.attribute_fault(&fault)=>if which==6{Ok(Value::Flag(false))}else if args.len()==3{Ok(args[2].clone())}else{
                // A module asked by name for a member it has not may answer through its own routine, as it does for a member read in the program.
                if let Value::Object(o)=&args[0]{if let Some(routine)=self.module_reader(o){self.invoke(&routine,vec![Value::text(name)])?;return self.drop_top().map_err(|words|Fault::Note(words));}}
                Err(fault)},Err(e)=>Err(e)}},
            4|5 if args.len()==if which==4{3}else{2}=>{let Value::Text(n)=&args[1]else{return Err(self.class_refusal());};self.class_write(one,n,args.get(2).cloned(),false)},
            7 if args.len()==1=>{let word=self.class_word("namespace").to_string();self.class_get(one,&word,true)},
            8 if args.len()==1=>{
                // A thing with a directory method of its own answers with
                // it, and the names it gives are put in order.
                if matches!(&one,Value::Object(_)) {
                    if let Some(answer)=self.special_call(&one,75,vec![]).map_err(Fault::Note)? {
                        let mut names=self.special_items(&answer).map_err(Fault::Note)?;
                        names.sort_by_key(Value::plain);
                        return Ok(Value::array(names));
                    }
                }
                let mut names=vec![];let class=match &one{Value::Class(c)=>Some(c),Value::Object(o)=>{names.extend(o.fields.borrow().iter().filter(|(n,_)|!n.starts_with('\0')).map(|(n,_)|n.clone()));Some(&o.class)},_=>None};
                if let Some(c)=class {for b in std::iter::once(c).chain(c.lineage.iter()){names.extend(b.shared.borrow().iter().map(|(n,_)|n.clone()));}}
                else if let Some((_,m))=self.function_members.iter().find(|(v,_)|v.equals(&one)){names.extend(m.fields.borrow().iter().map(|(n,_)|n.clone()));}
                names.sort();names.dedup();Ok(Value::array(names.iter().map(|n|Value::text(n)).collect()))
            }
            // A property is a thing of the property class, where the
            // definition spells the protocol; else the older wrapper.
            11 if !self.class_word("descriptor.get").is_empty()=>{let class=self.property_class();self.class_make(class,args)},
            9..=11 if !args.is_empty()=>Ok(Self::adapter(which-5,args)),
            _=>Err(self.class_refusal()),
        }
    }
    pub(super) fn class_super(&mut self,subject:Value,owner:&str,name:&str,args:Vec<Value>)->Flow<Value> {
        let receiver=match &subject{Value::Object(o)=>o.class.clone(),Value::Class(c)=>c.clone(),_=>return Err(self.class_refusal())};
        let mut sequence=vec![receiver.clone()];sequence.extend(receiver.lineage.iter().cloned());
        let at=sequence.iter().position(|c|c.name==owner || Self::own_class_value(c,self.class_word("qualified")).map_or(false,|v|v.plain()==owner)).ok_or_else(||self.class_refusal())?;
        for c in sequence.iter().skip(at+1) {
            // The kind a class stands on makes the thing, takes its
            // constructing in silence, and answers its kind's methods
            // through the worth the thing keeps.
            if let Some(word)=Self::own_kind(c) {
                if name==self.class_word("allocate"){return self.class_apply(Self::adapter(14,vec![Value::text(&word)]),args);}
                if self.lang.constructor.as_deref()==Some(name){return Ok(Value::Null);}
                if let (Some(worth),Some(op))=(Self::worth_of(&subject),self.lang.value_methods.get(name).cloned()) {
                    let mut positional=Vec::new();let mut named=Vec::new();
                    for (key,v) in self.call_items(args)? {match key{Some(k)=>named.push((k,v)),None=>positional.push(v)}}
                    return Ok(self.value_method(&worth,&op,positional,named)?);
                }
                continue;
            }
            if let Some(f)=Self::own_class_value(c,name){let mut all=if name==self.class_word("allocate"){vec![]}else{vec![subject.clone()]};all.extend(args);return self.class_apply(f,all);}
            if c.name==self.class_word("root") && name==self.class_word("allocate"){let allocator=self.class_get(Value::Class(c.clone()),name,true)?;return self.class_apply(allocator,args);}
            if c.name==self.class_word("root") {
                let f=self.class_get(Value::Class(c.clone()),name,true)?;
                let mut all=vec![subject.clone()];all.extend(args);return self.class_apply(f,all);
            }
        }
        Err(self.missing_member(&subject,name))
    }
}
