// A class keeps its direct forebears and the order in which their own
// members are sought. Calls and member wrappers use that order alike.
use super::*;

impl<'a> Machine<'a> {
    pub(super) fn detail(&self, key: &str) -> &str {
        match key {
            "defaults" => self.rules.detail_defaults,
            "call" => self.rules.detail_call,
            "descriptor.delete" => self.rules.detail_descriptor_delete,
            "descriptor.set" => self.rules.detail_descriptor_set,
            "remove" => self.rules.detail_delete,
            "subclass" => self.rules.detail_subclass,
            "set" => self.rules.detail_set,
            "get" => self.rules.detail_get,
            "allocate" => self.rules.detail_allocate,
            "descriptor.get" => self.rules.detail_descriptor_get,
            "doc" => self.rules.detail_doc,
            "getitem" => self.rules.detail_getitem,
            "kind" => self.rules.detail_kind,
            "main" => self.rules.detail_main,
            "module" => self.rules.detail_module,
            "mro" => self.rules.detail_mro,
            "name" => self.rules.detail_name,
            "namespace" => self.rules.detail_namespace,
            "order" => self.rules.detail_order,
            "qualified" => self.rules.detail_qualified,
            "receiver" => self.rules.detail_receiver,
            "root" => self.rules.detail_root,
            "unready" => self.rules.detail_unready,
            _ => self.table.single(&format!("ext.stmt.class.detail.{key}")).unwrap_or(""),
        }
    }
    // Read once when the roster itself was read, since no program still
    // running can change which words a class stands under: every value
    // read asks this, so it is a field on the table rather than a name
    // built afresh and looked into at each one.
    pub(super) fn has_class_order(&self)->bool {self.table.has_class_order}
    pub(super) fn class_unready(&self)->Escape {self.detail("unready").to_owned().into()}
    pub(super) fn common_ancestor(&mut self)->Rc<Blueprint> {
        if self.ancestor.is_none() {
            let title=self.detail("root").to_owned();
            self.ancestor=Some(Rc::new(Blueprint {presentation:Some(format!("<class '{title}'>")),name:title,
                parents:Vec::new(),ancestry:RefCell::new(Vec::new()),under:None,answers:Vec::new(),fields:Vec::new(),
                reaches:Vec::new(),methods:Vec::new(),constants:Vec::new(),shared:RefCell::new(Vec::new()),weak_slot:Cell::new(None),has_slot_storage: false, sealed:Cell::new(false), order_supplied:Cell::new(false), supplied_order: RefCell::new(Vec::new()), type_names: std::cell::RefCell::new(None)}));
        }
        self.ancestor.as_ref().unwrap().clone()
    }
    /// The blueprint standing for a native kind, made once for each word
    /// asked for. A thing of a blueprint beneath it keeps a worth of that
    /// kind among what it holds, under a name no program can write.
    pub(super) fn native_kind(&mut self,word:&str)->Rc<Blueprint> {
        if matches!(self.table.prims.get(word), Some(Prim::ClassWork(11))) && self.protocol_spelled() { return self.property_blueprint(); }
        if let Some((_,kind))=self.native_kinds.iter().find(|(w,_)|w==word){return kind.clone();}
        let mut root=self.common_ancestor();
        if self.table.prims.get(word)==Some(&Prim::Truthful) {
            let integer=self.table.prims.iter().find(|(_,p)|**p==Prim::AsInt).map(|(w,_)|w.to_string());
            if let Some(integer)=integer {root=self.native_kind(&integer);}
        }
        let mut ranks=vec![root.clone()];ranks.extend_from_slice(&root.ancestry.borrow());
        let mut protocols = if matches!(self.table.prims.get(word), Some(Prim::Zipped | Prim::Mapped | Prim::Filtered)) {
            let names = self.rules.specials;
            [(15usize, 4i64), (16, 3)].into_iter().filter_map(|(slot, operation)| names.get(slot).map(|name| (name.clone(), Self::wrap(120, vec![Value::Small(operation)])))).collect()
        } else { Vec::new() };
        if matches!(self.table.prims.get(word), Some(Prim::Uniques | Prim::Unchanging | Prim::Numbered | Prim::Zipped | Prim::Mapped | Prim::Filtered | Prim::Backwards)) {
            if let Some(method) = self.table.strings("ext.stmt.class.special").get(79) {
                protocols.push((method.to_owned(), Self::wrap(136, vec![Value::text(word)])));
            }
        }
        if word == "Union" {
            protocols.push(("__repr__".to_owned(), Self::wrap(127, Vec::new())));
            // The reference builds a union by subscription; the folded
            // members are the very shape the class questions read.
            protocols.push(("__class_getitem__".to_owned(), Self::wrap(140, Vec::new())));
        }
        if ["member_descriptor", "getset_descriptor"].contains(&word) && self.rules.closes_over {
            let names: Vec<_> = ["descriptor.get", "descriptor.set", "descriptor.delete"].iter()
                .map(|part| self.detail(part).to_owned()).filter(|key| !key.is_empty()).collect();
            protocols.extend(names.into_iter().map(|key| {
                let entry = Self::wrap(60, vec![Value::text(word), Value::text(&key)]);
                (key, entry)
            }));
        }
        if word == "code" && self.rules.closes_over {
            let mut names = self.rules.words_ext_stmt_class_detail_code_fields.to_vec();
            names.push(self.detail("argcount").to_owned());
            names.push(self.detail("varnames").to_owned());
            protocols.extend(names.into_iter().filter(|field| field.starts_with("co_")).map(|field| {
                let descriptor = Self::wrap(60, vec![Value::text(word), Value::text(&field)]);
                (field, descriptor)
            }));
        }
        if word == "GenericAlias" {
            protocols.push(("__iter__".to_owned(), Self::wrap(125, vec![Value::Small(5)])));
            let names = [self.detail("allocate"), "__repr__", "__call__", "__mro_entries__", "__eq__"];
            protocols.extend(names.iter().enumerate().map(|(i, n)| (n.to_string(), Self::wrap(125, vec![Value::Small(i as i64)]))));
        }
        if word == "super" {
            let init = self.table.single("ext.stmt.class.constructor").unwrap_or_default();
            protocols.push((init.to_owned(), Self::wrap(124, Vec::new())));
        }
        if word == "SimpleNamespace" {
            let init = self.rules.words_ext_stmt_class_constructor.first().map(String::as_str).unwrap_or_default();
            let operations = [self.detail("allocate"), init, "__repr__", "__eq__", "__ne__", "__reduce__", "__replace__"];
            protocols.extend(operations.iter().enumerate().map(|(operation, name)|
                (name.to_string(), Self::wrap(123, vec![Value::Small(operation as i64)]))));
            protocols.push(("__hash__".into(), Value::Nil));
        }
        let title = match word { "TypeVar" | "ParamSpec" | "TypeVarTuple" | "TypeAliasType" | "Generic" | "NoDefaultType" | "ParamSpecArgs" | "ParamSpecKwargs" => format!("typing.{word}"), "SimpleNamespace" | "GenericAlias" => format!("types.{word}"), "Union" => String::from("typing.Union"), _ => word.to_owned() };
        let kind=Rc::new(Blueprint {presentation:Some(format!("<class '{title}'>")),name:word.to_owned(),
            parents:vec![root.clone()],ancestry:RefCell::new(ranks),under:Some(root),answers:Vec::new(),fields:Vec::new(),
            reaches:Vec::new(),methods:Vec::new(),constants:vec![("\0native".to_owned(),Value::text(word))],shared:RefCell::new(protocols),weak_slot:Cell::new(None),has_slot_storage: false, sealed:Cell::new(false), order_supplied:Cell::new(false), supplied_order: RefCell::new(Vec::new()), type_names: std::cell::RefCell::new(None)});
        if !self.rules.words_ext_stmt_class_builder.is_empty() && matches!(self.table.prims.get(word), Some(Prim::ClassWork(9..=10))) {
            kind.shared.borrow_mut().push((self.detail("descriptor.get").to_owned(), Self::wrap(78, Vec::new())));
            if self.table.prims.get(word) == Some(&Prim::ClassWork(9)) {
                kind.shared.borrow_mut().push((self.detail("call").to_owned(), Self::wrap(79, Vec::new())));
            }
        }
        if word == "classmethod_descriptor" && self.protocol_spelled() {
            kind.shared.borrow_mut().push((self.detail("descriptor.get").to_string(), Self::wrap(235, Vec::new())));
        }
        if word == "module" && !self.detail("kind").is_empty() {
            if let Some(key) = self.rules.words_ext_stmt_class_constructor.first().map(String::as_str) {
                kind.shared.borrow_mut().push((key.to_owned(), Self::wrap(2, vec![Value::text("module")])));
            }
        }
        if word == "module" && !self.detail("kind").is_empty() {
            let dictionary = self.detail("namespace").to_owned();
            kind.shared.borrow_mut().push((dictionary.clone(), Self::wrap(32,
                vec![Value::text(&dictionary), Value::Blueprint(kind.clone()), Value::Small(-1)])));
            if let Some(represent) = self.rules.specials.get(1) {
                kind.shared.borrow_mut().push((represent.clone(), Self::wrap(60, vec![Value::text(word), Value::text(represent)])));
            }
        }
        if self.names_in_calls && word == "function" {
            let get = self.detail("descriptor.get").to_owned();
            kind.shared.borrow_mut().push((get, Self::wrap(31, Vec::new())));
        }
        if self.names_in_calls {
            match word {
                "getset_descriptor" | "member_descriptor" | "method_descriptor" | "wrapper_descriptor" | "classmethod_descriptor" => {
                    let reader = self.detail("descriptor.get").to_string();
                    kind.shared.borrow_mut().push((reader.clone(), self.kind_entry(word, &reader)));
                }
                _ => (),
            }
        }
        self.native_kinds.push((word.to_owned(),kind.clone()));
        if matches!(word, "function" | "builtin_function_or_method" | "method" | "method_descriptor" | "wrapper_descriptor" | "type" | "NoneType") {
            for (at, key) in self.rules.specials.iter().enumerate() {
                if at == 8 || at == 17 && word != "NoneType" { kind.shared.borrow_mut().push((key.clone(), self.kind_entry(word, key))); }
            }
        }
        if matches!(word, "int" | "float" | "str" | "tuple" | "bytes" | "bytearray" | "dict" | "set" | "frozenset" | "complex" | "list") {
            let allocation = self.detail("allocate").to_string();
            kind.shared.borrow_mut().push((allocation, self.native_allocation(word)));
        }
        if self.table.spells("ext.stmt.class.builtin", "bytes") && matches!(word, "bytes" | "bytearray") {
            kind.shared.borrow_mut().push(("__buffer__".to_owned(), self.kind_entry(word, "__buffer__")));
            if word == "bytearray" { kind.shared.borrow_mut().push(("__release_buffer__".to_owned(), self.kind_entry(word, "__release_buffer__"))); }
        }
        if let Some(representative) = self.kind_stand_in(word) {
            let owner = Value::Blueprint(kind.clone());
            let known = self.native_directory(&representative);
            let descriptors = known.into_iter().filter(|name|
                !["real", "imag", "numerator", "denominator", "start", "stop", "step", "__dir__", "__reduce_ex__"].contains(&name.as_str())
                && !(word == "generator" && name == "__setstate__")
                && !(self.table.prims.get(word) == Some(&Prim::Unchanging) && self.rules.words_ext_stmt_class_constructor.first().map(String::as_str) == Some(name.as_str()))
                && !kind.shared.borrow().iter().any(|(key, _)| key == name))
                .filter_map(|name| self.carried_by_kind(&owner, &name).map(|entry| (name, entry))).collect::<Vec<_>>();
            kind.shared.borrow_mut().extend(descriptors);
        }
        if word == "dict" {
            let mut entries = kind.shared.borrow_mut();
            entries.retain(|(name, _)| name != "fromkeys");
            entries.push(("fromkeys".to_owned(), Self::wrap(60, vec![Value::text(word), Value::text("fromkeys"), Value::Flag(true)])));
        }
        if word == "bytearray" {
            if let Some(task) = self.table.prims.get("bytearray.maketrans") {
                let function = Value::Intrinsic(*task, Rc::from("bytearray.maketrans"));
                kind.shared.borrow_mut().push(("maketrans".to_owned(), Self::wrap(4, vec![function])));
            }
        }
        kind
    }
    pub(super) fn native_word(b:&Blueprint)->Option<String> {b.constants.iter().find(|(k,_)|k=="\0native").map(|(_,v)|v.bare())}
    /// The seal builtin's mark, kept on the blueprint itself where no
    /// member write of the program's can reach it: the class takes no
    /// member writes and stands as no base.
    pub(super) fn sealed(b:&Blueprint)->bool {b.sealed.get()}
    // The root method gathers the same names as the ordinary builtin
    // path; an appointed directory hook is called by the caller instead.
    pub(super) fn ordinary_directory(&mut self, item: &Value) -> Res {
        // A routine lists the members it can honestly answer for,
        // alongside any it was given of its own.
        let routine=match item {
            Value::Method(code, _, _)=>Some(Value::Routine(code.clone())),
            Value::Wrapped(3,items) if matches!(items.first(),Some(Value::Routine(_)|Value::Bound(..)))=>Some(items[0].clone()),
            Value::Routine(_)|Value::Bound(..)=>Some(item.clone()),
            _=>None,
        };
        if let Some(routine)=routine {
            let mut names:Vec<String>=Vec::new();
            for part in ["name","qualified","doc","module","defaults","call"] {
                let word=self.detail(part);
                if !word.is_empty() { names.push(word.to_string()); }
            }
            names.extend(self.routine_holding_names(&routine));
            names.sort();names.dedup();
            return Ok(Value::Vector(crate::tuples::Sequence::plain(names.iter().map(|s|Value::text(s)).collect())));
        }
            if matches!(item, Value::Wrapped(62, _)) {
                let mut names = Vec::new();
                names.extend(self.rules.words_ext_stmt_async_generator_methods.iter().skip(3).cloned());
                names.extend([15, 16].iter().filter_map(|index| self.rules.specials.get(*index).cloned()));
                for label in ["ext.stmt.yield.send", "ext.stmt.yield.throw", "ext.stmt.yield.close"] {
                    names.extend(self.table.strings(label).iter().cloned());
                }
                names.sort(); names.dedup();
                return Ok(Value::Vector(crate::tuples::Sequence::plain(names.iter().map(|name| Value::text(name)).collect())));
            }
        // A walk over a routine's own body lists the walking pair it
        // answers to (through the mark below) and the few names a
        // language gives it for stepping it by hand.
        if let Value::Generator(_)=item {
            let mut names=self.native_directory(item);
                if self.is_async_generator(item) {
                    names.extend(self.rules.words_ext_stmt_async_generator_methods.iter().take(3).cloned());
                    names.extend(self.table.strings("ext.stmt.async.generator.fields").iter().cloned());
                    names.sort(); names.dedup();
                    return Ok(Value::Vector(crate::tuples::Sequence::plain(names.iter().map(|s|Value::text(s)).collect())));
                }
            for label in ["ext.stmt.yield.close","ext.stmt.yield.send","ext.stmt.yield.throw","ext.stmt.yield.running"] {
                if let Some(w)=self.table.strings(label).first() { names.push(w.clone()); }
            }
            for word in [14, 15, 21, 24].iter().filter_map(|i| self.rules.trace_words.get(*i).cloned()).filter(|word| !word.is_empty()) {
                if !word.is_empty() { names.push(word); }
            }
            names.sort();names.dedup();
            return Ok(Value::Vector(crate::tuples::Sequence::plain(names.iter().map(|s|Value::text(s)).collect())));
        }
        // A native kind, named as the kind itself or held as a value
        // of one, answers the members a value of that kind has: the
        // methods of the kind and the special names its mark answers.
        if let Some(sample)=self.directory_stand_in(item) {
            let named=self.native_directory(&sample);
            return Ok(Value::Vector(crate::tuples::Sequence::plain(named.iter().map(|word|Value::text(word)).collect())));
        }
        let mut names=Vec::new();
        let class = match item {
            Value::Blueprint(blueprint) => Some(blueprint.clone()),
            Value::Thing(instance) => {
                names.extend(instance.holds.borrow().iter().filter(|(key, _)| !key.starts_with('\0')).map(|(key, _)| key.clone()));
                if !self.rules.has_any_ext_builtin_bool_result { Some(instance.blueprint()) } else {
                let key = self.detail("kind").to_owned();
                let reported = match self.read_class_member(item.clone(), &key, false) {
                    Ok(value) => Some(value),
                    Err(missing) if self.missing_member_escape(&missing) => None,
                    Err(other) => return Err(other),
                };
                self.parent_blueprint_of(reported)
                }
            }
            _ => None,
        };
        if let Some(b)=&class {
            for c in Self::resolution_order(b).iter() {
                names.extend(c.shared.borrow().iter().filter(|(k,_)|!k.starts_with('\0')).map(|(k,_)|k.clone()));
                if let Some(word) = Self::native_word(c) {
                    if let Some(example) = self.kind_stand_in(&word) { names.extend(self.native_directory(&example)); }
                }
            }
        }
        else{names.extend(self.routine_holding_names(item));}
        if let Some(class) = &class {
            for part in ["kind", "get", "set", "remove", "allocate", "subclass"] {
                let name = self.detail(part); if !name.is_empty() { names.push(name.to_owned()); }
            }
            if let Some(name) = self.rules.words_ext_stmt_class_constructor.first().map(String::as_str) { names.push(name.to_owned()); }
            if self.allowed_slot(class, self.detail("namespace")) { names.push(self.detail("namespace").to_owned()); }
            if !self.rules.words_ext_builtin_weak_get.is_empty() && self.admits_weak(class) { names.push("__weakref__".to_owned()); }
            names.extend(self.rules.words_ext_stmt_class_detail_root_members.iter().cloned());
            names.extend(self.rules.specials.get(72).cloned());
        }
        names.sort_unstable();names.dedup();Ok(Value::Vector(crate::tuples::Sequence::plain(names.iter().map(|s|Value::text(s)).collect())))
    }

    /// The value whose kind a directory should describe: an empty value
    /// of the kind a native kind word names, or the value itself where
    /// it is one of a native kind. Nothing for a blueprint of a class's
    /// own or a thing of one, which list their own members instead.
    fn directory_stand_in(&self,value:&Value)->Option<Value> {
        if let Value::Intrinsic(_,word)=value { return self.kind_stand_in(word); }
        if let Value::OctetKind{changeable,..}=value { let word=self.octet_kind_word(*changeable).to_owned(); return self.kind_stand_in(&word); }
        if let Value::Blueprint(b)=value { return Self::native_word(b).and_then(|word|self.kind_stand_in(&word)); }
        if matches!(value,Value::Thing(_)) { return None; }
        if self.native_directory(value).is_empty() { return None; }
        Some(value.clone())
    }
    pub(super) fn native_beneath(b:&Blueprint)->Option<String> {
        std::iter::once(b).chain(b.ancestry.borrow().iter().map(Rc::as_ref)).find_map(Self::native_word)
    }

    /// Whether a blueprint stands on the native kind named, through any
    /// of its line: a blueprint written with several native parents
    /// stands on each of them, and answers to any asked after.
    pub(super) fn native_among(b:&Blueprint,word:&str)->bool {
        std::iter::once(b).chain(b.ancestry.borrow().iter().map(Rc::as_ref))
            .any(|held|Self::native_word(held).as_deref()==Some(word))
    }
    /// The blueprint every metaclass is built on: the kind primitive
    /// read as a class. A class built on it makes classes where an
    /// ordinary one makes things.
    pub(super) fn builder_blueprint(&mut self)->Rc<Blueprint> {
        if let Some(b)=&self.builder_kind{return b.clone();}
        let root=self.common_ancestor();
        let title=self.table.prims.iter().find(|(_,p)|**p==Prim::SortOf).map(|(w,_)|w.to_string()).unwrap_or_default();
        let kind=Rc::new(Blueprint {presentation:Some(format!("<class '{title}'>")),name:title,
            parents:vec![root.clone()],ancestry:RefCell::new(vec![root.clone()]),under:Some(root),answers:Vec::new(),fields:Vec::new(),
            reaches:Vec::new(),methods:Vec::new(),constants:Vec::new(),shared:RefCell::new(Vec::new()),weak_slot:Cell::new(None),has_slot_storage: false, sealed:Cell::new(false), order_supplied:Cell::new(false), supplied_order: RefCell::new(Vec::new()), type_names: std::cell::RefCell::new(None)});
        self.builder_kind=Some(kind.clone());
        for word in [self.detail("namespace"), self.detail("mro"), self.detail("order"), self.detail("name")] {
            if !word.is_empty() { kind.shared.borrow_mut().push((word.to_owned(), self.kind_entry("type", word))); }
        }
        for at in [8, 17] { if let Some(key) = self.rules.specials.get(at) {
            kind.shared.borrow_mut().push((key.clone(), self.kind_entry("type", key)));
        } }
        kind
    }
    pub(super) fn builds_classes(&self,b:&Rc<Blueprint>)->bool {self.builder_kind.as_ref().map_or(false,|k|Rc::ptr_eq(k,b))}
    /// The metaclass named for a blueprint when it was built.
    fn named_builder(b:&Blueprint)->Option<Rc<Blueprint>> {
        match b.constants.iter().find(|(k,_)|k=="\0metaclass").map(|(_,v)|v) {
            Some(Value::Blueprint(m))=>Some(m.clone()),
            _=>None,
        }
    }
    /// That metaclass through the whole line: a class is built by the
    /// metaclass of the nearest forebear that named one.
    pub(super) fn builder_over(b:&Blueprint)->Option<Rc<Blueprint>> {
        std::iter::once(b).chain(b.ancestry.borrow().iter().map(Rc::as_ref)).find_map(Self::named_builder)
    }
    /// Which metaclass builds a class: the one its header named, else the
    /// one its forebears were built by, and of two the deeper.
    pub(super) fn builder_for(&mut self,asked:Option<Value>,parents:&[Rc<Blueprint>])->Result<Option<Rc<Blueprint>>,Escape> {
        let named=match asked.map(|v|v.settled()) {
            None=>None,
            // The kind primitive names the plainest builder there is,
            // which is no metaclass of its own; so does its blueprint.
            Some(Value::Intrinsic(_,word)) if self.table.prims.get(word.as_ref())==Some(&Prim::SortOf)=>None,
            Some(Value::Blueprint(b)) if self.builds_classes(&b)=>None,
            Some(Value::Blueprint(b))=>Some(b),
            Some(other)=>return Err(self.core_complaint("core.uncallable",&other.kind_word()).into()),
        };
        let mut selected=named;
        for base in parents {
            let Some(candidate)=Self::builder_over(base) else {continue};
            if let Some(current)=selected.as_ref() {
                if Rc::ptr_eq(current,&candidate)||current.ancestry.borrow().iter().any(|p|Rc::ptr_eq(p,&candidate)){continue;}
                if !candidate.ancestry.borrow().iter().any(|p|Rc::ptr_eq(p,current)) {
                    return Err("TypeError: metaclass conflict: the metaclass of a derived class must be a (non-strict) subclass of the metaclasses of all its bases".to_owned().into());
                }
            }
            selected=Some(candidate);
        }
        Ok(selected)
    }
    /// The native worth a thing keeps, as it is kept: a row or a map
    /// stays in its cell, so that what is done to it through the thing
    /// is done to what the thing holds.
    /// The word the formatting protocol's complaints give what stood
    /// where a specification was wanted: the reference's own word for
    /// nothing, the plain kind word of anything else.
    pub(super) fn format_spec_complaint_kind(item:&Value)->String {
        match item.settled() { Value::Nil=>"None".to_owned(), other=>other.kind_word() }
    }
    pub(super) fn underlying(value:&Value)->Option<Value> {
        let Value::Thing(t)=value else{return None};
        t.holds.borrow().iter().find(|(k,_)|k=="\0underlying").map(|(_,v)|v.clone())
    }
    /// That worth, where the thing's blueprint appoints nothing at any of
    /// the given places of the protocol: the worth answers for what the
    /// blueprint leaves unsaid.
    pub(super) fn underlying_unless(&self,value:&Value,places:&[usize])->Option<Value> {
        let worth=Self::underlying(value)?;
        if places.iter().any(|&place|self.appointment(value,place).is_some()){return None;}
        Some(worth)
    }
    /// A thing of a blueprint standing on a native kind, its worth made
    /// by the primitive of that kind from what was given.
    fn optional_module_entry(&mut self, owner: &Value, key: &str) -> Res<Option<Value>> {
        match self.read_class_member(owner.clone(), key, false) {
            Ok(held) => Ok(Some(held.settled())),
            Err(escape) if self.missing_member_escape(&escape) => { Ok(None) },
            Err(escape) => Err(escape),
        }
    }
    fn module_wording(&mut self, held: &Value, represent: bool) -> Res<String> {
        match self.object_words(held, represent) {
            Ok(words) => Ok(words),
            Err(error) => Err(self.got_away.take().unwrap_or(Escape::Error(error))),
        }
    }
    fn namespace_provider_entries(&mut self, provider: &Value) -> Res<Option<Value>> {
        let module = ["_frozen_importlib_external", "importlib._bootstrap_external"].iter()
            .find_map(|name| self.imported.get(*name).cloned());
        match module {
            None => Ok(None),
            Some(module) => {
                let Some(class) = self.optional_module_entry(&module, "NamespaceLoader")? else { return Ok(None); };
                if matches!(class, Value::Nil) { return Ok(None); }
                let belongs = self.work_on_class(0, vec![provider.clone(), class])?;
                if !belongs.is_true() { return Ok(None); }
                let paths = self.read_class_member(provider.clone(), "_path", false)?;
                let entries = self.gathered_members(&paths)?;
                Ok(Some(Value::Vector(Rc::new(entries).into())))
            }
        }
    }
    pub(super) fn describe_module(&mut self, receiver: Value) -> Res {
        let native = match &receiver { Value::Thing(t) => Self::native_beneath(&t.blueprint()).as_deref() == Some("module"), _ => false };
        if !native && self.namespace_holding(&receiver).is_none() {
            return Err(format!("TypeError: descriptor '__repr__' requires a 'module' object but received a '{}'", Self::type_argument_kind(&receiver)).into());
        }
        let loader = self.optional_module_entry(&receiver, "__loader__")?.unwrap_or(Value::Nil);
        let specification = self.optional_module_entry(&receiver, "__spec__")?.unwrap_or(Value::Nil);
        if self.prim(Prim::Truthful, "bool", std::slice::from_ref(&specification))?.is_true() {
            let declared = self.read_class_member(specification.clone(), "name", false)?.settled();
            let title = match declared { Value::Nil => Value::text("?"), _ => declared.clone() };
            let source = self.read_class_member(specification.clone(), "origin", false)?.settled();
            let result;
            if matches!(source, Value::Nil) {
                let provider = self.read_class_member(specification, "loader", false)?.settled();
                let name = self.module_wording(&title, true)?;
                result = if matches!(provider, Value::Nil) { format!("<module {name}>") }
                    else { match self.namespace_provider_entries(&provider)? {
                        Some(paths) => format!("<module {name} (namespace) from {}>", self.module_wording(&paths, true)?),
                        None => format!("<module {name} ({})>", self.module_wording(&provider, true)?),
                    } };
            } else {
                let located = self.read_class_member(specification, "has_location", false)?;
                let from_file = self.prim(Prim::Truthful, "bool", &[located])?.is_true();
                let name = self.module_wording(if from_file { &title } else { &declared }, true)?;
                let origin = self.module_wording(&source, from_file)?;
                result = if from_file { format!("<module {name} from {origin}>") } else { format!("<module {name} ({origin})>") };
            }
            return Ok(Value::text(&result));
        }
        let name_value = self.optional_module_entry(&receiver, "__name__")?.unwrap_or_else(|| Value::text("?"));
        let name = self.module_wording(&name_value, true)?;
        let suffix = match self.optional_module_entry(&receiver, "__file__")? {
            Some(path) => format!(" from {}", self.module_wording(&path, true)?),
            None if matches!(loader, Value::Nil) => String::new(),
            None => format!(" ({})", self.module_wording(&loader, true)?),
        };
        Ok(Value::text(&format!("<module {name}{suffix}>")))
    }
    fn fill_module(&mut self, target: Value, handed: Vec<Value>) -> Res {
        match &target {
            Value::Thing(thing) if Self::native_beneath(&thing.blueprint()).as_deref() == Some("module") || self.namespace_holding(&target).is_some() => (),
            _ => return Err(format!("TypeError: descriptor '__init__' requires a 'module' object but received a '{}'", Self::type_argument_kind(&target)).into()),
        }
        let (plain, named) = self.open_arguments(handed)?;
        let count = plain.len() + named.len();
        if count > 2 { return Err(format!("TypeError: module() takes at most 2 arguments ({count} given)").into()); }
        let mut name = plain.first().cloned();
        let mut doc = plain.get(1).cloned();
        let mut unknown = None;
        for (key, item) in named {
            match key.as_str() {
                "name" | "doc" => {
                    let index = if key == "name" { 1 } else { 2 };
                    if plain.len() >= index { return Err(format!("TypeError: argument for module() given by name ('{key}') and position ({index})").into()); }
                    if index == 1 { name = Some(item); } else { doc = Some(item); }
                }
                _ => unknown = Some(key),
            }
        }
        let name = name.ok_or_else(|| "TypeError: module() missing required argument 'name' (pos 1)".to_owned())?;
        if let Some(key) = unknown { return Err(format!("TypeError: module() got an unexpected keyword argument '{key}'").into()); }
        let raw = Self::underlying(&name).unwrap_or_else(|| name.settled());
        if !matches!(raw, Value::Text(_) | Value::Unpaired(_)) {
            let described = match raw { Value::Nil => String::from("None"), _ => Self::type_argument_kind(&name) };
            return Err(format!("TypeError: module() argument 'name' must be str, not {described}").into());
        }
        let updates = [("__name__", name), ("__doc__", doc.unwrap_or(Value::Nil)), ("__package__", Value::Nil), ("__loader__", Value::Nil), ("__spec__", Value::Nil)];
        let Value::Thing(thing) = target else { unreachable!() };
        let mapping = thing.holds.borrow().iter().find(|entry| entry.0 == "\0dictionary").map(|entry| entry.1.clone());
        for (key, item) in updates {
            if let Some(book) = &mapping { Self::write_into_book(book, key, Some(item)); continue; }
            let mut attributes = thing.holds.borrow_mut();
            match attributes.iter_mut().find(|entry| entry.0 == key) {
                Some((_, Value::Shared(cell))) => *cell.borrow_mut() = item,
                Some(entry) => entry.1 = item,
                None => attributes.push((key.to_owned(), item)),
            }
        }
        Ok(Value::Nil)
    }
    fn thing_over_native(&mut self,class:Rc<Blueprint>,word:&str,given:Vec<Value>)->Res {
        if word == "Union" { return Err(String::from("TypeError: cannot create 'typing.Union' instances").into()); }
        if word == "cell" {
            match given.len() {
                0 | 1 => return Ok(Self::wrap(35, vec![Value::Shared(Rc::new(RefCell::new(given.into_iter().next().unwrap_or(Value::Unset))))])),
                n => return Err(format!("TypeError: cell expected at most 1 argument, got {n}").into()),
            }
        }
        if word == "method" {
            let (values, named) = self.open_arguments(given)?;
            if values.len() != 2 || !named.is_empty() { return Err(String::from("TypeError: method expected 2 arguments").into()); }
            if !matches!(self.work_on_class(2, vec![values[0].clone()])?, Value::Flag(true)) { return Err(String::from("TypeError: first argument must be callable").into()); }
            if matches!(values[1], Value::Nil) { return Err(String::from("TypeError: instance must not be None").into()); }
            return Ok(Self::wrap(132, values));
        }
        if word == "mappingproxy" {
            let (positional, keywords) = self.open_arguments(given)?;
            if positional.len() != 1 || !keywords.is_empty() { return Err(String::from("TypeError: mappingproxy() takes exactly one argument").into()); }
            let owner = positional[0].clone();
            match &owner {
                Value::Window(_, 'm') => {},
                _ => match owner.settled() {
                Value::Dict(_) | Value::Attributes(_) => {},
                Value::Thing(_) if self.appointment(&owner, 11).is_some() => {},
                other => return Err(format!("TypeError: mappingproxy() argument must be a mapping, not {}", other.kind_word()).into()),
                },
            }
            return Ok(Value::Window(Rc::new(owner), 'm'));
        }
        // None, Ellipsis and NotImplemented are singletons no native
        // kind builds twice over: calling the kind itself hands back
        // the one value there is of it, and any argument at all is
        // refused, since there is no other one to build from it.
        if let Some((singleton,shown))=match word {
            "NoneType"=>Some((Value::Nil,"NoneType")),
            "ellipsis"=>Some((Value::Ellipsis,"EllipsisType")),
            "NotImplementedType"=>Some((Value::Refusal(Rc::from(self.rules.words_ext_stmt_class_special_declined.first().map_or("NotImplemented",String::as_str))),"NotImplementedType")),
            _=>None,
        } {
            return if given.is_empty(){Ok(singleton)}else{Err(format!("TypeError: {shown} takes no arguments").into())};
        }
        if word=="module" {
            let (values, keywords)=self.open_arguments(given)?;
            if values.len()>2{return Err(String::from("TypeError: invalid module arguments").into());}
            let mut name=values.first().cloned();
            let mut doc=values.get(1).cloned().unwrap_or(Value::Nil);
            for (key,value) in keywords {
                if key=="name" && name.is_none(){name=Some(value);}
                else if key=="doc" && values.len()<2{doc=value;}
                else{return Err(String::from("TypeError: invalid module arguments").into());}
            }
            match name {
                Some(name @ Value::Text(_))=>{
                    self.made+=1;
                    let entries=vec![(String::from("__name__"),name),(String::from("__doc__"),doc),
                        (String::from("__loader__"),Value::Nil),(String::from("__spec__"),Value::Nil),(String::from("__package__"),Value::Nil)];
                    return Ok(Value::Thing(Rc::new(Thing{reclassified:RefCell::new(None),of:class,holds:RefCell::new(entries),turn:self.made})));
                }
                _=>return Err(String::from("TypeError: module name must be str").into()),
            }
        }
        // A routine framed by hand from a code value and a dictionary
        // of names: the reference's own way of making a function, which
        // keeps the dictionary it was handed and the builtins in force
        // where it was made.
        if word=="function" {
            let (Some(Value::Wrapped(7,code)),Some(globe))=(given.first(),given.get(1))else{return Err(self.class_unready());};
            let Some(Value::Routine(template))=code.first()else{return Err(self.class_unready());};
            let born=self.builtins_here();
            let mut made=(**template).clone();
            made.globe=Some(globe.clone());
            made.framed_in=match globe.settled() {
                Value::Dict(pairs)=>pairs.iter().find(|(k,_)|matches!(k,Value::Text(t) if t.as_ref()=="__name__")).and_then(|(_,v)|match v.settled() { Value::Text(named)=>Some(named.clone()), _=>None }),
                _=>None,
            };
            made.born=Some(born);
            return Ok(Value::Routine(Rc::new(made)));
        }
        if word == "method" {
            if given.len() != 2 { return Err("TypeError: method expected 2 arguments".to_owned().into()); }
            if matches!(given[1].settled(), Value::Nil) { return Err("TypeError: instance must not be None".to_owned().into()); }
            return Ok(Self::wrap(3, given));
        }
        // A module made by hand: ModuleType is the run's own module
        // kind, so calling it makes a thing carrying the name it was
        // handed and, when one was handed, its documentation.
        if word=="module" {
            // The module initializer, possibly overridden, fills the
            // namespace after allocation rather than here.
            let holds = Vec::new();
            self.made+=1;
            return Ok(Value::Thing(Rc::new(Thing{reclassified:RefCell::new(None), of:class, holds:RefCell::new(holds), turn:self.made})));
        }
        let Some(op)=self.table.prims.get(word).copied()else{return Err(self.class_unready());};
        let (positional,named)=self.open_arguments(given)?;
        let made=if !self.rules.words_ext_stmt_class_builder.is_empty() && matches!(op, Prim::ClassWork(9..=10)) {
            let mut values = positional;
            values.extend(named.into_iter().map(|(key, value)| Value::Couple(Rc::new((Value::text(&key), value)))));
            let Prim::ClassWork(operation) = op else { unreachable!() };
            self.work_on_class(operation, values)?
        } else if self.rules.class_builder && op == Prim::Dictionary { self.core_primitive(op,word,positional,named)? } else if named.is_empty(){self.prim(op,word,&positional)?}else{let mut arguments=positional;match self.builtin_names(op,word,&mut arguments,named)?{Some(result)=>result,None=>self.prim(op,word,&arguments)?}};
        let made = if matches!(word, "str" | "bytes" | "bytearray") { Self::underlying(&made).unwrap_or(made) } else { made };
        let kept=match made.settled(){
            held @ (Value::Vector(_)|Value::Dict(_))=>Value::Mutable(Rc::new(RefCell::new(held)),true),
            other=>other,
        };
        self.made+=1;
        Ok(Value::Thing(Rc::new(Thing{reclassified: RefCell::new(None), of:class,holds:RefCell::new(vec![("\0underlying".to_owned(),kept)]),turn:self.made})))
    }
    pub(super) fn build_class_value(&mut self,title:String,parents:Vec<Rc<Blueprint>>,entries:Vec<(String,Value)>)->Res {
        self.build_named_class(Value::text(&title),parents,entries)
    }
    fn build_named_class(&mut self, title_object:Value,mut parents:Vec<Rc<Blueprint>>,mut entries:Vec<(String,Value)>)->Res {
        let title=self.checked_type_name(&title_object)?;
        let prepared = entries.iter().position(|(key, _)| key == "\0prepared").map(|at| entries.remove(at).1);
        if let Some(index) = entries.iter().position(|entry| entry.0 == "\0header") {
            let header = entries.remove(index).1;
            let Value::Tuple(arguments) = header.settled() else { return Err(self.class_unready()); };
            let (positional, named) = self.open_arguments(arguments.as_ref().clone())?;
            let mut factory = None;
            let mut extra = Vec::new();
            for (key, item) in named {
                if self.table.spells("ext.stmt.class.metaclass", &key) { factory = Some(item); }
                else { extra.push((key, item)); }
            }
            let mut resolved = Vec::new();
            let source = Value::tuple(positional.clone());
            let key = self.detail("mro.entries").to_owned();
            let mut substituted = false;
            for item in positional {
                if !key.is_empty() && matches!(item.settled(), Value::Thing(_)) {
                    let member = self.read_class_member(item, &key, false)?;
                    let result = self.apply_class_member(member, vec![source.clone()])?;
                    match result.settled() {
                        Value::Tuple(sequence) => resolved.extend(sequence.iter().cloned()),
                        _ => return Err("TypeError: __mro_entries__ must return a tuple".to_string().into()),
                    }
                    substituted = true;
                } else { resolved.push(item); }
            }
            if substituted { entries.push((self.detail("original.bases").to_string(), source)); }
            match factory {
                Some(callable) if !matches!(callable, Value::Intrinsic(Prim::SortOf, _)) => {
                    let pairs = entries.iter().filter_map(|(key, item)| {
                        (!matches!(item, Value::Unset)).then(|| (Value::text(key), item.clone()))
                    }).collect::<Vec<_>>();
                    let mapping = prepared.unwrap_or_else(|| Value::Mutable(Rc::new(RefCell::new(Value::Dict(Rc::new(pairs.into())))), true));
                    let mut supplied = vec![Value::text(&title), Value::tuple(resolved), mapping];
                    for (key, item) in extra { supplied.push(Value::Couple(Rc::new((Value::text(&key), item)))); }
                    return self.apply_class_member(callable, supplied);
                }
                _ => {
                    parents.clear();
                    for base in resolved {
                        if self.table.has_any("ext.stmt.class.detail.mro.entries") && matches!(base.settled(), Value::Nil) { return Err(String::from("TypeError: NoneType takes no arguments").into()); }
                        parents.push(self.parent_from_type(&base)?);
                    }
                    for (key, item) in extra { entries.push((format!("\0handed:{key}"), item)); }
                }
            }
        }
        let declared_parents = parents.clone();
        if parents.is_empty(){parents.push(self.common_ancestor());}
        // What the header handed over by keyword is no entry of the
        // class: each goes by its name to the forebears' subclass hook.
        let (handed,mut entries):(Vec<_>,Vec<_>)=entries.into_iter().partition(|(k,_)|k.starts_with("\0handed:"));
        let handed:Vec<Value>=handed.into_iter().map(|(k,v)|Value::Couple(Rc::new((Value::text(&k["\0handed:".len()..]),v)))).collect();
        // The metaclass the header named is no entry either: it says
        // what builds the class.
        let mut asked=None;
        entries.retain(|(k,v)|if k=="\0metaclass"{asked=Some(v.clone());false}else{true});
        let builder=self.builder_for(asked,&parents)?;
        // A metaclass with a building of its own builds the class: it is
        // handed itself, the name, the parents and the body's namespace
        // as a map it may write into, and answers with the class.
        if let Some(m)=builder.clone() {
            if let Some(f)=self.inherited_entry(&m,self.detail("allocate")) {
                let listed=Value::tuple(declared_parents.into_iter().map(Value::Blueprint).collect());
                let pairs:Vec<(Value,Value)>=entries.iter().filter(|(_,v)|!matches!(v,Value::Unset))
                    .map(|(k,v)|(Value::text(k),v.clone())).collect();
                let namespace=prepared.unwrap_or_else(|| Value::Mutable(Rc::new(RefCell::new(Value::Dict(Rc::new(pairs.into())))),true));
                let mut given=vec![Value::Blueprint(m.clone()),Value::text(&title),listed.clone(),namespace.clone()];
                given.extend(handed.iter().cloned());
                let built=self.apply_class_member(f,given)?;
                // The metaclass is then told of what it has built.
                if let Some(begun)=self.rules.words_ext_stmt_class_constructor.first().map(String::as_str).and_then(|w|self.inherited_entry(&m,w)) {
                    let bound=self.member_binding(begun,Some(built.clone()),m)?;
                    let mut told=vec![Value::text(&title),listed,namespace];
                    told.extend(handed);
                    self.apply_class_member(bound,told)?;
                }
                return Ok(built);
            }
        }
        self.assemble_class(title,parents,entries,builder,handed,title_object)
    }
    /// A class laid out from what the kind primitive is given: the
    /// metaclass to remember, the name, the parents and the namespace,
    /// any further keyword being kept for a forebear's subclass hook.
    pub(super) fn class_of_parts(&mut self,values:Vec<Value>)->Res {
        let (mut plain,named)=self.open_arguments(values)?;
        let first=plain.first().map(Value::settled).ok_or_else(||Escape::from("TypeError: type.__new__(): not enough arguments".to_owned()))?;
        let explicit=match first {
            Value::Intrinsic(Prim::SortOf,_)=>self.builder_blueprint(),
            Value::Blueprint(b)=>b,
            Value::Intrinsic(op,word) if Self::names_a_kind(&op)=>self.native_kind(&word),
            other=>return Err(format!("TypeError: type.__new__(X): X is not a type object ({})",other.kind_word()).into()),
        };
        if !(self.builds_classes(&explicit)||explicit.ancestry.borrow().iter().any(|p|self.builds_classes(p))) {
            return Err(format!("TypeError: type.__new__({0}): {0} is not a subtype of type",explicit.name).into());
        }
        if plain.len()!=4 {return Err(format!("TypeError: type.__new__() takes exactly 3 arguments ({} given)",plain.len()-1).into());}
        let title=self.checked_type_name(&plain[1])?;
        let bases=plain[2].settled();
        let bases=Self::underlying(&bases).unwrap_or(bases).settled();
        let Value::Tuple(bases)=bases else{return Err(format!("TypeError: type.__new__() argument 2 must be tuple, not {}",plain[2].kind_word()).into())};
        let namespace=plain[3].settled();
        let namespace=Self::underlying(&namespace).unwrap_or(namespace).settled();
        let Value::Dict(pairs)=namespace else{return Err(format!("TypeError: type.__new__() argument 3 must be dict, not {}",plain[3].kind_word()).into())};
        let mut ancestors=Vec::new();
        for base in bases.iter(){
            if !self.stands_for_a_kind(&base.settled()) {
                // A parent that is no class but offers __mro_entries__
                // is turned away by name the way the reference turns it
                // away: type() never resolves entries; types.new_class()
                // does.
                let entry_word = self.detail("mro.entries").to_owned();
                if !entry_word.is_empty() && matches!(base.settled(), Value::Thing(_)) {
                    match self.read_class_member(base.clone(), &entry_word, false) {
                        Ok(_) => return Err("TypeError: type() doesn't support MRO entry resolution; use types.new_class()".to_owned().into()),
                        Err(escape) if self.missing_member_escape(&escape) => {}
                        Err(escape) => return Err(escape),
                    }
                }
                return Err("TypeError: metaclass conflict: the metaclass of a derived class must be a (non-strict) subclass of the metaclasses of all its bases".to_owned().into());
            }
            ancestors.push(self.parent_from_type(base)?);
        }
        if ancestors.is_empty(){ancestors.push(self.common_ancestor());}
        let selected=self.builder_for(Some(Value::Blueprint(explicit.clone())),&ancestors)?;
        if let Some(builder)=selected.as_ref().filter(|b|!Rc::ptr_eq(b,&explicit)) {
            if let Some(allocator)=self.inherited_entry(builder,self.detail("allocate")) {
                plain[0]=Value::Blueprint(builder.clone());
                plain.extend(named.into_iter().map(|(k,v)|Value::Couple(Rc::new((Value::text(&k),v)))));
                return self.apply_class_member(allocator,plain);
            }
        }
        let body=pairs.iter().map(|(k,v)|(k.bare(),v.clone())).collect();
        let keywords=named.into_iter().map(|(k,v)|Value::Couple(Rc::new((Value::text(&k),v)))).collect();
        self.assemble_class(title,ancestors,body,selected,keywords,plain[1].settled())
    }
    pub(super) fn reject_type_surrogates(&mut self, text: &Value) -> Result<(), Escape> {
        let Value::Unpaired(numbers) = text.settled() else { return Ok(()); };
        for (begin, &number) in numbers.iter().enumerate() {
            if !(0xd800..0xe000).contains(&number) { continue; }
            let mut finish = begin + 1;
            while finish < numbers.len() && (0xd800..0xe000).contains(&numbers[finish]) { finish += 1; }
            let kind = self.furnished_kind(43).ok_or_else(|| self.class_unready())?;
            let fault = self.make_fault(kind, vec![Value::text("utf-8"), text.clone(), Value::Small(begin as i64), Value::Small(finish as i64), Value::text("surrogates not allowed")], Value::Nil);
            return Err(Escape::Thrown(fault));
        }
        Ok(())
    }
    pub(super) fn type_argument_kind(value: &Value) -> String {
        if let Value::Thing(thing) = value.settled() {
            if let Some(names) = thing.blueprint().type_names.borrow().as_ref() {
                return names.short.type_text().bare();
            }
        }
        value.kind_word()
    }
    fn checked_type_name(&mut self, given: &Value) -> Result<String, Escape> {
        if self.detail("name").is_empty() { return Ok(Self::underlying(given).unwrap_or_else(|| given.settled()).bare()); }
        let native = given.type_text();
        self.reject_type_surrogates(&native)?;
        match native {
            Value::Text(word) if word.contains('\0') => Err(String::from("ValueError: type name must not contain null characters").into()),
            Value::Text(word) => Ok(word.to_string()),
            _ => Err(format!("TypeError: type.__new__() argument 1 must be str, not {}", Self::type_argument_kind(given)).into()),
        }
    }
    pub(super) fn ancestry_includes(class: &Rc<Blueprint>, target: &Rc<Blueprint>) -> bool {
        fn visit(class: &Rc<Blueprint>, target: &Rc<Blueprint>, seen: &mut Vec<*const Blueprint>) -> bool {
            if class.order_supplied.get() {
                return class.supplied_order.borrow().iter().flat_map(|ancestor| ancestor.upgrade()).any(|base| Rc::ptr_eq(&base, target));
            }
            if Rc::ptr_eq(class, target) || class.ancestry.borrow().iter().any(|base| Rc::ptr_eq(base, target)) { return true; }
            let address = Rc::as_ptr(class);
            if seen.contains(&address) { return false; }
            seen.push(address);
            if class.fields.iter().any(|(key, held)| key == "\0also-under"
                && matches!(held, Value::Blueprint(other) if visit(other, target, seen))) { return true; }
            if class.parents.iter().any(|base| visit(base, target, seen)) { return true; }
            class.under.as_ref().is_some_and(|base| visit(base, target, seen))
        }
        visit(class, target, &mut Vec::new())
    }
    fn visible_blueprint(&self,class:Rc<Blueprint>)->Value {
        if self.builds_classes(&class){return self.kind_builder_word();}
        if let Some(word)=Self::native_word(&class) {
            if let Some(public) = self.kind_by_word(&word) { return public; }
        }
        Value::Blueprint(class)
    }
    pub(super) fn resolution_order(owner: &Rc<Blueprint>) -> Vec<Rc<Blueprint>> {
        if owner.order_supplied.get() { return owner.supplied_order.borrow().iter().flat_map(|entry| entry.upgrade()).collect(); }
        let mut entries = owner.ancestry.borrow().clone();
        if !owner.order_supplied.get() { entries.insert(0, owner.clone()); }
        entries
    }
    fn root_in_resolution(&self, owner: &Rc<Blueprint>) -> bool {
        if !owner.order_supplied.get() { return true; }
        let Some(root) = &self.ancestor else { return false; };
        owner.supplied_order.borrow().iter().any(|member| member.upgrade().is_some_and(|base| Rc::ptr_eq(root,&base)))
    }
    fn object_descriptor_target(&mut self, value: &Value) -> Result<(String, bool), Escape> {
        let actual = self.class_from_type(vec![value.clone()])?;
        let name = actual.kind_it_names().ok_or_else(|| self.class_unready())?;
        let owner = self.parent_type_arg(&actual)?;
        let root = self.common_ancestor();
        Ok((name, Self::ancestry_includes(&owner, &root)))
    }
    fn default_attribute_slots(&self, owner: &Rc<Blueprint>) -> bool {
        if owner.order_supplied.get() {
            for member in Self::resolution_order(owner).iter() {
                if self.ancestor.as_ref().is_some_and(|root| Rc::ptr_eq(root, member)) { return true; }
                if Self::native_word(member).as_deref() == Some("module") { return true; }
            }
            return false;
        }
        true
    }
    fn storage_ancestor(&self,blueprint:&Rc<Blueprint>)->Rc<Blueprint> {
        if Self::native_word(blueprint).is_some()||self.builds_classes(blueprint){return blueprint.clone();}
        if blueprint.has_slot_storage {
            return blueprint.clone();
        }
        match &blueprint.under {Some(parent)=>self.storage_ancestor(parent),None=>blueprint.clone()}
    }
    fn primary_parent(&self,parents:&[Rc<Blueprint>])->Result<Option<Rc<Blueprint>>,Escape> {
        let mut preferred:Option<Rc<Blueprint>>=None;
        for parent in parents {
            let candidate=self.storage_ancestor(parent);
            if let Some(old)=preferred.as_ref() {
                let previous=self.storage_ancestor(old);
                if Rc::ptr_eq(&candidate,&previous)||previous.ancestry.borrow().iter().any(|p|Rc::ptr_eq(p,&candidate)){continue;}
                if !candidate.ancestry.borrow().iter().any(|p|Rc::ptr_eq(p,&previous)) {
                    return Err(self.table.single("ext.stmt.class.layout").unwrap_or(self.detail("unready")).to_owned().into());
                }
            }
            preferred=Some(parent.clone());
        }
        Ok(preferred)
    }
    /// The class itself, laid out from its name, its parents, its
    /// entries and the metaclass it is to remember. This is the building
    /// the kind primitive does, which a metaclass reaches through its
    /// forebears once it has made a namespace of its own.
    fn combine_orders(&self, parents: &[Rc<Blueprint>]) -> Result<Vec<Rc<Blueprint>>, Escape> {
        let mut queues=Vec::new();
        for base in parents {
            queues.push(Self::resolution_order(base));
        }
        queues.push(parents.to_vec());
        let mut ranks=Vec::new();
        loop {
            queues.retain(|q|!q.is_empty());
            if queues.is_empty(){break;}
            let mut chosen=None;
            'candidate: for q in &queues {
                for other in &queues {if other[1..].iter().any(|c|Rc::ptr_eq(c,&q[0])){continue 'candidate;}}
                chosen=Some(q[0].clone());break;
            }
            let next=chosen.ok_or_else(||self.detail("mro.amiss").to_owned())?;
            if ranks.iter().any(|b|Rc::ptr_eq(b,&next)){return Err(self.detail("mro.amiss").to_owned().into());}
            for q in &mut queues {if Rc::ptr_eq(&q[0],&next){q.remove(0);}}
            ranks.push(next);
        }
        Ok(ranks)
    }
    pub(super) fn assemble_class(&mut self,title:String,parents:Vec<Rc<Blueprint>>,mut entries:Vec<(String,Value)>,
        builder:Option<Rc<Blueprint>>,handed:Vec<Value>,title_object:Value)->Res {
        let mut class_cell = None;
        let cell_word = self.detail("classcell").to_owned();
        if !cell_word.is_empty() {
            if let Some(at) = entries.iter().position(|(key, _)| key == &cell_word) {
                let cell = entries.remove(at).1;
                if !matches!(cell, Value::Wrapped(35, _)) { return Err("TypeError: __classcell__ must be a nonlocal cell".to_owned().into()); }
                class_cell = Some(cell);
            }
        }

        // A place only an arm of a conditional writes to may stay
        // unwritten. Nothing stands in it, and the class is given no
        // entry for it: a name a conditional never bound is no member.
        entries.retain(|(_,v)|!matches!(v,Value::Unset));
        let allocation = self.detail("allocate").to_owned();
        if !allocation.is_empty() {
            for (word, entry) in &mut entries {
                if word == &allocation && matches!(entry, Value::Routine(_) | Value::Bound(..)) {
                    *entry = Self::wrap(4, vec![entry.clone()]);
                }
            }
        }
        if !self.table.strings("ext.system.module.cache").is_empty() {
            for (key, value) in &entries {
                if key == "__abc_tpflags__" && matches!(value.settled(), Value::Small(bits) if bits & 96 == 96) {
                    return Err(format!("TypeError: type {title} has both Py_TPFLAGS_SEQUENCE and Py_TPFLAGS_MAPPING set").into());
                }
            }
        }
        let ranks = self.combine_orders(&parents)?;
        // One thing cannot keep worths of two native kinds.
        let mut natives:Vec<String>=ranks.iter().filter_map(|b|Self::native_word(b)).collect();
        natives.dedup();
        if natives.len()>1 {return Err(self.table.strings("ext.stmt.class.layout").first().map(String::as_str).unwrap_or(self.detail("unready")).to_owned().into());}
        let primary=self.primary_parent(&parents)?;
        let namespace = match self.frames_named.last() {
            Some(caller) => self.routine_module(caller),
            None => Value::text(self.detail("main")),
        };
        if entries.iter().all(|(k,_)|k!=self.detail("module")){entries.push((self.detail("module").into(),namespace));}
        if let Some((_, candidate)) = entries.iter().find(|(k, _)| k == self.detail("qualified")) {
            if !matches!(if self.detail("name").is_empty() { Self::underlying(candidate).unwrap_or_else(|| candidate.settled()) } else { candidate.type_text() }, Value::Text(_) | Value::Unpaired(_)) { return Err(format!("TypeError: type __qualname__ must be a str, not {}", Self::type_argument_kind(candidate)).into()); }
        }
        let type_names = if self.detail("name").is_empty() { None } else {
            self.checked_type_name(&Value::text(&title))?;
            let full = entries.iter().find(|entry| entry.0 == self.detail("qualified")).map_or_else(|| title_object.clone(), |entry| entry.1.clone());
            match entries.iter().find(|entry| entry.0 == self.detail("doc")) {
                Some((_, held)) => { let text = held.type_text(); self.reject_type_surrogates(&text)?; }
                None => {
                    let doc_slot = entries.iter().find(|(key, _)| key == self.detail("slots")).is_some_and(|(_, value)| match value.settled() {
                        Value::Text(word) => word.as_ref() == self.detail("doc"),
                        Value::Tuple(words) => words.iter().any(|word| word.bare() == self.detail("doc")),
                        Value::Vector(words) => words.iter().any(|word| word.bare() == self.detail("doc")),
                        _ => false,
                    });
                    if !doc_slot { entries.push((self.detail("doc").to_owned(), Value::Nil)); }
                },
            }
            entries.retain(|entry| entry.0 != self.detail("qualified"));
            Some(crate::data::TypeNames { short: title_object, declared: full.clone(), full, module_key: self.detail("module").to_owned() })
        };
        let native_name = self.table.strings("ext.stmt.class.native.name").first().map(String::as_str)
            .and_then(|key| entries.iter().position(|(name, _)| name == key))
            .map(|position| entries.remove(position).1.settled());
        if native_name.as_ref().is_some_and(|value| !matches!(value, Value::Text(_))) {
            return Err(String::from("TypeError: native type name must be a str").into());
        }
        let shown=entries.iter().find(|(k,_)|k==self.detail("qualified")).map_or(title.clone(),|(_,v)|v.bare());
        // Keep the allocation decision separate from the writable slot
        // declaration: editing that declaration cannot resize a class.
        let storage = entries.iter().find(|entry| entry.0 == self.detail("slots")).map(|entry| {
            let declared = match entry.1.settled() { Value::Tuple(items) | Value::Vector(items) => items.as_ref().clone(), one => vec![one] };
            declared.into_iter().any(|item| match item { Value::Text(name) => name.as_ref() != "__dict__" && name.as_ref() != "__weakref__", _ => true })
        }).unwrap_or(false);
        let mut fixed = Vec::new();
        if let Some(name) = native_name { fixed.push(("\0native-name".to_string(), name)); }
        if let Some(owner) = builder { fixed.push(("\0metaclass".to_owned(), Value::Blueprint(owner))); }
        let module = entries.iter().find(|(key, _)| key == self.detail("module")).map(|(_, held)| held.bare()).unwrap_or_default();
        let class=Rc::new(Blueprint {presentation:Some(format!("<class '{module}.{shown}'>")),name:title,
            under:primary,parents,ancestry:RefCell::new(ranks),answers:vec![],fields:vec![],reaches:vec![],
            methods:vec![],constants:fixed,
            shared:RefCell::new(entries),weak_slot:Cell::new(None),has_slot_storage: storage, sealed:Cell::new(false), order_supplied:Cell::new(false), supplied_order: RefCell::new(Vec::new()), type_names: RefCell::new(type_names)});
        if let Some(cell) = &class_cell { let word = self.detail("cell.contents").to_owned(); self.alter_class_member(cell.clone(), &word, Some(Value::Blueprint(class.clone())), true)?; }

        // A builder whose own answering of the order is written out is
        // asked for it once while the class stands, and its answer is
        // the order the class keeps, measured the way the reference
        // measures it: never empty, every entry a class, and no two
        // native kinds kept in the one thing.
        let order_word = self.detail("order");
        if !order_word.is_empty() {
            if let Some(builder) = Self::builder_over(&class) {
                let builder_line = Self::resolution_order(&builder);
                let reader = builder_line.iter().find_map(|base| Self::own_entry(base, order_word));
                if let Some(reader @ (Value::Routine(_) | Value::Bound(..))) = reader {
                    let bound = self.member_binding(reader, Some(Value::Blueprint(class.clone())), builder)?;
                    let answer = self.apply_class_member(bound, Vec::new())?;
                    let given = self.gathered_members(&answer)?;
                    if given.is_empty() { return Err("TypeError: type MRO must not be empty".to_owned().into()); }
                    let mut adopted = Vec::new();
                    for item in &given {
                        let base = match item.settled() {
                            Value::Blueprint(b) => b,
                            Value::Intrinsic(op, word) if Self::names_a_kind(&op) => self.native_kind(word.as_ref()),
                            other => return Err(format!("TypeError: mro() returned a non-class ('{}')", self.parent_tp_name(&other)).into()),
                        };
                        adopted.push(base);
                    }
                    let layout = std::iter::once(&class).chain(class.ancestry.borrow().iter()).find_map(|b| Self::native_word(b));
                    if let Some(amiss) = adopted.iter().find(|b| Self::native_word(b).is_some() && Self::native_word(b) != layout) {
                        return Err(format!("TypeError: mro() returned base with unsuitable layout ('{}')", amiss.name).into());
                    }
                    // Keep the returned tuple verbatim; parents describe layout,
                    // not the position (or presence) of this blueprint in it.
                    class.supplied_order.replace(adopted.iter().map(Rc::downgrade).collect());
                    class.ancestry.replace(adopted.into_iter().filter(|member| !Rc::ptr_eq(member, &class)).collect());
                    class.order_supplied.set(true);
                }
            }
        }

        if !self.rules.words_ext_builtin_weak_get.is_empty() { crate::ghost::note(crate::ghost::Ghost::Blueprint(Rc::downgrade(&class))); }
        self.name_slots(&class)?;
        let dictionary_word = self.detail("namespace");
        let introduces_dictionary = self.allowed_slot(&class, dictionary_word)
            && class.parents.iter().all(|base| !self.allowed_slot(base, dictionary_word));
        if self.table.has_any("ext.stmt.class.builder") && introduces_dictionary && Self::own_entry(&class, dictionary_word).is_none() {
            let getter = Self::wrap(32, vec![Value::text(dictionary_word), Value::Blueprint(class.clone()), Value::Small(-2)]);
            class.shared.borrow_mut().push((dictionary_word.to_string(), getter));
        }
        let documentation = self.detail("doc");
        if Self::own_entry(&class, documentation).is_none() { class.shared.borrow_mut().push((documentation.to_owned(), Value::Nil)); }
        // Every entry whose blueprint wants its name is given it now, the
        // class standing, and before the forebears hear of it.
        if self.protocol_spelled() {
            let entries_now=class.shared.borrow().clone();
            for (key,held) in entries_now {
                if let Some(hook)=self.protocol_entry(&held,"descriptor.name") {
                    self.through_descriptor(&held,hook,vec![Value::Blueprint(class.clone()),Value::text(&key)])?;
                }
            }
        }
        if class.order_supplied.get() { self.parent_check(&class, &Value::Blueprint(class.clone()))?; }
        let chain = Self::resolution_order(&class);
        let tail = chain.iter().position(|entry| Rc::ptr_eq(entry, &class)).map_or(chain.len(), |at| at + 1);
        let hook=chain[tail..].iter().find_map(|b|Self::own_entry(b,self.detail("subclass")));
        if let Some(f)=hook {
            if matches!(&f,Value::Wrapped(5,_)){let bound=self.member_binding(f,None,class.clone())?;self.apply_class_member(bound,handed)?;}
            else{let mut given=vec![Value::Blueprint(class.clone())];given.extend(handed);self.apply_class_member(f,given)?;}
        }
        // What was handed over with no hook to take it is refused.
        else if !handed.is_empty(){return Err(format!("TypeError: {}.__init_subclass__() takes no keyword arguments", class.name).into());}
        // The class is told of to each forebear it was built under,
        // loosely, so the forebear's __subclasses__ names it while it
        // stands and never after.
        for parent in &class.parents {
            self.class_children.borrow_mut().entry(Rc::as_ptr(parent) as usize).or_default().push(Rc::downgrade(&class));
        }
        Ok(Value::Blueprint(class))
    }
    fn protocol_spelled(&self)->bool {!self.detail("descriptor.get").is_empty()}
    /// A blueprint's slots become entries of it: each a descriptor
    /// keeping the slot's worth in the thing under the slot's name and
    /// the blueprint that declared it, so a descendant declaring the
    /// same slot keeps its own.
    fn name_slots(&mut self,class:&Rc<Blueprint>)->Result<(),Escape> {
        if !self.protocol_spelled(){return Ok(());}
        class.weak_slot.set(Some(self.admits_weak(class)));
        let Some(declared)=Self::own_entry(class,self.detail("slots")) else {
            if !self.rules.words_ext_builtin_weak_get.is_empty() && self.admits_weak(class) && class.parents.iter().all(|parent| !self.admits_weak(parent)) {
                let read = Self::wrap(32, vec![Value::text("__weakref__"), Value::Blueprint(class.clone())]);
                class.shared.borrow_mut().push((String::from("__weakref__"), read));
            }
            return Ok(());
        };
        let words=match declared.settled(){
            Value::Tuple(items)|Value::Vector(items)=>items.as_ref().clone(),
            // A mapping stands a name at each key; the worth a key
            // holds is documentation for the slot, nothing a layout reads.
            Value::Dict(mapping)=>mapping.pairs().iter().map(|(key,_)|key.clone()).collect(),
            // A set stands its members as the names, in its own order.
            Value::Set(store)=>store.borrow().entries.iter().map(|(_,value)|value.clone()).collect(),
            alone=>vec![alone]};
        let native = Self::native_beneath(class);
        if !words.is_empty() && matches!(native.as_deref(), Some("tuple" | "bytes" | "int")) {
            return Err(format!("TypeError: nonempty __slots__ not supported for subtype of '{}'", native.unwrap()).into());
        }
        // Every slot is a name of its own: a string holding a plain
        // identifier, never another kind of value and never one that
        // could not be written after `self.`. `__dict__` and
        // `__weakref__` are written at most once each, since each
        // stands for one layout a thing may carry, not several.
        let mut dict_seen=0u32;
        let mut weakref_seen=0u32;
        for word in &words {
            let Value::Text(name)=word else{return Err("TypeError: __slots__ items must be strings".to_owned().into())};
            if !Self::valid_slot_name(name){return Err("TypeError: __slots__ items must be identifiers".to_owned().into());}
            match name.as_ref() {"__dict__"=>dict_seen+=1,"__weakref__"=>weakref_seen+=1,_=>{}}
        }
        if dict_seen>1{return Err("TypeError: __dict__ slot disallowed: we already got one".to_owned().into());}
        if weakref_seen>1{return Err("TypeError: __weakref__ slot disallowed: we already got one".to_owned().into());}
        // A forebear built of a program's own classes, none of which
        // named any slots, already carries a dict and a weak reference
        // wherever it stands, so naming either again here only doubles
        // what is already had. A forebear standing on a native kind
        // carries neither by itself, no more than the common ancestor
        // does, so it is passed over exactly as that ancestor is.
        if dict_seen==1||weakref_seen==1 {
            let root=self.common_ancestor();
            let slots_word=self.detail("slots").to_owned();
            let weak_parent = class.parents.iter().find(|b| self.admits_weak(b));
            let already = if weakref_seen == 1 { weak_parent.is_some() } else {
                class.ancestry.borrow().iter().any(|b|!Rc::ptr_eq(b,&root)&&Self::native_word(b).is_none()&&Self::own_entry(b,&slots_word).is_none())
            };
            if already {
                let which=if dict_seen==1{"__dict__"}else{"__weakref__"};
                return Err(format!("TypeError: {which} slot disallowed: we already got one").into());
            }
        }
        for word in words {
            let Value::Text(name)=word else{unreachable!()};
            let name = Rc::<str>::from(crate::build::member_spelling(&class.name, &name));
            if name.as_ref()==self.detail("namespace"){continue;}
            if Self::own_entry(class,&name).is_some(){return Err(format!("ValueError: '{name}' in __slots__ conflicts with class variable").into());}
            let descriptor=Self::wrap(32,vec![Value::Text(name.clone()),Value::Blueprint(class.clone())]);
            class.shared.borrow_mut().push((name.to_string(),descriptor));
        }
        Ok(())
    }
    /// Whether a slot's own name could follow `self.` in this language:
    /// not empty, opening on a letter or an underscore, and holding
    /// nothing after but letters, figures and underscores.
    fn valid_slot_name(word:&str)->bool {
        let mut letters=word.chars();
        match letters.next() {
            Some(c) if c=='_'||c.is_alphabetic() => {}
            _ => return false,
        }
        letters.all(|c|c=='_'||c.is_alphanumeric())
    }
    /// The key a slot's worth is kept under in a thing. A thing not of
    /// the declaring blueprint's line has no such key, and is told so.
    pub(super) fn slot_key(&self,thing:&Value,parts:&[Value])->Result<String,Escape> {
        let (Value::Thing(t),Some(Value::Blueprint(declared)))=(thing,parts.get(1)) else{return Err(self.class_unready())};
        let name=parts[0].bare();
        let of_line=Rc::ptr_eq(&t.blueprint(),declared)||t.blueprint().ancestry.borrow().iter().any(|b|Rc::ptr_eq(b,declared));
        if !of_line {
            let words=self.table.strings("ext.stmt.class.detail.descriptor.foreign");
            if words.len()!=4{return Err(self.class_unready());}
            return Err(format!("{}{name}{}{}{}{}{}",words[0],words[1],declared.name,words[2],t.blueprint().name,words[3]).into());
        }
        Ok(format!("\0slot:{name}:{:p}",Rc::as_ptr(declared)))
    }
    fn slot_value(&self,thing:&Value,parts:&[Value])->Res {
        if matches!(parts.get(2), Some(Value::Small(-1))) {
            match thing {
                Value::Thing(owner) if self.namespace_holding(thing).is_some() || Self::native_beneath(&owner.blueprint()).as_deref() == Some("module") => {
                    let space = owner.holds.borrow().iter().find(|entry| entry.0 == "\0dictionary").map(|entry| entry.1.clone());
                    return Ok(space.unwrap_or_else(|| Value::Attributes(owner.clone())));
                }
                _ => return Err(format!("TypeError: descriptor '__dict__' for 'module' objects doesn't apply to a '{}' object", Self::type_argument_kind(thing)).into()),
            }
        }
        let key=self.slot_key(thing,parts)?;
        let Value::Thing(t)=thing else{return Err(self.class_unready())};
        if matches!(parts.get(2), Some(Value::Small(-2))) {
            let saved = t.holds.borrow().iter().find(|entry| entry.0 == "\0dictionary").map(|entry| entry.1.clone());
            return Ok(saved.unwrap_or_else(|| Value::Attributes(t.clone())));
        }
        if parts[0].bare() == "__weakref__" && !self.rules.words_ext_builtin_weak_get.is_empty() {
            return Ok(crate::ghost::refs_for(thing).into_iter().next().unwrap_or(Value::Nil));
        }
        let held=t.holds.borrow().iter().find(|(k,_)|*k==key).map(|(_,v)|v.clone());
        held.ok_or_else(||self.absent_attribute(thing,&parts[0].bare()))
    }
    fn slot_change(&self,thing:&Value,parts:&[Value],replacement:Option<Value>)->Res {
        if parts.get(2).is_some_and(|item| matches!(item, Value::Small(-1))) {
            let _ = self.slot_value(thing, parts)?;
            return Err(self.detail("property.readonly").to_owned().into());
        }
        let key=self.slot_key(thing,parts)?;
        let Value::Thing(t)=thing else{return Err(self.class_unready())};
        if matches!(parts.get(2), Some(Value::Small(-2))) { return self.install_namespace(t, replacement); }

        if parts[0].bare() == "__weakref__" && !self.rules.words_ext_builtin_weak_get.is_empty() { return Err(format!("AttributeError: attribute '__weakref__' of '{}' objects is not writable", t.blueprint().name).into()); }
        if !Self::change_entry(&mut t.holds.borrow_mut(),&key,replacement){return Err(self.absent_attribute(thing,&parts[0].bare()));}
        Ok(Value::Nil)
    }
    /// The blueprint of every property, made once. It holds the
    /// workings of the protocol as its entries -- reading, writing and
    /// removing through the accessors a property keeps, and the calls
    /// that make a fresh property from one with an accessor changed --
    /// and a class built on it takes them all.
    pub(super) fn property_blueprint(&mut self)->Rc<Blueprint> {
        if let Some(b)=&self.property_kind{return b.clone();}
        let root=self.common_ancestor();
        let title=self.table.prims.iter().find(|(_,op)|**op==Prim::ClassWork(11)).map(|(w,_)|w.to_string()).unwrap_or_default();
        let mut entries=Vec::new();
        for (part,tag) in [("descriptor.get",50u8),("descriptor.set",51),("descriptor.delete",52),("property.getter",53),("property.deleter",55),("descriptor.name",57)] {
            entries.push((self.detail(part).to_owned(),Self::wrap(tag,Vec::new())));
        }
        if let Some(word)=self.table.single("ext.stmt.class.property.setter"){entries.push((word.to_owned(),Self::wrap(54,Vec::new())));}
        if let Some(word)=self.rules.words_ext_stmt_class_constructor.first().map(String::as_str){entries.push((word.to_owned(),Self::wrap(56,Vec::new())));}
        for part in ["property.fget","property.fset","property.fdel","doc","property.is_abstract"] {
            entries.push((self.detail(part).to_owned(),Self::wrap(58,vec![Value::text(Self::accessor_key(part))])));
        }
        entries.push((self.detail("name").to_owned(),Self::wrap(58,vec![Value::text("\0name")])));
        let kind=Rc::new(Blueprint {presentation:Some(format!("<class '{title}'>")),name:title,
            parents:vec![root.clone()],ancestry:RefCell::new(vec![root.clone()]),under:Some(root),answers:Vec::new(),fields:Vec::new(),
            reaches:Vec::new(),methods:Vec::new(),constants:Vec::new(),shared:RefCell::new(entries),weak_slot:Cell::new(None),has_slot_storage: false, sealed:Cell::new(false), order_supplied:Cell::new(false), supplied_order: RefCell::new(Vec::new()), type_names: std::cell::RefCell::new(None)});
        self.property_kind=Some(kind.clone());
        kind
    }
    /// The keys a property keeps its accessors under, out of reach of
    /// any name a program can write.
    fn accessor_key(part:&str)->&'static str {
        match part {"property.fget"=>"\0fget","property.fset"=>"\0fset","property.fdel"=>"\0fdel","doc"=>"\0doc","property.is_abstract"=>"\0abstract",_=>"\0name"}
    }
    /// Whether a property subclass can keep a docstring on the thing
    /// itself: a class laying out slots keeps one only where a slot
    /// names it, and the property blueprint, which keeps none, is no
    /// namespace for a subclass.
    fn property_keeps_doc(&self,class:&Rc<Blueprint>)->bool {
        let Some(property)=&self.property_kind else { return true };
        let slots_word=self.detail("slots");
        let doc_word=self.detail("doc");
        let names=self.detail("namespace");
        let names_it=|listed:&[Value]| listed.iter().any(|s| matches!(s.settled(),Value::Text(t) if t.as_ref()==doc_word || t.as_ref()==names));
        let mut current=class.clone();
        loop {
            if Rc::ptr_eq(&current,property){return false;}
            match Self::own_entry(&current,slots_word) {
                Some(declared)=>{
                    let listed:Vec<Value>=match declared.settled(){Value::Tuple(v)|Value::Vector(v)=>v.to_vec(),other=>vec![other]};
                    if names_it(&listed){return true;}
                }
                None=>return true,
            }
            let Some(base)=current.under.clone() else { return false };
            current=base;
        }
    }
    /// The blueprint of every classmethod, made once. It keeps the
    /// routine it was given under a name no program can spell, and read
    /// through a class it binds the routine to that class, as the
    /// reference's own classmethod does.
    pub(super) fn classmethod_blueprint(&mut self)->Rc<Blueprint> {
        if let Some(b)=&self.classmethod_kind{return b.clone();}
        let root=self.common_ancestor();
        let title=self.table.prims.iter().find(|(_,op)|**op==Prim::ClassWork(10)).map(|(w,_)|w.to_string()).unwrap_or_default();
        let kind=Rc::new(Blueprint {presentation:Some(format!("<class '{title}'>")),name:title,
            parents:vec![root.clone()],ancestry:RefCell::new(vec![root.clone()]),under:Some(root.clone()),answers:Vec::new(),fields:Vec::new(),
            reaches:Vec::new(),methods:Vec::new(),constants:Vec::new(),
            shared:RefCell::new(vec![
                (self.detail("descriptor.get").to_owned(),Self::wrap(201,Vec::new())),
                (self.rules.words_ext_stmt_class_constructor.first().map(String::as_str).unwrap_or_default().to_owned(),Self::wrap(200,Vec::new())),
            ]),
            weak_slot:Cell::new(None),has_slot_storage: false, sealed:Cell::new(false), order_supplied: std::cell::Cell::new(false), supplied_order: RefCell::new(Vec::new()), type_names: std::cell::RefCell::new(None)});
        self.classmethod_kind=Some(kind.clone());
        kind
    }
    /// The blueprint of every staticmethod, made once: it keeps the
    /// routine it was given the same way, and read through anything it
    /// hands the routine back unbound.
    pub(super) fn staticmethod_blueprint(&mut self)->Rc<Blueprint> {
        if let Some(b)=&self.staticmethod_kind{return b.clone();}
        let root=self.common_ancestor();
        let title=self.table.prims.iter().find(|(_,op)|**op==Prim::ClassWork(9)).map(|(w,_)|w.to_string()).unwrap_or_default();
        let kind=Rc::new(Blueprint {presentation:Some(format!("<class '{title}'>")),name:title,
            parents:vec![root.clone()],ancestry:RefCell::new(vec![root.clone()]),under:Some(root.clone()),answers:Vec::new(),fields:Vec::new(),
            reaches:Vec::new(),methods:Vec::new(),constants:Vec::new(),
            shared:RefCell::new(vec![
                (self.detail("descriptor.get").to_owned(),Self::wrap(203,Vec::new())),
                (self.rules.words_ext_stmt_class_constructor.first().map(String::as_str).unwrap_or_default().to_owned(),Self::wrap(202,Vec::new())),
            ]),
            weak_slot:Cell::new(None),has_slot_storage: false, sealed:Cell::new(false), order_supplied: std::cell::Cell::new(false), supplied_order: RefCell::new(Vec::new()), type_names: std::cell::RefCell::new(None)});
        self.staticmethod_kind=Some(kind.clone());
        kind
    }
    /// Whether a value stands for the classmethod builtin read as a
    /// class: its word, before anything else was bound to that name.
    pub(super) fn spells_classmethod_kind(&self,value:&Value)->bool {
        self.spells_wrapper_kind(value,10)
    }
    /// Whether a value stands for the staticmethod builtin read as a
    /// class.
    pub(super) fn spells_staticmethod_kind(&self,value:&Value)->bool {
        self.spells_wrapper_kind(value,9)
    }
    fn spells_wrapper_kind(&self,value:&Value,which:u8)->bool {
        match value {
            // Intrinsics already carry their role; looking up a spelling
            // again makes every ordinary isinstance pay for three wrappers.
            Value::Intrinsic(operation, _) => *operation == Prim::ClassWork(which),
            Value::Wrapped(8, parts) => match parts.first() {
                Some(Value::Text(word)) => self.table.prims.get(word.as_ref()) == Some(&Prim::ClassWork(which)),
                _ => false,
            },
            _ => false,
        }
    }
    /// Recognize the property primitive by its role, including named wrapper adapters.
    pub(super) fn spells_property_kind(&self,value:&Value)->bool {
        self.spells_wrapper_kind(value, 11)
    }
    fn kept_accessor(property:&Thing,key:&str)->Option<Value> {
        property.holds.borrow().iter().find(|(k,_)|k==key).map(|(_,v)|v.clone()).filter(|v|!matches!(v,Value::Nil))
    }
    /// A property complaining of an accessor it lacks: the label's words
    /// around its name, where it was given one, and the blueprint of
    /// the thing it was asked about.
    fn accessor_complaint(&self,part:&str,property:&Thing,about:&Value)->Escape {
        let words=self.table.strings(&format!("ext.stmt.class.detail.{part}"));
        if words.len()!=3{return self.class_unready();}
        let title=Self::kept_accessor(property,"\0name").map(|n|format!(" '{}'",n.bare())).unwrap_or_default();
        let of=match about {Value::Thing(t)=>{ let b=t.blueprint(); let shown=b.type_names.borrow().as_ref().map_or_else(||b.name.clone(),|names|names.full.bare()); shown },Value::Blueprint(b)=>if self.detail("name").is_empty(){b.name.clone()}else{Self::builder_over(b).map_or_else(||b.name.clone(),|builder|builder.type_names.borrow().as_ref().map_or_else(||builder.name.clone(),|names|names.short.type_text().bare()))},other=>other.bare()};
        format!("{}{title}{}{of}{}",words[0],words[1],words[2]).into()
    }
    /// What a property's kept accessor shows: itself; or, for a first
    /// string, the one given, else the getter's own. A name is the one a
    /// class gave it, else the getter's; whether it is abstract any of
    /// its accessors says.
    fn accessor_shown(&mut self,property:&Thing,key:&str)->Res {
        // Whether the property stands for a question nobody has
        // answered: a plain yes or no, taken from the marks on its
        // getter, its setter and its deleter alike. What a mark holds
        // is weighed as any truth is, and a complaint other than a
        // mark simply not being there travels on.
        if key=="\0abstract" {
            let word=self.detail("property.is_abstract").to_owned();
            for accessor in ["\0fget","\0fset","\0fdel"] {
                let Some(held)=Self::kept_accessor(property,accessor) else { continue };
                let marked=match self.read_class_member(held,&word,false) {
                    Ok(worth)=>worth,
                    Err(escape) if self.missing_member_escape(&escape)=>continue,
                    Err(escape)=>return Err(escape),
                };
                if self.object_truth(&marked)? { return Ok(Value::Flag(true)); }
            }
            return Ok(Value::Flag(false));
        }
        if let Some(v)=Self::kept_accessor(property,key){return Ok(v);}
        if key=="\0doc" {
            if let Some(Value::Routine(getter)|Value::Bound(getter,_))=Self::kept_accessor(property,"\0fget") {
                return Ok(getter.doc.as_ref().map_or(Value::Nil,|d|Value::text(d)));
            }
            return Ok(Value::Nil);
        }
        if key=="\0name" {
            if let Some((_,held))=property.holds.borrow().iter().find(|(k,_)|k=="\0name") { return Ok(held.clone()); }
            if let Some(getter)=Self::kept_accessor(property,"\0fget") {
                let name_word=self.detail("name").to_owned();
                return match self.read_class_member(getter,&name_word,false) {
                    Ok(held)=>Ok(held),
                    Err(escaped) if self.missing_member_escape(&escaped)=>Err("AttributeError: 'property' object has no attribute '__name__'".to_owned().into()),
                    Err(escaped)=>Err(escaped),
                };
            }
            return Err("AttributeError: 'property' object has no attribute '__name__'".to_owned().into());
        }
        Ok(Value::Nil)
    }
    /// Whether a property still waits on an implementation: one of the
    /// accessors it keeps carries a mark that counts as true. An
    /// accessor with no mark counts as not abstract, and a mark that
    /// cannot be counted raises as it would.

    /// The workings of the property blueprint. Each is handed the
    /// property first, then whatever the program gave.
    fn property_operation(&mut self,tag:u8,values:Vec<Value>)->Res {
        let Some(Value::Thing(property))=values.first().cloned() else{return Err(self.class_unready())};
        let rest=&values[1..];
        match tag {
            50=>{
                let about=rest.first().cloned().unwrap_or(Value::Nil);
                if matches!(about,Value::Nil){return Ok(values[0].clone());}
                let Some(getter)=Self::kept_accessor(&property,"\0fget") else{return Err(self.accessor_complaint("property.unreadable",&property,&about))};
                self.apply_class_member(getter,vec![about])
            }
            51=>{
                let [about,worth]=rest else{return Err(self.class_unready())};
                let Some(setter)=Self::kept_accessor(&property,"\0fset") else{return Err(self.accessor_complaint("property.unwritable",&property,about))};
                self.apply_class_member(setter,vec![about.clone(),worth.clone()])?;
                Ok(Value::Nil)
            }
            52=>{
                let [about]=rest else{return Err(self.class_unready())};
                let Some(remover)=Self::kept_accessor(&property,"\0fdel") else{return Err(self.accessor_complaint("property.undeletable",&property,about))};
                self.apply_class_member(remover,vec![about.clone()])?;
                Ok(Value::Nil)
            }
            // A fresh property of the same blueprint with one accessor
            // changed; the class that takes it will name it.
            53|54|55=>{
                let [accessor]=rest else{return Err(self.class_unready())};
                let key=match tag {53=>"\0fget",54=>"\0fset",_=>"\0fdel"};
                let kept=|part:&str| if part==key { Some(accessor.clone()) } else { Self::kept_accessor(&property,part) };
                // A copy is built the way any property is, from the
                // accessors and the docstring the reference carries over:
                // where the docstring came from a getter it is picked
                // again from the accessor the copy is made with.
                let carried=property.holds.borrow().iter().find(|(k,_)|k=="\0getterdoc").map(|(_,v)|matches!(v,Value::Flag(true))).unwrap_or(false);
                let doc=if carried && kept("\0fget").is_some() { Value::Nil } else {
                    property.holds.borrow().iter().find(|(k,_)|k=="\0doc").map(|(_,v)|v.clone()).unwrap_or(Value::Nil)
                };
                // The copy is made by calling the class, as the reference
                // does, so a subclass's own making has its say.
                let made=self.make_instance(property.blueprint().clone(),vec![kept("\0fget").unwrap_or(Value::Nil),kept("\0fset").unwrap_or(Value::Nil),kept("\0fdel").unwrap_or(Value::Nil),doc])?;
                if let Value::Thing(thing)=&made {
                    let class=thing.blueprint();
                    if self.property_kind.as_ref().map_or(false,|known| Rc::ptr_eq(&class,known) || class.ancestry.borrow().iter().any(|b| Rc::ptr_eq(b,known))) {
                        let named = property.holds.borrow().iter().find_map(|(key, value)| (key == "\0name").then(|| value.clone()));
                        if let Some(value) = named { Self::change_entry(&mut thing.holds.borrow_mut(), "\0name", Some(value)); }
                    }
                }
                Ok(made)
            }
            56=>{
                let parts=["property.fget","property.fset","property.fdel","property.doc"];
                let (positional,named)=self.open_arguments(rest.to_vec())?;
                if positional.len()>4{return Err(self.class_unready());}
                let mut taken=vec![Value::Nil;4];
                for (i,v) in positional.into_iter().enumerate(){taken[i]=v;}
                for (key,v) in named {
                    let Some(i)=parts.iter().position(|p|self.detail(p)==key) else{return Err(self.argument_fault("ext.syntax.call.amiss.unknown",Some(&key)).into())};
                    taken[i]=v;
                }
                // Clear native metadata and install the supplied callbacks
                // first; a doc lookup error must not restore older callbacks.
                let plain=self.property_kind.as_ref().map_or(false,|known| Rc::ptr_eq(known,&property.blueprint()));
                {
                    let mut holds=property.holds.borrow_mut();
                    holds.retain(|(key,_)| !["\0fget", "\0fset", "\0fdel", "\0name", "\0doc", "\0getterdoc"].contains(&key.as_str()));
                    for (key,v) in ["\0fget","\0fset","\0fdel"].into_iter().zip(taken.iter().take(3)) { holds.push((key.to_owned(),v.clone())); }
                    holds.extend([(String::from("\0doc"),Value::Nil),(String::from("\0getterdoc"),Value::Flag(false))]);
                }
                // A docstring given is kept as it is; where none was given
                // the getter's own is taken, and that it came from there is
                // remembered so a later copy takes the new getter's.
                let mut doc=taken[3].clone();
                let mut getter_doc=false;
                if matches!(doc,Value::Nil) && !matches!(taken[0],Value::Nil) {
                    let doc_word=self.detail("doc").to_owned();
                    match self.read_class_member(taken[0].clone(),&doc_word,false) {
                        Ok(found)=>{ if !matches!(found.settled(),Value::Nil){ doc=found; getter_doc=true; } }
                        Err(escaped) if self.missing_member_escape(&escaped)=>{}
                        Err(escaped)=>return Err(escaped),
                    }
                }
                {
                    let mut holds=property.holds.borrow_mut();
                    Self::change_entry(&mut holds,"\0doc",Some(doc.clone()));
                    Self::change_entry(&mut holds,"\0getterdoc",Some(Value::Flag(getter_doc)));
                }
                if !plain {
                    // A subclass keeps its docstring on the thing itself, as
                    // the reference does, so the class's own __doc__ does not
                    // shadow it. A class with nowhere to keep one drops an
                    // ordinary docstring and refuses one that came from a
                    // getter, which is the reference's own exception.
                    let doc_word=self.detail("doc").to_owned();
                    if self.property_keeps_doc(&property.blueprint()) {
                        self.alter_class_member(Value::Thing(property.clone()),&doc_word,Some(doc),false)?;
                    } else if getter_doc {
                        return Err("AttributeError: readonly attribute".to_owned().into());
                    }
                }
                Ok(Value::Nil)
            }
            57=>{
                if rest.len()!=2 { return Err(format!("TypeError: __set_name__() takes 2 positional arguments but {} were given",rest.len()).into()); }
                Self::change_entry(&mut property.holds.borrow_mut(),"\0name",Some(rest[1].clone()));
                Ok(Value::Nil)
            }
            _=>Err(self.class_unready()),
        }
    }
    /// The hook a member answers a part of the protocol with: the entry
    /// of its blueprint under the word the table gives that part.
    fn protocol_entry(&self,member:&Value,part:&str)->Option<Value> {
        let word=self.detail(part);
        if word.is_empty(){return None;}
        match member.settled() {Value::Thing(t)=>self.inherited_entry(&t.blueprint(),word),_=>None}
    }
    fn through_descriptor(&mut self,member:&Value,hook:Value,values:Vec<Value>)->Res {
        let member = member.settled();
        let Value::Thing(t)=&member else{return Err(self.class_unready())};
        // Native descriptor slots already receive their descriptor explicitly.
        // Keep the live hook lookup, including every subclass override, but
        // avoid allocating a temporary bound wrapper for these intrinsic slots.
        if matches!(&hook, Value::Wrapped(50..=57 | 201 | 203, _)) {
            let mut supplied = Vec::with_capacity(values.len() + 1);
            supplied.push(member); supplied.extend(values);
            if let Value::Wrapped(tag @ 50..=57, _) = &hook { return self.property_operation(*tag, supplied); }
            return self.apply_class_member(hook, supplied);
        }
        let bound=self.member_binding(hook,Some(member.clone()),t.blueprint().clone())?;
        self.apply_class_member(bound,values)
    }
    /// A member that takes writes speaks before a thing's own entries;
    /// one that only reads gives way to them.
    fn writes_too(&self,member:&Value)->bool {
        if let Value::Wrapped(tag,_)=member {return matches!(tag,6|32|58);}
        if self.names_in_calls {
            match member {
                Value::Nil | Value::Small(_) | Value::Huge(_) | Value::Flag(_) | Value::Frac(_) | Value::Text(_) | Value::Vector(_) | Value::Tuple(_) | Value::Dict(_) | Value::Set(_) | Value::Octets { .. } | Value::Routine(_) | Value::Bound(..) | Value::Method(..) => return false,
                _ => {},
            }
        }
        self.protocol_entry(member,"descriptor.set").is_some()||self.protocol_entry(member,"descriptor.delete").is_some()
    }
    /// Whether an escape tells of a member that is not there, in words
    /// or as a raised thing of that kind.
    pub(super) fn missing_member_escape(&self,escape:&Escape)->bool {
        let Some(kind)=self.detail("attribute.amiss").split(':').next().filter(|k|!k.is_empty()) else{return false};
        match escape {
            Escape::Error(words)=>words.split(':').next()==Some(kind),
            Escape::Thrown(Value::Thing(t))=>t.blueprint().name==kind||t.blueprint().ancestry.borrow().iter().any(|b|b.name==kind),
            _=>false,
        }
    }
    pub(super) fn own_entry(b:&Blueprint,key:&str)->Option<Value> {
        if let Some((_, held)) = b.shared.borrow().iter().find(|(word, _)| word == key) {
            return (!matches!(held.settled(), Value::Unset)).then(|| held.clone());
        }
        b.methods.iter().find(|(n,_)|n==key).map(|(_,p)|Value::Routine(p.clone()))
            .or_else(||b.constants.iter().find(|(n,_)|n==key).map(|(_,v)|v.clone()))
    }
    pub(super) fn native_declares_protocol(&self, spelling: &str, key: &str) -> bool {
        self.rules.words_ext_stmt_class_detail_native_protocols.chunks_exact(2)
            .any(|pair| pair[0] == spelling && pair[1].split_whitespace().any(|entry| entry == key))
    }
    pub(super) fn allocation_changed(&self, class: &Blueprint) -> bool {
        if self.rules.words_ext_stmt_class_detail_native_protocols.is_empty() { return false; }
        let key = self.detail("allocate");
        Self::lookup_ordered(class, |parent| {
            match Self::own_entry(parent, key) {
                Some(value) => Some(!matches!(value, Value::Wrapped(14, _))),
                None if Self::native_word(parent).is_some_and(|word| self.native_declares_protocol(&word, key)) => Some(false),
                None => None,
            }
        }).unwrap_or(false)
    }
    fn lookup_ordered<T>(owner: &Blueprint, mut lookup: impl FnMut(&Blueprint) -> Option<T>) -> Option<T> {
        if !owner.order_supplied.get() {
            return lookup(owner).or_else(|| owner.ancestry.borrow().iter().find_map(|base| lookup(base)));
        }
        for ancestor in owner.supplied_order.borrow().iter() {
            if let Some(base) = ancestor.upgrade() {
                if let result @ Some(_) = lookup(&base) { return result; }
            }
        }
        None
    }

    pub(super) fn inherited_entry(&self,b:&Blueprint,key:&str)->Option<Value> {
        Self::lookup_ordered(b, |parent| {
            Self::own_entry(parent, key).or_else(|| {
                if !self.table.has_class_order { return None; }
                let spelling = Self::native_word(parent)?;
                if self.rules.specials.get(8).is_some_and(|hash| hash == key) {
                    match spelling.as_str() { "dict" | "set" | "bytearray" | "list" => return Some(Value::Nil), _ => {} }
                }
                if !self.native_declares_protocol(&spelling, key) { return None; }
                let sample = self.kind_stand_in(&spelling)?;
                let protocols = self.rules.specials;
                if protocols.get(8).is_some_and(|hash| hash == key)
                    && protocols.get(2).is_some_and(|eq| self.native_member(&sample, eq))
                    && !self.native_member(&sample, key) { return Some(Value::Nil); }
                if !self.native_member(&sample, key) { return None; }
                Some(self.kind_entry(&spelling, key))
            })
        })
    }
    fn capture_title(word: &str) -> &str {
        match word.strip_prefix("#completed_class") { Some(_) => "__class__", None => word }
    }
    fn wrap(tag:u8,items:Vec<Value>)->Value {Value::Wrapped(tag,Rc::new(items).into())}
    pub(super) fn check_constructor_expansion(&self, class: &Rc<Blueprint>, values: &[Value]) -> Res<()> {
        for item in values {
            let Value::Couple(pair)=item else{continue};
            if !matches!(pair.0,Value::Flag(true)){continue}
            let source=pair.1.settled();
            let expanded=Self::underlying(&source).unwrap_or(source).settled();
            if !matches!(expanded,Value::Dict(_)|Value::Attributes(_)) {
                let namespace=Self::own_entry(class,self.detail("module"));
                let title=namespace.filter(|home|home.bare()!=self.builtin_module()).map_or_else(||class.name.clone(),|home|format!("{}.{}",home.bare(),class.name));
                return Err(format!("TypeError: {title}() argument after ** must be a mapping, not {}",pair.1.kind_word()).into());
            }
        }
        Ok(())
    }
    pub(super) fn apply_class_member(&mut self,f:Value,mut values:Vec<Value>)->Res {
        if let Value::Blueprint(class)=&f { self.check_constructor_expansion(class, &values)?; }
        let f = if matches!(f, Value::Shared(_)) { self.what_it_spells(f) } else { f };
        match f {
            Value::Wrapped(82, _) => {
                // A group's own maker: the kind to make, then its heading
                // and its members, as the reference's __new__.
                match values.first().map(Value::settled) {
                    Some(Value::Blueprint(kind)) if self.stands_under(&kind, 37) => {
                        self.allocate_fault(kind, values[1..].to_vec())
                    }
                    Some(other) => {
                        let word = other.kind_it_names().or_else(|| self.kind_spelling(&other).map(|name| name.to_string()));
                        let complaint = match word {
                            Some(name) => format!("TypeError: BaseExceptionGroup.__new__({0}): {0} is not a subtype of BaseExceptionGroup", name),
                            None => format!("TypeError: BaseExceptionGroup.__new__(X): X is not a type object ({})", Self::type_argument_kind(&other)),
                        };
                        Err(complaint.into())
                    }
                    None => Err(String::from("TypeError: BaseExceptionGroup.__new__(): not enough arguments").into()),
                }
            }
            Value::Bound(code,environment)=>self.invoke(code,environment,values),
            Value::Routine(code)=>self.invoke(code,self.outermost.clone(),values),
            Value::Intrinsic(Prim::ClassWork(op), _) if !self.rules.words_ext_stmt_class_builder.is_empty() => self.work_on_class(op, values),
            Value::Intrinsic(Prim::SortOf, _) if !self.rules.words_ext_stmt_class_builder.is_empty() => self.class_from_type(values),
            Value::Intrinsic(operation, name) => {
                let (mut input, keywords) = self.arguments_for_native(&name, values)?;
                if let Some(answer) = self.builtin_names(operation, &name, &mut input, keywords)? { return Ok(answer); }
                let result = self.prim(operation, &name, &input);
                if let Some(raised) = self.got_away.take() { return Err(raised); }
                Ok(result?)
            }
            // A method tied to a value of a native kind, reached as a
            // value of its own and then called.
            Value::Member(receiver,operation)=>{
                if self.rules.words_ext_stmt_class_detail_native_protocols.is_empty() {
                    self.value_member(&receiver,&operation,values,Vec::new())
                } else {
                    let (positional, named) = self.open_arguments(values)?;
                    let answer = self.value_member(&receiver, &operation, positional, named);
                    if let Some(escape) = self.got_away.take() { return Err(escape); }
                    answer
                }
            },
            Value::Method(code,thing,_)=>{values.insert(0,Value::Thing(thing));self.invoke(code,self.outermost.clone(),values)},
            // A thing called stands on its own call member, which may
            // be a thing again. Reaching through one makes no frame, so
            // the step is counted among the calls standing all the
            // same: a thing whose call member is a thing of its own
            // kind is refused at the depth the table allows, as any
            // call that never comes back is.
            Value::Thing(t)=>{let called=self.inherited_entry(&t.blueprint(),self.detail("call")).ok_or_else(|| {
                let message = self.core_complaint("core.uncallable", &t.blueprint().name);
                if message.is_empty() { self.class_unready() } else { message.into() }
            })?;self.deeper()?;values.insert(0,Value::Thing(t));let answer=self.apply_class_member(called,values);self.standing-=1;answer},
            Value::Blueprint(c)=>self.construct_ordered(c,values),
            Value::Wrapped(235, _) => {
                let (mut input, keywords) = self.open_arguments(values)?;
                let Some(first) = input.first().map(Value::settled) else { return Err(String::from("TypeError: descriptor '__get__' of 'classmethod_descriptor' object needs an argument").into()); };
                match &first {
                    Value::Wrapped(60, fields) if fields.len() == 3 => (),
                    other => return Err(format!("TypeError: descriptor '__get__' requires a 'classmethod_descriptor' object but received a '{}'", other.kind_word()).into()),
                }
                input.remove(0);
                self.get_native_classmethod(&first, input, keywords)
            },
            Value::Wrapped(tag,kept)=>{
                // A loose `dict.fromkeys` descriptor is the class method:
                // its first argument is the iterable, not a receiver.
                if tag == 60 && kept.len() == 2 && self.table.spells("ext.builtin.method.fromkeys", &kept[1].bare()) {
                    let class = self.native_kind(&kept[0].bare());
                    return self.value_member(&Value::Blueprint(class), "fromkeys", values, Vec::new());
                }
                match tag {
                    130 => {
                        if !values.is_empty() { return Err(String::from("TypeError: function takes no arguments").into()); }
                        Ok(self.prim(Prim::Weigh, "eval", &kept)?)
                    }
                    127 => {
                        let first = values.first().cloned().ok_or_else(|| self.class_unready())?;
                        let row = self.read_class_member(first, "__args__", true)?;
                        let mut parts = Vec::new();
                        for member in self.gathered_members(&row)? {
                            let label = match member.settled() {
                                Value::Blueprint(_) => {
                                    let namespace = self.read_class_member(member.clone(), "__module__", false)?.bare();
                                    let qualified = self.read_class_member(member.clone(), "__qualname__", false)?.bare();
                                    if namespace == "builtins" { qualified } else { namespace + "." + &qualified }
                                }
                                _ => match member.kind_it_names() { Some(name) => name, None => self.prim(Prim::Quoted, "repr", &[member])?.bare() },
                            };
                            parts.push(if label == "NoneType" { String::from("None") } else { label });
                        }
                        Ok(Value::text(&parts.join(" | ")))
                    }
                    140 => {
                        let subscript = values.last().cloned().unwrap_or(Value::Nil).settled();
                        let members: Vec<Value> = match subscript {
                            Value::Tuple(row) if row.is_empty() => return Err(String::from("TypeError: Cannot take a Union of no types.").into()),
                            Value::Tuple(row) => row.to_vec(),
                            other => vec![other],
                        };
                        let mut folded: Option<Value> = None;
                        for member in members {
                            folded = Some(match folded { None => member, Some(acc) => self.combined_types(&[acc, member]) });
                        }
                        Ok(folded.unwrap_or(Value::Nil))
                    }
                    125 => self.alias_action(&kept, values),
                    124 => self.parent_initialised(values),
                    123 => self.namespace_action(&kept, values),
                    9 if kept.is_empty() && values.len() == 2 => {
                        values = values.iter().map(Value::settled).collect();
                        if let Value::Blueprint(_) = values[0] { Ok(Value::Wrapped(9, Rc::new(values).into())) }
                        else { Err(self.class_unready()) }
                    },
                    45 if !values.is_empty() => {
                        let alias = self.alias_from_thunk(values.remove(0))?;
                        if let (Value::Thing(object), Some(parameters)) = (&alias, values.first()) {
                            let mut attributes = object.holds.borrow_mut();
                            if let Some(entry) = attributes.iter_mut().find(|entry| entry.0 == "__type_params__") { entry.1 = parameters.clone(); }
                        }
                        Ok(alias)
                    }
                    76 if values.len() == 1 => {
                        let mut mask = values[0].clone();
                        loop {
                            match mask {
                                Value::Mutable(cell, _) | Value::Shared(cell) => {
                                    let current = cell.borrow().clone();
                                    if let Value::Vector(row) = current {
                                        let mut entries = row.as_ref().clone();
                                        entries[0] = Value::Flag(true);
                                        *cell.borrow_mut() = Value::Vector(Rc::new(entries).into());
                                        return Ok(Value::Nil);
                                    }
                                    mask = current;
                                },
                                _ => return Err(String::from("TypeError: annotation execution mask is not mutable").into()),
                            }
                        }
                    }
                    74 if values.len() == 3 => {
                        if let Value::Blueprint(class) = values[0].settled() {
                            if let Some(entry) = Self::own_entry(&class, &values[1].bare()) { return Ok(entry.settled()); }
                        }
                        self.apply_class_member(values[2].clone(), Vec::new())
                    }
                    77 => match values.as_slice() {
                        [owner @ Value::Blueprint(_), _] if !self.table.strings("ext.stmt.type_params.open").is_empty() => Ok(owner.clone()),
                        _ => Err(self.class_unready()),
                    },
                    154 => {
                        let module = self.load_namespace("typing")?;
                        let function = self.read_class_member(module, &kept[0].bare(), false)?;
                        values.insert(0, kept[1].clone());
                        self.apply_class_member(function, values)
                    }
                    152 | 153 => {
                        if tag == 152 {
                            if let Some(Value::Blueprint(owner)) = values.first() {
                                if let Some(parameters) = Self::own_entry(owner, "__type_params__") {
                                    if parameters.is_true() {
                                        owner.shared.borrow_mut().push(("__parameters__".into(), parameters));
                                        return Ok(Value::Nil);
                                    }
                                }
                            }
                        }
                        let module = self.load_namespace("typing")?;
                        let word = if tag == 152 { "_generic_init_subclass" } else { "_generic_class_getitem" };
                        let function = self.read_class_member(module, word, false)?;
                        self.apply_class_member(function, values)
                    }
                    49 if values.len() == 1 => {
                        let space = self.load_namespace("typing")?;
                        let constructor = self.read_class_member(space, "_GenericAlias", false)?;
                        let owner = Value::Blueprint(self.native_kind("Generic"));
                        self.apply_class_member(constructor.settled(), vec![owner, values[0].clone()])
                    }
                    49 if values.is_empty() => {
                        self.type_support_namespace();
                        Ok(Value::Blueprint(self.native_kind("Generic")))
                    }
                    48 => {
                        if values.len() != 1 { return Err(String::from("TypeError: constevaluator.__call__() takes exactly 1 argument (0 given)").into()); }
                        let string_format = self.prim(Prim::Eq, "", &[values[0].clone(), Value::Small(4)])?.is_true();
                        let answer = kept[0].clone();
                        if string_format {
                            let spelling = answer.bare();
                            return Ok(Value::text(spelling.trim_start_matches("<class '").trim_end_matches("'>")));
                        }
                        Ok(answer)
                    }
                    47 => {
                        if values.len() > 1 { return Err(String::from("TypeError: evaluator takes at most one argument").into()); }
                        let format = values.first().cloned().unwrap_or(Value::Small(1)).settled();
                        if self.prim(Prim::Gt, "", &[format, Value::Small(2)])?.is_true() { return Err(String::from("NotImplementedError: ").into()); }
                        self.apply_class_member(kept[0].clone(), Vec::new())
                    }
                    46 if values.len() == 5 => {
                        let kind = self.native_kind(&values[1].bare());
                        let parameter = self.make_type_parameter(kind, vec![values[0].clone()])?;
                        if let Value::Thing(object) = &parameter {
                            let mut attributes = object.holds.borrow_mut();
                            if !matches!(values[2], Value::Nil) {
                                attributes.push((String::from("\0deferred-bound"), values[2].clone()));
                                attributes.push((String::from("\0constraints-syntax"), values[4].clone()));
                            }
                            if !matches!(values[3], Value::Nil) {
                                attributes.retain(|entry| entry.0 != "__default__");
                                attributes.push((String::from("\0deferred/__default__"), values[3].clone()));
                            }
                            if object.blueprint().name == "ParamSpec" {
                                if let Some(entry) = attributes.iter_mut().find(|entry| entry.0 == "__bound__") { entry.1 = Value::Nil; }
                            }
                            if let Some(entry) = attributes.iter_mut().find(|entry| entry.0 == "__infer_variance__") { entry.1 = Value::Flag(true); }
                            if let Some(display) = attributes.iter_mut().find(|entry| entry.0 == "\0type-display") { display.1 = values[0].clone(); }
                        }
                        Ok(parameter)
                    }
                    44 => {
                        if values.len() != 1 { return Err(String::from("TypeError: __annotate__() requires one argument").into()); }
                        let format = values[0].settled();
                        let rejected = self.prim(Prim::Gt, "", &[format, Value::Small(2)])?;
                        if rejected.is_true() { return Err(String::from("NotImplementedError: ").into()); }
                        match &kept[0] {
                            Value::Blueprint(owner) => self.resolve_blueprint_annotations(owner),
                            Value::Vector(rows) => {
                                let mut annotations = Vec::new();
                                for index in (0..rows.len()).step_by(2) {
                                    let Some(source) = rows.get(index + 1) else { break; };
                                    let source = source.settled();
                                    if matches!(source, Value::Unset) { continue; }
                                    let found = match source { Value::Routine(_) | Value::Bound(..) => self.apply_class_member(source, Vec::new())?, other => other };
                                    annotations.push((rows[index].clone(), found));
                                }
                                let mapping = Value::Dict(Rc::new(annotations.into()));
                                Ok(self.collection_cell(mapping))
                            }
                            _ => Err(self.class_unready()),
                        }
                    }
                    // The native drawing descriptor retains its Python
                    // entry for index protocols, cold instances and refusals.
                    133=>{
                        let operation=kept[0].bare();
                        let needed=if operation=="next" {1} else {2};
                        let ordinary=values.len()==needed
                            && matches!(values.first().map(Value::settled),Some(Value::Thing(_)))
                            && (needed==1 || matches!(values[1].settled(),Value::Small(n) if n>=0));
                        if ordinary {
                            let stream=self.read_class_member(values[0].clone(),"_stream",false);
                            match stream {
                                Ok(Value::Small(mark))=>{
                                    let mut request=vec![Value::text(&operation),Value::Small(mark)];
                                    request.extend(values.into_iter().skip(1));
                                    return self.apply_class_member(Value::text("__random"),request);
                                }
                                Err(error) if !self.missing_member_escape(&error)=>return Err(error),
                                _=>{},
                            }
                        }
                        self.apply_class_member(kept[1].clone(),values)
                    }
                    134=>{
                        // Exact floats in the operation's regular domain are
                        // the C module's common case. Other values retain
                        // all of the library's protocol and error handling.
                        let operation=kept[0].bare();
                        if operation == "normal_pdf" {
                            if let [distribution, input] = values.as_slice() {
                                let input = input.settled();
                                if matches!(input, Value::Frac(_)) {
                                    let center = self.read_class_member(distribution.clone(), "_mu", false)?.settled();
                                    let spread = self.read_class_member(distribution.clone(), "_sigma", false)?.settled();
                                    if let (Value::Frac(a), Value::Frac(b), Value::Frac(c)) = (&input, &center, &spread) {
                                        let coordinates = [a, b, c].map(|r| crate::data::nearest_binary(&r.above, &r.beneath));
                                        let variance = coordinates[2] * coordinates[2];
                                        if coordinates.iter().all(|n| n.is_finite()) && variance > 0.0 && variance.is_finite() {
                                            return self.apply_class_member(Value::text("__math"), vec![kept[0].clone(), input, center, spread]);
                                        }
                                    }
                                }
                            }
                        }
                        if operation == "sqrt_frac_rto" && values.len() == 2 && values.iter().all(|value| matches!(value.settled(), Value::Flag(_) | Value::Huge(_) | Value::Small(_))) {
                            return self.apply_class_member(Value::text("__math"), vec![kept[0].clone(), values[0].clone(), values[1].clone()]);
                        }
                        if operation == "isqrt" {
                            if let [integer] = values.as_slice() {
                                let native = integer.settled();
                                if matches!(native, Value::Flag(_) | Value::Huge(_) | Value::Small(_)) {
                                    return self.apply_class_member(Value::text("__math"), vec![kept[0].clone(), native]);
                                }
                            }
                        }
                        if operation == "fsum" && self.table.flag("ext.builtin.math.fsum") {
                            if let [sequence] = values.as_slice() {
                                let items = match sequence.settled() { Value::Vector(items) | Value::Tuple(items) => Some(items), _ => None };
                                let native = items.is_some_and(|items| items.iter().all(|item| matches!(item.settled(), Value::Frac(_) | Value::Flag(_) | Value::Huge(_) | Value::Small(_))));
                                if native { return self.apply_class_member(Value::text("__math"), vec![kept[0].clone(), sequence.clone()]); }
                            }
                        }
                        if let [integer]=values.as_slice() {
                            let worth=integer.settled();
                            if operation=="floor" && matches!(worth,Value::Small(_)|Value::Huge(_)) {return Ok(worth);}
                        }
                        let regular=match values.as_slice() {
                            [number]=>match number.settled() {
                                Value::Small(n)=>Some((n>0,n==0)),
                                Value::Frac(r) if r.places.is_some() && !r.beneath.is_zero()=>{
                                    // Read the exact ratio's domain; width
                                    // conversion belongs to the operation.
                                    let zero=r.above.is_zero();
                                    Some((!zero && r.above.sign()==r.beneath.sign(),zero))
                                }
                                _=>None,
                            }
                            _=>None,
                        };
                        let fast=regular.is_some_and(|(positive,zero)|match operation.as_str() {
                            "exp"|"floor"|"fabs"=>true,
                            "frexp"=>self.table.flag("ext.builtin.math.frexp"),
                            "sqrt"=>positive || zero,
                            "lgamma"|"log"|"log2"=>positive,
                            _=>false,
                        });
                        if fast {
                            let mut operands=vec![kept[0].clone()];
                            operands.extend(values);
                            self.apply_class_member(Value::text("__math"),operands)
                        } else {self.apply_class_member(kept[1].clone(),values)}
                    }
                    0=>Ok(kept[0].clone()),
                    136=>{
                        let word = kept[0].bare();
                        let Some(first) = values.first() else { return Err(format!("TypeError: unbound method {word}.__reduce__() needs an argument").into()); };
                        let base = Self::underlying(first).unwrap_or_else(|| first.settled()).settled();
                        let walking_type = matches!(self.table.prims.get(&word), Some(Prim::Numbered | Prim::Zipped | Prim::Mapped | Prim::Filtered | Prim::Backwards));
                        if walking_type && matches!(base, Value::Iterator(_)) && base.kind_word() == word {
                            if values.len() > 1 { return Err(format!("TypeError: {word}.__reduce__() takes no arguments ({} given)", values.len()-1).into()); }
                            return self.reduce_iterator(first).map_err(Escape::from);
                        }
                        let fixed = self.table.prims.get(&word) == Some(&Prim::Unchanging);
                        if !matches!(base, Value::Set(_)) || base.set_sealed() != fixed {
                            return Err(format!("TypeError: descriptor '__reduce__' for '{word}' objects doesn't apply to a '{}' object", first.kind_word()).into());
                        }
                        if values.len() != 1 { return Err(format!("TypeError: {}.__reduce__() takes no arguments ({} given)", first.kind_word(), values.len() - 1).into()); }
                        self.reduction_of_set(first)
                    },
                    135=>{
                        if values.len() != 0 { return Err(format!("TypeError: builtin_function_or_method.__reduce__() takes no arguments ({} given)", values.len()).into()); }
                        Ok(kept[0].clone())
                    },
                    1 if !values.is_empty()=>{
                        let Some(Value::Blueprint(c))=values.first() else{return Err(format!("TypeError: object.__new__(X): X is not a type object ({})",values.first().map_or_else(|| "NoneType".to_owned(),Value::kind_word)).into())};
                        let buffer_kind = c.shared.borrow().iter().any(|(word, bit)| word == "\0buffer_allocator" && bit.is_true());
                        if buffer_kind {
                            let module = Self::own_entry(c, self.detail("module")).map(|home| home.bare()).unwrap_or_default();
                            let title = format!("{module}.{}", c.name);
                            return Err(format!("TypeError: object.__new__({title}) is not safe, use {title}.__new__()").into());
                        }
                        if self.is_fault_kind(c) { return Ok(self.make_fault(c.clone(), values[1..].to_vec(), Value::Nil)); }
                        self.abstract_turned_away(c)?;
                        if values.len()>1{self.root_turns_away(c,'n')?;}
                        self.made+=1;Ok(Value::Thing(Rc::new(Thing{reclassified: RefCell::new(None), of:c.clone(),holds:RefCell::new(vec![]),turn:self.made})))
                    }
                    2 if kept.first().map_or(false, |item| item.bare() == "module") => {
                        let mut handed = values.into_iter();
                        let target = handed.next().ok_or_else(|| "TypeError: descriptor '__init__' of 'module' object needs an argument".to_owned())?;
                        self.fill_module(target, handed.collect())
                    }
                    2 if kept.first().is_some_and(|v| v.bare() == "__init_subclass__") => {
                        let (positional, keywords) = self.open_arguments(values)?;
                        let Some(Value::Blueprint(owner)) = positional.first() else { return Err(self.class_unready()); };
                        if positional.len() > 1 || !keywords.is_empty() { return Err(format!("TypeError: {}.__init_subclass__() takes no keyword arguments", owner.name).into()); }
                        Ok(Value::Nil)
                    }
                    2 if !kept.is_empty() => {
                        let word=kept[0].bare();
                        let receiver=values.remove(0).settled();
                        let Value::Thing(object)=receiver else { return Err(self.class_unready()); };
                        let operation=*self.table.prims.get(&word).ok_or_else(||self.class_unready())?;
                        if operation == Prim::Listed {
                            let allocation=self.inherited_entry(&object.blueprint(),self.detail("allocate"));
                            if allocation.is_some_and(|entry| !matches!(entry,Value::Wrapped(..))) { values=self.open_arguments(values)?.0; }
                        }
                        let initialized=self.apply_held(Value::Intrinsic(operation,Rc::from(word)),values)?;
                        let mut holds=object.holds.borrow_mut();
                        let Some((_,under))=holds.iter_mut().find(|(name,_)|name=="\0underlying") else { return Err(self.class_unready()); };
                        match under {
                            Value::Mutable(cell,_)|Value::Shared(cell)=>*cell.borrow_mut()=initialized.settled(),
                            _=>*under=initialized.keep(false),
                        }
                        Ok(Value::Nil)
                    }
                    2 if matches!(values.first(), Some(Value::Thing(t)) if self.is_fault_kind(&t.blueprint())) => {
                        let Value::Thing(receiver) = values.remove(0) else { unreachable!() };
                        let key = self.rules.words_ext_stmt_class_constructor.first().map(String::as_str).unwrap_or_default().to_owned();
                        self.fault_method(receiver, &key, &values)
                    }
                    2 if matches!(values.first().map(Value::settled), Some(Value::Tuple(_))) => Ok(Value::Nil),
                    2 if values.len()==1=>Ok(Value::Nil),
                    2 if values.len()>1=>{
                        let Some(Value::Thing(t))=values.first() else{return Err(self.class_unready())};
                        self.root_turns_away(&t.blueprint(),'i')?;
                        Ok(Value::Nil)
                    }
                    // A native forebear's constructing, read off the
                    // parent walk: it runs upon the thing the walk was
                    // made of, past the class it was made against, as
                    // the parent call of the method spelled out runs.
                    126 => {
                        let ahead = kept[1].bare();
                        let called = kept[2].bare();
                        self.next_ancestor_call(kept[0].clone(), &ahead, &called, values)
                    }
                    73 if kept.len() == 3 => {
                        let receiver = kept[0].clone();
                        let declared = kept[1].bare();
                        let key = kept[2].bare();
                        self.next_ancestor_call(receiver, &declared, &key, values)
                    }
                    73 => self.initialise_type_object(values),
                    70 => self.class_of_parts(values),
                    71 => {
                        let (mut positional, keywords) = self.open_arguments(values)?;
                        if positional.is_empty() { return Err(self.class_unready()); }
                        let first = positional.remove(0);
                        for (word, held) in keywords { positional.push(Value::Couple(Rc::new((Value::text(&word), held)))); }
                        match first {
                            Value::Blueprint(target) => self.construct_plainly(target, positional),
                            native if self.stands_for_a_kind(&native) => self.apply_class_member(native, positional),
                            _ => Err(self.class_unready()),
                        }
                    }
                    122 => {
                        if !values.is_empty() { return Err(String::from("TypeError: function takes no arguments").into()); }
                        self.prim(Prim::Weigh, "eval", &[kept[0].clone(), kept[1].clone()]).map_err(Escape::from)
                    }
                    143 => {
                        if !values.is_empty() { return Err(String::from("TypeError: this code object takes no arguments").into()); }
                        self.text_performed(true, "eval", &kept[..2]).map_err(|message| self.got_away.take().unwrap_or(Escape::Error(message)))
                    }
                    43 => {

                        self.apply_class_member(kept[0].clone(), values)
                    }
                    200|202 => {
                        // A wrapper's own making: the callable it is
                        // given is kept under a name no program can
                        // spell.
                        let Some(Value::Thing(t))=values.first() else{return Err(self.class_unready())};
                        let Some(callable)=values.get(1) else{return Err(self.class_unready())};
                        t.holds.borrow_mut().push(("\0callable".to_owned(),callable.clone()));
                        Ok(Value::Nil)
                    }
                    201 => {
                        // classmethod.__get__: the kept callable is
                        // bound to the class the read came through, as
                        // the reference binds it, never to nothing.
                        let Some(Value::Thing(t))=values.first() else{return Err(self.class_unready())};
                        let Some(callable)=t.holds.borrow().iter().find(|(k,_)|k=="\0callable").map(|(_,v)|v.clone()) else{return Err(self.class_unready())};
                        let bound=values[1..].iter().find_map(|v| match v { c @ Value::Blueprint(_) => Some(c.clone()), _ => None })
                            .unwrap_or_else(|| values.get(1).cloned().unwrap_or(Value::Nil));
                        Ok(Self::wrap(3,vec![callable,bound]))
                    }
                    203 => {
                        // staticmethod.__get__: the kept callable
                        // itself, unbound.
                        let Some(Value::Thing(t))=values.first() else{return Err(self.class_unready())};
                        let Some(callable)=t.holds.borrow().iter().find(|(k,_)|k=="\0callable").map(|(_,v)|v.clone()) else{return Err(self.class_unready())};
                        Ok(callable)
                    }
                    204 => {
                        let Value::Blueprint(c)=&kept[0] else{return Err(self.class_unready())};
                        let mut standing=Vec::new();
                        let mut children=self.class_children.borrow_mut();
                        if let Some(list)=children.get_mut(&(Rc::as_ptr(c) as usize)) {
                            list.retain(|weak|weak.upgrade().is_some());
                            standing=list.iter().filter_map(|weak|weak.upgrade().map(Value::Blueprint)).collect();
                        }
                        drop(children);
                        Ok(Value::Vector(Rc::new(standing).into()))
                    }
                    72 => Ok(Value::Mutable(Rc::new(RefCell::new(Value::Dict(Rc::new(Vec::new().into())))), true)),
                    36=>{
                        let named=kept[0].bare();
                        // State and reduction answer for the thing itself,
                        // not for the native worth beneath it.
                        let holds_state = self.rules.words_ext_stmt_class_detail_root_members.iter().position(|word| word == &named).is_some_and(|at| at == 10 || at == 11 || at == 12);
                        if !holds_state && kept.get(1).is_some_and(|owner|owner.bare()!=self.detail("root")) {
                            if let Some(native)=values.first().and_then(Self::underlying){values[0]=native.settled();}
                        }
                        self.root_answers(&named,values)
                    }
                    // The maker of a native kind: the blueprint to make a
                    // thing of, then what the kind's primitive takes.
                    233 if values.len()==1=>self.display_clock_record(&values[0]),
                    234 if values.len()==1=>self.reduce_clock_record(&values[0]),
                    14 if !values.is_empty()=>{
                        let target = values.remove(0);
                        let word=kept[0].bare();
                        if word=="struct_time" {return self.clock_record(target,values)}
                        if self.has_class_order() && word == "super" {
                            let Value::Blueprint(of) = &target else { return Err(self.class_unready()); };
                            if !Self::parent_kind_descended(of) { return Err(self.class_unready()); }
                            self.made += 1;
                            return Ok(Value::Thing(Rc::new(Thing { reclassified: RefCell::new(None), of: of.clone(), holds: RefCell::new(Vec::new()), turn: self.made })));
                        }
                        if let Value::Intrinsic(op, spelling) = &target {
                            if matches!(op, Prim::Uniques | Prim::Unchanging) && spelling.as_ref() == word {
                                if *op == Prim::Uniques { values.clear(); }
                                return self.prim(*op, spelling, &values).map_err(Escape::from);
                            }
                        }
                        if self.table.prims.get(&word) == Some(&Prim::Truthful) {
                            return match &target {
                                Value::Intrinsic(Prim::Truthful, spelling) if spelling.as_ref() == word => {
                                    if values.len() > 1 {
                                        Err(format!("TypeError: bool expected at most 1 argument, got {}", values.len()).into())
                                    } else { self.prim(Prim::Truthful, spelling, &values).map_err(Escape::from) }
                                }
                                Value::Intrinsic(_, spelling) => Err(format!("TypeError: bool.__new__({spelling}): {spelling} is not a subtype of bool").into()),
                                Value::Blueprint(class) => Err(format!("TypeError: bool.__new__({0}): {0} is not a subtype of bool", class.name).into()),
                                other => Err(format!("TypeError: bool.__new__(X): X is not a type object ({})", other.kind_word()).into()),
                            };
                        }
                        if matches!(&target, Value::Intrinsic(Prim::Truthful, _)) && self.table.prims.get(&word) == Some(&Prim::AsInt) {
                            return Err("TypeError: int.__new__(bool) is not safe, use bool.__new__()".to_owned().into());
                        }
                        if let Value::Intrinsic(operation,title)=&target {
                            if operation.names_a_kind() && title.as_ref()==word {return self.apply_held(target,values)}
                            if operation.names_a_kind() {return Err(format!("TypeError: {word}.__new__({title}): {title} is not a subtype of {word}").into())}
                        }
                        let Value::Blueprint(c)=target else{return Err(format!("TypeError: {word}.__new__(X): X is not a type object ({})",target.kind_word()).into())};
                        if Self::native_beneath(&c).as_deref()!=Some(word.as_str()){return Err(format!("TypeError: {word}.__new__({0}): {0} is not a subtype of {word}",c.name).into())}
                        if matches!(self.table.prims.get(&word), Some(Prim::Uniques | Prim::Listed | Prim::Dictionary)) { values.clear(); }
                        self.thing_over_native(c,&word,values)
                    }
                    181 => {
                        let Value::Wrapped(180, descriptor) = &kept[0] else { return Err(self.class_unready()); };
                        let field = descriptor[0].bare();
                        if kept[1].bare() != self.detail("descriptor.get") {
                            return Err(format!("AttributeError: attribute '{field}' of 'type' objects is not writable").into());
                        }
                        if values.len() < 1 || values.len() > 2 { return Err("TypeError: __get__ expected 1 or 2 arguments".to_owned().into()); }
                        let object = values[0].settled();
                        if matches!(object, Value::Nil) { return Ok(kept[0].clone()); }
                        if !matches!(&object, Value::Blueprint(_)) && !matches!(&object, Value::Intrinsic(primitive, _) if primitive.names_a_kind()) {
                            return Err(format!("TypeError: descriptor '{field}' for 'type' objects doesn't apply to a '{}' object", object.kind_word()).into());
                        }
                        self.read_class_member(object, &field, true)
                    }
                    120 if values.len() == 1 => {
                        let operation = match kept[0] { Value::Small(n) => n, _ => return Err(self.class_unready()) };
                        match self.recipe_advance(operation, &values[0])? {
                            Some(answer) => Ok(answer),
                            None => Err(self.core_complaint("core.exhausted", "").into()),
                        }
                    }
                    3 | 132=>{values.insert(0,kept[1].clone());if !self.rules.words_ext_stmt_class_builder.is_empty(){self.apply_held(kept[0].clone(),values)}else{self.apply_class_member(kept[0].clone(),values)}},
                    // An entry a native kind carries, standing loose:
                    // the first value handed to it is the one it works
                    // upon, the rest being what the entry itself takes.
                    // Called with none at all, it names the kind and
                    // itself as CPython's unbound method does; handed a
                    // receiver of the wrong kind, it names the entry,
                    // the kind and the receiver's own, as CPython's
                    // descriptor does.
                    60=>{
                        let entry=kept[1].bare();
                        let word=kept[0].bare();
                        if values.is_empty() {
                            if word == "module" && self.rules.specials.get(1).is_some_and(|name| name == &entry) {
                                return Err("TypeError: descriptor '__repr__' of 'module' object needs an argument".to_owned().into());
                            }
                            let words=self.table.strings("ext.stmt.class.detail.descriptor.unbound");
                            return if words.len()==3 {
                                Err(format!("{}{word}{}{entry}{}",words[0],words[1],words[2]).into())
                            } else {
                                Err(self.class_unready())
                            };
                        }
                        // The value comes in as the cell that holds
                        // it, so a member that writes writes into the
                        // very one the caller named.
                        if word == "type" && entry == self.detail("order") {
                            let (positional, named) = self.open_arguments(values)?;
                            let has_keywords = !named.is_empty();
                            let mut arguments = positional.into_iter();
                            let target = arguments.next().ok_or_else(|| Escape::from("TypeError: unbound method type.mro() needs an argument".to_string()))?.settled();
                            if !self.stands_for_a_kind(&target) {
                                return Err(format!("TypeError: descriptor 'mro' for 'type' objects doesn't apply to a '{}' object", self.parent_tp_name(&target)).into());
                            }
                            let called_on = if kept.get(2).is_some_and(Value::is_true) { self.parent_tp_name(&target) } else { word };
                            if has_keywords { return Err(format!("TypeError: {called_on}.mro() takes no keyword arguments").into()); }
                            let count = arguments.count();
                            if count != 0 { return Err(format!("TypeError: {called_on}.mro() takes no arguments ({count} given)").into()); }
                            let owner = self.parent_type_arg(&target)?;
                            let bases = self.combine_orders(&owner.parents)?;
                            let result = std::iter::once(owner).chain(bases).map(|base| self.visible_blueprint(base)).collect();
                            return Ok(Value::Vector(crate::tuples::Sequence::plain(result)).keep(true));
                        }
                        let held = values.remove(0);
                        let subject = if matches!(held.settled(), Value::Thing(_)) { held.settled() } else { held.keep(false) };
                        // A thing of a class standing on the very kind
                        // this word names answers as its worth would,
                        // since the loose entry is the kind's own and
                        // not the class's: `set.union(s, ...)` for `s`
                        // a subclass of `set` works upon what `s` keeps
                        // of a set.
                        if word == "module" && self.rules.specials.get(1).is_some_and(|key| key == &entry) {
                            if !values.is_empty() { return Err(format!("TypeError: expected 0 arguments, got {}", values.len()).into()); }
                            return self.describe_module(subject);
                        }
                        let receiver=match &subject {
                            Value::Thing(t) if Self::native_among(&t.blueprint(), &word) =>
                                Self::underlying(&subject).unwrap_or_else(||subject.clone()),
                            _=>subject.clone(),
                        };
                        let subscription = self.rules.specials.get(11).map_or(false, |slot| slot == &entry);
                        if subscription && word == "dict" && values.len() == 1 {
                            if let Value::Thing(instance) = &subject {
                                if let Value::Dict(store) = receiver.settled() {
                                    let location = self.map_locate(&store, Some(&store), &values[0])?.0;
                                    if let Some(index) = location { return Ok(store[index].1.clone()); }
                                    let missing = self.table.strings("ext.stmt.class.missing").first().map(String::as_str).unwrap_or("");
                                    if let Some(handler) = self.inherited_entry(&instance.blueprint(), missing) {
                                        let bound = self.member_binding(handler, Some(subject.clone()), instance.blueprint().clone())?;
                                        return self.apply_class_member(bound, values);
                                    }
                                    return Err(self.absent_key(&values[0]).into());
                                }
                            }
                        }
                        // A loose entry is the kind's own alone, so a
                        // receiver of some other kind is refused before
                        // the entry is even looked up, even where it
                        // happens to answer to an entry of the same
                        // name some other kind carries (`list.count`,
                        // `tuple.count`); a receiver of the kind itself,
                        // or standing under it the way a flag stands
                        // under the whole-number kind, still reaches
                        // the entry as before.
                        let of_own_kind=self.table.prims.get(word.as_str()).copied().filter(Self::names_a_kind)
                            .map_or_else(||word==receiver.kind_word(),|op|self.kind_covers(&op,&word,&receiver.settled()));
                        if word == "type" && self.stands_for_a_kind(&receiver) {
                            let slots = self.rules.specials;
                            let calls = slots.get(17).map_or(false, |name| name == &entry);
                            let hashes = slots.get(8).map_or(false, |name| name == &entry);
                            if calls {
                                // The native type call constructs without asking the
                                // metaclass to dispatch this same call again.
                                return match receiver {
                                    Value::Blueprint(class) => self.construct_plainly(class, values),
                                    other => self.apply_class_member(other, values),
                                };
                            }
                            if hashes && values.is_empty() { return self.prim(Prim::Hashed, &entry, &[receiver]).map_err(Escape::from); }
                        }
                        // Buffer wrappers share native method validation and the canonical exporter.
                        if of_own_kind && self.table.spells("ext.stmt.class.builtin", "bytes")
                            && matches!(word.as_str(), "bytes" | "bytearray")
                            && matches!(entry.as_str(), "__buffer__" | "__release_buffer__") {
                            let mut inputs = vec![subject]; inputs.extend(values);
                            return self.apply_class_member(Value::Intrinsic(Prim::ValueMethod, Rc::from(format!("{word}.{entry}"))), inputs);
                        }
                        if entry == self.detail("get") && (!matches!(word.as_str(), "module" | "type") || of_own_kind) {
                            let (inputs, keywords) = self.open_arguments(values)?;
                            if !keywords.is_empty() {
                                return Err(format!("TypeError: wrapper {entry}() takes no keyword arguments").into());
                            }
                            match inputs.as_slice() {
                                [attribute] => match attribute.settled() {
                                    Value::Text(named) => return self.read_class_member(subject, &named, true),
                                    other => return Err(format!("TypeError: attribute name must be string, not '{}'", other.kind_word()).into()),
                                },
                                _ => return Err(format!("TypeError: expected 1 argument, got {}", inputs.len()).into()),
                            }
                        }
                        let found=if of_own_kind && self.native_member(&receiver, &entry) {
                            Some(Value::Member(Rc::new(subject.clone()), entry.clone()))
                        } else if of_own_kind{self.attribute(&receiver,&entry)}else{None};
                        match found {
                            Some(bound)=>self.apply_class_member(bound,values),
                            None=>{
                                let words=self.table.strings("ext.stmt.class.detail.descriptor.foreign");
                                if words.len()==4 {
                                    Err(format!("{}{entry}{}{word}{}{}{}",words[0],words[1],words[2],receiver.kind_word(),words[3]).into())
                                } else {
                                    Err(self.absent_attribute(&subject,&entry))
                                }
                            }
                        }
                    }
                    4 if !self.rules.words_ext_stmt_class_builder.is_empty() => self.apply_held(kept[0].clone(), values),
                    4|8=>self.apply_class_member(kept[0].clone(),values),
                    78 => {
                        if !self.open_arguments(values.clone())?.1.is_empty() {
                            return Err(format!("TypeError: wrapper {}() takes no keyword arguments", self.detail("descriptor.get")).into());
                        }
                        let count = values.len().saturating_sub(1);
                        if count == 0 { return Err(format!("TypeError: {} expected at least 1 argument, got 0", self.detail("descriptor.get")).into()); }
                        if count > 2 { return Err(format!("TypeError: {} expected at most 2 arguments, got {}", self.detail("descriptor.get"), count).into()); }
                        let [descriptor, instance, rest @ ..] = values.as_slice() else { return Err(self.class_unready()); };
                        let supplied = rest.first().filter(|owner| !matches!(owner.settled(), Value::Nil));
                        if matches!(instance.settled(), Value::Nil) && supplied.is_none() { return Err("TypeError: __get__(None, None) is invalid".to_owned().into()); }
                        let held = Self::underlying(descriptor).unwrap_or_else(|| descriptor.settled());
                        let Value::Wrapped(tag, items) = held else { return Err(self.class_unready()); };
                        match tag {
                            4 => Ok(items[0].clone()),
                            5 => {
                                let owner = match supplied { Some(owner) => owner.clone(), None => self.apply_held(self.kind_builder_word(), vec![instance.clone()])? };
                                Ok(Self::wrap(3, vec![items[0].clone(), owner]))
                            }
                            _ => Err(self.class_unready()),
                        }
                    }
                    79 => {
                        if values.is_empty() { return Err(self.class_unready()); }
                        let descriptor = values.remove(0);
                        let held = Self::underlying(&descriptor).ok_or_else(|| self.class_unready())?;
                        let Value::Wrapped(4, items) = held else { return Err(self.class_unready()); };
                        self.apply_held(items[0].clone(), values)
                    }
                    5=>Err(self.core_complaint("core.uncallable","classmethod").into()),
                    // The root's formatting of a thing to a specification.
                    59 if values.len()==2=>{
                        let spec=match &values[1] {
                            Value::Text(spec)=>spec.to_string(),
                            other=>match Self::underlying(other).map(|worth|worth.settled()) {
                                Some(Value::Text(held))=>held.to_string(),
                                _=>{
                                    let words=self.table.strings("ext.stmt.class.format.argument");
                                    return Err(format!("{}{}",words.first().map_or("",String::as_str),Self::format_spec_complaint_kind(other)).into());
                                }
                            },
                        };
                        if !spec.is_empty() { return Err(format!("TypeError: unsupported format string passed to {}.__format__", values[0].kind_word()).into()); }
                        self.object_words(&values[0],false).map(|word|Value::text(&word)).map_err(Escape::from)
                    }
                    13 if values.len()==1=>{
                        let Value::Wrapped(6, property)=&kept[0] else{return Err(self.class_unready());};
                        let mut parts=property.as_ref().clone();
                        parts.resize(2, Value::Nil); parts[1]=values.remove(0);
                        Ok(Self::wrap(6,parts))
                    }
                    10|11|12=>{
                        let subject=values.first().cloned().ok_or_else(|| self.class_unready())?;
                        if kept.first().is_some_and(|owner| owner.bare() == self.detail("root")) {
                            let (actual, valid) = self.object_descriptor_target(&subject)?;
                            if !valid {
                                let method = self.detail(if tag == 10 { "get" } else if tag == 11 { "set" } else { "remove" });
                                return Err(format!("TypeError: descriptor '{method}' requires a '{}' object but received a '{actual}'", self.detail("root")).into());
                            }
                        }
                        if values.len() < 2 { return Err(self.class_unready()); }
                        let Value::Text(key)=&values[1]else{return Err("TypeError: attribute name must be string".to_string().into());};
                        if tag==10 {self.read_class_member(subject,key,true)}
                        else {self.alter_class_member(subject,key,if tag==11{values.get(2).cloned()}else{None},true)}
                    }
                    // A binding member's reader, called as the program
                    // calls it: with the thing, or nothing and the class.
                    31 if kept.is_empty() => {
                        let (mut positional, named) = self.open_arguments(values)?;
                        if !named.is_empty() { return Err(String::from("TypeError: wrapper __get__() takes no keyword arguments").into()); }
                        let Some(function) = positional.first().map(Value::settled) else { return Err(String::from("TypeError: descriptor '__get__' of 'function' object needs an argument").into()) };
                        let genuine = matches!(&function, Value::Routine(_) | Value::Bound(..)) || matches!(&function, Value::Wrapped(122 | 130, _));
                        if !genuine { return Err(format!("TypeError: descriptor '__get__' requires a 'function' object but received a '{}'", function.kind_word()).into()); }
                        let total = positional.len() - 1;
                        match total {
                            0 => return Err(String::from("TypeError: __get__ expected at least 1 argument, got 0").into()),
                            1 | 2 => {},
                            _ => return Err(format!("TypeError: __get__ expected at most 2 arguments, got {total}").into()),
                        }
                        let instance = positional.remove(1);
                        if matches!(instance.settled(), Value::Nil) {
                            let absent = positional.get(1).map_or(true, |owner| matches!(owner.settled(), Value::Nil));
                            if absent { return Err(String::from("TypeError: __get__(None, None) is invalid").into()); }
                            return Ok(function);
                        }
                        let class = match positional.get(1) { Some(Value::Blueprint(class)) => class.clone(), _ => self.common_ancestor() };
                        if matches!(&function, Value::Routine(_) | Value::Bound(..)) { self.member_binding(function, Some(instance), class) }
                        else { Ok(Self::wrap(3, vec![function, instance])) }
                    }
                    31 if matches!(values.len(),1|2)=>{
                        let receiver=match &values[0]{Value::Nil=>None,other=>Some(other.clone())};
                        let owner=match (values.get(1),&receiver) {
                            (Some(Value::Blueprint(b)),_)=>b.clone(),
                            (Some(native), _) if self.stands_for_a_kind(native) => {
                                let word = native.kind_it_names().map(Rc::<str>::from).or_else(|| self.kind_spelling(native)).ok_or_else(|| self.class_unready())?;
                                self.native_kind(&word)
                            }
                            (_,Some(Value::Thing(t)))=>t.blueprint().clone(),
                            (_,Some(_))=>self.common_ancestor(),
                            (_,None)=>return Err(self.class_unready()),
                        };
                        self.member_binding(kept[0].clone(),receiver,owner)
                    }
                    33 if values.len()==2=>self.slot_change(&values[0],&kept,Some(values[1].clone())),
                    34 if values.len()==1=>self.slot_change(&values[0],&kept,None),
                    50..=57=>self.property_operation(tag,values),
                    _=>Err(self.class_unready()),
                }
            }
            Value::Text(word)=>{
                let Some(op)=self.table.prims.get(word.as_ref()).copied()else{return Err(self.class_unready());};
                match op {Prim::ClassWork(k)=>self.work_on_class(k,values),Prim::SortOf=>self.class_from_type(values),_=>self.apply_class_member(Value::Intrinsic(op, word), values)}
            }
            other => Err(self.core_complaint("core.uncallable", &other.kind_word()).into()),
        }
    }
    /// Making a thing of a class. A class built by a metaclass is called
    /// through that metaclass's own call, which says what comes of it.
    pub(super) fn type_support_namespace(&mut self) -> Value {
        if let Some(cached) = self.imported.get("_typing") { return cached.clone(); }
        let mut entries = vec![(String::from("__name__"), Value::text("_typing"))];
        entries.push(("Union".into(), Value::Blueprint(self.native_kind("Union"))));
        for title in ["TypeVar", "ParamSpec", "TypeVarTuple", "TypeAliasType", "Generic", "NoDefaultType", "ParamSpecArgs", "ParamSpecKwargs"] {
            let kind = self.native_kind(title);
            if title != "NoDefaultType" {
                kind.shared.borrow_mut().push((String::from("__module__"), Value::text("typing")));
            }
            if self.table.has_any("ext.stmt.type_params.open") && title == "Generic" {
                for (word, operation) in [("__class_getitem__", 153), ("__init_subclass__", 152)] {
                    kind.shared.borrow_mut().push((word.into(), Self::wrap(5, vec![Self::wrap(operation, Vec::new())])));
                }
            }
            entries.push((title.to_owned(), Value::Blueprint(kind)));
        }
        self.made += 1;
        let nothing = Value::Thing(Rc::new(Thing { reclassified: RefCell::new(None), of: self.native_kind("NoDefaultType"),
            holds: RefCell::new(vec![(String::from("\0type-display"), Value::text("typing.NoDefault"))]), turn: self.made }));
        entries.push((String::from("NoDefault"), nothing));
        entries.push((String::from("_idfunc"), Value::Intrinsic(Prim::ClassWork(23), Rc::from("_idfunc"))));
        self.made += 1;
        let space = Value::Thing(Rc::new(Thing { reclassified: RefCell::new(None), of: self.native_kind("module"),
            holds: RefCell::new(entries), turn: self.made }));
        self.imported.insert(String::from("_typing"), space.clone()); self.namespace_places.borrow_mut().take();
        space
    }
    fn absent_type_default(&mut self) -> Value {
        match self.type_support_namespace() {
            Value::Thing(space) => space.holds.borrow().iter().find(|entry| entry.0 == "NoDefault").unwrap().1.clone(),
            _ => unreachable!(),
        }
    }
    fn make_type_parameter(&mut self, kind: Rc<Blueprint>, input: Vec<Value>) -> Res {
        let (values, options) = self.open_arguments(input)?;
        if kind.name == "NoDefaultType" {
            if !values.is_empty() || !options.is_empty() { return Err(String::from("TypeError: NoDefaultType takes no arguments").into()); }
            return Ok(self.absent_type_default());
        }
        if kind.name == "TypeVar" && values.len() == 2 { return Err(String::from("TypeError: A single constraint is not allowed").into()); }
        if (kind.name == "ParamSpec" || kind.name == "TypeVarTuple") && values.len() > 1 { return Err(format!("TypeError: {}() takes exactly 1 positional argument ({} given)", kind.name.to_lowercase(), values.len()).into()); }
        let title = match values.first().map(Value::settled) {
            Some(Value::Text(title)) => title,
            _ => return Err(String::from("TypeError: name must be a str").into()),
        };
        let origin = self.namespace_named().unwrap_or_else(|| String::from("__main__"));
        let mut attributes = vec![(String::from("__name__"), Value::Text(title.clone())),
            (String::from("__module__"), Value::text(&origin)), (String::from("\0type-display"), Value::Text(title))];
        if kind.name == "TypeAliasType" {
            if values.len() != 2 { return Err(String::from("TypeError: TypeAliasType requires a name and a value").into()); }
            attributes.extend([(String::from("__type_params__"), Value::tuple(Vec::new())),
                (String::from("__value__"), values[1].clone())]);
        } else {
            attributes.extend([(String::from("__constraints__"), Value::tuple(values[1..].to_vec())),
                (String::from("__bound__"), Value::Nil), (String::from("__default__"), self.absent_type_default())]);
            for flag in ["__covariant__", "__contravariant__", "__infer_variance__"] {
                attributes.push((flag.to_owned(), Value::Flag(false)));
            }
        }
        match kind.name.as_str() {
            "TypeVarTuple" => attributes.retain(|entry| !matches!(entry.0.as_str(), "__bound__" | "__constraints__" | "__covariant__" | "__contravariant__" | "__infer_variance__")),
            "ParamSpec" => {
                attributes.retain(|entry| entry.0 != "__constraints__");
                if let Some(entry) = attributes.iter_mut().find(|entry| entry.0 == "__bound__") { entry.1 = Value::Blueprint(self.native_kind("NoneType")); }
            }, _ => {},
        }
        for (option, value) in options {
            let destination = match option.as_str() {
                "type_params" => "__type_params__", "infer_variance" => "__infer_variance__", "default" => "__default__",
                "covariant" => "__covariant__", "contravariant" => "__contravariant__", "bound" => "__bound__",
                _ => return Err(format!("TypeError: {}() got an unexpected keyword argument '{option}'", kind.name).into()),
            };
            let value = match option.as_str() {
                "covariant" | "contravariant" | "infer_variance" => self.apply_class_member(Value::Intrinsic(Prim::Truthful, Rc::from("bool")), vec![value])?,
                "bound" if kind.name == "ParamSpec" || !matches!(value.settled(), Value::Nil) => {
                    let typing = self.load_namespace("typing")?;
                    let validate = self.read_class_member(typing, "_type_check", false)?.settled();
                    self.apply_class_member(validate, vec![value, Value::text("Bound must be a type.")])?
                }
                _ => value,
            };
            if let Some(entry) = attributes.iter_mut().find(|entry| entry.0 == destination) { entry.1 = value; }
            else { return Err(format!("TypeError: {}() got an unexpected keyword argument '{option}'", kind.name.to_lowercase()).into()); }
        }

        if kind.name == "TypeVar" || kind.name == "ParamSpec" {
            let switches: Vec<bool> = ["__covariant__", "__contravariant__", "__infer_variance__"].iter().map(|name| attributes.iter().find(|entry| &entry.0 == name).is_some_and(|entry| entry.1.is_true())).collect();
            if switches[0] && switches[1] { return Err(String::from("ValueError: Bivariant types are not supported.").into()); }
            if switches[2] && (switches[0] || switches[1]) { return Err(String::from("ValueError: Variance cannot be specified with infer_variance.").into()); }
            if kind.name == "TypeVar" && values.len() > 1 && attributes.iter().any(|entry| entry.0 == "__bound__" && !matches!(entry.1, Value::Nil)) { return Err(String::from("TypeError: Constraints cannot be combined with bound=...").into()); }
            let prefix = match switches.as_slice() { [_, _, true] => "", [true, _, _] => "+", [_, true, _] => "-", _ => "~" };
            let shown = String::from(prefix) + &values[0].bare();
            attributes.iter_mut().find(|entry| entry.0 == "\0type-display").unwrap().1 = Value::text(&shown);
        }
        self.made += 1;
        Ok(Value::Thing(Rc::new(Thing { reclassified: RefCell::new(None), of: kind, holds: RefCell::new(attributes), turn: self.made })))
    }
    fn alias_from_thunk(&mut self, thunk: Value) -> Res {
        let code = match thunk.settled() {
            Value::Routine(code) | Value::Bound(code, _) => code,
            _ => return Err(self.class_unready()),
        };
        let kind = self.native_kind("TypeAliasType");
        let alias = self.make_type_parameter(kind, vec![Value::text(&code.ident), Value::Nil])?;
        if let Value::Thing(instance) = &alias {
            let mut attributes = instance.holds.borrow_mut();
            attributes.retain(|entry| entry.0 != "__value__");
            attributes.push((String::from("\0deferred/__value__"), thunk));
        }
        Ok(alias)
    }
    pub(super) fn construct_ordered(&mut self,class:Rc<Blueprint>,given:Vec<Value>)->Res {
        if self.has_class_order() && Self::parent_kind_descended(&class) { return self.parent_constructed(class, given); }
        if Self::native_word(&class).is_some() && matches!(class.name.as_str(), "TypeVar" | "ParamSpec" | "TypeVarTuple" | "TypeAliasType" | "NoDefaultType") {
            return self.make_type_parameter(class, given);
        }

        if Self::native_word(&class).as_deref() == Some("function") && !self.rules.trace_words.is_empty() {
            let (positional, keywords) = self.open_arguments(given)?;
            let mut options = vec![None; 6];
            for (slot, value) in positional.into_iter().enumerate() {
                if slot >= options.len() { return Err(String::from("TypeError: function() takes at most 6 arguments").into()); }
                options[slot] = Some(value);
            }
            for (word, value) in keywords {
                let Some(slot) = ["code", "globals", "name", "argdefs", "closure", "kwdefaults"].iter().position(|name| *name == word) else {
                    return Err(format!("TypeError: function() got an unexpected keyword argument '{word}'").into());
                };
                if options[slot].replace(value).is_some() { return Err(String::from("TypeError: invalid function arguments").into()); }
            }
            if let Some(value @ Value::Thing(_)) = options[0].as_ref().map(Value::settled) {
                let recognized = match &value {
                    Value::Thing(instance) => self.code_kind.as_ref().is_some_and(|kind| Rc::ptr_eq(kind, &instance.blueprint())),
                    _ => false,
                };
                if recognized {
                    let namespace = options[1].clone().ok_or_else(|| String::from("TypeError: function() missing required argument 'globals'"))?;
                    let dictionaries = matches!(namespace.settled(), Value::Dict(_)) || Self::underlying(&namespace).is_some_and(|contents| matches!(contents.settled(), Value::Dict(_)));
                    if !dictionaries { return Err(String::from("TypeError: function() argument 'globals' must be dict").into()); }
                    let title = match options[2].as_ref().map(Value::settled) {
                        Some(title @ Value::Text(_)) => title,
                        None | Some(Value::Nil) => Value::text("<module>"),
                        _ => return Err(String::from("TypeError: arg 3 (name) must be None or string").into()),
                    };
                    let defaults = options[3].as_ref().map(Value::settled).unwrap_or(Value::Nil);
                    if !matches!(defaults, Value::Nil | Value::Tuple(_)) {
                        return Err(String::from("TypeError: arg 4 (defaults) must be None or tuple").into());
                    }
                    let closure = options[4].as_ref().map(Value::settled).unwrap_or(Value::Nil);
                    match &closure {
                        Value::Nil => {},
                        Value::Tuple(cells) if cells.is_empty() => {},
                        Value::Tuple(_) => return Err(String::from("ValueError: module code requires closure of length 0").into()),
                        _ => return Err(String::from("TypeError: arg 5 (closure) must be None or tuple").into()),
                    }
                    return Ok(Self::wrap(122, vec![value, namespace, title, defaults, closure]));
                }
            }
            let Some(Value::Wrapped(7, body)) = options[0].as_ref().map(Value::settled) else { return Err(String::from("TypeError: function() argument 'code' must be code").into()); };
            let Some(Value::Routine(origin) | Value::Bound(origin, _)) = body.first() else { return Err(String::from("TypeError: function() argument 'code' must be code").into()); };
            let Some(globals) = options[1].clone() else { return Err(String::from("TypeError: function() missing required argument 'globals'").into()); };
            let dictionaries = matches!(globals.settled(), Value::Dict(_)) || Self::underlying(&globals).is_some_and(|contents| matches!(contents.settled(), Value::Dict(_)));
            if !dictionaries { return Err(String::from("TypeError: function() argument 'globals' must be dict").into()); }
            let mut fresh = (**origin).clone();
            fresh.globe = Some(globals.clone());
            fresh.born = Some(self.builtins_here());
            fresh.framed_in = match globals.settled() {
                Value::Dict(entries) => entries.iter().find(|(key, _)| key.bare() == "__name__").and_then(|(_, value)| match value.settled() { Value::Text(name) => Some(name), _ => None }),
                _ => None,
            };
            if let Some(Value::Text(name)) = options[2].as_ref().map(Value::settled) { fresh.ident = name.to_string(); fresh.qualification = name.to_string(); }
            let closure = match options[4].as_ref().map(Value::settled) {
                None | Some(Value::Nil) => Vec::new(),
                Some(Value::Tuple(row)) => row.to_vec(),
                _ => return Err(String::from("TypeError: arg 5 (closure) must be tuple").into()),
            };
            if closure.len() != fresh.reaching.len() { return Err(String::from("ValueError: function requires a closure of the right length").into()); }
            let mut layers = Vec::new();
            let mut parent = self.outermost.clone();
            let depth = fresh.reaching.iter().map(|address| address.up).max().unwrap_or(0);
            for level in (1..=depth).rev() {
                let extent = fresh.reaching.iter().filter(|address| address.up == level).map(|address| address.at + 1).max().unwrap_or(0);
                let frame = Env::make(extent, Some(parent));
                parent = frame.clone();
                layers.insert(0, frame);
            }
            let mut free: Vec<_> = fresh.reaching.iter().collect();
            free.sort_by(|left, right| left.ident.cmp(&right.ident));
            for (address, cell) in free.into_iter().zip(closure.iter()) {
                let Value::Wrapped(35, items) = cell.settled() else { return Err(String::from("TypeError: arg 5 (closure) must contain cells").into()); };
                let held = match self.cell_place(&items) {
                    Some((frame, slot)) => frame.cells.borrow().get(slot).cloned().unwrap_or(Value::Unset),
                    None if matches!(items.as_slice(), [Value::Shared(_)]) => self.cell_contents(&items).unwrap_or(Value::Unset),
                    None => return Err(String::from("TypeError: arg 5 (closure) must contain cells").into()),
                };
                if address.up > 0 {
                    layers[address.up - 1].cells.borrow_mut()[address.at] = held;
                    if let Some((frame, slot)) = self.cell_place(&items) {
                        if frame.capture_slots.borrow().contains(&slot) { layers[address.up - 1].capture_slots.borrow_mut().insert(address.at); }
                    }
                }
            }
            let shared_room = closure.first().and_then(|cell| match cell.settled() {
                Value::Wrapped(35, items) => match items.first() { Some(Value::Bound(_, room)) => Some(room.clone()), _ => None },
                _ => None,
            }).filter(|room| closure.iter().all(|cell| match cell.settled() {
                Value::Wrapped(35, items) => matches!(items.first(), Some(Value::Bound(_, other)) if Rc::ptr_eq(other, room)),
                _ => false,
            }));
            fresh.framed_in = match globals.settled() {
                Value::Dict(pairs)=>pairs.iter().find(|(k,_)|matches!(k,Value::Text(t) if t.as_ref()=="__name__")).and_then(|(_,v)|match v.settled() { Value::Text(named)=>Some(named.clone()), _=>None }),
                _=>None,
            };
            fresh.globe = Some(globals.clone());
            let word = self.table.strings("ext.system.module.builtins").first().cloned().unwrap_or_default();
            fresh.born = Some(match self.mapping_read(&globals, &word)? {
                Some(value) => value,
                None => self.builtins_here(),
            });
            let source = Rc::new(fresh);
            let base = shared_room.or_else(|| layers.first().cloned()).unwrap_or_else(|| self.outermost.clone());
            let (source, room) = if let Some(Value::Tuple(defaults)) = options[3].as_ref().map(Value::settled) {
                let (adjusted, kept) = self.respared(&source, &base, base.clone(), Some(defaults.as_ref().clone()), None);
                (Rc::new(adjusted), kept)
            } else { (source, base) };
            let named_defaults = match options[5].as_ref().map(Value::settled) {
                Some(Value::Dict(pairs)) => pairs.iter().map(|(name, value)| (name.bare(), value.clone())).collect(),
                None | Some(Value::Nil) => Vec::new(),
                _ => return Err(String::from("TypeError: arg 6 (kwdefaults) must be None or dict").into()),
            };
            let (adjusted, scope) = self.respared(&source, &room, room.clone(), None, Some(named_defaults));
            let callable = Value::Bound(Rc::new(adjusted), scope);

            if matches!(origin.ident.as_str(), "<generator>" | "<genexpr>") { return Ok(Value::Wrapped(43, Rc::new(vec![callable]).into())); }
            return Ok(callable);
        }
        if let Some(builder)=Self::builder_over(&class) {
            if let Some(f)=self.inherited_entry(&builder,self.detail("call")) {
                let mut values=vec![Value::Blueprint(class)];values.extend(given);
                return self.apply_class_member(f,values);
            }
        }
        self.construct_plainly(class,given)
    }
    fn initialise_type_object(&mut self,arguments:Vec<Value>)->Res {
        let (values,named)=self.open_arguments(arguments)?;
        let Some(first)=values.first() else{return Err("TypeError: descriptor '__init__' of 'type' object needs an argument".to_owned().into())};
        let receiver=first.settled();
        if !self.stands_for_a_kind(&receiver){return Err(format!("TypeError: descriptor '__init__' requires a 'type' object but received a '{}'",receiver.kind_word()).into());}
        match values.len()-1 {
            3=>Ok(Value::Nil),
            1 if named.is_empty()=>Ok(Value::Nil),
            1=>Err("TypeError: type.__init__() takes no keyword arguments".to_owned().into()),
            _=>Err("TypeError: type.__init__() takes 1 or 3 arguments".to_owned().into()),
        }
    }
    fn finish_class_construction(&mut self,built:&Value,called:&Rc<Blueprint>,arguments:Vec<Value>)->Result<(),Escape> {
        if let Value::Blueprint(class)=built {
            let builder=match Self::builder_over(class){Some(b)=>b,None=>self.builder_blueprint()};
            if Rc::ptr_eq(&builder,called)||builder.ancestry.borrow().iter().any(|p|Rc::ptr_eq(p,called)) {
                let hook=self.rules.words_ext_stmt_class_constructor.first().map(String::as_str).and_then(|key|self.inherited_entry(&builder,key));
                if let Some(hook)=hook {
                    let bound=self.member_binding(hook,Some(built.clone()),builder)?;
                    let result=self.apply_class_member(bound,arguments)?;
                    if !matches!(result,Value::Nil){return Err(format!("TypeError: __init__() should return None, not '{}'",result.kind_word()).into());}
                }
            }
        }
        Ok(())
    }
    /// The making itself, as the kind primitive does it: the class
    /// allocates a thing and constructs it.
    pub(super) fn construct_plainly(&mut self,class:Rc<Blueprint>,given:Vec<Value>)->Res {
        self.abstract_turned_away(&class)?;
        let native = match Self::native_beneath(&class) {
            Some(word) if word == "Generic" && !self.table.strings("ext.stmt.type_params.open").is_empty() => None,
            other => other,
        };
        let allocator=self.inherited_entry(&class,self.detail("allocate")).filter(|value| {
            !matches!(value, Value::Wrapped(14, _))
                || Self::own_entry(&class, self.detail("allocate")).is_some()
        });
        // A metaclass called outright builds a class, the way the kind
        // primitive does, from a name, parents and a namespace.
        let metaclass=self.builds_classes(&class)||class.ancestry.borrow().iter().any(|b|self.builds_classes(b));
        let created=match (allocator,&native) {
            (None,_) if metaclass=>{
                let mut values=vec![Value::Blueprint(class.clone())];values.extend(given.clone());
                self.class_of_parts(values)?
            },
            (Some(allocator),_)=>{let mut args=vec![Value::Blueprint(class.clone())];args.extend(given.clone());self.apply_class_member(allocator,args)?},
            (None,Some(word))=>{
                if matches!(word.as_str(),"str_iterator"|"str_ascii_iterator"){
                    return Err(format!("TypeError: cannot create '{}' instances",word).into());
                }
                let initial = match self.table.prims.get(word) {
                    Some(Prim::Uniques) => Vec::new(),
                    // A dict-derived constructor receives the original input after
                    // an empty native dictionary has been allocated for its instance.
                    Some(Prim::Dictionary) if self.rules.words_ext_stmt_class_constructor.first().map(String::as_str).and_then(|key| self.inherited_entry(&class, key)).is_some() => Vec::new(),
                    Some(Prim::AsReal) if self.rules.words_ext_stmt_class_constructor.first().map(String::as_str).and_then(|key| self.inherited_entry(&class, key)).is_some() => {
                        self.open_arguments(given.clone())?.0.into_iter().take(1).collect()
                    },
                    // A mutable container takes its members in its own
                    // `__init__`; where the class writes one, the kind
                    // primitive is asked only to allocate the empty thing
                    // and the class's method is handed the arguments.
                    Some(Prim::Listed | Prim::Dictionary) if self.rules.words_ext_stmt_class_constructor.first().map(String::as_str).and_then(|key| self.inherited_entry(&class, key)).is_some() => Vec::new(),
                    Some(Prim::Filtered) if self.rules.words_ext_stmt_class_constructor.first().map(String::as_str).and_then(|key| self.inherited_entry(&class, key)).is_some() => self.open_arguments(given.clone())?.0,
                    Some(Prim::Unchanging | Prim::Tupling) if self.rules.words_ext_stmt_class_constructor.first().map(String::as_str).and_then(|key| self.inherited_entry(&class, key)).is_some() => self.open_arguments(given.clone())?.0,
                    _ => given.clone(),
                };
                self.thing_over_native(class.clone(),word,initial)?
            },
            (None,None)=>{self.made+=1;Value::Thing(Rc::new(Thing {reclassified: RefCell::new(None), of:class.clone(),holds:RefCell::new(Vec::new()),turn:self.made}))},
        };
        if metaclass {self.finish_class_construction(&created,&class,given.clone())?;}
        if let Value::Thing(thing)=&created {
            // A thing whose class bids farewell is noted, so that a round
            // holding it can be found when the program asks.
            if crate::ghost::bidding()&&crate::ghost::farewell_of(&thing.of).is_some() {
                crate::ghost::note(crate::ghost::Ghost::Thing(Rc::downgrade(thing)));
                if self.rules.trace_words.first().map(String::as_str).is_some() { crate::ghost::anchor(thing); }
            }
            if self.table.strings("ext.stmt.class.destructor").first().map(String::as_str).is_some()
                || self.table.strings("ext.stmt.class.finaliser").first().map(String::as_str).is_some() {
                self.things.borrow_mut().push(Rc::downgrade(thing));
            }
            let belongs=Rc::ptr_eq(&thing.blueprint(),&class)||thing.blueprint().ancestry.borrow().iter().any(|c|Rc::ptr_eq(c,&class));
            if belongs {
                let constructor=self.rules.words_ext_stmt_class_constructor.first().map(String::as_str).and_then(|word|self.inherited_entry(&thing.blueprint(),word));
                if let Some(f)=constructor {
                    let bound=self.member_binding(f,Some(created.clone()),thing.blueprint().clone())?;
                    if !matches!(self.apply_class_member(bound,given)?,Value::Nil){return Err(self.class_unready());}
                }else if let Some(under @ Value::Set(_)) = Self::underlying(&created).filter(|v| !v.set_sealed()) {
                    let (positional, named) = self.open_arguments(given)?;
                    let key = self.rules.words_ext_stmt_class_constructor.first().map(String::as_str).unwrap_or_default().to_owned();
                    self.value_member(&under, &key, positional, named)?;
                }else if native.is_none()&&self.inherited_entry(&class,self.detail("allocate")).is_none(){
                    let (positional, keywords) = self.open_arguments(given)?;
                    if !positional.is_empty() || !keywords.is_empty() { self.root_turns_away(&class,'n')?; }
                }
            }
        }
        Ok(created)
    }
    pub(super) fn get_native_classmethod(&mut self, entry: &Value, input: Vec<Value>, options: Vec<(String, Value)>) -> Res {
        let count = input.len();
        if options.len() != 0 { return Err(String::from("TypeError: wrapper __get__() takes no keyword arguments").into()); }
        match count {
            0 => return Err(String::from("TypeError: __get__ expected at least 1 argument, got 0").into()),
            1 | 2 => (),
            _ => return Err(format!("TypeError: __get__ expected at most 2 arguments, got {count}").into()),
        }
        let candidate = input.get(1).map(Value::settled).unwrap_or(Value::Nil);
        let target = if matches!(candidate, Value::Nil) {
            if matches!(input[0].settled(), Value::Nil) { return Err(String::from("TypeError: __get__(None, None) is invalid").into()); }
            self.class_from_type(vec![input[0].clone()])?
        } else { candidate };
        let Value::Wrapped(60, metadata) = entry else { return Err(self.class_unready()); };
        let limited_name = |mut text: String| {
            while text.len() > 100 { text.pop(); }
            text
        };
        let home = metadata[0].bare();
        let public = metadata[1].bare();
        let bad_type_name = if let Value::Thing(instance) = &target {
            let kind = instance.blueprint();
            let native_buffer = kind.shared.borrow().iter().any(|entry| entry.0 == "\0buffer_allocator" && entry.1.is_true());
            let named = kind.type_names.borrow().is_some();
            if named && !native_buffer { Self::type_argument_kind(&target) }
            else { self.full_class_name(&kind) }
        } else { Self::type_argument_kind(&target) };
        let class = if let Value::Blueprint(type_object) = &target { type_object.clone() } else if matches!(&target, Value::Intrinsic(Prim::Truthful, _)) { self.native_kind("bool") } else { match self.parent_from_type(&target) { Ok(kind) => kind, Err(_) => {
            return Err(format!("TypeError: descriptor '{public}' for type '{home}' needs a type, not a '{}' as arg 2", limited_name(bad_type_name)).into());
        }} };
        let parent = self.kind_by_word(&home).ok_or_else(|| self.class_unready())?;
        let exported = class.shared.borrow().iter().any(|entry| entry.0 == "\0buffer_allocator" && entry.1.is_true());
        let public_type = match class.type_names.borrow().as_ref() {
            Some(names) if !exported => names.short.type_text().bare(),
            _ => self.full_class_name(&class),
        };
        if self.is_beneath(&target, &parent, true)? == false {
            return Err(format!("TypeError: descriptor '{public}' requires a subtype of '{home}' but received '{}'", limited_name(public_type)).into());
        }
        self.member_binding(entry.clone(), None, class)
    }
    pub(super) fn full_class_name(&self, kind: &Blueprint) -> String {
        let entries = kind.shared.borrow();
        let local = kind.type_names.borrow().as_ref().map(|names| names.full.bare()).unwrap_or_else(|| entries.iter().find(|(key, _)| key == self.detail("qualified")).map_or_else(|| kind.name.clone(), |(_, value)| value.bare()));
        entries.iter().find(|(key, _)| key == self.detail("module")).map_or(local.clone(), |(_, value)| format!("{}.{}", value.bare(), local))
    }

    fn explain_absence(&self, escaped: Escape, value: &Value, key: &str) -> Escape {
        let Escape::Thrown(Value::Thing(fault)) = &escaped else { return escaped };
        if !self.missing_member_escape(&escaped) { return escaped; }
        let said = match value {
            Value::Blueprint(kind) => format!("type object '{}' has no attribute '{key}'", self.full_class_name(kind)),
            Value::Thing(thing) if !self.rules.words_ext_system_module_name.is_empty() && Self::native_beneath(&thing.blueprint()).as_deref() == Some("module") => {
                let held = thing.holds.borrow();
                let book = held.iter().find(|(name, _)| name == "_namespace").map(|(_, item)| item.settled());
                let title = if let Some(Value::Dict(entries)) = &book {
                    entries.iter().find_map(|(name, item)| matches!(name, Value::Text(word) if word.as_ref() == "__name__").then_some(item))
                } else {
                    held.iter().find(|(name, _)| name == "__name__").map(|(_, item)| item)
                };
                let label = title.and_then(|item| match item.settled() { Value::Text(word) => Some(word.to_string()), _ => None });
                label.map_or_else(|| format!("module has no attribute '{key}'"), |label| format!("module '{label}' has no attribute '{key}'"))
            }
            Value::Thing(thing) => format!("'{}' object has no attribute '{key}'", self.full_class_name(&thing.blueprint())),
            _ => return escaped,
        };
        let mut held = fault.holds.borrow_mut();
        let args_key = self.rules.words_ext_builtin_exceptions_args.first().map(String::as_str).unwrap_or("args");
        let old = held.iter().find(|(name, _)| name == "\0raised-values").and_then(|(_, item)| match item { Value::Arguments(items) => Some(items.clone()), _ => None });
        if old.as_ref().is_some_and(|items| items.is_empty() || items.len() == 1 && items[0].bare() == key) {
            let fresh = crate::tuples::Sequence::plain(vec![Value::text(&said)]);
            for (name, item) in held.iter_mut() {
                if name == "\0raised-values" { *item = Value::Arguments(fresh.clone()); }
                if name == args_key { *item = Value::Tuple(fresh.clone()); }
            }
        }
        for (name, item) in held.iter_mut() {
            if self.rules.words_ext_builtin_exceptions_name.first().map(String::as_str) == Some(name.as_str()) { *item = Value::text(key); }
            if self.table.strings("ext.builtin.exceptions.object").first().map(String::as_str) == Some(name.as_str()) { *item = value.clone(); }
        }
        drop(held);
        escaped
    }

    fn absent_attribute(&self,value:&Value,member:&str)->Escape {
        // A namespace and a kind go by their own name, in words of
        // their own, as CPython names them.
        let named=self.member_named_missing(value,member);
        if !named.is_empty(){return named.into();}
        // A thing and a blueprint go by their own name; anything else
        // by the name its kind goes under.
        let name=match value{Value::Thing(t) => {
            let blueprint = t.blueprint();
            let public = if Self::native_word(&blueprint).is_some() {
                blueprint.presentation.as_ref().and_then(|s| s.strip_suffix("'>")?.strip_prefix("<class '")).map(String::from)
            } else { None };
            public.unwrap_or_else(|| blueprint.name.clone())
        },Value::Blueprint(b)=>b.name.clone(),other=>other.kind_word()};
        let parts=self.table.strings("ext.stmt.class.detail.attribute.amiss");
        if parts.len()<3{return self.class_unready();}
        format!("{}{name}{}{member}{}",parts[0],parts[1],parts[2]).into()
    }
    /// An entry a thing reads but will neither write over nor let go:
    /// its blueprint names the entries its things hold and holds an
    /// entry of its own under this name, which no write to a thing
    /// reaches.
    fn readonly_attribute(&self,value:&Value,member:&str)->Escape {
        let parts=self.table.strings("ext.stmt.class.detail.attribute.readonly");
        if parts.len()<3{return self.unwritable_attribute(value,member);}
        let name=match value{Value::Thing(t)=>t.blueprint().name.clone(),other=>other.kind_word()};
        format!("{}{name}{}{member}{}",parts[0],parts[1],parts[2]).into()
    }
    /// An entry a value will not take: it keeps no namespace of its own
    /// to hold one, its blueprint naming the entries it holds or the
    /// value being of a builtin kind.
    fn unwritable_attribute(&self,value:&Value,member:&str)->Escape {
        let told=self.member_unwritable(value,member);
        if told.is_empty(){return self.absent_attribute(value,member);}
        told.into()
    }
    pub(super) fn member_binding(&mut self,entry:Value,receiver:Option<Value>,owner:Rc<Blueprint>)->Res {
        if receiver.is_some() {
            if let Value::Intrinsic(Prim::Textual(_), spelling) = &entry {
                if spelling.contains('.') { return Ok(Self::wrap(3, vec![entry.clone(), receiver.clone().unwrap()])); }
            }
        }
        if let Value::Wrapped(tag @ 10..=12, state) = &entry {
            if state.first().is_some_and(|owner| owner.bare() == self.detail("root")) {
                if let Some(target) = receiver {
                    let (actual, valid) = self.object_descriptor_target(&target)?;
                    if !valid {
                        let method = self.detail(if *tag == 10 { "get" } else if *tag == 11 { "set" } else { "remove" });
                        return Err(format!("TypeError: descriptor '{method}' for '{}' objects doesn't apply to a '{actual}' object", self.detail("root")).into());
                    }
                    return Ok(Self::wrap(3, vec![entry, target]));
                }
                return Ok(entry);
            }
        }
        if let Value::Wrapped(60, parts) = &entry {
            if parts.len() == 3 {
                let owner = if let Some(Value::Thing(instance)) = &receiver { instance.blueprint() }
                    else if let Some(Value::Blueprint(class)) = &receiver { class.clone() } else { owner };
                let qualified = format!("{}.{}", parts[0].bare(), parts[1].bare());
                if Self::native_word(&owner).as_deref() == Some(parts[0].bare().as_str()) {
                    if let Some(task) = self.table.prims.get(&qualified) { return Ok(Value::Intrinsic(*task, Rc::from(qualified))); }
                }
                return Ok(Value::Member(Rc::new(Value::Blueprint(owner)), parts[1].bare()));

            }
            let key = parts[1].bare();
            if parts[0].bare() == "type" && receiver.is_some() && ["namespace", "mro", "name"].iter().any(|part| key == self.detail(part)) {
                let target = receiver.as_ref().unwrap().settled();
                let owner = match target {
                    Value::Blueprint(b) => b,
                    Value::Intrinsic(Prim::SortOf, _) => self.builder_blueprint(),
                    other => {
                        if !self.stands_for_a_kind(&other) { return Err(self.class_unready()); }
                        let word = other.kind_it_names().map(Rc::<str>::from).or_else(|| self.kind_spelling(&other)).ok_or_else(|| self.class_unready())?;
                        self.native_kind(&word)
                    },
                };
                if key == self.detail("name") {
                    let spelling = owner.type_names.borrow().as_ref().map(|names| names.short.type_text().bare()).unwrap_or_else(|| owner.name.clone());
                    return Ok(Value::text(&spelling));
                }
                if key == self.detail("mro") {
                    let ranks = Self::resolution_order(&owner).into_iter().map(|base| self.visible_blueprint(base)).collect();
                    return Ok(Value::tuple(ranks));
                }
                return Ok(Value::Window(Rc::new(Value::Blueprint(owner)), 'm'));
            }
            if let Some(instance) = receiver.as_ref() {
                let slot = Value::loose_member_descriptor(&parts[0].bare(), &key);
                if self.names_in_calls && matches!(slot, Some(("attribute" | "member", _))) {
                    let raw = Self::underlying(instance).unwrap_or_else(|| instance.clone()).settled();
                    return match self.attribute(&raw, &key) {
                        Some(Value::Member(..)) => self.method_of_value(raw, &key),
                        Some(value) => Ok(value),
                        None => self.read_class_member(raw, &key, false),
                    };
                }
            }
            if parts[0].bare() == "int" {
                let value = receiver.as_ref().cloned().unwrap_or(Value::Blueprint(owner.clone()));
                if let Some(integer) = self.integer_attribute(&value, &parts[1].bare()) { return Ok(integer); }
            }
            if parts[0].bare() == "dict" && self.table.spells("ext.builtin.method.fromkeys", &parts[1].bare()) {
                return Ok(Value::Member(Rc::new(Value::Blueprint(owner)), parts[1].bare()));
            }
        }
        if matches!(&entry, Value::Wrapped(124, _)) {
            let fits = receiver.as_ref().map(Value::settled).map_or(true, |v| {
                matches!(v, Value::Thing(t) if Self::parent_kind_descended(&t.blueprint()))
            });
            if !fits {
                let kind = self.class_from_type(vec![receiver.as_ref().unwrap().clone()])?;
                let received = kind.kind_it_names().ok_or_else(|| self.class_unready())?;
                return Err(format!("TypeError: descriptor '__init__' for 'super' objects doesn't apply to a '{received}' object").into());
            }
        }
        if let Value::Wrapped(60, fields) = &entry {
            if fields[0].bare() == "type" && fields[1].bare() == self.detail("order") {
                return match receiver {
                    Some(target) => {
                        let mut state = fields.to_vec();
                        state.push(Value::Flag(true));
                        Ok(Self::wrap(3, vec![Self::wrap(60, state), target]))
                    },
                    None => Ok(entry),
                };
            }
        }
        match &entry {
            Value::Wrapped(134, ref parts) if parts[0].bare() == "normal_pdf" && receiver.is_some() => return Ok(Self::wrap(3, vec![entry.clone(), receiver.unwrap()])),
            Value::Wrapped(2, items) if !items.is_empty() && receiver.is_some() => return Ok(Self::wrap(3, vec![entry.clone(), receiver.unwrap()])),
            Value::Wrapped(120, _) if receiver.is_some() => return Ok(Self::wrap(3, vec![entry.clone(), receiver.unwrap()])),
            Value::Wrapped(133 | 136 | 60 | 120 | 123 | 124 | 125 | 127, _) if receiver.is_some() => return Ok(Self::wrap(3, vec![entry.clone(), receiver.unwrap()])),
            Value::Wrapped(4,items)=>return Ok(items[0].clone()),
            Value::Wrapped(5,items)=>return Ok(Self::wrap(3,vec![items[0].clone(),Value::Blueprint(owner)])),
            Value::Wrapped(6,items) if receiver.is_some()=>return self.apply_class_member(items[0].clone(),vec![receiver.unwrap()]),
            Value::Wrapped(32,items) if receiver.is_some()=>return self.slot_value(&receiver.unwrap(),items),
            // A working of the property blueprint, reached through a
            // property, is tied to it; a kept accessor reads at once.

            Value::Wrapped(60, _) if receiver.is_some()=>return Ok(Self::wrap(3,vec![entry.clone(),receiver.unwrap()])),
            Value::Wrapped(36 | 50..=57 | 78..=79 | 200..=203 | 233..=235,_) if receiver.is_some()=>return Ok(Self::wrap(3,vec![entry.clone(),receiver.unwrap()])),
            Value::Wrapped(58,items)=>return match receiver {Some(Value::Thing(t))=>self.accessor_shown(&t,&items[0].bare()),_=>Ok(entry)},
            _=>{}
        }
        // A member whose blueprint furnishes a reader answers through it,
        // told the thing -- nothing, for a read on the class -- and the
        // class the read came through.
        if let Some(reader)=self.protocol_entry(&entry,"descriptor.get") {
            return self.through_descriptor(&entry,reader,vec![receiver.unwrap_or(Value::Nil),Value::Blueprint(owner)]);
        }
        match receiver {
            Some(Value::Thing(t))=>match entry {Value::Routine(code)=>Ok(Value::method(code, t)),Value::Bound(..)=>Ok(Self::wrap(3,vec![entry,Value::Thing(t)])),_=>Ok(entry)},
            Some(other) if matches!(entry,Value::Routine(_)|Value::Bound(..))=>Ok(Self::wrap(3,vec![entry,other])),
            _=>Ok(entry),
        }
    }
    /// The entry a routine keeps the namespace handed to it under.
    const HANDED: &'static str = "\0handed";
    /// A routine's program and frame as calls of it now run.
    pub(super) fn routine_standing(&self,value:&Value)->(Rc<Routine>,Rc<Env>) {
        match value {
            Value::Bound(code,room)=>self.as_now_written(code.clone(),room.clone()),
            Value::Routine(code)|Value::Method(code, _, _)=>self.as_now_written(code.clone(),self.outermost.clone()),
            _=>unreachable!("only a routine stands this way"),
        }
    }
    fn written_key(value:&Value,outermost:&Rc<Env>)->(usize,usize) {
        match value {
            Value::Bound(code,room)=>(Rc::as_ptr(code) as usize,Rc::as_ptr(room) as usize),
            Value::Routine(code)|Value::Method(code, _, _)=>(Rc::as_ptr(code) as usize,Rc::as_ptr(outermost) as usize),
            _=>(0,0),
        }
    }
    /// The program whose code a routine runs: its own, or the one whose
    /// code was written over it.
    fn code_run_by(&self,value:&Value)->Rc<Routine> {
        if let Some(entry)=self.written_over.get(&Self::written_key(value,&self.outermost)) {return entry.4.clone();}
        match value {Value::Bound(code,_)|Value::Routine(code)|Value::Method(code, _, _)=>code.clone(),_=>unreachable!("only a routine runs code")}
    }
    /// The builtins a routine reaches its unbound names through: what
    /// the dictionary it was handed keeps under that name, else the
    /// builtins in force where it was made, else the kernel's own
    /// dictionary.
    fn routine_builtins(&mut self,code:&Routine)->Res {
        if let Some(word)=self.table.strings("ext.system.module.builtins").first().map(String::as_str).map(str::to_owned) {
            if let Some(globe)=&code.globe {
                if let Some(held)=self.mapping_read(globe,&word)?{return Ok(held);}
            }
        }
        if let Some(born)=&code.born{return Ok(born.clone());}
        Ok(Value::Mutable(self.natives_kept(),true))
    }
    /// What a routine holds of its own under a name: its name, full name
    /// or account as the program wrote them, the namespace handed to it,
    /// or an entry of the namespace it keeps.
    fn routine_holding(&self,value:&Value,key:&str)->Option<Value> {
        let (_,members)=self.routine_members.iter().find(|(f,_)|f.revive().is_some_and(|key| key.equals(value)))?;
        let holds=members.holds.borrow();
        let annotation_key = format!("\0{key}\0");
        if let Some((_, value)) = holds.iter().find(|(word, _)| *word == annotation_key) { return Some(value.clone()); }
        let apart=format!("{key}\0");
        if let Some((_,v))=holds.iter().find(|(k,_)|*k==apart){return Some(v.clone());}
        let Some((_,book))=holds.iter().find(|(k,_)|k==Self::HANDED) else{return holds.iter().find(|(k,_)|k==key).map(|(_,v)|v.clone())};
        if key==self.detail("namespace"){return Some(book.clone());}
        match book.settled() {
            Value::Dict(entries)=>entries.iter().find(|(k,_)|matches!(k,Value::Text(t) if t.as_ref()==key)).map(|(_,v)|v.clone()),
            _=>None,
        }
    }
    fn routine_holding_names(&self,value:&Value)->Vec<String> {
        let Some((_,members))=self.routine_members.iter().find(|(f,_)|f.revive().is_some_and(|key| key.equals(value))) else{return Vec::new()};
        let holds=members.holds.borrow();
        match holds.iter().find(|(k,_)|k==Self::HANDED) {
            Some((_,book))=>match book.settled() {Value::Dict(entries)=>entries.iter().filter_map(|(k,_)|if let Value::Text(t)=k{Some(t.to_string())}else{None}).collect(),_=>Vec::new()},
            None=>holds.iter().filter(|(k,_)|!k.ends_with('\0')&&!k.starts_with('\0')).map(|(k,_)|k.clone()).collect(),
        }
    }
    /// An entry written into a handed namespace, or taken out of it,
    /// through the cell it lives in so every name for it sees it.
    pub(super) fn write_into_book(book:&Value,key:&str,replacement:Option<Value>)->bool {
        let (Value::Mutable(cell,_)|Value::Shared(cell))=book else{return false};
        let mut held=cell.borrow_mut();
        let Value::Dict(entries)=&mut *held else{return false};
        let pairs=Rc::make_mut(entries);
        let found=pairs.iter().position(|(k,_)|matches!(k,Value::Text(t) if t.as_ref()==key));
        match (found,replacement) {
            (Some(at),Some(v))=>pairs[at].1=v,
            (None,Some(v))=>pairs.push((Value::text(key),v)),
            (Some(at),None)=>{pairs.remove(at);}
            (None,None)=>return false,
        }
        true
    }
    fn namespace_refused(&self,handed:&Value)->Escape {
        let words=self.table.strings("ext.stmt.class.detail.namespace.amiss");
        if words.len()!=2{return self.class_unready();}
        format!("{}{}{}",words[0],handed.kind_word(),words[1]).into()
    }
    /// The spare worths a routine carries, place by place: those taken
    /// in order (`p`) or those taken by name alone (`n`).
    fn spare_worths(&self,code:&Routine,room:&Env,manner:char)->Vec<(usize,Value)> {
        let cells=room.cells.borrow();
        code.carried.iter().filter_map(|slot|{
            let at=code.formal_slots.iter().position(|s|s==slot)?;
            let named=code.taking.as_ref().map_or(false,|rules|rules[at]=='n');
            (named==(manner=='n')).then(||(at,cells.get(*slot).cloned().unwrap_or(Value::Unset)))
        }).collect()
    }
    /// The frame a routine's calls stand under.
    fn standing_under(&self,code:&Routine,room:&Rc<Env>)->Rc<Env> {
        if code.carried.is_empty(){room.clone()}else{room.outer.clone().unwrap_or_else(||self.outermost.clone())}
    }
    /// A program carrying other spare worths, and the frame it is bound
    /// to: those taken in order laid over the last such places, those
    /// taken by name alone over the places so named; what is not given
    /// is kept as it was read from `from`.
    fn respared(&self,code:&Routine,from:&Rc<Env>,under:Rc<Env>,in_order:Option<Vec<Value>>,by_name:Option<Vec<(String,Value)>>)->(Routine,Rc<Env>) {
        let manner=|at:usize|code.taking.as_ref().map_or('b',|rules|rules[at]);
        let ordered:Vec<usize>=(0..code.formals.len()).filter(|at|matches!(manner(*at),'b'|'p')).collect();
        let mut kept:Vec<(usize,Value)>=Vec::new();
        {
            let cells=from.cells.borrow();
            for slot in &code.carried {
                let keep=match code.formal_slots.iter().position(|s|s==slot) {
                    None=>true,
                    Some(at)=>if manner(at)=='n'{by_name.is_none()}else{in_order.is_none()},
                };
                if keep {kept.push((*slot,cells.get(*slot).cloned().unwrap_or(Value::Unset)));}
            }
        }
        let mut made=code.clone();
        if let Some(worths)=in_order {
            let fitted=worths.len().min(ordered.len());
            for (at,worth) in ordered[ordered.len()-fitted..].iter().zip(&worths[worths.len()-fitted..]) {kept.push((code.formal_slots[*at],worth.clone()));}
            made.local_defaults.retain(|slot|code.formal_slots.iter().position(|s|s==slot).map_or(true,|at|manner(at)=='n'));
        }
        if let Some(pairs)=by_name {
            for (name,worth) in pairs {
                if let Some(at)=(0..code.formals.len()).find(|at|manner(*at)=='n'&&code.formals[*at]==name) {kept.push((code.formal_slots[at],worth));}
            }
            made.local_defaults.retain(|slot|code.formal_slots.iter().position(|s|s==slot).map_or(true,|at|manner(at)!='n'));
        }
        kept.sort_by_key(|(slot,_)|*slot);
        made.carried=kept.iter().map(|(slot,_)|*slot).collect();
        made.least=ordered.len()-ordered.iter().filter(|at|made.carried.contains(&code.formal_slots[**at])).count();
        if made.carried.is_empty(){return (made,under);}
        let room=Env::make(made.idents.len(),Some(under));
        {
            let mut cells=room.cells.borrow_mut();
            for (slot,worth) in kept {cells[slot]=worth;}
        }
        (made,room)
    }
    /// A routine's spare worths, its keyword-only spare worths or its code
    /// written over or taken away, as calls of it will run from now on;
    /// what cannot stand there is refused in CPython's words.
    fn write_routine_over(&mut self,subject:&Value,key:&str,replacement:Option<Value>)->Res<()> {
        let refused=|me:&Self,part:&str|->Escape {me.detail(part).to_owned().into()};
        let (code,room)=self.routine_standing(subject);
        let under=self.standing_under(&code,&room);
        let mut runs=self.code_run_by(subject);
        let (made,bound)=if key==self.detail("code") {
            let Some(Value::Wrapped(7,parts))=replacement.as_ref().map(Value::settled) else{return Err(refused(self,"code.amiss"))};
            let Some(source@(Value::Routine(_)|Value::Bound(..)))=parts.first() else{return Err(refused(self,"code.amiss"))};
            let (source_code,source_room)=self.routine_standing(source);
            // Writing a routine's own program back leaves it as it was
            // made: the shadow the earlier write put there is taken away
            // rather than layered over once more.
            let own=match subject {Value::Bound(c,_)=>c.clone(),Value::Routine(c)|Value::Method(c, _, _)=>c.clone(),_=>return Err(self.class_unready())};
            let source_program=match source {Value::Routine(p)|Value::Bound(p,_)=>p.clone(),_=>return Err(self.class_unready())};
            if Rc::ptr_eq(&source_program,&own) {
                self.written_over.remove(&Self::written_key(subject,&self.outermost));
                return Ok(());
            }
            if (source_code.flags ^ code.flags) & 0x2a0 != 0 {
                let told=self.table.strings("ext.stmt.class.detail.code.mismatch").first().cloned().unwrap_or_default();
                self.warn_like(26,&told)?;
            }
            if source_code.reaching.len()!=code.reaching.len() {
                let words=self.table.strings("ext.stmt.class.detail.code.free");
                if words.len()!=3{return Err(self.class_unready());}
                return Err(format!("{}{}{}{}{}{}",words[0],code.ident,words[1],code.reaching.len(),words[2],source_code.reaching.len()).into());
            }
            runs=self.code_run_by(source);
            let mut taken=(*source_code).clone();
            taken.annotator=code.annotator.clone();
            taken.ident=code.ident.clone();
            taken.qualification=code.qualification.clone();
            taken.doc=code.doc.clone();
            let in_order=self.spare_worths(&code,&room,'p').into_iter().map(|(_,v)|v).collect();
            let by_name=self.spare_worths(&code,&room,'n').into_iter().map(|(at,v)|(code.formals[at].clone(),v)).collect();
            self.respared(&taken,&source_room,under,Some(in_order),Some(by_name))
        } else if key==self.detail("keywords") {
            let pairs=match replacement.as_ref().map(Value::settled) {
                None|Some(Value::Nil)=>Vec::new(),
                Some(Value::Dict(entries))=>entries.iter().map(|(k,v)|(k.bare(),v.clone())).collect(),
                Some(_)=>return Err(refused(self,"keywords.amiss")),
            };
            self.respared(&code,&room,under,None,Some(pairs))
        } else {
            let worths=match replacement.as_ref().map(Value::settled) {
                None|Some(Value::Nil)=>Vec::new(),
                Some(Value::Tuple(items))=>items.as_ref().clone(),
                Some(_)=>return Err(refused(self,"defaults.amiss")),
            };
            self.respared(&code,&room,under,Some(worths),None)
        };
        let (was,was_room)=match subject {
            Value::Bound(c,r)=>(c.clone(),r.clone()),
            Value::Routine(c)|Value::Method(c, _, _)=>(c.clone(),self.outermost.clone()),
            _=>return Err(self.class_unready()),
        };
        let place=Self::written_key(subject,&self.outermost);
        self.written_over.insert(place,(was,was_room,Rc::new(made),bound,runs));
        Ok(())
    }
    /// The frame a cell stands in and its place there: the name a
    /// routine reaches, found from where its calls stand.
    pub(super) fn cell_place(&self,items:&[Value])->Option<(Rc<Env>,usize)> {
        if let [Value::Bound(_, room), Value::Small(at), Value::Flag(true)] = items {
            let at = usize::try_from(*at).ok()?;
            return (at < room.cells.borrow().len()).then(|| (room.clone(), at));
        }
        let [Value::Bound(code,room),Value::Small(which)]=items else{return None};
        let address=code.reaching.get(usize::try_from(*which).ok()?)?;
        let mut frame=self.standing_under(code,room);
        for _ in 1..address.up {frame=frame.outer.clone()?;}
        Some((frame,address.at))
    }
    /// What a cell holds, or nothing where it is empty.
    pub(super) fn cell_contents(&self,items:&[Value])->Option<Value> {
        if let [Value::Shared(cell)] = items {
            let value = cell.borrow().clone();
            return (!matches!(value, Value::Unset)).then_some(value);
        }
        let (frame,at)=self.cell_place(items)?;
        let mut held=frame.cells.borrow().get(at).cloned()?;
        if frame.capture_slots.borrow().contains(&at) {
            let Value::Shared(binding)=held else { unreachable!() };
            let content=binding.borrow().clone();
            held=content;
        }
        (!matches!(held,Value::Unset)).then_some(held)
    }
    /// The root's making (`n`) or constructing (`i`), handed more than
    /// the class or the thing, refused as CPython refuses it: naming
    /// the root where the class wrote over that working, and the class
    /// where it wrote over neither.
    fn root_turns_away(&self,class:&Rc<Blueprint>,working:char)->Res<()> {
        let makes=self.inherited_entry(class,self.detail("allocate")).is_some();
        let builds=self.rules.words_ext_stmt_class_constructor.first().map(String::as_str).map_or(false,|word|self.inherited_entry(class,word).is_some());
        let (mine,theirs)=if working=='n'{(makes,builds)}else{(builds,makes)};
        let label=if working=='n'{"arguments.new"}else{"arguments.init"};
        let (label,named)=match (mine,theirs) {
            (true,_)=>(label,self.detail("root").to_owned()),
            (false,false)=>(if working=='n'{"arguments.none"}else{label},class.name.clone()),
            (false,true)=>return Ok(()),
        };
        let words=self.table.strings(&format!("ext.stmt.class.detail.{label}"));
        if words.len()!=2{return Err(self.class_unready());}
        Err(format!("{}{named}{}",words[0],words[1]).into())
    }
    /// A blueprint whose maker left it a set of names nobody answered is
    /// turned away from making, the names counted off in order: the
    /// label's words are the entry the set is kept under, the complaint's
    /// opening, and its two middles, one name or many.
    fn abstract_turned_away(&self,c:&Rc<Blueprint>)->Res<()> {
        let words=self.rules.words_ext_stmt_class_detail_abstract;
        if words.len()!=4 { return Ok(()); }
        let identity = Rc::as_ptr(c) as usize;
        if self.unmarked_classes.borrow().get(&identity).is_some_and(|known| known.as_ptr() == Rc::as_ptr(c) && known.strong_count() != 0) { return Ok(()); }
        let Some(held)=Self::own_entry(c,&words[0]) else {
            let mut known = self.unmarked_classes.borrow_mut();
            if known.len() > 128 { known.retain(|_, class| class.strong_count() != 0); }
            known.insert(identity, Rc::downgrade(c));
            return Ok(());
        };
        let mut names:Vec<String>=match held.settled() {
            Value::Set(store)=>store.borrow().entries.iter().filter_map(|(_,v)|match v {Value::Text(t)=>Some(t.to_string()),_=>None}).collect(),
            _=>Vec::new(),
        };
        if names.is_empty() { return Ok(()); }
        names.sort();
        let listed=names.iter().map(|n|format!("'{n}'")).collect::<Vec<_>>().join(", ");
        let middle=if names.len()==1 {&words[2]} else {&words[3]};
        Err(format!("{}{}{}{}",words[1],c.name,middle,listed).into())
    }
    /// A member every class and thing has from the root: one the table
    /// names among the root's members, or, read off a thing, one of the
    /// hooks for making, constructing, reading, writing, removing and
    /// formatting, left loose to be bound to it.
    fn common_allocation(&self) -> Value {
        self.loose_members.borrow_mut().entry("allocate the root".to_owned())
            .or_insert_with(|| Self::wrap(1, Vec::new())).clone()
    }
    fn common_initialiser(&self) -> Value {
        let spelling = self.table.single("ext.stmt.class.constructor").unwrap_or_default().to_owned();
        self.loose_entries.borrow_mut().entry((String::from("object"), spelling)).or_insert_with(|| Self::wrap(2, Vec::new())).clone()
    }
    fn from_the_root(&self,key:&str,of_a_thing:bool,class:&Blueprint)->Option<Value> {
        if key.is_empty(){return None;}
        if self.rules.words_ext_stmt_class_detail_root_members.iter().any(|word|word==key) {
            let owner = Self::native_beneath(class).unwrap_or_else(|| self.detail("root").to_owned());
            let cache_key = format!("{owner} member {key}");
            let mut cache = self.loose_members.borrow_mut();
            return Some(cache.entry(cache_key).or_insert_with(|| Self::wrap(36,vec![Value::text(key),Value::text(&owner)])).clone());
        }
        if !of_a_thing{return None;}
        let tag=if key==self.detail("allocate"){1}
            else if self.rules.words_ext_stmt_class_constructor.first().map(String::as_str)==Some(key)||key==self.detail("subclass"){2}
            else if key==self.detail("get"){10}else if key==self.detail("set"){11}else if key==self.detail("remove"){12}
            else if self.rules.specials.get(72).map_or(false,|word|word==key){59}
            else{return None};
        let state = match tag {
            10..=12 if self.table.has_any("ext.stmt.class.builder") => vec![Value::text(self.detail("root"))],
            _ if key == self.detail("subclass") => vec![Value::text(key)],
            _ => Vec::new(),
        };
        Some(if tag == 1 { self.common_allocation() } else if tag == 2 && self.table.single("ext.stmt.class.constructor") == Some(key) { self.common_initialiser() } else { Self::wrap(tag, state) })
    }
    /// The root's own answer for one of its members, the value it
    /// answers about coming first.
    pub(super) fn root_answers(&mut self,named:&str,values:Vec<Value>)->Res {
        let which=self.rules.words_ext_stmt_class_detail_root_members.iter().position(|word|word==named);
        if which == Some(9) && values.len() != 1 { return Err(self.method_fault("arguments").into()); }
        let not_mine=Value::Refusal(Rc::from(self.rules.words_ext_stmt_class_special_declined.first().map(String::as_str).unwrap_or("NotImplemented")));
        let Some(first)=values.first().cloned() else{return Err(self.class_unready())};
        let second=values.get(1).cloned();
        let same=|a:&Value,b:&Value|match (a,b){(Value::Thing(x),Value::Thing(y))=>Rc::ptr_eq(x,y),_=>contained_equal(a,b)};
        Ok(match which {
            Some(0)=>match &second{Some(other) if same(&first,other)=>Value::Flag(true),_=>not_mine},
            Some(1)=>{
                let Some(other)=second else{return Err(self.class_unready())};
                match self.ask_special(&first,2,&[other.clone()])? {
                    Some(said@Value::Refusal(_))=>said,
                    Some(said)=>Value::Flag(!self.object_truth(&said)?),
                    None=>if same(&first,&other){Value::Flag(false)}else{not_mine},
                }
            }
            Some(2..=5)|Some(14)=>not_mine,
            Some(6)=>match &first{
                Value::Thing(t)=>Value::Small(t.turn as i64),
                other=>self.prim(Prim::Hashed,"",&[other.clone()])?,
            },
            Some(7)=>match &first{
                Value::Thing(t)=>{
                    let module=self.namespace_named().unwrap_or_else(|| self.detail("main").to_owned());
                    Value::text(&if t.blueprint().under.is_none() && t.blueprint().name == "object" { "<object object at 0x1>".to_string() }
                        else if let Some(title) = t.blueprint().python_title() { format!("<{title} object at 0x1>") }
                        else if module.is_empty() { format!("<{} object>", t.blueprint().name) }
                        else { format!("<{module}.{} object at 0x1>", t.blueprint().name) })
                }
                other=>self.prim(Prim::Quoted,"",&[other.clone()])?,
            },
            Some(8)=>self.prim(Prim::Quoted,"",&[first])?,
            Some(9)=>self.ordinary_directory(&first)?,
            Some(10)=>Self::held_as_state(&first),
            Some(11|12)=>{
                let version = if which == Some(11) { 0 } else {
                    if values.len() != 2 { return Err(String::from("TypeError: __reduce_ex__() takes exactly one argument").into()); }
                    let argument = values[1].settled();
                    let whole = match argument {
                        Value::Small(_) | Value::Huge(_) | Value::Flag(_) => argument,
                        _ => self.ask_special(&argument, 43, &[])?.ok_or_else(|| format!("TypeError: '{}' object cannot be interpreted as an integer", argument.kind_word()))?,
                    };
                    whole.as_big()?.to_i32().ok_or_else(|| String::from("OverflowError: Python int too large to convert to C int"))?
                };
                if which == Some(12) {
                    if let Some(reduction) = self.ask_special(&first, 79, &[])? { return Ok(reduction); }
                    if let Value::Thing(owner) = &first {
                        if self.is_fault_kind(&owner.blueprint()) {
                            let method = self.table.single("ext.builtin.exceptions.reduce").unwrap_or_default().to_owned();
                            return self.fault_method(owner.clone(), &method, &[]);
                        }
                    }
                }
                self.reduction_permitted(&first)?;
                let native = Self::underlying(&first).unwrap_or_else(|| first.settled());
                if matches!(native.settled(), Value::Set(_)) { return self.reduction_of_set(&first); }
                return self.reduction_for_object(&first, version);
            }
            Some(13)=>Value::Small(match &first{Value::Thing(t) if t.blueprint().name!=self.detail("root")=>24,_=>16}),
            _=>return Err(self.class_unready()),
        })
    }
    /// What a thing holds of its own as a dictionary, or nothing where
    /// it holds nothing.
    pub(super) fn reduction_of_set(&mut self, value: &Value) -> Res<Value> {
        let class = self.class_from_type(vec![value.clone()])?;
        let items = self.core_collect(value)?;
        let state = match self.read_class_member(value.clone(), "__getstate__", false) {
            Ok(method) => self.apply_class_member(method, vec![])?,
            Err(fault) if self.missing_member_escape(&fault) => Self::held_as_state(value),
            Err(fault) => return Err(fault),
        };
        Ok(Value::tuple(vec![class, Value::tuple(vec![Value::Vector(crate::tuples::Sequence::plain(items))]), state]))
    }
    fn reduction_for_object(&mut self, value: &Value, version: i32) -> Res {
        self.reduction_permitted(value)?;
        let copyreg = self.load_namespace("copyreg")?;
        if version <= 1 {
            let operation = self.read_class_member(copyreg, "_reduce_ex", false)?;
            return self.apply_class_member(operation.settled(), vec![value.clone(), Value::Small(version as i64)]);
        }
        let mut positional = Vec::new();
        let mut named = Value::Dict(Rc::new(Vec::new().into()));
        let mut extension = false;
        let extra = self.read_class_member(value.clone(), "__getnewargs_ex__", true);
        match extra {
            Err(ref failed) if self.missing_member_escape(failed) => (),
            Err(failed) => return Err(failed),
            Ok(callable) => {
                let pair = self.apply_class_member(callable, vec![])?.settled();
                let Value::Tuple(parts) = pair else { return Err(format!("TypeError: __getnewargs_ex__ should return a tuple, not '{}'", pair.kind_word()).into()); };
                if parts.len() != 2 { return Err(format!("ValueError: __getnewargs_ex__ should return a tuple of length 2, not {}", parts.len()).into()); }
                let Value::Tuple(args) = parts[0].settled() else { return Err(format!("TypeError: first item of the tuple returned by __getnewargs_ex__ must be a tuple, not '{}'", parts[0].kind_word()).into()); };
                named = parts[1].settled();
                if !matches!(named, Value::Dict(_)) { return Err(format!("TypeError: second item of the tuple returned by __getnewargs_ex__ must be a dict, not '{}'", named.kind_word()).into()); }
                positional.extend(args.iter().cloned()); extension = true;
            }
        }
        if !extension {
            match self.read_class_member(value.clone(), "__getnewargs__", true) {
                Err(ref absent) if self.missing_member_escape(absent) => (),
                Err(absent) => return Err(absent),
                Ok(callable) => {
                    let obtained = self.apply_class_member(callable, vec![])?.settled();
                    if let Value::Tuple(args) = obtained { positional.extend(args.iter().cloned()); }
                    else { return Err(format!("TypeError: __getnewargs__ should return a tuple, not '{}'", obtained.kind_word()).into()); }
                }
            }
        }
        let kind = self.class_from_type(vec![value.clone()])?;
        let (helper, args) = match &named {
            Value::Dict(entries) if !entries.is_empty() => ("__newobj_ex__", vec![kind, Value::tuple(positional), named]),
            _ => { positional.insert(0, kind); ("__newobj__", positional) }
        };
        let maker = self.read_class_member(copyreg, helper, false)?;
        let state = match self.read_class_member(value.clone(), "__getstate__", false) {
            Ok(getter) => self.apply_class_member(getter, vec![])?,
            Err(ref missing) if self.missing_member_escape(missing) => Self::held_as_state(value),
            Err(missing) => return Err(missing),
        };
        let underlying = Self::underlying(value).unwrap_or_else(|| value.settled());
        let row = if matches!(underlying.settled(), Value::Vector(_)) {
            self.core_primitive(Prim::Iterator, "iter", vec![value.clone()], vec![])?
        } else { Value::Nil };
        let mapping = if matches!(underlying.settled(), Value::Dict(_)) {
            let member = self.read_class_member(value.clone(), "items", false)?;
            let pairs = self.apply_class_member(member, vec![])?;
            self.core_primitive(Prim::Iterator, "iter", vec![pairs], vec![])?
        } else { Value::Nil };
        Ok(Value::tuple(vec![maker.settled(), Value::tuple(args), state, row, mapping]))
    }
    pub(super) fn held_as_state(value:&Value)->Value {
        let Value::Thing(t)=value else{return Value::Nil};
        let pairs:Vec<(Value,Value)>=t.holds.borrow().iter().filter(|(k,v)|!k.starts_with('\0')&&!matches!(v,Value::Unset)).map(|(k,v)|(Value::text(k),v.clone())).collect();
        let ordinary = if pairs.is_empty() { Value::Nil } else { Value::Dict(Rc::new(pairs.into())) };
        let mut slotted = Vec::new();
        for (key, held) in t.holds.borrow().iter() {
            if let Some(named) = key.strip_prefix("\0slot:").and_then(|rest| rest.rsplit_once(':')) {
                if !matches!(held, Value::Unset) { slotted.push((Value::text(named.0), held.clone())); }
            }
        }
        if slotted.is_empty() { ordinary } else { Value::tuple(vec![ordinary, Value::Dict(Rc::new(slotted.into()))]) }
    }
    /// One of a routine's own readings that must answer the selfsame
    /// object on every asking -- its name, full name and module -- put
    /// where the program's own writes to them go, so the next read
    /// finds it and a later write writes over it.
    /// The name of the module a routine was written in: the module the
    /// file it came from was read as, or the run's own name where the
    /// routine is the program itself.
    /// The module a blueprint being drawn up belongs to: the module the
    /// routine whose activation stands around the drawing was written
    /// in. The program's own activations answer as the program.
    pub(super) fn routine_home(&self, code: &Routine) -> String {
        code.written_in.as_ref().and_then(|place| self.loaded_spaces.get(place)).cloned()
            .unwrap_or_else(|| self.detail("main").to_owned())
    }

    /// The module a routine answers to as a function, as the reference
    /// answers it of the function's own: the module an explicit write
    /// left on the routine, the name the namespace a routine framed by
    /// hand was made in gave itself (nothing where the namespace named
    /// none), or the module the file it came of was read as.
    pub(super) fn routine_module(&self, code: &Rc<Routine>) -> Value {
        let apart = format!("{}\0", self.detail("module"));
        let kept = self.routine_members.iter().find(|(held, _)| matches!(held.revive(), Some(Value::Routine(r) | Value::Bound(r, _) | Value::Method(r, _, _)) if Rc::ptr_eq(&r, code)));
        if let Some((_, members)) = kept {
            if let Some((_, v)) = members.holds.borrow().iter().find(|(k, _)| **k == apart) { return v.clone(); }
        }
        if code.globe.is_some() || code.definition.is_some() {
            // The name a routine was made beside was caught when it was
            // made, and no later change of that namespace moves it.
            return match &code.framed_in { Some(named) => Value::text(named), None => Value::Nil };
        }
        Value::text(&self.routine_home(code))
    }
    /// The row of type parameters a routine was declared with: one
    /// holder per name the declaration wrote, made by the hinting
    /// module's own maker, in the order the names were written. No
    /// names written, an empty row.
    fn routine_type_row(&mut self, code: &Routine) -> Res {
        let mut items = Vec::new();
        if !code.type_params.is_empty() {
            let maker = self.hint_maker()?;
            for name in &code.type_params {
                items.push(self.apply_class_member(maker.clone(), vec![Value::text(name)])?);
            }
        }
        Ok(Value::tuple(items))
    }
    /// The maker of type-parameter names: the hinting module's own
    /// maker, from the module the program already holds where it holds
    /// one, and read in from the library where it holds none.
    fn hint_maker(&mut self) -> Res {
        self.type_support_namespace();
        Ok(Value::Blueprint(self.native_kind("TypeVar")))
    }
    fn routine_kept(&mut self, value: &Value, key: &str, fresh: Value) -> Value {
        let at=self.routine_storage(value);
        let apart=format!("{key}\0");
        if let Some(held)=self.routine_members[at].1.holds.borrow().iter().find(|(k,_)|*k==apart).map(|(_,v)|v.clone()){return held;}
        self.routine_members[at].1.holds.borrow_mut().push((apart,fresh.clone()));
        fresh
    }
    pub(super) fn routine_storage(&mut self, code: &Value) -> usize {
        self.constructor_records.borrow_mut().clear();
        match self.routine_members.iter().position(|(candidate, _)| candidate.revive().is_some_and(|key| key.equals(code))) {
            Some(found) => found,
            None => {
                // A new namespace can change only this routine's lookup.
                // Keep unrelated negative records: clearing them all makes
                // ordinary calls rescan every function after each definition.
                // The collector still clears the cache when it moves records.
                if let Value::Routine(body) | Value::Bound(body, _) = code {
                    self.constructor_records.borrow_mut().remove(&(Rc::as_ptr(body) as usize));
                }
                let of = self.common_ancestor(); self.made += 1;
                let holder = Rc::new(Thing {reclassified: RefCell::new(None),  of, turn: self.made, holds: RefCell::new(Vec::new()) });
                let entry = (crate::ghost::ghost_of(code).expect("routine is weakly held"), holder);
                // Reuse a dead function's slot without moving live indices;
                // a caller can keep an index while annotation code reenters.
                if let Some(vacant) = self.routine_members.iter().position(|(key, _)| key.departed()) {
                    self.routine_members[vacant] = entry;
                    vacant
                } else {
                    self.routine_members.push(entry);
                    self.routine_members.len() - 1
                }
            }
        }
    }
    /// A blueprint's annotations: its own, worked out from the routines
    /// its body kept the first time they are asked for and held after
    /// that. A body that annotated nothing leaves an empty map of its
    /// own, and a parent's annotations never come down.
    pub(super) fn blueprint_annotations(&mut self,b:&Rc<Blueprint>)->Res {
        let word=self.rules.words_ext_stmt_class_annotations.first().map(String::as_str).unwrap_or_default().to_owned();
        if let Some(own)=Self::own_entry(b,&word){return Ok(own);}
        let title = self.rules.words_ext_stmt_class_detail_code_fields.get(10).cloned().unwrap_or_default();
        let made = match Self::own_entry(b, &title).or_else(|| Self::own_entry(b, "__annotate_func__")).map(|entry| entry.settled()) {
            Some(Value::Nil) => self.collection_cell(Value::Dict(Rc::new(Vec::new().into()))),
            Some(evaluator) => self.apply_class_member(evaluator, vec![Value::Small(1)])?,
            None => self.resolve_blueprint_annotations(b)?,
        };
        b.shared.borrow_mut().push((word, made.clone()));
        Ok(made)
    }
    fn resolve_blueprint_annotations(&mut self, b: &Rc<Blueprint>) -> Res {
        let mut pairs=Vec::new();
        // The entry is kept in a cell, as the blueprint's members are; keys
        // and routines alternate along it.
        if let Some(Value::Vector(row))=Self::own_entry(b,crate::data::ANNOTATE_WORD).map(|kept|kept.settled()) {
            for pair in row.chunks(2) {
                let [key,worth]=pair else{break};
                let worth = worth.settled();
                if matches!(worth, Value::Unset) { continue; }
                let value=match worth {Value::Routine(_)|Value::Bound(..)=>self.apply_class_member(worth,Vec::new())?,other=>other};
                pairs.push((key.clone(),value));
            }
        }
        let made=Value::Dict(Rc::new(pairs.into()));
        Ok(self.collection_cell(made))
    }
    /// A read that ends with the member missing -- the blueprint's own
    /// reading hook having said so, or a property's getter, or nothing
    /// found -- goes to the fallback reader before it is reported. A
    /// direct read, the root's own, has none.
    /// The reference's own docstring for a native kind's word, where
    /// this runtime keeps one. Nothing for a word left undocumented,
    /// which reads as the kind having no attribute of that name.
    pub(super) fn builtin_kind_doc(word:&str)->Option<&'static str> {
        match word {
            "enumerate" => Some("Return an enumerate object.\n\n  iterable\n    an object supporting iteration\n\nThe enumerate object yields pairs containing a count (from start, which\ndefaults to zero) and a value yielded by the iterable argument.\n\nenumerate is useful for obtaining an indexed list:\n    (0, seq[0]), (1, seq[1]), (2, seq[2]), ..."),
            "reversed" => Some("Return a reverse iterator over the values of the given sequence."),
            _ => None,
        }
    }
    pub(super) fn clock_record_type(&mut self) -> Rc<Blueprint> {
        for (spelling, class) in &self.native_kinds { if spelling == "struct_time" { return class.clone(); } }
        let parent=self.native_kind("tuple");
        let mut ranks=parent.ancestry.borrow().clone(); ranks.insert(0,parent.clone());
        let mut members=vec![("__module__".to_string(),Value::text("time")),("__new__".to_string(),self.native_allocation("struct_time"))];
        for (name, tag) in [("__repr__",233),("__str__",233),("__reduce__",234)] { members.push((name.into(),Self::wrap(tag,Vec::new()))); }
        for (name, count) in [("n_sequence_fields",9),("n_fields",11),("n_unnamed_fields",0)] { members.push((name.into(),Value::Small(count))); }
        let result=Rc::new(Blueprint { name:String::from("struct_time"),presentation:Some(String::from("<class 'time.struct_time'>")),under:Some(parent.clone()),parents:vec![parent],ancestry:RefCell::new(ranks),answers:Vec::new(),fields:Vec::new(),reaches:Vec::new(),methods:Vec::new(),constants:vec![("\0native".into(),Value::text("struct_time"))],shared:RefCell::new(members),weak_slot:Cell::new(None),has_slot_storage:true,sealed:Cell::new(true),order_supplied:Cell::new(false),supplied_order:RefCell::new(Vec::new()),type_names:RefCell::new(None) });
        self.native_kinds.push(("struct_time".into(),result.clone())); result
    }
    pub(super) fn clock_record(&mut self, target:Value, parameters:Vec<Value>) -> Res {
        let Value::Blueprint(class)=target else{return Err("TypeError: struct_time.__new__(X): X is not a type object".to_owned().into())};
        let (ordered,named)=self.open_arguments(parameters)?;
        if ordered.len()>2{return Err("TypeError: struct_time() takes at most 2 arguments".to_owned().into())}
        let mut source=ordered.get(0).cloned();let mut attributes=ordered.get(1).cloned();
        for (name,held) in named { match name.as_str(){"sequence"=>source=Some(held),"dict"=>attributes=Some(held),_=>return Err(format!("TypeError: '{name}' is an invalid keyword argument for struct_time()").into())} }
        let source=source.ok_or_else(|| String::from("TypeError: struct_time() missing required argument 'sequence' (pos 1)"))?;
        let mut row=self.core_collect(&source)?;
        if row.len()<9 {return Err(format!("TypeError: time.struct_time() takes an at least 9-sequence ({}-sequence given)",row.len()).into())}
        if row.len()>11 {return Err(format!("TypeError: time.struct_time() takes an at most 11-sequence ({}-sequence given)",row.len()).into())}
        let attributes=attributes.unwrap_or_else(||Value::Dict(Rc::new(Vec::new().into())));
        let attributes=Self::underlying(&attributes).unwrap_or(attributes).settled();
        let Value::Dict(attributes)=attributes else{return Err("TypeError: time.struct_time() takes a dict as second arg, if any".to_owned().into())};
        let words=["tm_year","tm_mon","tm_mday","tm_hour","tm_min","tm_sec","tm_wday","tm_yday","tm_isdst","tm_zone","tm_gmtoff"];
        for name in &words[row.len()..] {row.push(attributes.iter().find_map(|(key,value)|matches!(key,Value::Text(text) if text.as_ref()==*name).then(||value.clone())).unwrap_or(Value::Nil));}
        let mut holds=vec![("\0underlying".to_owned(),Value::tuple(row[..9].to_vec()))];
        holds.extend(words.into_iter().zip(row).map(|(name,held)|(name.into(),held)));
        self.made+=1;Ok(Value::Thing(Rc::new(Thing {of:class,reclassified:RefCell::new(None),holds:RefCell::new(holds),turn:self.made})))
    }
    fn display_clock_record(&mut self, instance:&Value) -> Res {
        let keys=["tm_year","tm_mon","tm_mday","tm_hour","tm_min","tm_sec","tm_wday","tm_yday","tm_isdst"];
        let mut text=String::from("time.struct_time(");
        for name in keys {if !text.ends_with('('){text.push_str(", ");}text.push_str(name);text.push('=');let value=self.read_class_member(instance.clone(),name,false)?;text.push_str(&self.object_words(&value,true)?);}
        text.push(')');Ok(Value::text(&text))
    }
    fn reduce_clock_record(&mut self, instance:&Value) -> Res {
        let entries=Self::underlying(instance).ok_or_else(||String::from("TypeError: invalid struct_time receiver"))?;
        let zone=self.read_class_member(instance.clone(),"tm_zone",false)?;let offset=self.read_class_member(instance.clone(),"tm_gmtoff",false)?;
        let details=Value::Dict(Rc::new(vec![(Value::text("tm_zone"),zone),(Value::text("tm_gmtoff"),offset)].into()));
        let kind=Value::Blueprint(self.clock_record_type());Ok(Value::tuple(vec![kind,Value::tuple(vec![entries,details])]))
    }
    fn native_allocation(&self, spelling: &str) -> Value {
        let mut saved = self.loose_members.borrow_mut();
        saved.entry(format!("allocate {spelling}")).or_insert_with(|| Self::wrap(14, vec![Value::text(spelling)])).clone()
    }
    pub(super) fn integer_attribute(&self, value: &Value, key: &str) -> Option<Value> {
        if let Value::Intrinsic(Prim::Truthful, title) = value {
            if key == self.detail("allocate") {
                return Some(Self::wrap(14, vec![Value::text(title)]));
            }
        }
        let words = self.table.strings("ext.stmt.class.detail.integer.layout");
        if words.len() < 7 { return None; }
        let mut derived = false;
        let is_type = match value {
            Value::Intrinsic(Prim::AsInt | Prim::Truthful, _) => true,
            Value::Blueprint(b) => {
                let primitive = Self::native_beneath(b)?;
                if self.table.prims.get(&primitive) != Some(&Prim::AsInt) { return None; }
                derived = Self::native_word(b).is_none();
                true
            }
            Value::Thing(_) => {
                if !matches!(Self::underlying(value)?, Value::Small(_) | Value::Huge(_)) { return None; }
                false
            }
            Value::Small(_) | Value::Huge(_) | Value::Flag(_) => false,
            _ => return None,
        };
        if !is_type && self.table.spells("ext.builtin.bytes.from_int",key) {
            return Some(Value::Member(Rc::new(value.clone()),"integer_bytes".into()));
        }
        if self.table.strings("ext.builtin.bytes.to_int").iter().any(|word| word.rsplit('.').next() == Some(key)) {
            let owner = if let Value::Thing(instance) = value { Value::Blueprint(instance.blueprint().clone()) }
                else if is_type { value.clone() }
                else { Value::Intrinsic(Prim::AsInt, Rc::from(self.table.strings("builtin.to_int").first().map(String::as_str)?)) };
            return Some(Value::Member(Rc::new(owner), "integer_from_bytes".into()));
        }
        let at = words.iter().take(3).position(|word| word == key)?;
        if at == 2 { return Some(Value::Member(Rc::new(value.clone()), "integer_size".into())); }
        if !is_type { return None; }
        let slot = if at == 1 { 4 } else if derived { 6 } else { 3 };
        words[slot].parse().ok().map(Value::Small)
    }
    pub(super) fn read_class_member(&mut self,value:Value,key:&str,direct:bool)->Res {
        if self.spells_property_kind(&value) { let owner = self.property_blueprint(); return self.read_class_member(Value::Blueprint(owner), key, direct); }
        let value = if !self.rules.words_ext_stmt_class_builder.is_empty() { value.settled() } else { value };
        if self.names_in_calls && !key.starts_with("__") {
            let slot = match &value {
                Value::Thing(thing) => {
                    let blueprint = thing.blueprint();
                    let has_slots = blueprint.has_slot_storage || blueprint.ancestry.borrow().iter().any(|base| base.has_slot_storage);
                    if has_slots && (direct || self.inherited_entry(&blueprint, self.detail("get")).is_none()) {
                        self.inherited_entry(&blueprint, key)
                    } else { None }
                }
                _ => None,
            };
            if let Some(Value::Wrapped(32, layout)) = slot {
                if let Ok(stored) = self.slot_value(&value, &layout) { return Ok(stored); }
            }
        }
        if let Value::Thing(item) = &value {
            let replace = self.table.strings("ext.stmt.class.detail.code.replace").first();
            if replace.is_some_and(|word| word == key) && self.code_kind.as_ref().is_some_and(|kind| Rc::ptr_eq(kind, &item.blueprint())) {
                return Ok(Value::Member(Rc::new(value.clone()), String::from("code_replace")));
            }
        }
        if let Value::Wrapped(122, fields) = &value {
            let selected = match key {
                "__code__" => Some(fields[0].clone()),
                "__globals__" => Some(fields[1].clone()),
                "__name__" => Some(fields[2].clone()),
                "__qualname__" => Some(Value::text("<module>")),
                "__defaults__" => Some(fields[3].clone()),
                "__closure__" => Some(fields[4].clone()),
                "__kwdefaults__" => Some(Value::Nil),
                _ => None,
            };
            if let Some(attribute) = selected { return Ok(attribute); }
        }
        if key == self.detail("doc") && matches!(value.settled(), Value::Nil) {
            return Ok(Value::text("The type of the None singleton."));
        }

        if key == self.detail("allocate") && matches!(value.settled(), Value::Nil | Value::Ellipsis | Value::Refusal(_)) {
            let owner = self.kind_named_after(&value.settled());
            return self.read_class_member(owner, key, direct);
        }

        match &value {
            Value::Intrinsic(Prim::ValueMethod, spelling) if self.table.spells("ext.builtin.method.getformat", spelling) && key == self.detail("name") => {
                return Ok(Value::text(spelling.rsplit('.').next().unwrap_or(spelling)));
            }
            _ => {}
        }

        if let Value::TextCall { subject, name, .. } = &value {
            if key == self.detail("receiver") { return Ok(Value::Text(subject.clone())); }
            if key == self.detail("name") { return Ok(Value::Text(name.clone())); }
            if key == self.detail("qualified") { return Ok(Value::text(&["str", name.as_ref()].join("."))); }
        }

        if let Value::Wrapped(143, payload) = value.settled() {
            for (word, position) in [("code", 0), ("globals", 1), ("name", 2), ("qualified", 2)] {
                if key == self.detail(word) { return Ok(payload[position].clone()); }
            }
            if key == self.detail("kind") { return Ok(Value::Blueprint(self.native_kind("function"))); }
            if key == self.detail("call") { return Ok(value.clone()); }
            if ["defaults", "keywords", "closure"].iter().any(|word| key == self.detail(word)) { return Ok(Value::Nil); }
        }

        if let Value::Wrapped(180, parts) = &value {
            if ["descriptor.get", "descriptor.set", "descriptor.delete"].iter().any(|field| self.detail(field) == key) {
                return Ok(Self::wrap(181, vec![value.clone(), Value::text(key)]));
            }
            if key == self.detail("name") { return Ok(parts[0].clone()); }
        }

        let value = if !self.rules.words_ext_stmt_class_builder.is_empty() { value.settled() } else { value };
        if !self.rules.words_ext_builtin_weak_get.is_empty() {
            if let Value::Thing(t) = value.settled() {
                if ["ProxyType", "CallableProxyType"].contains(&t.blueprint().name.as_str()) {
                    let held = t.holds.borrow().iter().find_map(|(name, v)| (name == "\0weak").then(|| v.clone()));
                    if let Some(Value::Dim(reference)) = held {
                        match reference.ghost.revive() {
                            Some(target) => return self.read_class_member(target, key, direct),
                            None => return Err("ReferenceError: weakly-referenced object no longer exists".to_owned().into()),
                        }
                    }
                }
            }
        }

        if let Value::Wrapped(134, kept) = &value {
            if key == self.detail("module") { return self.read_class_member(kept[1].clone(), key, direct); }
            if self.table.strings("ext.stmt.class.special").get(79).is_some_and(|word| word == key) { return Ok(Self::wrap(135, vec![kept[0].clone()])); }
        }
        match &value {
            Value::Wrapped(133|134,parts) if matches!(key,"__doc__"|"__qualname__"|"__name__")=>{
                let original=parts[1].clone();
                return self.read_class_member(original,key,direct);
            }
            _=>{},
        }
        if key.as_bytes().first()==Some(&0) {
            if let Value::Thing(instance)=&value {
                if instance.holds.borrow().iter().any(|entry|entry.0=="\0immutable"){return Err(self.absent_attribute(&value,key));}
            }
        }
        if let Value::Wrapped(9, binding) = &value {
            if let [Value::Blueprint(defining), instance] = binding.as_slice() {
                let actual = match instance {
                    Value::Thing(t) => t.blueprint().clone(),
                    Value::Blueprint(b) => {
                        let belongs = Rc::ptr_eq(b, defining) || b.ancestry.borrow().iter().any(|parent| Rc::ptr_eq(parent, defining));
                        if belongs { b.clone() } else { Self::builder_over(b).unwrap_or_else(|| b.clone()) }
                    }
                    _ => return Err(self.class_unready()),
                };
                if let Some(found) = self.parent_walk(defining, instance, actual, key)? { return Ok(found); }
                return Err(self.absent_attribute(&value, key));
            }
        }
        // A thing of the parent class reads a member past the class it
        // was made against: the same walk the parent proxy takes, over
        // the line of the class the thing it stands on answers as. What
        // the walk never finds, the plain reading of the thing answers,
        // the very way the reference has a name the classes past know
        // nothing of fall to the parent thing's own members.
        if let Value::Thing(t) = &value {
            if self.has_class_order() && Self::parent_kind_descended(&t.blueprint()) && key != self.detail("kind") {
                let (owner, stands_on, answers_as) = {
                    let holds = t.holds.borrow();
                    (holds.iter().find(|(name, _)| name == "__thisclass__").map(|(_, held)| held.clone()),
                     holds.iter().find(|(name, _)| name == "__self__").map(|(_, held)| held.clone()),
                     holds.iter().find(|(name, _)| name == "\0objtype").map(|(_, held)| held.clone()))
                };
                if key == "__self_class__" { return Ok(answers_as.unwrap_or(Value::Nil)); }
                if matches!(key, "__thisclass__" | "__self__") && owner.is_none() { return Ok(Value::Nil); }

                if let (Some(owner), Some(receiver), Some(actual)) = (self.parent_blueprint_of(owner), stands_on, self.parent_blueprint_of(answers_as)) {
                    if !matches!(receiver, Value::Nil) {
                        if let Some(found) = self.parent_walk(&owner, &receiver, actual, key)? { return Ok(found); }
                    }
                }
            }
        }
        let sought=self.seek_class_member(value.clone(),key,direct);
        if direct{return sought;}
        let Err(escape) = &sought else { return sought };
        if !self.missing_member_escape(escape) { return sought; }
        if let Value::Blueprint(kind) = &value {
            if let Some(builder) = Self::builder_over(kind) {
                if let Some(entry) = self.table.strings("ext.stmt.class.reader").first().map(String::as_str).and_then(|key| self.inherited_entry(&builder, key)) {
                    let bound = self.member_binding(entry, Some(value.clone()), builder)?;
                    return self.apply_class_member(bound, vec![Value::text(key)]).map_err(|escaped| self.explain_absence(escaped, &value, key));
                }
            }
            return sought;
        }
        let Value::Thing(t) = &value else { return sought };
        let Some(fallback)=self.table.strings("ext.stmt.class.reader").first().map(String::as_str).and_then(|n|self.inherited_entry(&t.blueprint(),n)) else{return sought};
        self.sought_in_vain.take();
        let bound=self.member_binding(fallback,Some(value.clone()),t.blueprint().clone())?;
        self.apply_class_member(bound,vec![Value::text(key)]).map_err(|escaped| self.explain_absence(escaped, &value, key))
    }
    fn seek_class_member(&mut self,value:Value,key:&str,direct:bool)->Res {
        if self.names_in_calls && !key.starts_with("__") {
            if let Value::Thing(instance) = &value {
                let class = instance.blueprint();
                let ordinary = Self::native_word(&class).is_none() && Self::native_beneath(&class).map_or(true, |base| base == self.detail("root"));
                if ordinary {
                    let attributes = instance.holds.borrow();
                    let isolated = attributes.iter().all(|(word, _)| !word.starts_with('\0'));
                    let stored = isolated.then(|| attributes.iter().find(|(word, _)| word == key)
                        .map(|(_, worth)| worth.clone())).flatten();
                    drop(attributes);
                    if let Some(stored) = stored {
                        let intercepts = !direct && self.inherited_entry(&class, self.detail("get")).is_some();
                        let data = self.inherited_entry(&class, key).is_some_and(|entry| self.writes_too(&entry));
                        if !intercepts && !data { return Ok(stored); }
                    }
                }
            }
        }
        let defining = match &value {
            Value::Wrapped(14, fields) => self.kind_by_word(&fields[0].bare()),
            Value::Wrapped(1, fields) if fields.is_empty() => Some(Value::Blueprint(self.common_ancestor())),
            Value::Wrapped(40, _) => Some(self.kind_builder_word()),
            _ => None,
        };
        if let Some(kind) = defining {
            match key {
                "__self__" => return Ok(kind),
                "__name__" => return Ok(Value::text("__new__")),
                "__qualname__" => { let title = self.read_class_member(kind.clone(), "__name__", false)?.bare(); return Ok(Value::text(&[title, "__new__".into()].join("."))); }
                "__module__" => return Ok(Value::Nil),
                "__reduce__" => { let get = Value::Intrinsic(Prim::GetMember, Rc::from("getattr")); return Ok(Self::wrap(135, vec![Value::tuple(vec![get, Value::tuple(vec![kind, Value::text("__new__")])])])); }
                "__reduce_ex__" => return Ok(Value::Member(Rc::new(value), String::from("callable_reduce_ex"))),
                _ => (),
            }
        }
        if key == self.detail("allocate") && matches!(value.settled(), Value::Small(_) | Value::Huge(_) | Value::Frac(_) | Value::Complex(_) | Value::Text(_) | Value::Unpaired(_) | Value::Vector(_) | Value::Dict(_) | Value::Tuple(_) | Value::Octets { .. } | Value::Set(_)) {
            let class = self.kind_named_after(&value.settled());
            return self.read_class_member(class, key, direct);
        }
        if key == "__reduce__" {
            let parts = match &value {
                Value::Method(code, receiver, _) => Some((Value::Routine(code.clone()), Value::Thing(receiver.clone()))),
                Value::Wrapped(3, entries) if matches!(entries.first(), Some(Value::Routine(_) | Value::Bound(..) | Value::Intrinsic(Prim::Textual(_), _))) => Some((entries[0].clone(), entries[1].clone())),
                _ => None,
            };
            if let Some((function, receiver)) = parts {
                let name = self.read_class_member(function, self.detail("name").to_owned().as_str(), false)?;
                let getter = Value::Intrinsic(Prim::GetMember, Rc::from("getattr"));
                return Ok(Self::wrap(135, vec![Value::tuple(vec![getter, Value::tuple(vec![receiver, name])])]));
            }
        }
        if key == self.detail("allocate") && matches!(&value, Value::Intrinsic(op, _) if !Self::names_a_kind(op)) { return Ok(self.common_allocation()); }
        if key == "__reduce_ex__" && matches!(&value, Value::Intrinsic(working, _) if !Self::names_a_kind(working)) || key == "__reduce_ex__" && matches!(&value, Value::Member(..) | Value::TextCall { .. } | Value::Method(..) | Value::Wrapped(3, _)) {
            return Ok(Value::Member(Rc::new(value), "callable_reduce_ex".into()));
        }
        let raw_descriptor = matches!(&value, Value::Wrapped(4 | 5, _)) || matches!(&value, Value::Wrapped(60, parts) if parts.len() == 3);
        if raw_descriptor {
            if let Some(slot) = self.table.strings("ext.stmt.class.special").iter().position(|word| word == key).filter(|slot| [79, 81].contains(slot)) {
                let operation = if slot == 79 { "descriptor_reduce" } else { "descriptor_reduce_ex" };
                return Ok(Value::Member(Rc::new(value.clone()), operation.to_owned()));
            }
        }
        if self.table.single("ext.stmt.class.constructor") == Some(key) && matches!(&value, Value::Blueprint(class) if self.ancestor.as_ref().is_some_and(|root| Rc::ptr_eq(root, class))) { return Ok(self.common_initialiser()); }
        if key == "__getformat__" {
            let class = match value.settled() {
                Value::Intrinsic(Prim::AsReal, _) => Some(value.clone()),
                Value::Frac(_) => self.kind_by_word("float"),
                Value::Blueprint(kind) if Self::native_word(&kind).or_else(|| Self::native_beneath(&kind)).as_deref() == Some("float") => Some(value.clone()),
                Value::Thing(thing) if Self::native_beneath(&thing.blueprint()).as_deref() == Some("float") => Some(Value::Blueprint(thing.blueprint())),
                _ => None,
            };
            if let Some(class) = class { return Ok(Value::Member(Rc::new(class), String::from("float_getformat"))); }
        }
        let binding = match &value {
            Value::Member(receiver, working) => {
                let public = match working.as_str() {
                    "classmethod_get" => "__get__", "callable_reduce_ex" => "__reduce_ex__", "integer_size" => "__sizeof__",
                    "integer_bytes" => "to_bytes", "integer_from_bytes" => "from_bytes",
                    "complex_from_number" | "float_from_number" => "from_number",
                    "bytearray_fromhex" | "bytes_fromhex" | "float_fromhex" => "fromhex",
                    "float_getformat" => "__getformat__", other => other,
                };
                Some((receiver.as_ref().clone(), public.to_string()))
            },
            Value::TextCall { subject, name, .. } => Some((Value::Text(subject.clone()), name.to_string())),
            Value::Wrapped(3, parts) => match parts.first() {
                Some(Value::Wrapped(60, descriptor)) => Some((parts[1].clone(), descriptor[1].bare())),
                Some(Value::Intrinsic(_, label)) => Some((parts[1].clone(), label.rsplit('.').next().unwrap_or(label).to_string())),
                _ => None,
            },
            _ => None,
        };
        if let Some((receiver, word)) = binding {
            if key == self.detail("name") { return Ok(Value::text(&word)); }
            if key == self.detail("receiver") { return Ok(receiver); }
            if key == self.detail("qualified") {
                let defining = if matches!(&receiver, Value::Blueprint(_)) { self.read_class_member(receiver.clone(), key, false)?.bare() } else if let Value::Intrinsic(_, spelling) = &receiver { spelling.to_string() } else { receiver.kind_word() };
                return Ok(Value::text(&format!("{}.{}", defining, word)));
            }
            if key == self.detail("module") { return Ok(Value::Nil); }
            if self.table.strings("ext.stmt.class.special").get(79).is_some_and(|entry| entry == key) {
                let restore = Value::Intrinsic(Prim::GetMember, Rc::from("getattr"));
                return Ok(Self::wrap(135, vec![Value::tuple(vec![restore, Value::tuple(vec![receiver, Value::text(&word)])])]));
            }
        }
        if self.table.single("ext.stmt.annotation.adapter") == Some(key) {
            if let Value::Wrapped(44, stored) = &value {
                if stored.len() == 1 {
                    let rows = match &stored[0] {
                        Value::Blueprint(class) => class.shared.borrow().iter().find_map(|(word, held)|
                            (word == crate::data::ANNOTATE_WORD).then(|| held.settled())).unwrap_or_else(|| Value::Vector(crate::tuples::Sequence::plain(Vec::new()))),
                        rows => rows.clone(),
                    };
                    return Ok(rows);
                }
            }
        }
        if let Value::Wrapped(14, arguments) = &value {
            if key == self.detail("receiver") {
                if let Some(owner) = self.kind_by_word(&arguments[0].bare()) { return Ok(owner); }
            }
        }
        if let Value::Wrapped(60, description) = &value {
            if self.table.strings("ext.stmt.class.special").get(79).is_some_and(|entry| entry == key) {
                let class = self.kind_by_word(&description[0].bare()).ok_or_else(|| self.class_unready())?;
                let restore = Value::Intrinsic(Prim::GetMember, Rc::from("getattr"));
                let arguments = Value::tuple(vec![class, description[1].clone()]);
                return Ok(Self::wrap(135, vec![Value::tuple(vec![restore, arguments])]));
            }
        }
        if matches!(&value, Value::Intrinsic(Prim::SinceEpoch, word) if word.as_ref() == "ctime") && key == self.detail("module") {
            return Ok(Value::text("time"));
        }
        match &value {
            Value::Intrinsic(working, spelling) if !working.names_a_kind() && !self.detail("name").is_empty() => {
                if let Some((owner_word, member)) = spelling.rsplit_once('.') {
                    if let Some(kind) = self.kind_by_word(owner_word) {
                        let bound_to_type = ["fromkeys", "fromhex", "from_bytes", "from_number", "__getformat__"].contains(&member);
                        let unbound_static = member == "maketrans";
                        if key == self.detail("receiver") && (bound_to_type || unbound_static) { return Ok(if bound_to_type { kind.clone() } else { Value::Nil }); }
                        if key == "__objclass__" && !(bound_to_type || unbound_static) { return Ok(kind.clone()); }
                        if key == self.detail("module") && (bound_to_type || unbound_static) { return Ok(Value::Nil); }
                        if self.table.strings("ext.stmt.class.special").get(79).is_some_and(|entry| entry == key) {
                            let lookup = Value::Intrinsic(Prim::GetMember, Rc::from("getattr"));
                            let pair = Value::tuple(vec![lookup, Value::tuple(vec![kind, Value::text(member)])]);
                            return Ok(Self::wrap(135, vec![pair]));
                        }
                    }
                }
                if key == self.detail("qualified") { return Ok(Value::text(spelling)); }
                if key == self.detail("name") { return Ok(Value::text(spelling.rsplit('.').next().unwrap_or(spelling))); }
                if key == self.detail("module") { return Ok(Value::text(self.builtin_module())); }
                if self.table.strings("ext.stmt.class.special").get(79).is_some_and(|entry| entry == key) {
                    return Ok(Self::wrap(135, vec![Value::text(spelling)]));
                }
            }
            _ => ()
        }
        if let Value::Intrinsic(Prim::SortOf, title) = &value {
            if key == self.detail("name") || key == self.detail("qualified") { return Ok(Value::text(title)); }
            if key == self.detail("module") { return Ok(Value::text(self.builtin_module())); }
        }
        if let Some(spelling) = self.kind_spelling(&value) {
            if key == self.detail("name") || key == self.detail("qualified") { return Ok(Value::text(&spelling)); }
            if key == self.detail("module") { return Ok(Value::text(self.builtin_module())); }
        }
        if matches!(&value, Value::Wrapped(62, _)) {
            if let Some(method) = self.attribute(&value, key) { return Ok(method); }
        }
        if matches!(&value, Value::Wrapped(235, _)) {
            let metadata = if key == self.detail("name") { Some(Value::text("__get__")) }
                else if key == self.detail("qualified") { Some(Value::text("classmethod_descriptor.__get__")) }
                else if key == "__objclass__" { Some(Value::Blueprint(self.native_kind("classmethod_descriptor"))) }
                else { None };
            if let Some(answer) = metadata { return Ok(answer); }
        }
        if let Value::Wrapped(60, parts) = &value {
            if let [Value::Text(kind), Value::Text(word), ..] = parts.as_slice() {
                if parts.len() == 3 && key == self.detail("descriptor.get") { return Ok(Value::Member(Rc::new(value.clone()), String::from("classmethod_get"))); }
                if key==self.detail("qualified") { return Ok(Value::text(&format!("{}.{}", kind, word))); }
                if key==self.detail("name") { return Ok(Value::text(word)); }
                if key=="__objclass__" {
                    if let Some(owner)=self.kind_by_word(kind) { return Ok(owner); }
                }
            }
        }
        if let Some(member) = self.activation_member(&value, key) { return Ok(member); }
        if matches!(&value, Value::Backtrace(_)) {
            if let Some(directory) = self.directory_attribute(&value, key) { return Ok(directory); }
        }
        if let Value::Backtrace(link) = &value {
            let names = self.rules.trace_words;
            if names.get(1).map_or(false, |n| n == key) { return Ok(Value::Small(link.location)); }
            if names.get(2).map_or(false, |n| n == key) { return Ok(link.following.borrow().clone()); }
            if names.get(3).map_or(false, |n| n == key) { return Ok(Value::Thing(link.activation.clone())); }
            if names.get(26).map_or(false, |n| n == key) { return Ok(Value::Small(link.instruction)); }
            for (index, offset) in [(16, link.extent.map_or(Some(link.location as u32), |x| Some(x.2))), (17, link.extent.map(|x| x.1)), (18, link.extent.map(|x| x.3))] {
                if names.get(index).map_or(false, |word| word == key) { return Ok(offset.map(|n| Value::Small(n as i64)).unwrap_or(Value::Nil)); }
            }
            return Err(self.absent_attribute(&value, key));
        }
        if matches!(&value,Value::Wrapped(6,_)) && self.table.spells("ext.stmt.class.property.setter",key) {return Ok(Self::wrap(13,vec![value]));}
        if key==self.detail("namespace") {
            let settled=value.settled();
            let native=match &settled {
                Value::Intrinsic(op,word) if Self::names_a_kind(op)=>Some(word.clone()),
                other=>self.kind_spelling(other),
            };
            if let Some(word)=native {
                if word.as_ref() == "type" {
                    let actual = self.builder_blueprint();
                    return Ok(Value::Window(Rc::new(Value::Blueprint(actual)), 'm'));
                }
                let mut names = self.kind_stand_in(&word).map_or_else(Vec::new, |sample| self.native_directory(&sample));
                if word.as_ref() == "type" { let slots = self.rules.specials; names.extend([8, 17].iter().filter_map(|at| slots.get(*at).cloned())); }
                if word.as_ref() == "dict" { names.extend(self.table.strings("ext.builtin.method.fromkeys").iter().cloned()); }
                let mut pairs = names.into_iter().filter_map(|name| {
                    if word.as_ref() == "dict" && name == "fromkeys" {
                        Some((Value::text(&name), Self::wrap(60, vec![Value::text("dict"), Value::text(&name), Value::Flag(true)])))
                    } else { self.carried_by_kind(&value, &name).map(|descriptor| (Value::text(&name), descriptor)) }
                }).collect::<Vec<_>>();
                if matches!(word.as_ref(), "int" | "float" | "str" | "tuple" | "bytes" | "bytearray" | "dict" | "set" | "frozenset" | "complex" | "list") {
                    pairs.push((Value::text(self.detail("allocate")), Self::wrap(14, vec![Value::text(&word)])));
                }
                // Expose stored Python slots on the public native class dictionary.
                let kind = self.native_kind(&word);
                for (key, member) in kind.shared.borrow().iter() {
                    if !key.starts_with('\0') && !pairs.iter().any(|(name, _)| name.bare() == *key) {
                        pairs.push((Value::text(key), member.clone()));
                    }
                }
                return Ok(Value::Window(Rc::new(Value::Dict(Rc::new(pairs.into()))),'m'));
            }
        }
        // Routines, wrapped routines and slots are members that bind, and
        // read as such; a slot writes and removes besides.
        if let Value::Wrapped(32, parts) = &value {
            if matches!(parts.get(2), Some(Value::Small(-2))) {
                match key {
                    "__objclass__" => return Ok(parts[1].clone()),
                    _ if key == self.detail("name") => return Ok(parts[0].clone()),
                    _ if key == self.detail("doc") => return Ok(Value::text("dictionary for instance variables")),
                    _ if key == self.detail("qualified") => {
                        let Value::Blueprint(owner) = &parts[1] else { return Err(self.class_unready()); };
                        let qualified = self.read_class_member(Value::Blueprint(owner.clone()), key, false)?;
                        return Ok(Value::text(&format!("{}.{}", qualified.bare(), parts[0].bare())));
                    },
                    _ => {},
                }
            }
        }
        if self.protocol_spelled() {
            let binds=matches!(&value,Value::Routine(_)|Value::Bound(..))||matches!(&value,Value::Wrapped(4|5|10|11|12|32|60|124,_))||matches!(&value,Value::Intrinsic(Prim::Textual(_), name) if name.contains('.'));
            if binds && key==self.detail("descriptor.get"){return Ok(Self::wrap(31,vec![value]));}
            if let Value::Wrapped(32,parts)=&value {
                if key==self.detail("descriptor.set"){return Ok(Self::wrap(33,parts.as_ref().clone()));}
                if key==self.detail("descriptor.delete"){return Ok(Self::wrap(34,parts.as_ref().clone()));}
            }
        }
        if matches!(&value, Value::Intrinsic(Prim::SortOf, _))
            || (!self.rules.words_ext_stmt_class_builder.is_empty() && (matches!(&value, Value::Blueprint(b) if self.builds_classes(b) || Self::native_word(b).is_some_and(|word| self.table.prims.get(&word) == Some(&Prim::SortOf)))
                || self.kind_spelling(&value).is_some_and(|word| self.table.prims.get(word.as_ref()) == Some(&Prim::SortOf)))) {
            if self.rules.words_ext_stmt_class_constructor.first().map(String::as_str)==Some(key){return Ok(Self::wrap(73,Vec::new()));}
            let operation = [("allocate", 70), ("call", 71), ("prepare", 72), ("get", 10), ("set", 11), ("remove", 12)]
                .into_iter().find_map(|(part, tag)| (key == self.detail(part)).then_some(tag));
            if let Some(tag) = operation { return Ok(Self::wrap(tag, Vec::new())); }
        }
        if !self.detail("base").is_empty()&&(key==self.detail("base")||key==self.detail("bases")) {
            let blueprint=match &value {
                Value::Blueprint(b)=>Some(b.clone()),
                Value::Intrinsic(Prim::SortOf,_)=>Some(self.builder_blueprint()),
                Value::Intrinsic(op,word) if Self::names_a_kind(op)=>Some(self.native_kind(word)),
                _=>None,
            };
            if let Some(class)=blueprint {
                if let Some(owner) = Self::builder_over(&class) {
                    if let Some(entry) = self.inherited_entry(&owner, key) {
                        if !self.writes_too(&entry) {
                            if let Some(own)=self.inherited_entry(&class,key){return self.member_binding(own,None,class);}
                        }
                        return self.member_binding(entry, Some(value.clone()), owner);
                    }
                }
                if key==self.detail("base"){return Ok(class.under.clone().map_or(Value::Nil,|p|self.visible_blueprint(p)));}
                return Ok(Value::tuple(class.parents.iter().cloned().map(|p|self.visible_blueprint(p)).collect()));
            }
        }
        // A native kind's word read as a class: its maker, and its name.
        if matches!(&value, Value::OctetKind { .. }) {
            if let Some(member) = self.carried_by_kind(&value, key) { return Ok(member); }
        }
        // A row of bytes answers to the methods its kind keeps, whose
        // words are the ones text goes by, bound to the row they were
        // read from.
        if let Value::Octets { changeable, .. } = &value {
            if let Some(working) = self.octet_member(key, *changeable) {
                return Ok(Value::Member(Rc::new(value.clone()), working.to_string()));
            }
        }
        if key == self.detail("call") && self.rules.closes_over {
            let native_callable = match &value {
                Value::Intrinsic(op, _) => !Self::names_a_kind(op),
                Value::Member(..) => true,
                Value::Wrapped(tag, _) => *tag == 60,
                _ => false,
            };
            if native_callable { return Ok(value); }
        }
        if matches!(&value,Value::Intrinsic(op,_) if !Self::names_a_kind(op)) {
            if let Some(member)=self.attribute(&value,key) { return Ok(member); }
        }
        if let Value::Intrinsic(op,word)=&value {
            if *op == Prim::AsReal && self.rules.words_ext_builtin_method_from_number.iter().any(|spelling| spelling.rsplit('.').next() == Some(key)) {
                return Ok(Value::Member(Rc::new(value.clone()), String::from("float_from_number")));
            }
            if *op == Prim::ComplexMade && self.rules.words_ext_builtin_method_from_number.iter().any(|spelling| spelling.rsplit('.').next() == Some(key)) {
                return Ok(Value::Member(Rc::new(value.clone()), String::from("complex_from_number")));
            }
            if Self::names_a_kind(op) {
                if let Some(size) = self.integer_attribute(&value, key) { return Ok(size); }
                if !key.is_empty() && key == self.detail("bases") {
                    let ancestor = if word.as_ref() == "bool" { self.kind_by_word("int") }
                        else { None };
                    let parent = ancestor.unwrap_or_else(|| Value::Blueprint(self.common_ancestor()));
                    return Ok(Value::tuple(vec![parent]));
                }
                if key==self.detail("module") { return Ok(Value::text(self.builtin_module())); }
                if key==self.detail("qualified") { return Ok(Value::text(word)); }
                if key=="__getformat__" && word.as_ref()=="float" {
                    return Ok(Value::Member(Rc::new(value.clone()), "float_getformat".to_owned()));
                }
                if key==self.detail("allocate"){
                    // Making a property outright allocates the subclass it
                    // is handed, the way the root's making does.
                    if *op == Prim::ClassWork(11) { return Ok(Self::wrap(1, Vec::new())); }
                    return Ok(self.native_allocation(word));
                }
                if key==self.detail("name")||self.table.spells("ext.builtin.class.name",key){return Ok(Value::text(word));}
                // The flags of the kind read as a class: a native kind
                // is never sealed and its values are no collector's,
                // so only the standing bits answer here.
                if key==self.detail("flags") {
                    let kind=self.native_kind(word);
                    let mut bits=1024;
                    if self.allowed_slot(&kind,self.detail("namespace")) {
                        bits|=16;
                        if Self::native_beneath(&kind).is_none() { bits|=4; }
                    }
                    return Ok(Value::Small(bits));
                }
                if key==self.detail("doc") {
                    if let Some(doc)=Self::builtin_kind_doc(word) { return Ok(Value::text(doc)); }
                }
                if key==self.detail("namespace") {
                    let mut entries=self.kind_member_names(word);
                    entries.push(key.to_owned());
                    let members:Vec<(Value,Value)>=entries.into_iter().filter_map(|name| {
                        let member = self.carried_by_kind(&value, &name)?;
                        Some((Value::text(&name), member))
                    }).collect();
                    return Ok(Value::Window(Rc::new(Value::Dict(Rc::new(members.into()))),'m'));
                }
                // Read as a class the kind stands on the root and on
                // nothing further, which is the whole of its line.
                if key == self.detail("bases") {
                    let root = self.common_ancestor();
                    return Ok(Value::tuple(vec![Value::Blueprint(root)]));
                }
                if key == self.detail("order") && self.table.has_any("ext.stmt.class.builder") {
                    let entry = self.kind_entry("type", key);
                    if *op == Prim::SortOf { return Ok(entry); }
                    let builder = self.builder_blueprint();
                    return self.member_binding(entry, Some(value.clone()), builder);
                }
                if key==self.detail("mro")||key==self.detail("order"){
                    let listed=key==self.detail("order");
                    let word=word.to_string();
                    let kind=if *op==Prim::SortOf {self.builder_blueprint()} else {self.native_kind(&word)};
                    let mut ranks=vec![self.visible_blueprint(kind.clone())];ranks.extend(kind.ancestry.borrow().iter().cloned().map(|p|self.visible_blueprint(p)));
                    let line=Value::tuple(ranks);
                    return Ok(if listed {Self::wrap(0,vec![line])} else {line});
                }
            }
            // Everything a value of the kind answers to the kind itself
            // carries, standing loose: the value worked upon is the
            // first the entry is handed when it is called.
            if let Some(carried)=self.carried_by_kind(&value,key) { return Ok(carried); }
            if Self::names_a_kind(op) && self.table.single("ext.stmt.class.constructor") == Some(key) { return Ok(self.common_initialiser()); }
            if Self::names_a_kind(op) {
                let native_class = self.native_kind(&word);
                if let Some(inherited) = self.from_the_root(key, false, &native_class) { return Ok(inherited); }
            }
        }
        if self.table.spells("ext.stmt.class.builtin", "bytes") && (key == "__buffer__" || key == "__release_buffer__") {
            let provider = match &value {
                Value::OctetKind { changeable, .. } | Value::Octets { changeable, .. } => key == "__buffer__" || *changeable,
                _ => false,
            };
            if provider {
                return Ok(match value {
                    Value::Blueprint(ref class) => {
                        let word = Self::native_word(class).or_else(|| Self::native_beneath(class)).unwrap_or_default();
                        self.kind_entry(&word, key)
                    },
                    Value::OctetKind { changeable, .. } => self.kind_entry(self.octet_kind_word(changeable), key),
                    _ => Value::Member(Rc::new(value.clone()), key.to_string()),
                });
            }
        }
        if key == "__class_getitem__" && self.table.spells("ext.stmt.class.builtin", "tuple") {
            let base = match &value {
                Value::Blueprint(class) if self.inherited_entry(class, key).is_none() => Self::native_word(class).or_else(|| Self::native_beneath(class)),
                Value::Intrinsic(op, word) if Self::names_a_kind(op) => Some(word.to_string()),
                _ => None,
            };
            if base.as_deref().is_some_and(|word| ["list", "tuple", "set", "dict", "frozenset"].contains(&word)) {
                return Ok(Value::Member(Rc::new(value.clone()), key.to_owned()));
            }
        }
        if let Value::OctetKind { changeable, .. } = &value {
            {
                let word = self.octet_kind_word(*changeable).to_owned();
                let blueprint = self.native_kind(&word);
                return self.read_class_member(Value::Blueprint(blueprint), key, direct);
            }
        }
        if let Value::Blueprint(b)=&value {
            if key == "__weakref__" && self.admits_weak(b) {
                return Ok(Self::wrap(32, vec![Value::text(key), Value::Blueprint(b.clone())]));
            }
            if self.builds_classes(b) {
                if key == self.detail("allocate") { return Ok(Self::wrap(70, Vec::new())); }
                if key == self.detail("call") { return Ok(Self::wrap(71, Vec::new())); }
            }
            if !self.detail("name").is_empty() {
                if let Some(builder) = Self::builder_over(b) {
                    if let Some(entry) = self.inherited_entry(&builder, key).filter(|entry| self.writes_too(entry) && (matches!(entry, Value::Wrapped(6 | 32 | 58, _)) || self.protocol_entry(entry, "descriptor.get").is_some())) {
                        return self.member_binding(entry, Some(value.clone()), builder);
                    }
                }
            }
            if key == self.detail("kind") && self.rules.closes_over {
                return self.class_from_type(vec![value.clone()]);
            }
            let builtin_origin = Self::native_word(b).is_some()
                || (b.under.is_none() && b.name == self.detail("root"));
            if builtin_origin && !key.is_empty() && key == self.detail("module") {
                return Ok(Self::own_entry(b,key).unwrap_or_else(||Value::text(self.builtin_module())));
            }
            if key == self.detail("flags") {
                // The seal takes the base-standing bit off and puts the
                // unchangeable one on.
                let mut bits = if Self::sealed(b) { 256 } else { 1024 };
                if Self::native_word(b).is_none() && !(b.under.is_none() && b.name == self.detail("root")) { bits |= 512; }
                if b.constants.iter().any(|(label, _)| label == "\0native-name") { bits |= 256; }
                // Things of a class's own making answer to the cycle
                // collector; what stands on an atomic worth does not.
                if Self::native_word(b).is_none() && !matches!(Self::native_beneath(b).as_deref(), Some("tuple" | "int" | "float" | "complex" | "str" | "bytes" | "bytearray")) { bits |= 16384; }
                if self.allowed_slot(b, self.detail("namespace")) {
                    bits |= 16;
                    if Self::native_beneath(b).is_none() { bits |= 4; }
                }
                if let Some(Value::Small(protocol)) = self.inherited_entry(b, "__abc_tpflags__").map(|v| v.settled()) { bits |= protocol & 96; }
                return Ok(Value::Small(bits));
            }
            if (Self::native_word(b).as_deref() == Some("float") || Self::native_beneath(b).as_deref() == Some("float")) && self.rules.words_ext_builtin_method_from_number.iter().any(|spelling| spelling.rsplit('.').next() == Some(key)) {
                return Ok(Value::Member(Rc::new(value.clone()), String::from("float_from_number")));
            }
            if (Self::native_word(b).as_deref() == Some("complex") || Self::native_beneath(b).as_deref() == Some("complex")) && self.rules.words_ext_builtin_method_from_number.iter().any(|spelling| spelling.rsplit('.').next() == Some(key)) {
                return Ok(Value::Member(Rc::new(value.clone()), String::from("complex_from_number")));
            }
            if Self::native_beneath(b).as_deref() == Some("float") && key == "__getformat__" {
                return Ok(Value::Member(Rc::new(value.clone()), String::from("float_getformat")));
            }
            if key == "fromhex" {
                let base = Self::native_beneath(b);
                if base.as_deref() == Some("bytes") || base.as_deref() == Some("bytearray") {
                    return Ok(Value::Member(Rc::new(value.clone()), format!("{}_fromhex", base.unwrap())));
                }
            }
            if Self::native_beneath(b).as_deref() == Some("float") && key == "fromhex" {
                return Ok(Value::Member(Rc::new(value.clone()), String::from("float_fromhex")));
            }
            // Look for a stored override only before the byte primitive
            // in the ancestry. Its position also shields the primitive
            // from mixins placed after it, while the builder's writable
            // descriptors have already had their turn.
            if matches!(key, "fromhex" | "maketrans")
                && matches!(Self::native_beneath(b).as_deref(), Some("bytes" | "bytearray")) {
                let owner = std::iter::once(b.as_ref()).chain(b.ancestry.borrow().iter().map(Rc::as_ref))
                    .find_map(|parent| {
                        match Self::native_word(parent).filter(|word| word == "bytes" || word == "bytearray") {
                            Some(word) => Some(Err(word)),
                            None => Self::own_entry(parent, key).map(Ok),
                        }
                    });
                match owner {
                    Some(Ok(stored)) => return self.member_binding(stored, None, b.clone()),
                    Some(Err(word)) if key == "fromhex" => {
                        return Ok(Value::Member(Rc::new(value.clone()), word + "_fromhex"));
                    }
                    Some(Err(_)) => return Ok(Value::Intrinsic(Prim::Octets(40), Rc::from("bytes.maketrans"))),
                    None => {},
                }
            }
            if key == self.detail("module") && !self.table.strings("ext.stmt.class.detail.module").is_empty() {
                match Self::own_entry(b, key) {
                    Some(metadata) => return Ok(metadata),
                    None if Self::native_word(b).is_some() || self.builds_classes(b)
                        || self.ancestor.as_ref().is_some_and(|base| Rc::ptr_eq(base, b)) => {
                        return Ok(Value::text(self.builtin_module()));
                    }
                    None => {},
                }
            }
            if let Some(word)=Self::native_word(b) {
                if key == self.detail("allocate") && self.table.prims.get(&word).is_some_and(Self::names_a_kind) {
                    return Ok(self.native_allocation(&word));
                }
                if key==self.detail("module") { return Ok(Value::text(if matches!(word.as_str(), "SimpleNamespace" | "GenericAlias") { "types" } else if word == "Union" { "typing" } else { self.builtin_module() })); }
                if key==self.detail("qualified") { return Ok(Value::text(&word)); }
            }
            if key==self.detail("name"){let names=b.type_names.borrow();return Ok(names.as_ref().map_or_else(|| Value::text(&b.name), |names| names.short.clone()));}
            if key==self.detail("qualified"){return Ok(b.type_names.borrow().as_ref().map(|names| names.full.clone()).unwrap_or_else(|| self.inherited_entry(b,key).unwrap_or_else(||Value::text(&b.name))));}
            if key==self.detail("namespace"){
                let title = self.rules.words_ext_stmt_class_detail_code_fields.get(10).cloned().unwrap_or_default();
                if !title.is_empty() && Self::own_entry(b, crate::data::ANNOTATE_WORD).is_some() && Self::own_entry(b, &title).is_none() && Self::own_entry(b, "__annotate_func__").is_none() {
                    self.read_class_member(value.clone(), &title, true)?;
                }

                if let Some(builder)=Self::builder_over(b) {
                    if let Some(descriptor)=self.inherited_entry(&builder,key).filter(|entry| self.writes_too(entry)) {
                        return self.member_binding(descriptor,Some(value.clone()),builder);
                    }
                }
                // A blueprint standing for a native kind keeps no
                // entries of its own; what it names are the ones a
                // value of that kind answers to.
                return Ok(Value::Window(Rc::new(Value::Blueprint(b.clone())), 'm'));
            }
            if key==self.detail("bases"){return Ok(Value::tuple(b.parents.iter().map(|p|self.visible_blueprint(p.clone())).collect()));}
            if key == self.detail("order") && self.table.has_any("ext.stmt.class.builder") {
                if let Some(entry) = self.inherited_entry(b, key) { return self.member_binding(entry, None, b.clone()); }
                if let Some(builder) = Self::builder_over(b) {
                    if let Some(entry) = self.inherited_entry(&builder, key) { return self.member_binding(entry, Some(value.clone()), builder); }
                }
                let entry = self.kind_entry("type", key);
                let builder = self.builder_blueprint();
                return self.member_binding(entry, Some(value.clone()), builder);
            }
            if key==self.detail("mro")||key==self.detail("order"){
                let all=Self::resolution_order(b).into_iter().map(|p|self.visible_blueprint(p)).collect();
                let result=Value::tuple(all);return Ok(if key==self.detail("order"){Self::wrap(0,vec![result])}else{result});
            }
            if self.table.single("ext.stmt.annotation.adapter") == Some(key) {
                if self.rules.words_ext_stmt_class_detail_code_fields.get(10)
                    .is_some_and(|public| Self::own_entry(b, public).is_some()) { return Ok(Value::Nil); }
                return Ok(Self::own_entry(b, crate::data::ANNOTATE_WORD).map(|held| held.settled())
                    .unwrap_or_else(|| Value::Vector(crate::tuples::Sequence::plain(Vec::new()))));
            }
            if self.rules.words_ext_stmt_class_detail_code_fields.get(10).is_some_and(|word| word == key) {
                if let Some(own) = Self::own_entry(b, key).or_else(|| Self::own_entry(b, "__annotate_func__")) { return Ok(own.settled()); }
                let present = Self::own_entry(b, crate::data::ANNOTATE_WORD).is_some_and(|value| match value.settled() {
                    Value::Vector(row) => row.chunks(2).any(|entry| matches!(entry.get(1), Some(Value::Bound(code, _) | Value::Routine(code)) if !code.annotation_is_text)),
                    _ => false,
                });
                let present = Self::own_entry(b, "\0string_annotations").map_or(present, |mode| !mode.is_true());
                if !present { return Ok(Value::Nil); }
                let evaluator = Self::wrap(44, vec![value.clone()]);
                b.shared.borrow_mut().push((String::from("__annotate_func__"), evaluator.clone()));
                return Ok(evaluator);
            }
            if self.rules.words_ext_stmt_class_annotations.first().map(String::as_str)==Some(key){return self.blueprint_annotations(b);}
            if let Some(found)=self.inherited_entry(b,key){return self.member_binding(found,None,b.clone());}
            // The classes built beneath this one, the live ones, as the
            // reference's own type.__subclasses__ tells them.
            if key==self.detail("subclasses") && !self.detail("subclasses").is_empty() { return Ok(Self::wrap(204, vec![value.clone()])); }
            if let Some(size) = self.integer_attribute(&value, key) { return Ok(size); }
            // A blueprint standing on a native kind reads that kind's
            // own class method too, bound to the blueprint, so that
            // `dictlike.fromkeys` reaches `dict.fromkeys` and hands
            // back a dictlike.
            if self.table.spells("ext.builtin.method.fromkeys", key) {
                if let Some(base) = std::iter::once(b).chain(b.ancestry.borrow().iter()).find(|base| Self::native_word(base).is_some()) {
                    if self.carried_by_kind(&Value::Blueprint(base.clone()), key).is_some() {
                        return Ok(Value::Member(Rc::new(value.clone()), key.to_owned()));
                    }
                }
            }
            if let Some(entry)=self.carried_by_kind(&value,key){return Ok(entry);}
            if Self::native_word(b).is_none() && !self.table.spells("ext.builtin.method.fromkeys", key) {
                for ancestor in b.ancestry.borrow().iter() {
                    if Self::native_word(ancestor).is_some() {
                        if let Some(method) = self.carried_by_kind(&Value::Blueprint(ancestor.clone()), key) { return Ok(method); }
                        break;
                    }
                }
            }
            if key == self.detail("allocate") {
                let inherited = Self::native_beneath(b).and_then(|word| self.table.prims.get(&word).filter(|op| Self::names_a_kind(op)).map(|_| word));
                return Ok(match inherited {
                    Some(word) => self.native_allocation(&word),
                    None => self.common_allocation(),
                });
            }
            // A class reads what the metaclass that built it holds as
            // well, each entry bound to the class itself, as a thing's
            // method is bound to the thing.
            if let Some(builder)=Self::builder_over(b) {
                if let Some(found)=self.inherited_entry(&builder,key){return self.member_binding(found,Some(value.clone()),builder);}
            }
            if self.names_in_calls && key == self.detail("call") {
                return Ok(Self::wrap(3, vec![Self::wrap(71, Vec::new()), value.clone()]));
            }
            // The formatting every blueprint has from the root: a thing
            // and a specification, answered as the format builtin would.
            if self.rules.specials.get(72).map_or(false,|word|word==key){return Ok(Self::wrap(59,Vec::new()));}
            {
                let tag=if key==self.detail("allocate")&&(self.builds_classes(b)||b.ancestry.borrow().iter().any(|p|self.builds_classes(p))){70}else if key==self.detail("allocate"){1}else if self.rules.words_ext_stmt_class_constructor.first().map(String::as_str)==Some(key)&&(self.builds_classes(b)||b.ancestry.borrow().iter().any(|p|self.builds_classes(p))){73}else if self.rules.words_ext_stmt_class_constructor.first().map(String::as_str)==Some(key)||key==self.detail("subclass"){2}
                    else if key==self.detail("get"){10}else if key==self.detail("set"){11}else if key==self.detail("remove"){12}else{255};
                if tag!=255 {
                    let state = match tag {
                        10..=12 if self.table.has_any("ext.stmt.class.builder") => vec![Value::text(self.detail("root"))],
                        _ if key == self.detail("subclass") => vec![Value::text(key)],
                        _ => Vec::new(),
                    };
                    return Ok(Self::wrap(tag,state));
                }
            }
            if let Some(root)=self.from_the_root(key,false,b){return Ok(root);}
        }else if let Value::Thing(t)=&value {
            if self.namespace_holding(&value).is_some() || Self::native_beneath(&t.blueprint()).as_deref() == Some("module") {
                let title = self.rules.words_ext_stmt_class_detail_code_fields.get(10).cloned().unwrap_or_default();
                let dictionary = t.holds.borrow().iter().find(|entry| entry.0 == "\0dictionary").map(|entry| entry.1.clone());
                let lookup = |wanted: &str| {
                    dictionary.as_ref().and_then(|value| if let Value::Dict(entries) = value.settled() {
                        entries.iter().find(|entry| entry.0.bare() == wanted).map(|entry| entry.1.settled())
                    } else { None }).or_else(|| t.holds.borrow().iter().find(|entry| entry.0 == wanted).map(|entry| entry.1.settled()))
                };
                let own = lookup(key);
                if key == title { return Ok(own.unwrap_or(Value::Nil)); }
                if self.rules.words_ext_stmt_class_annotations.first().map(String::as_str) == Some(key) {
                    if let Some(existing) = own.filter(|value| !matches!(value, Value::Unset)) { return Ok(existing); }
                    let source = lookup(&title);
                    let dictionary = match source.filter(|value| !matches!(value, Value::Nil | Value::Unset)) {
                        Some(source) => self.apply_class_member(source, vec![Value::Small(1)])?,
                        None => self.collection_cell(Value::Dict(Rc::new(Vec::new().into()))),
                    };
                    let book = t.holds.borrow().iter().find(|entry| entry.0 == "\0dictionary").map(|entry| entry.1.clone());
                    if let Some(book) = book {
                        Self::write_into_book(&book, key, Some(dictionary.clone()));
                    } else { Self::change_entry(&mut t.holds.borrow_mut(), key, Some(dictionary.clone())); }
                    return Ok(dictionary);
                }
            }

            if Self::native_word(&t.blueprint()).as_deref() == Some("Union") {
                match key {
                    "__origin__" => return Ok(Value::Blueprint(self.native_kind("Union"))),
                    "__parameters__" => {
                        let arguments = t.holds.borrow().iter().find(|entry| entry.0 == "__args__").unwrap().1.clone();
                        let typing = self.load_namespace("typing")?;
                        let collect = self.read_class_member(typing, "_collect_type_parameters", false)?.settled();
                        return self.apply_class_member(collect, vec![arguments]);
                    }
                    _ => {},
                }
            }
            if Self::native_word(&t.blueprint()).is_some_and(|kind| matches!(kind.as_str(), "TypeVar" | "ParamSpec" | "TypeVarTuple" | "TypeAliasType")) {
            if t.blueprint().name == "ParamSpec" && (key == "args" || key == "kwargs") {
                let kind = self.native_kind(if key == "args" { "ParamSpecArgs" } else { "ParamSpecKwargs" });
                let name = t.holds.borrow().iter().find(|entry| entry.0 == "__name__").map_or(String::new(), |entry| entry.1.bare());
                self.made += 1;
                let attributes = vec![(String::from("__origin__"), value.clone()), (String::from("\0type-display"), Value::text(&format!("{name}.{key}")))];
                return Ok(Value::Thing(Rc::new(Thing { reclassified: RefCell::new(None), of: kind, holds: RefCell::new(attributes), turn: self.made })));
            }
            if key == "__parameters__" && t.blueprint().name == "TypeAliasType" {
                let attributes = t.holds.borrow();
                return Ok(attributes.iter().find(|entry| entry.0 == "__type_params__").map_or_else(|| Value::tuple(Vec::new()), |entry| entry.1.clone()));
            }
            if let Some(property) = match key {
                "evaluate_value" => Some("__value__"), "evaluate_default" => Some("__default__"),
                "evaluate_bound" if t.blueprint().name == "TypeVar" => Some("__bound__"),
                "evaluate_constraints" if t.blueprint().name == "TypeVar" => Some("__constraints__"), _ => None,
            } {
                let attributes = t.holds.borrow();
                let source = attributes.iter().find(|entry| entry.0 == format!("\0deferred/{property}")).map(|entry| entry.1.clone())
                    .or_else(|| if property == "__bound__" || property == "__constraints__" {
                        let tuple = attributes.iter().any(|entry| entry.0 == "\0constraints-syntax" && entry.1.is_true());
                        if (property == "__constraints__") == tuple { attributes.iter().find(|entry| entry.0 == "\0deferred-bound").map(|entry| entry.1.clone()) } else { None }
                    } else { None });
                if let Some(source) = source { return Ok(Self::wrap(47, vec![source])); }
                if let Some(entry) = attributes.iter().find(|entry| entry.0 == property) {
                    let absent = matches!(&entry.1, Value::Nil) || matches!(&entry.1, Value::Tuple(items) if items.is_empty());
                    if absent && (property == "__bound__" || property == "__constraints__") { return Ok(Value::Nil); }
                    return Ok(Self::wrap(48, vec![entry.1.clone()]));
                }
            }
            if key == "__bound__" || key == "__constraints__" {
                let constraints = t.holds.borrow().iter().any(|entry| entry.0 == "\0constraints-syntax" && entry.1.is_true());
                let source = if (key == "__constraints__") == constraints {
                    t.holds.borrow().iter().find(|entry| entry.0 == "\0deferred-bound").map(|entry| entry.1.clone())
                } else { None };
                if let Some(source) = source.filter(|_| !t.holds.borrow().iter().any(|entry| entry.0 == "\0bound-computed")) {
                    let bound = self.apply_class_member(source, Vec::new())?;
                    let tuple = constraints;
                    let mut attributes = t.holds.borrow_mut();
                    attributes.push((String::from("\0bound-computed"), Value::Flag(true)));
                    for entry in attributes.iter_mut() {
                        match entry.0.as_str() {
                            "__bound__" => entry.1 = if tuple { Value::Nil } else { bound.clone() },
                            "__constraints__" => entry.1 = if tuple { bound.clone() } else { Value::tuple(Vec::new()) },
                            _ => {},
                        }
                    }
                }
            }
            let lazy = t.holds.borrow().iter().find(|entry| entry.0 == format!("\0deferred/{key}")).map(|entry| entry.1.clone());
            if let Some(thunk) = lazy {
                if let Some(entry) = t.holds.borrow().iter().find(|entry| entry.0 == key) { return Ok(entry.1.clone()); }
                let computed = self.apply_class_member(thunk, Vec::new())?;
                t.holds.borrow_mut().push((key.to_owned(), computed.clone()));
                return Ok(computed);
            }
            let substitution = match (t.blueprint().name.as_str(), key) {
                ("TypeVar", "__typing_subst__") => Some("_typevar_subst"),
                ("ParamSpec", "__typing_subst__") => Some("_paramspec_subst"),
                ("ParamSpec", "__typing_prepare_subst__") => Some("_paramspec_prepare_subst"),
                ("TypeVarTuple", "__typing_prepare_subst__") => Some("_typevartuple_prepare_subst"),
                _ => None,
            };
            if let Some(function) = substitution { return Ok(Self::wrap(154, vec![Value::text(function), value.clone()])); }
            if key == "has_default" && matches!(t.blueprint().name.as_str(), "TypeVar" | "ParamSpec" | "TypeVarTuple") {
                let sentinel = self.absent_type_default();
                let present = t.holds.borrow().iter().any(|entry| entry.0 == "\0deferred/__default__") || t.holds.borrow().iter().find(|entry| entry.0 == "__default__").is_some_and(|entry| !entry.1.selfsame(&sentinel));
                return Ok(Self::wrap(0, vec![Value::Flag(present)]));
            }

            }
            if !direct {if let Some(reader)=self.inherited_entry(&t.blueprint(),self.detail("get")){return self.apply_class_member(reader,vec![value.clone(),Value::text(key)]);}}
            let fallback_reader = self.table.strings("ext.stmt.class.reader").first().and_then(|name| self.inherited_entry(&t.blueprint(),name));
            if !self.default_attribute_slots(&t.blueprint()) && fallback_reader.is_none() { return Err(self.absent_attribute(&value,key)); }
            if key == self.detail("kind") {
                if self.namespace_holding(&value).is_some() { return Ok(self.kind_named_after(&value)); }
                match self.inherited_entry(&t.blueprint(), key) {
                    Some(entry) => return self.member_binding(entry, Some(value.clone()), t.blueprint().clone()),
                    None if self.root_in_resolution(&t.blueprint()) => return Ok(Value::Blueprint(t.blueprint())),
                    None => return Err(self.absent_attribute(&value,key)),
                }
            }
            if key==self.detail("namespace"){
                if let Some(descriptor)=self.inherited_entry(&t.blueprint(),key) {
                    if Self::native_beneath(&t.blueprint()).as_deref() == Some("module") && !self.writes_too(&descriptor) {
                        if let Some(held) = t.holds.borrow().iter().find(|entry| entry.0 == key).map(|entry| entry.1.clone()) { return Ok(held); }
                    }
                    return self.member_binding(descriptor,Some(value.clone()),t.blueprint().clone());
                }
                // A blueprint naming its slots without the namespace among them has things without one.
                if t.blueprint().order_supplied.get() || t.blueprint().type_names.borrow().is_some() || !self.allowed_slot(&t.blueprint(),key){return Err(self.absent_attribute(&value,key));}
                if let Some((_, mapping)) = t.holds.borrow().iter().find(|entry| entry.0 == "\0dictionary") { return Ok(mapping.clone()); }
                return Ok(Value::Attributes(t.clone()));
            }
            let from_class=self.inherited_entry(&t.blueprint(),key);
            // An entry that takes writes is heard before what the thing
            // holds itself; every other entry after.
            if from_class.as_ref().map_or(false,|e|self.writes_too(e)){return self.member_binding(from_class.unwrap(),Some(value.clone()),t.blueprint().clone());}
            let dictionary = t.holds.borrow().iter().find(|entry| entry.0 == "\0dictionary").map(|entry| entry.1.clone());
            if let Some(mapping) = dictionary {
                // The namespace may be another thing's own holds, shared
                // by handing one `__dict__` to another; the name is read
                // out of those holds then.
                if let Value::Attributes(view) = mapping.settled() {
                    if let Some((_, held)) = view.holds.borrow().iter().find(|(name, held)| name.as_str() == key && !matches!(held.settled(), Value::Unset)) { return Ok(held.clone()); }
                }
                let native = Self::underlying(&mapping).unwrap_or(mapping);
                if let Value::Dict(entries) = native.settled() {
                    for (word, item) in entries.iter() {
                        if matches!(word, Value::Text(word) if word.as_ref() == key) { return Ok(item.clone()); }
                    }
                }
            }
            let own=t.holds.borrow().iter().find(|(k,_)|k==key).map(|(_,v)|v.clone());
            if let Some(v)=own.filter(|held| !matches!(held.settled(), Value::Unset)){return Ok(v);}
            if let Some(v)=from_class{return self.member_binding(v,Some(value.clone()),t.blueprint().clone());}
            if self.is_fault_kind(&t.blueprint()) && self.fault_method_word(key) { return Ok(Value::Member(Rc::new(value.clone()), key.to_owned())); }
            if self.rules.words_ext_stmt_class_detail_root_members.get(9).is_some_and(|word| word == key) {
                let root = Self::wrap(36, vec![Value::text(key)]);
                return Ok(Self::wrap(3, vec![root, value.clone()]));
            }
            if matches!(key, "__reduce__" | "__reduce_ex__" | "__getstate__") {
                if let Some(method) = self.from_the_root(key, true, &t.blueprint()) {
                    return Ok(Self::wrap(3, vec![method, value.clone()]));
                }
            }
            if key == "__getnewargs__" {
                if let Some(base) = Self::underlying(&value) {
                    match base.settled() {
                        Value::Small(_) | Value::Huge(_) | Value::Frac(_) | Value::Tuple(_) | Value::Text(_) | Value::Unpaired(_) | Value::Octets { changeable: false, .. } => return Ok(Value::Member(Rc::new(value.clone()), "__getnewargs__".to_owned())),
                        _ => (),
                    }
                }
            }
            if Self::underlying(&value).is_some_and(|base| matches!(base.settled(), Value::Set(_))) {
                if let Some(slot) = self.table.strings("ext.stmt.class.special").iter().position(|word| word == key).filter(|slot| *slot == 79 || *slot == 81) {
                    let method = if slot == 79 { "set_reduce" } else { "set_reduce_ex" };
                    return Ok(Value::Member(Rc::new(value.clone()), method.to_owned()));
                }
            }
            // The worth a thing keeps answers for the methods of its kind.
            let native=Self::underlying(&value);
            // A mapping subclass's subscript member stays bound to the
            // thing itself, so a key the mapping does not hold reaches
            // its `__missing__` the way a plain subscript does.
            if self.rules.specials.get(11).map_or(false, |word| word == key)
                && native.as_ref().map_or(false, |under| matches!(under.settled(), Value::Dict(_))) {
                return Ok(Value::Member(Rc::new(value.clone()), key.to_owned()));
            }
            if let Some(set)=native.as_ref().filter(|v|matches!(v.settled(),Value::Set(_))) {
                let protocol = self.rules.specials.iter().position(|word| word == key);
                if protocol == Some(79) || protocol == Some(81) {
                    return Ok(Value::Member(Rc::new(value.clone()), key.to_owned()));
                }
                if let Some(member)=self.attribute(&set.settled(),key) { return Ok(member); }
            }
            if let Some(text @ Value::Text(_)) = &native {
                if let Some(member) = self.attribute(text, key) { return Ok(member); }
            }
            if let Some(size) = self.integer_attribute(&value, key) { return Ok(size); }
            if let Some(root)=if self.root_in_resolution(&t.blueprint()) { self.from_the_root(key,true,&t.blueprint()) } else { None } {
                if !native.as_ref().map_or(false,|under|Self::kind_method_named(self.table,key).is_some()||self.attribute(&under.settled(),key).is_some()) {
                    // The maker takes a class and stays loose; the hook
                    // for a class stood on hears from the class; the
                    // rest are bound to the thing.
                    if key==self.detail("allocate"){return Ok(root);}
                    let bound_to=if key==self.detail("subclass"){Value::Blueprint(t.blueprint().clone())}else{value.clone()};
                    return Ok(Self::wrap(3,vec![root,bound_to]));
                }
            }
            if let Some(under)=native.filter(|v|!matches!(v.settled(),Value::Set(_))) {
                let named_slot = self.rules.specials.iter().position(|word| word == key);
                let container_member = named_slot.map_or(false, |index| index >= 10 && index <= 14);
                if self.native_member(&under,key) && (container_member || matches!(under.settled(), Value::Complex(_))) {
                    if named_slot == Some(11) && matches!(under.settled(), Value::Dict(_)) {
                        return Ok(Self::wrap(3, vec![Self::wrap(60, vec![Value::text("dict"), Value::text(key)]), value.clone()]));
                    }
                    return Ok(Value::Member(Rc::new(value.clone()),key.to_owned()));
                }
                if let Some(operation)=Self::kind_method_named(self.table,key){
                    // The parts of a complex number are members read and
                    // not methods called, as on the number itself.
                    if matches!(operation.as_str(),"real"|"imag") {
                        if let Value::Complex(pair)=under.settled() {
                            return Ok(crate::complex::decimal_value(if operation=="real"{pair.0}else{pair.1}));
                        }
                    }
                    // `fromkeys` belongs to the class, so a thing of a
                    // mapping kind is handed over itself and not its
                    // worth, that its own class may make it.
                    if operation == "fromkeys" {
                        return Ok(Value::Member(Rc::new(value.clone()),operation));
                    }
                    return Ok(Value::Member(Rc::new(value.clone()),operation));
                }
                // A row of bytes answers to the methods its kind keeps,
                // which the worth beneath the thing carries out.
                if let Value::Octets{changeable,..}=under.settled() {
                    if let Some(working)=self.octet_member(key,changeable) {
                        return Ok(Value::Member(Rc::new(under),working.to_string()));
                    }
                }
                // A set answers some of its methods with primitives that
                // take the receiver first, so one read through the thing
                // is tied to the worth it keeps.
                if let Some(operation @ Prim::SetCall(1..=17)) = self.table.prims.get(key).copied() {
                    let callable = if !self.rules.words_ext_stmt_class_builder.is_empty() {
                        Value::Intrinsic(operation, Rc::from(key))
                    } else { Value::text(key) };
                    return Ok(Self::wrap(3,vec![callable,under]));
                }
            }
            if self.namespace_holding(&value).is_some() {
                let handler = t.holds.borrow().iter().find_map(|(word, held)| {
                    if word == "__getattr__" {
                        let candidate = held.settled();
                        if !matches!(candidate, Value::Unset) { return Some(candidate); }
                    }
                    None
                });
                if let Some(handler) = handler { return self.apply_class_member(handler, vec![Value::text(key)]).map(|answer| answer.settled()); }
            }
        }else if let Value::Method(code,t,_)=&value {
            if key==self.detail("receiver"){return Ok(Value::Thing(t.clone()));}
            if key==self.detail("function"){return Ok(Value::Routine(code.clone()));}
            return self.read_class_member(Value::Routine(code.clone()),key,true);
        }else if let Value::Routine(_)|Value::Bound(..)=&value {
            let (code,room)=self.routine_standing(&value);
            if let Some(held)=self.routine_holding(&value,key){return Ok(held);}
            // The row of type parameters the declaration wrote, made on
            // the first asking and kept, so every asking answers the
            // selfsame row.
            if key==self.detail("type_params") {
                let index=self.routine_storage(&value);
                let made=self.routine_type_row(&code)?;
                self.routine_members[index].1.holds.borrow_mut().push((format!("\0{key}\0"),made.clone()));
                return Ok(made);
            }
            // Annotation expressions stand outside the function's own
            // frame, including any frame that carries default values.
            let annotation_room = if code.carried.is_empty() { room.clone() }
                else { room.outer.clone().unwrap_or_else(|| self.outermost.clone()) };
            let annotations = self.rules.words_ext_stmt_class_annotations;
            if annotations.first().map_or(false, |word| word == key) {
                let label = self.rules.words_ext_stmt_class_detail_code_fields.get(10).cloned().unwrap_or_default();
                let written = self.routine_holding(&value, &label);
                let source = written.or_else(|| code.annotator.clone().map(|a| Value::Bound(a, annotation_room.clone())));
                let contents = if let Some(evaluator) = source.filter(|a| !matches!(a.settled(), Value::Nil)) {
                    self.apply_class_member(evaluator, vec![Value::Small(1)])?
                } else { self.collection_cell(Value::Dict(Rc::new(Vec::new().into()))) };
                let index = self.routine_storage(&value);
                self.routine_members[index].1.holds.borrow_mut().push((format!("\0{key}\0"), contents.clone()));
                return Ok(contents);
            }
            let fields = self.rules.words_ext_stmt_class_detail_code_fields;
            if fields.get(10).map_or(false, |word| word == key) {
                return Ok(code.annotator.clone().map_or(Value::Nil, |a| Value::Bound(a, annotation_room.clone())));
            }
            if key==self.detail("globals"){
                // A routine framed by hand keeps the dictionary it was
                // handed; one the program wrote answers the outermost
                // dictionary of the run it was written in.
                if let Some(globe)=&code.globe { return Ok(globe.clone()); }
                if code.written_in.is_none(){return Ok(Value::Shared(self.constructor_world(&code).unwrap_or_else(||self.book_about(true))));}
            }
            // The builtins a routine reaches its unbound names through.
            if self.table.strings("ext.system.module.builtins").iter().any(|word| word==key) {
                return self.routine_builtins(&code);
            }
            if key==self.detail("keywords"){
                let named=self.spare_worths(&code,&room,'n');
                if named.is_empty(){return Ok(Value::Nil);}
                return Ok(Value::Dict(Rc::new(named.into_iter().map(|(at,v)|(Value::text(&code.formals[at]),v)).collect())));
            }
            // The cells a routine reaches out to, ordered by the names
            // they hold, or nothing where it reaches none.
            if key==self.detail("closure"){
                if code.reaching.is_empty(){return Ok(Value::Nil);}
                let mut order:Vec<usize>=(0..code.reaching.len()).collect();
                order.sort_by(|a,b|Self::capture_title(&code.reaching[*a].ident).cmp(Self::capture_title(&code.reaching[*b].ident)));
                let reacher=Value::Bound(code.clone(),room.clone());
                return Ok(Value::tuple(order.into_iter().map(|at|Self::wrap(35,vec![reacher.clone(),Value::Small(at as i64)])).collect()));
            }
            if key==self.detail("name"){return Ok(self.routine_kept(&value,key,Value::text(&code.ident)));}
            if key==self.detail("qualified"){let qualified=code.qualification.clone();return Ok(self.routine_kept(&value,key,Value::text(&qualified)));}
            if key==self.detail("doc"){let fresh=code.doc.as_ref().map_or(Value::Nil,|d|Value::text(d));return Ok(self.routine_kept(&value,key,fresh));}
            if key==self.detail("module"){let place=self.routine_module(&code);return Ok(self.routine_kept(&value,key,place));}
            if key==self.detail("code"){let ran=self.code_run_by(&value);return Ok(self.code_handle(&ran));}
            if key==self.detail("namespace"){let index=self.routine_storage(&value);return Ok(Value::Attributes(self.routine_members[index].1.clone()));}
            if key==self.detail("defaults"){
                let defaults:Vec<Value>=if matches!(value,Value::Bound(..))||self.written_over.contains_key(&Self::written_key(&value,&self.outermost)) {self.spare_worths(&code,&room,'p').into_iter().map(|(_,v)|v).collect()} else {Vec::new()};
                let unavailable = code.local_defaults.iter().any(|slot| {
                    code.formal_slots.iter().position(|formal| formal == slot)
                        .is_some_and(|index| code.taking.as_ref().map_or(true, |rules| rules[index] != 'n'))
                });
                if defaults.is_empty() && unavailable { return Err(self.class_unready()); }
                return Ok(if defaults.is_empty(){Value::Nil}else{Value::tuple(defaults)});
            }
            // A routine answers its own call, so reading it back and
            // calling it does what calling the routine directly does.
            if key==self.detail("call"){return Ok(value.clone());}
        }else if let Value::Generator(state)=&value {
            if self.is_async_generator(&value) {
                if let Some(index) = self.table.strings("ext.stmt.async.generator.fields").iter().position(|word| word == key) {
                    if index == 2 { return Ok(Value::Flag(self.async_running(&value))); }
                    let held = state.try_borrow().map_err(|_| self.class_unready())?;
                    return Ok(match index {
                        0 => held.of.as_ref().map_or(Value::Nil, |body| self.code_handle(body)),
                        1 => if held.ended { Value::Nil } else { held.trace_state.clone().map_or(Value::Nil, Value::Thing) },
                        2 => Value::Flag(false),
                        _ => match held.inner.clone().unwrap_or(Value::Nil) { Value::Wrapped(63, parts) => parts[0].clone(), other => other },
                    });
                }
                if let Some(method) = self.attribute(&value, key) { return Ok(method); }
            }
            let running=self.table.strings("ext.stmt.yield.running");
            if !self.is_async_generator(&value) && running.first().map_or(false,|w|w==key) {
                // Running exactly while its own frame is on the way
                // through the machine, which is exactly when the cell
                // that holds it cannot be borrowed a second time.
                return Ok(Value::Flag(state.try_borrow().is_err()));
            }
        }else if let Value::Wrapped(tag,items)=&value {
            if *tag==4||*tag==5 {
                if key==self.detail("function"){return Ok(items[0].clone());}
                // Both wrapper kinds hold the routine they were given
                // under `__wrapped__`, and answer for its name, full
                // name, module, account and annotations as it would.
                if key=="__wrapped__" { return Ok(items[0].clone()); }
                // The routine within answers for its own abstract mark
                // through the wrapper; a routine with no mark says no.
                if key==self.detail("abstractmethod") && !self.detail("abstractmethod").is_empty() {
                    return match self.read_class_member(items[0].clone(),key,true) {
                        Ok(held)=>Ok(held),
                        Err(escape) if self.missing_member_escape(&escape)=>Ok(Value::Flag(false)),
                        Err(escape)=>Err(escape),
                    };
                }
                let carried=key==self.detail("module")||key==self.detail("qualified")||key==self.detail("name")||key==self.detail("doc")
                    || self.rules.words_ext_stmt_class_annotations.first().map_or(false,|word|word==key)
                    || self.table.strings("ext.stmt.class.detail.code.fields").get(10).is_some_and(|word| word == key);
                if carried { return self.read_class_member(items[0].clone(),key,true); }
            }
            // A method bound to its thing answers for the thing and the
            // function by the table's words, and for anything else as
            // the function itself would: a method of a class formed in
            // a function is bound this way, and its name, its full name
            // and what it says of itself are the function's.
            if matches!(*tag, 3 | 132) {
                if *tag == 3 && key == self.detail("kind") { return Ok(self.kind_named_after(&value)); }
                if key==self.detail("receiver"){return Ok(items[1].clone());}
                if key==self.detail("function"){return Ok(items[0].clone());}
                if *tag == 132 && key == self.detail("kind") {
                    return self.class_from_type(vec![value.clone()]);
                }
                if *tag == 132 && (key == self.detail("qualified") || key == self.detail("name")) {
                    if let Value::Intrinsic(_, ident) = &items[0] { return Ok(Value::text(ident)); }
                }
                return self.read_class_member(items[0].clone(),key,true);
            }
            if *tag==35&&key==self.detail("cell.contents"){
                return self.cell_contents(items).ok_or_else(||self.detail("cell.empty").to_owned().into());
            }
            if *tag==7 {
                if let Value::Routine(p)|Value::Bound(p,_)=&items[0] {
                    if self.table.strings("ext.stmt.class.detail.code.replace").first().map_or(false, |word| word == key) {
                        return Ok(Value::Member(Rc::new(value.clone()), String::from("code_replace")));
                    }
                    let fields = self.rules.words_ext_stmt_class_detail_code_fields;
                    if let Some(index) = fields.iter().position(|word| word == key) {
                        let result = match index {
                            0 => Value::text(if p.ident == "<generator>" { "<genexpr>" } else if p.ident == "<program>" { self.rules.trace_words.get(10).map_or("<module>", String::as_str) } else { &p.ident }),
                            1 => Value::text(&p.qualification),
                            2 | 3 => Value::Small(p.taking.as_ref().map_or(0, |rules| rules.iter().filter(|r| **r == if index == 2 { 'p' } else { 'n' }).count()) as i64),
                            4 => Value::Small(p.locals.len() as i64),
                            5 => Value::tuple(p.referenced.iter().map(|n| Value::text(n)).collect()),
                            6 => {
                                let mut members = Vec::new();
                                for v in &p.literals { members.push(match v { Value::Routine(r) => self.code_handle(r), _ => v.clone() }); }
                                Value::tuple(members)
                            }
                            7 => Value::Small(p.flags),
                            8 => Value::Text(p.written_in.clone().unwrap_or_else(|| self.entry_file.clone())),
                            9 => Value::Small(i64::from(p.declared_on.max(1))),
                            11 => {
                                let mut captured: Vec<&str> = p.reaching.iter().map(|address| Self::capture_title(&address.ident)).collect();
                                captured.sort_unstable();
                                captured.dedup();
                                Value::tuple(captured.into_iter().map(Value::text).collect())
                            }
                            _ => return Err(self.absent_attribute(&value, key)),
                        };
                        return Ok(result);
                    }
                    let words = self.rules.trace_words;
                    if words.get(6).map_or(false, |word| word == key) { return Ok(Value::text(if p.ident == "<program>" { &words[10] } else { &p.ident })); }
                    if words.get(7).map_or(false, |word| word == key) { return Ok(Value::Text(p.written_in.clone().unwrap_or_else(|| self.entry_file.clone()))); }
                    if words.get(8).map_or(false, |word| word == key) { return Ok(Value::Small(p.declared_on.max(1) as i64)); }
                    if key==self.detail("argcount"){return Ok(Value::Small(p.taking.as_ref().map_or(p.formals.len(),|rules|rules.iter().filter(|r|matches!(r,'b'|'p')).count()) as i64));}
                    if key==self.detail("varnames"){return Ok(Value::tuple(p.locals.iter().map(|s|Value::text(s)).collect()));}
                }
            }
        }
        // A namespace may hold a routine, under the table's word for it,
        // that answers for names the namespace has not.
        if let (false, Value::Thing(t), Some(word)) = (self.asking_presence, &value, self.table.strings("ext.system.module.getattr").first().map(String::as_str)) {
            let answerer = t.holds.borrow().iter().find(|(n, _)| n == word).map(|(_, held)| match held { Value::Shared(cell) => cell.borrow().clone(), other => other.settled() });
            if let Some(routine @ (Value::Routine(_) | Value::Bound(..))) = answerer {
                return self.apply_class_member(routine, vec![Value::text(key)]).map_err(|escaped| self.explain_absence(escaped, &value, key));
            }
        }
        if key == self.detail("allocate") && matches!(value, Value::Nil | Value::Ellipsis | Value::Refusal(_)) {
            let owner = self.kind_named_after(&value);
            return self.read_class_member(owner, key, direct);
        }
        // Whatever is no thing is of the kind the kind primitive names
        // for it, where it names one.
        if key==self.detail("kind")&&!matches!(value,Value::Thing(_)) {
            if let Ok(kind)=self.class_from_type(vec![value.clone()]) {return Ok(kind);}
        }
        self.sought_in_vain = Some((key.to_owned(), value.clone()));
        Err(self.absent_attribute(&value,key))
    }
    /// Whether the blueprint, or one it stands on, names the entries its
    /// things may hold. Such a thing keeps no namespace of its own.
    fn storage_keys(&self, class: &Blueprint) -> Vec<(String, String)> {
        let mut result = Vec::new();
        let mut gather = |base: &Blueprint| {
            for (word, member) in base.shared.borrow().iter() {
                if matches!(word.as_str(), "__weakref__" | "__dict__") { continue; }
                if let Value::Wrapped(32, data) = member {
                    if let Some(Value::Blueprint(declared)) = data.get(1) {
                        result.push((word.clone(), format!("\0slot:{word}:{:p}", Rc::as_ptr(declared))));
                    }
                }
            }
        };
        gather(class);
        for base in class.ancestry.borrow().iter() { gather(base); }
        result.sort_by_key(|entry| entry.0.clone());
        result
    }
    fn storage_shape(&self, class: &Blueprint) -> (Vec<String>, bool, bool) {
        let mut fields = Vec::new();
        let mut has_dictionary = !self.slots_named(class);
        let collect = |base: &Blueprint, fields: &mut Vec<String>, has_dictionary: &mut bool| {
            for (word, member) in base.shared.borrow().iter() {
                if matches!(member, Value::Wrapped(32, _)) {
                    match word.as_str() {
                        "__dict__" => *has_dictionary = true,
                        "__weakref__" => (),
                        _ => fields.push(word.clone()),
                    }
                }
            }
        };
        collect(class, &mut fields, &mut has_dictionary);
        for ancestor in class.ancestry.borrow().iter() { collect(ancestor, &mut fields, &mut has_dictionary); }
        fields.sort_unstable();
        (fields, has_dictionary, self.admits_weak(class))
    }
    fn slots_named(&self,b:&Blueprint)->bool {
        if Self::own_entry(b,self.detail("slots")).is_some(){return true;}
        b.parents.iter().any(|p|p.name!=self.detail("root")&&self.slots_named(p))
    }
    /// A weak-reference slot is inherited even when a child declares no
    /// new storage. Variable-sized integer, tuple and byte layouts cannot add it.
    pub(super) fn admits_weak(&self, b: &Blueprint) -> bool {
        if let Some(stored) = b.weak_slot.get() { return stored; }
        let underlying = Self::native_beneath(b);
        if matches!(underlying.as_deref(), Some("bytes" | "int" | "tuple")) { return false; }
        match Self::native_word(b) {
            Some(word) => return word == "frozenset" || word == "set",
            None if b.parents.is_empty() => return false,
            None => {}
        }
        let Some(storage) = Self::own_entry(b, self.detail("slots")) else { return true; };
        let has_slot = match storage.settled() {
            Value::Tuple(items) | Value::Vector(items) => items.iter().any(|v| matches!(v, Value::Text(s) if &**s == "__weakref__")),
            Value::Text(name) => &*name == "__weakref__",
            _ => false
        };
        has_slot || b.parents.iter().any(|parent| self.admits_weak(parent))
    }
    fn install_namespace(&self, t: &Rc<Thing>, replacement: Option<Value>) -> Res {
        if let Some(Value::Attributes(view)) = &replacement { if Rc::ptr_eq(t,view) { return Ok(Value::Nil); } }
        let fresh:Vec<(String,Value)>=match replacement.as_ref().map(|item| Self::underlying(item).unwrap_or_else(|| item.clone()).settled()) {
            None=>Vec::new(),
            Some(Value::Dict(entries))=>entries.iter().filter_map(|(k,v)|if let Value::Text(k)=k{Some((k.to_string(),v.clone()))}else{None}).collect(),
            Some(Value::Attributes(view))=>view.holds.borrow().iter().filter(|(name,_)|!name.starts_with('\0')).cloned().collect(),
            Some(other)=>return Err(self.namespace_refused(&other)),
        };
        let mut holds=t.holds.borrow_mut();
        holds.retain(|(k,_)| k.starts_with('\0') && k != "\0dictionary");
        match replacement { Some(mapping) => holds.push(("\0dictionary".to_owned(), mapping.keep(true))), None => holds.push(("\0dictionary".to_owned(), Value::Mutable(Rc::new(RefCell::new(Value::Dict(Rc::new(Vec::new().into())))), true))) }
        let _ = fresh;
        return Ok(Value::Nil);
    }
    fn allowed_slot(&self,b:&Blueprint,key:&str)->bool {
        if self.table.has_any("ext.stmt.class.builder") && self.ancestor.as_ref().is_some_and(|root| std::ptr::eq(root.as_ref(),b)) { return false; }
        if matches!(Self::native_word(b).as_deref(), Some("SimpleNamespace" | "module")) { return true; }
        let Some(declared)=Self::own_entry(b,self.detail("slots"))else{return Self::native_word(b).is_none();};
        let slots=declared.settled();
        let matching=|x:&Value|matches!(x,Value::Text(s) if s.as_ref()==key||s.as_ref()==self.detail("namespace"));
        if match &slots{Value::Tuple(s)|Value::Vector(s)=>s.iter().any(matching),v=>matching(v)}{return true;}
        b.parents.iter().any(|p|p.name!=self.detail("root")&&self.allowed_slot(p,key))
    }
    pub(super) fn change_entry(entries:&mut Vec<(String,Value)>,key:&str,replacement:Option<Value>)->bool {
        if let Some(i)=entries.iter().position(|(k,_)|k==key){
            if let Some(v)=replacement{entries[i].1=v;}else{entries.remove(i);}true
        }else if let Some(v)=replacement{entries.push((key.to_owned(),v));true}else{false}
    }
    pub(super) fn alter_class_member(&mut self,subject:Value,key:&str,replacement:Option<Value>,direct:bool)->Res {
        // Assigning or removing the abstract marker invalidates only this class.
        if self.rules.words_ext_stmt_class_detail_abstract.first().is_some_and(|name| name == key) {
            if let Value::Blueprint(class) = subject.settled() { self.unmarked_classes.borrow_mut().remove(&(Rc::as_ptr(&class) as usize)); }
        }
        if let Value::Backtrace(link) = &subject {
            if self.rules.trace_words.get(2).map_or(false, |n| n == key) {
                return match replacement {
                    None => Err(format!("TypeError: can't delete {key} attribute").into()),
                    Some(Value::Nil) => { *link.following.borrow_mut() = Value::Nil; Ok(Value::Nil) }
                    Some(fresh @ Value::Backtrace(_)) => {
                        if Self::trace_reaches(&fresh, link) { return Err("ValueError: traceback loop detected".to_owned().into()); }
                        *link.following.borrow_mut() = fresh;
                        Ok(Value::Nil)
                    }
                    Some(other) => Err(format!("TypeError: expected traceback object, got '{}'", other.settled().kind_word()).into()),
                };
            }
            return Err(format!("AttributeError: 'traceback' object attribute '{key}' is read-only").into());
        }
        // Native lowering must still honor namespace changes made after import.
        if self.rules.names_shadow_builtins && self.table.prims.contains_key(key) {
            if let Some(module) = self.namespace_holding(&subject) {
                if self.rules.words_ext_system_names_module.first().is_some_and(|id| id == &module) {
                    let original = self.table.prims.get(key).copied();
                    match replacement.as_ref().map(Value::settled) {
                        Some(Value::Intrinsic(op, _)) if Some(op) == original => { self.displaced_primitives.remove(key); }
                        Some(Value::OctetKind { changeable, .. }) if matches!(original,
                            Some(Prim::Octets(tag @ 0..=1)) if changeable == (tag != 0)) => {
                            self.displaced_primitives.remove(key);
                        }
                        _ => { self.displaced_primitives.insert(key.to_owned()); }
                    }
                }
                self.revised_namespaces.extend(self.loaded_spaces.iter()
                    .filter(|(_, owner)| *owner == &module).map(|(site, _)| site.clone()));
                self.wildcard_source.borrow_mut().take();
            }
        }
        if let Value::Thing(object) = &subject {
            if let Some(kind) = Self::native_word(&object.blueprint()).filter(|kind| matches!(kind.as_str(), "TypeVar" | "ParamSpec" | "TypeVarTuple" | "TypeAliasType" | "NoDefaultType")) {
                if key == "__name__" || kind == "ParamSpec" && key == "__bound__" { return Err(String::from("AttributeError: readonly attribute").into()); }
                let fixed = key == "__default__" || kind != "TypeVarTuple" && ["__bound__", "__constraints__", "__covariant__", "__contravariant__", "__infer_variance__"].contains(&key);
                if fixed || kind == "TypeAliasType" || kind == "NoDefaultType" { return Err(format!("AttributeError: attribute '{key}' of 'typing.{kind}' objects is not writable").into()); }
                if !Self::change_entry(&mut object.holds.borrow_mut(), key, replacement) { return Err(self.absent_attribute(&subject, key)); }
                return Ok(Value::Nil);
            }
        }
        if !self.rules.words_ext_builtin_weak_get.is_empty() {
            if let Value::Thing(t) = subject.settled() {
                if ["ProxyType", "CallableProxyType"].contains(&t.blueprint().name.as_str()) {
                    let held = t.holds.borrow().iter().find_map(|(name, v)| (name == "\0weak").then(|| v.clone()));
                    if let Some(Value::Dim(reference)) = held {
                        let target = reference.ghost.revive().ok_or_else(|| Escape::Error("ReferenceError: weakly-referenced object no longer exists".to_owned()))?;
                        return self.alter_class_member(target, key, replacement, direct);
                    }
                }
            }
        }

        if !self.table.strings("ext.stmt.class.detail.name").is_empty() && !matches!(&subject, Value::Blueprint(_)) {
            if let Some(kind) = self.kind_word_of(&subject.settled()) {
                return Err(format!("TypeError: cannot set '{key}' attribute of immutable type '{kind}'").into());
            }
        }
        if matches!(&subject, Value::Wrapped(62, _)) {
            let known = self.attribute(&subject, key).is_some() || ["__class__", "__doc__"].contains(&key);
            let message = if known { format!("AttributeError: '{}' object attribute '{}' is read-only", subject.kind_word(), key) }
                else { format!("AttributeError: '{}' object has no attribute '{}' and no __dict__ for setting new attributes", subject.kind_word(), key) };
            return Err(message.into());
        }
        if let Value::Thing(instance) = &subject {
            if instance.holds.borrow().iter().any(|entry| entry.0 == "\0immutable") {
                return Err(format!("AttributeError: '{}' object is immutable", subject.kind_word()).into());
            }
        }
        if let Value::Generator(g) = &subject {
            if self.table.strings("ext.stmt.yield.running").first().map(String::as_str) == Some(key) {
                let words = self.table.strings("ext.stmt.class.detail.method.fixed");
                return Err([words[0].as_str(), key, words[1].as_str(), &subject.kind_word(), words[2].as_str()].concat().into());
            }
            let slot = ["name", "qualified"].iter().position(|part| key == self.detail(part));
            if let Some(slot) = slot {
                match replacement.as_ref().map(Value::settled) {
                    Some(Value::Text(text)) => {
                        g.try_borrow_mut().map_err(|_| self.table.strings("ext.stmt.yield.busy")[0].clone())?.titles[slot] = text.to_string();
                        return Ok(Value::Nil);
                    }
                    _ => {
                        let refusal = self.table.strings("ext.stmt.class.detail.text.amiss");
                        return Err(format!("{}{}{}", refusal[0], key, refusal[1]).into());
                    }
                }
            }
        }
        if self.rules.words_ext_builtin_exceptions_traceback_member.first().map(String::as_str) == Some(key) {
            if let Value::Thing(thing) = &subject {
                if self.is_fault_kind(&thing.blueprint()) {
                    if replacement.is_none() { return Err(String::from("TypeError: __traceback__ may not be deleted").into()); }
                    if !matches!(&replacement, Some(Value::Nil | Value::Backtrace(_))) { return Err(String::from("TypeError: __traceback__ must be a traceback or None").into()); }
                }
            }
        }
        let mut replacement = replacement.map(|worth| {
            if matches!(worth, Value::Dict(_)) { worth.keep(true) } else { worth }
        });
        if let Value::Thing(t) = &subject {
            if self.is_fault_kind(&t.blueprint()) {
                if (self.table.single("ext.builtin.exceptions.group.message") == Some(key) || self.table.single("ext.builtin.exceptions.group.members") == Some(key))
                    && t.blueprint().has_public_field("\0gathers") {
                    return Err(format!("AttributeError: attribute '{key}' of '{}' objects is not writable", t.blueprint().name).into());
                }
                for (label, description) in [("ext.builtin.exceptions.cause", "cause"), ("ext.builtin.exceptions.context", "context")] {
                    if self.table.single(label) != Some(key) { continue; }
                    let Some(v) = &replacement else { return Err(format!("TypeError: {key} may not be deleted").into()); };
                    let valid = match v.settled() {
                        Value::Nil => true,
                        Value::Thing(e) => self.is_fault_kind(&e.blueprint()),
                        _ => false,
                    };
                    if !valid { return Err(format!("TypeError: exception {description} must be None or derive from BaseException").into()); }
                }
                if self.table.strings("ext.builtin.exceptions.suppress").first().map(String::as_str) == Some(key) {
                    if replacement.is_none() { return Err(String::from("TypeError: can't delete numeric/char attribute").into()); }
                    if !matches!(replacement.as_ref().map(Value::settled), Some(Value::Flag(_))) {
                        return Err(String::from("TypeError: attribute value type must be bool").into());
                    }
                }
                if replacement.is_none() {
                    if self.rules.words_ext_builtin_exceptions_args.first().map(String::as_str) == Some(key) { return Err(format!("TypeError: {key} may not be deleted").into()); }
                    let reset = match key {
                        "msg" | "filename" | "lineno" | "offset" | "text" | "end_lineno" | "end_offset" | "print_file_and_line" | "_metadata" if self.stands_under(&t.blueprint(), 36) => true,
                        "msg" | "name" | "path" | "name_from" if self.stands_under(&t.blueprint(), 19) => true,
                        "code" if self.stands_under(&t.blueprint(), 17) => true,
                        "value" if self.is_stop_kind(&t.blueprint()) => true,
                        "name" if self.stands_under(&t.blueprint(), 10) || self.stands_under(&t.blueprint(), 12) => true,
                        "obj" if self.stands_under(&t.blueprint(), 12) => true,
                        "encoding" | "object" | "reason" if (43..=45).any(|i| self.stands_under(&t.blueprint(), i)) => true,
                        _ => self.stands_under(&t.blueprint(), 20) && (key == "filename2" || self.table.strings("ext.builtin.exceptions.os").iter().any(|n| n == key)),
                    };
                    if reset { replacement = Some(Value::Nil); }
                }
            }
        }
        if let Value::Thing(instance)=&subject {
            if Self::native_word(&instance.blueprint()).as_deref()==Some("struct_time") {
                let member=instance.holds.borrow().iter().any(|(field,_)|field==key&&!field.starts_with('\0'));
                return Err(if member {String::from("AttributeError: readonly attribute")}else{format!("AttributeError: 'time.struct_time' object has no attribute '{key}'")}.into());
            }
        }
        let writing=replacement.is_some();
        // A singleton's class is as fixed as the singleton itself,
        // by builtin as by statement.
        if matches!(subject.settled(), Value::Refusal(_) | Value::Ellipsis) && key==self.detail("kind") {
            return Err(self.detail(if writing{"kind.fixed"}else{"kind.kept"}).to_owned().into());
        }
        let success=match &subject {
            Value::Thing(t)=>{
                let named_sequence = t.holds.borrow().iter().any(|entry| entry.0 == "\0named-sequence");
                if named_sequence {
                    if key.starts_with("st_") {return Err("AttributeError: readonly attribute".to_string().into());}
                    return Err(format!("AttributeError: 'os.stat_result' object has no attribute '{}' and no __dict__ for setting new attributes", key).into());
                }
                if self.namespace_holding(&subject).is_some() || Self::native_beneath(&t.blueprint()).as_deref() == Some("module") {
                    let evaluator = self.rules.words_ext_stmt_class_detail_code_fields.get(10).cloned().unwrap_or_default();
                    if key == evaluator {
                        match replacement.as_ref() {
                            None => return Err(String::from("TypeError: cannot delete __annotate__ attribute").into()),
                            Some(value) if !matches!(value.settled(), Value::Nil) => {
                                if !self.work_on_class(2, vec![value.clone()])?.is_true() { return Err(String::from("TypeError: __annotate__ must be callable or None").into()); }
                                if let Some(annotation) = self.rules.words_ext_stmt_class_annotations.first().map(String::as_str) {
                                    t.holds.borrow_mut().retain(|entry| entry.0 != annotation);
                                    let book = t.holds.borrow().iter().find(|entry| entry.0 == "\0dictionary").map(|entry| entry.1.clone());
                                    if let Some(book) = book { Self::write_into_book(&book, annotation, None); }
                                }
                            }
                            _ => {},
                        }
                    } else if self.rules.words_ext_stmt_class_annotations.first().map(String::as_str) == Some(key) {
                        let book = t.holds.borrow().iter().find(|entry| entry.0 == "\0dictionary").map(|entry| entry.1.clone());
                        if let Some(book) = book { Self::write_into_book(&book, &evaluator, Some(Value::Nil)); }
                        else { Self::change_entry(&mut t.holds.borrow_mut(), &evaluator, Some(Value::Nil)); }
                    }
                }


                if self.is_fault_kind(&t.blueprint()) && self.rules.words_ext_builtin_exceptions_args.first().map(String::as_str) == Some(key) {
                    if let Some(supplied) = replacement.as_ref() {
                        let kept = match supplied.settled() {
                            Value::Arguments(items) | Value::Tuple(items) | Value::Vector(items) => items,
                            other => return Err(format!("TypeError: '{}' object is not iterable", other.kind_word()).into()),
                        };
                        let mut storage = t.holds.borrow_mut();
                        Self::change_entry(&mut storage, key, Some(Value::Tuple(kept.clone())));
                        Self::change_entry(&mut storage, "\0raised-values", Some(Value::Arguments(kept)));
                        return Ok(Value::Nil);
                    }
                }
                if !direct{
                    let hook=self.detail(if replacement.is_some(){"set"}else{"remove"});
                    if let Some(f)=self.inherited_entry(&t.blueprint(),hook){let mut args=vec![subject.clone(),Value::text(key)];args.extend(replacement);return self.apply_class_member(f,args);}
                    if let Some(Value::Wrapped(6,property))=self.inherited_entry(&t.blueprint(),key){
                        if let (Some(setter),Some(v))=(property.get(1),replacement.clone()){return self.apply_class_member(setter.clone(),vec![subject.clone(),v]);}
                        return Err(self.absent_attribute(&subject,key));
                    }
                }
                if !self.default_attribute_slots(&t.blueprint()) {
                    let operation = if replacement.is_some() { "assign to" } else { "del" };
                    let read_only = self.table.strings("ext.stmt.class.reader").first().is_some_and(|name| self.inherited_entry(&t.blueprint(),name).is_some());
                    let availability = if read_only { "only read-only attributes" } else { "no attributes" };
                    return Err(format!("TypeError: '{}' object has {} ({} .{})", t.blueprint().name, availability, operation, key).into());
                }
                // An entry that takes writes takes this one: a slot keeps
                // the worth, a property's kept accessor will not have it,
                // and any other asks its blueprint's writer or remover.
                if let Some(entry)=self.inherited_entry(&t.blueprint(),key) {
                    match &entry {
                        Value::Wrapped(32,parts)=>return self.slot_change(&subject,parts,replacement),
                        // A property's own document string is kept among
                        // its fields and takes a write; its accessors do
                        // not, as CPython keeps them read-only.
                        Value::Wrapped(58,parts)=>{
                            let place=parts.first().map_or(String::new(),|p|p.bare());
                            if place=="\0doc" {
                                let mut holds=t.holds.borrow_mut();
                                Self::change_entry(&mut holds,"\0doc",replacement);
                                return Ok(Value::Nil);
                            }
                            if place=="\0name" {
                                let mut holds=t.holds.borrow_mut();
                                Self::change_entry(&mut holds,"\0name",replacement);
                                return Ok(Value::Nil);
                            }
                            return Err(self.detail("property.readonly").to_owned().into());
                        }
                        _=>{}
                    }
                    if self.writes_too(&entry) {
                        let part=if replacement.is_some(){"descriptor.set"}else{"descriptor.delete"};
                        let hook=self.protocol_entry(&entry,part).ok_or_else(||self.absent_attribute(&subject,key))?;
                        let mut given=vec![subject.clone()];given.extend(replacement);
                        self.through_descriptor(&entry,hook,given)?;
                        return Ok(Value::Nil);
                    }
                }
                if key == self.detail("namespace") && self.namespace_holding(&subject).is_some()
                    && self.inherited_entry(&t.blueprint(), key).is_none() {
                    return Err(self.detail("property.readonly").to_owned().into());
                }
                if key == self.detail("namespace") && Self::native_beneath(&t.blueprint()).as_deref() == Some("module")
                    && self.inherited_entry(&t.blueprint(), key).is_some() {
                    if !Self::change_entry(&mut t.holds.borrow_mut(), key, replacement) { return Err(self.absent_attribute(&subject, key)); }
                    return Ok(Value::Nil);
                }
                // The thing's own namespace handed back to it, an entry
                // having been put in through it, is already in place.
                if key==self.detail("namespace") {
                    if let Some(Value::Attributes(view))=&replacement {if Rc::ptr_eq(view,t){return Ok(Value::Nil);}}
                }
                // Taking a thing's namespace away leaves an empty one in
                // its place; a dictionary handed over becomes its entries.
                if key==self.detail("namespace"){
                    return self.install_namespace(t, replacement);
                }
                if key == self.detail("kind") {
                    match replacement {
                        Some(Value::Blueprint(next)) => {
                            let previous = t.blueprint();
                            let compatible = Self::own_entry(&previous, self.detail("module")).is_some()
                                && Self::own_entry(&next, self.detail("module")).is_some()
                                && Self::native_beneath(&previous) == Self::native_beneath(&next)
                                && self.storage_shape(&previous) == self.storage_shape(&next);
                            if !compatible { return Err("TypeError: __class__ assignment: object layout differs".to_owned().into()); }
                            let previous_places = self.storage_keys(&previous);
                            let next_places = self.storage_keys(&next);
                            for entry in t.holds.borrow_mut().iter_mut() {
                                if let Some(position) = previous_places.iter().position(|(_, key)| key == &entry.0) {
                                    entry.0.clone_from(&next_places[position].1);
                                }
                            }
                            t.reclassified.replace(Some(next));
                            return Ok(Value::Nil);
                        }
                        _ => return Err("TypeError: __class__ must be set to a class".to_owned().into()),
                    }
                }
                // A loaded namespace keeps each binding in a cell its own
                // code reads through; a new value goes into the cell, and
                // one taken away leaves the cell empty so every reading
                // of the name -- compiled ones included -- finds it gone.
                if self.namespace_holding(&subject).is_some() {
                    self.space_member_mirror(t, key, &replacement);
                    let link = t.holds.borrow().iter().find(|(k, _)| k == key).and_then(|(_, held)| match held { Value::Shared(link) => Some(link.clone()), _ => None });
                    if let (Some(link), Some(v)) = (link.as_ref(), replacement.clone()) { *link.borrow_mut() = self.collection_cell(v.settled()); return Ok(Value::Nil); }
                    if replacement.is_none() {
                        // The cell is emptied first, so the name is gone
                        // from compiled readings of it too; a name that
                        // was never there falls through to the refusal
                        // every other absent member gets.
                        if let Some(link) = link.as_ref() { *link.borrow_mut() = Value::Unset; }
                        if Self::change_entry(&mut t.holds.borrow_mut(), key, None) { return Ok(Value::Nil); }
                    }
                    if let Some(v) = replacement.clone() {
                        // A binding of the module's own put back after
                        // it was taken away goes into the very cell its
                        // code reads, where the name had one of its own.
                        let again = t.holds.borrow().iter().find(|(word, _)| word == "\0bindings").and_then(|(_, table)| match table {
                            Value::Dict(entries) => entries.iter().find(|(name, _)| spells_key(name, key)).map(|(_, link)| link.clone()),
                            _ => None,
                        });
                        if let Some(Value::Shared(cell)) = again {
                            *cell.borrow_mut() = self.collection_cell(v.settled());
                            Self::change_entry(&mut t.holds.borrow_mut(), key, Some(Value::Shared(cell)));
                            return Ok(Value::Nil);
                        }
                    }
                }
                // A blueprint naming the entries its things hold, and
                // holding one of its own under a name not among them,
                // has that name read-only: no write to a thing reaches
                // the blueprint's own holding, and the thing keeps no
                // namespace to put an entry of its own in.
                if self.slots_named(&t.blueprint())&&!self.allowed_slot(&t.blueprint(),key)&&self.inherited_entry(&t.blueprint(),key).is_some() {
                    return Err(self.readonly_attribute(&subject,key));
                }
                if replacement.is_some() && Self::native_beneath(&t.blueprint()).is_none() {
                    let mut held = t.holds.borrow_mut();
                    let compact = held.iter().all(|entry| entry.0 != "\0dictionary" && entry.0 != "\0wide-attributes");
                    if compact && held.iter().filter(|entry| !entry.0.starts_with('\0')).count() >= 30 {
                        // Namespace handles still refer to this owner after its
                        // compact layout has exhausted the available places.
                        held.push(("\0wide-attributes".to_owned(), Value::Flag(true)));
                    }
                }
                let mapping = t.holds.borrow().iter().find(|entry| entry.0 == "\0dictionary").map(|entry| entry.1.clone());
                if let Some(mapping) = mapping {
                    let native = Self::underlying(&mapping).unwrap_or_else(|| mapping.keep(false));
                    if let Some(item) = replacement {
                        let patch = Value::Dict(Rc::new(vec![(Value::text(key), item)].into()));
                        self.value_member(&native, "update", vec![patch], Vec::new())?;
                    } else { self.value_member(&native, "pop", vec![Value::text(key)], Vec::new())?; }
                    return Ok(Value::Nil);
                }
                // A name a module takes from outside that it never bound
                // for itself becomes one of its own globals, standing in
                // the cell its routines read it from, as the reference's
                // module.__dict__ write is a global write.
                if replacement.is_some() && !t.holds.borrow().iter().any(|(k,_)| k==key)
                    && self.imported.values().any(|held| matches!(held, Value::Thing(space) if Rc::ptr_eq(space, t))) {
                    let wanted = format!("\0import/{}/{key}", t.blueprint().name);
                    // A module read in again owns fresh cells at the
                    // end of the roster; the newest of the name answers.
                    if let Some(at) = self.idents.iter().rposition(|word| word == &wanted) {
                        let linked = Value::Shared(Rc::new(RefCell::new(replacement.clone().unwrap())));
                        self.outermost.cells.borrow_mut()[at] = linked.clone();
                        t.holds.borrow_mut().push((key.to_owned(), linked));
                        return Ok(Value::Nil);
                    }
                }
                if replacement.is_some()&&!self.allowed_slot(&t.blueprint(),key){false}else{Self::change_entry(&mut t.holds.borrow_mut(),key,replacement)}
            }
            Value::Blueprint(b)=>{
                if !direct {
                    let protocol = if replacement.is_none() { "remove" } else { "set" };
                    if let Some(factory) = Self::builder_over(b) {
                        if let Some(handler) = self.inherited_entry(&factory, self.detail(protocol)) {
                            let called = self.member_binding(handler, Some(subject.clone()), factory)?;
                            let arguments = match replacement {
                                None => vec![Value::text(key)],
                                Some(ref worth) => vec![Value::text(key), worth.clone()],
                            };
                            return self.apply_class_member(called, arguments);
                        }
                    }
                }
                if !self.detail("name").is_empty() && !Self::sealed(b) {
                    if let Some(builder) = Self::builder_over(b) {
                        if let Some(entry) = self.inherited_entry(&builder, key).filter(|entry| self.writes_too(entry)) {
                            let part = if replacement.is_some() { "descriptor.set" } else { "descriptor.delete" };
                            let hook = self.protocol_entry(&entry, part).ok_or_else(|| Escape::Error(format!("AttributeError: {}", self.detail(part))))?;
                            let mut values = vec![subject.clone()]; values.extend(replacement);
                            self.through_descriptor(&entry, hook, values)?;
                            return Ok(Value::Nil);
                        }
                    }
                }
                // A class the seal marked unchangeable takes no write to
                // a member of it, setting one and taking one off alike.
                if Self::sealed(b) || b.constants.iter().any(|(label, _)| label == "\0native-name") || (b.type_names.borrow().is_none() && ["name", "qualified", "doc"].iter().any(|part| !self.detail(part).is_empty() && key == self.detail(part))) {
                    return Err(format!("TypeError: cannot set '{key}' attribute of immutable type '{}'", b.name).into());
                }
                // The kinds of the two named singletons take no entry
                // of their own and give none up, as the reference fixes them.
                if matches!(Self::native_word(b).as_deref(),Some("NotImplementedType")|Some("ellipsis")) {
                    return Err(format!("TypeError: cannot set '{key}' attribute of immutable type '{}'", b.name).into());
                }
                if b.type_names.borrow().is_some() && key == self.detail("doc") && replacement.is_none() {
                    return Err(format!("TypeError: cannot delete '{key}' attribute of immutable type '{}'", b.type_names.borrow().as_ref().unwrap().short.type_text().bare()).into());
                }
                if b.type_names.borrow().is_some() && [self.detail("name"), self.detail("qualified")].contains(&key) {
                    let Some(handed) = replacement.as_ref() else { return Err(format!("TypeError: cannot delete '{key}' attribute of immutable type '{}'", b.type_names.borrow().as_ref().unwrap().short.type_text().bare()).into()); };
                    let text = handed.type_text();
                    if !matches!(text, Value::Text(_) | Value::Unpaired(_)) { return Err(format!("TypeError: can only assign string to {}.{key}, not '{}'", b.type_names.borrow().as_ref().unwrap().short.type_text().bare(), Self::type_argument_kind(handed)).into()); }
                    if key == self.detail("qualified") { b.type_names.borrow_mut().as_mut().unwrap().full = handed.settled(); }
                    else { self.checked_type_name(&text)?; b.type_names.borrow_mut().as_mut().unwrap().short = handed.settled(); }
                    return Ok(Value::Nil);
                }
                if !self.detail("base").is_empty()&&key==self.detail("base") {
                    if let Some(entry)=Self::builder_over(b).and_then(|owner|self.inherited_entry(&owner,key)) {
                        if self.writes_too(&entry) {
                            let action=if replacement.is_some(){"descriptor.set"}else{"descriptor.delete"};
                            let method=self.protocol_entry(&entry,action).ok_or_else(||self.absent_attribute(&subject,key))?;
                            let mut handed=vec![subject.clone()];handed.extend(replacement);
                            return self.through_descriptor(&entry,method,handed);
                        }
                        if !Self::change_entry(&mut b.shared.borrow_mut(),key,replacement){return Err(self.absent_attribute(&subject,key));}
                        return Ok(Value::Nil);
                    }
                    return Err("AttributeError: readonly attribute".to_owned().into());
                }
                if key == self.detail("bases") && b.type_names.borrow().is_some() {
                    if let Some(Value::Tuple(proposed)) = replacement.as_ref().map(Value::settled) {
                        let mut normalized = Vec::new();
                        for item in proposed.iter() { normalized.push(self.parent_from_type(item)?); }
                        let unchanged = normalized.len() == b.parents.len() && normalized.iter().zip(&b.parents).all(|(next, previous)| Rc::ptr_eq(next, previous));
                        if unchanged { return Ok(Value::Nil); }
                    }
                }
                if key == self.detail("qualified") {
                    if let Some(worth) = replacement.as_ref().map(Value::settled) {
                        if !matches!(worth, Value::Text(_)) { return Err(format!("TypeError: can only assign string to {}.__qualname__, not '{}'", b.name, worth.kind_word()).into()); }
                    } else { return Err(self.class_unready()); }
                } else if ["name","kind","base","bases","mro","namespace","order"].iter().any(|part|key==self.detail(part)){return Err(self.class_unready());}
                let annotation_function = self.rules.words_ext_stmt_class_detail_code_fields.get(10).cloned().unwrap_or_default();
                if !annotation_function.is_empty() && key == annotation_function {
                    match replacement.as_ref() {
                        None => return Err(String::from("TypeError: cannot delete __annotate__ attribute").into()),
                        Some(value) if !matches!(value.settled(), Value::Nil) => {
                            if !self.work_on_class(2, vec![value.clone()])?.is_true() { return Err(String::from("TypeError: __annotate__ must be callable or None").into()); }
                            if let Some(annotation) = self.rules.words_ext_stmt_class_annotations.first().map(String::as_str) { b.shared.borrow_mut().retain(|entry| entry.0 != annotation); }
                        }
                        _ => {},
                    }
                }
                if key == self.detail("type_params") && replacement.is_none() {
                    return Err(String::from("TypeError: cannot delete '__type_params__' attribute of immutable type").into());
                }
                if key == self.detail("module") {
                    b.shared.borrow_mut().retain(|entry| entry.0 != "__firstlineno__");
                }
                if self.rules.words_ext_stmt_class_annotations.first().map(String::as_str) == Some(key) {
                    b.shared.borrow_mut().retain(|entry| entry.0 != "__annotate_func__");
                    let evaluator = self.rules.words_ext_stmt_class_detail_code_fields.get(10).cloned().unwrap_or_default();
                    Self::change_entry(&mut b.shared.borrow_mut(), &evaluator, Some(Value::Nil));
                }
                Self::change_entry(&mut b.shared.borrow_mut(),key,replacement)
            },
            Value::Routine(_)|Value::Bound(..)=>{
                if key == self.detail("namespace") {
                    let at = self.routine_storage(&subject);
                    if let Some(Value::Attributes(storage)) = &replacement {
                        if Rc::ptr_eq(storage, &self.routine_members[at].1) { return Ok(Value::Nil); }
                    }
                    let Some(handed)=replacement else{return Err(self.detail("namespace.kept").to_owned().into())};
                    if !matches!(handed.settled(),Value::Dict(_)){return Err(self.namespace_refused(&handed.settled()));}
                    let mut holds=self.routine_members[at].1.holds.borrow_mut();
                    holds.retain(|(k,_)|k.ends_with('\0'));
                    holds.push((Self::HANDED.to_owned(),handed));
                    return Ok(Value::Nil);
                }
                if key==self.detail("globals")||key==self.detail("closure"){return Err(self.detail("property.readonly").to_owned().into());}
                // The builtins a routine reaches its unbound names
                // through are read off it, never written over.
                if self.table.strings("ext.system.module.builtins").iter().any(|word| word==key){return Err(self.detail("property.readonly").to_owned().into());}
                if key==self.detail("code")||key==self.detail("defaults")||key==self.detail("keywords"){
                    self.write_routine_over(&subject,key,replacement)?;
                    return Ok(Value::Nil);
                }
                let evaluator_key = self.rules.words_ext_stmt_class_detail_code_fields.get(10).cloned().unwrap_or_default();
                if !evaluator_key.is_empty() && key == evaluator_key {
                    let incoming = match replacement { Some(v) => v, None => return Err("TypeError: __annotate__ cannot be deleted".to_owned().into()) };
                    if !matches!(incoming.settled(), Value::Nil) && !self.work_on_class(2, vec![incoming.clone()])?.is_true() {
                        return Err("TypeError: __annotate__ must be callable or None".to_owned().into());
                    }
                    let index = self.routine_storage(&subject);
                    let mut holds = self.routine_members[index].1.holds.borrow_mut();
                    if !matches!(incoming.settled(), Value::Nil) {
                        if let Some(annotation) = self.rules.words_ext_stmt_class_annotations.first() { holds.retain(|(word, _)| word != &format!("\0{annotation}\0")); }
                    }
                    Self::change_entry(&mut holds, &format!("\0{key}\0"), Some(incoming));
                    return Ok(Value::Nil);
                }
                if self.rules.words_ext_stmt_class_annotations.first().map_or(false, |s| s == key) {
                    let item = match replacement.as_ref().map(Value::settled) {
                        Some(Value::Dict(_)) => replacement.unwrap(),
                        None | Some(Value::Nil) => self.collection_cell(Value::Dict(Rc::new(Vec::new().into()))),
                        _ => return Err(format!("TypeError: {key} must be set to a dict object").into()),
                    };
                    let index = self.routine_storage(&subject);
                    let mut holds = self.routine_members[index].1.holds.borrow_mut();
                    Self::change_entry(&mut holds, &format!("\0{key}\0"), Some(item));
                    Self::change_entry(&mut holds, &format!("\0{evaluator_key}\0"), Some(Value::Nil));
                    return Ok(Value::Nil);
                }
                if key==self.detail("type_params") {
                    // The row of type parameters takes a row and nothing
                    // else, and is never taken away.
                    if !matches!(replacement.as_ref().map(Value::settled),Some(Value::Tuple(_))) {
                        return Err(self.detail("defaults.amiss").to_owned().into());
                    }
                    let index=self.routine_storage(&subject);
                    Self::change_entry(&mut self.routine_members[index].1.holds.borrow_mut(), &format!("\0{key}\0"), replacement);
                    return Ok(Value::Nil);
                }
                // The name and the full name take text and nothing else;
                // they and the account of the routine stand apart from its
                // namespace, and the account taken away is none.
                let named=key==self.detail("name")||key==self.detail("qualified");
                if named&&!matches!(replacement.as_ref().map(Value::settled),Some(Value::Text(_))) {
                    let words=self.table.strings("ext.stmt.class.detail.text.amiss");
                    return Err(if words.len()==2{format!("{}{key}{}",words[0],words[1]).into()}else{self.class_unready()});
                }
                let index=self.routine_storage(&subject);
                if named||key==self.detail("doc")||key==self.detail("module") {
                    let apart=format!("{key}\0");
                    Self::change_entry(&mut self.routine_members[index].1.holds.borrow_mut(),&apart,Some(replacement.unwrap_or(Value::Nil)));
                    return Ok(Value::Nil);
                }
                let handed=self.routine_members[index].1.holds.borrow().iter().find(|(k,_)|k==Self::HANDED).map(|(_,v)|v.clone());
                match handed {
                    Some(book)=>Self::write_into_book(&book,key,replacement),
                    None=>Self::change_entry(&mut self.routine_members[index].1.holds.borrow_mut(),key,replacement),
                }
            }
            // A cell takes what it holds, and holds nothing once that is
            // taken away.
            Value::Wrapped(35,items) if key==self.detail("cell.contents") => {
                if let [Value::Shared(cell)] = items.as_slice() { *cell.borrow_mut() = replacement.unwrap_or(Value::Unset); return Ok(Value::Nil); }
                let Some((room,at))=self.cell_place(items) else{return Err(self.absent_attribute(&subject,key))};
                let value=replacement.unwrap_or(Value::Unset);
                if room.capture_slots.borrow().contains(&at) {
                    let Value::Shared(binding)=&room.cells.borrow()[at] else { unreachable!() };
                    *binding.borrow_mut()=value;
                } else {
                    room.cells.borrow_mut()[at]=value;
                }
                return Ok(Value::Nil);
            }
            // A method holds nothing of its own: its thing and routine
            // are fixed, its account is its routine's, and nothing else
            // can be written into it or taken out.
            Value::Method(..)|Value::Wrapped(3 | 132,_)=>{
                if key==self.detail("receiver")||key==self.detail("function"){return Err(self.detail("property.readonly").to_owned().into());}
                if key==self.detail("kind"){return Err(self.detail(if writing{"kind.fixed"}else{"kind.kept"}).to_owned().into());}
                if key==self.detail("doc") {
                    let words=self.table.strings("ext.stmt.class.detail.method.fixed");
                    if words.len()==3{return Err(format!("{}{key}{}{}{}",words[0],words[1],subject.kind_word(),words[2]).into());}
                }
                false
            }
            _=>false,
        };
        if success{return Ok(Value::Nil);}
        // A thing whose blueprint names the entries it holds, and a
        // value of a builtin kind, have nowhere to put a new entry.
        // Only a write says so; a taking-away names the member that
        // was never there and no more.
        let nowhere=match &subject {
            Value::Blueprint(_)|Value::Routine(_)|Value::Bound(..)=>false,
            _=>writing,
        };
        if nowhere{return Err(self.unwritable_attribute(&subject,key));}
        Err(self.absent_attribute(&subject,key))
    }
    pub(super) fn parent_from_type(&mut self, parent: &Value) -> Res<Rc<Blueprint>> {
        let settled = parent.settled();
        if self.rules.class_builder && self.spells_property_kind(&settled) { return Ok(self.property_blueprint()); }
        if self.table.has_any("ext.builtin.bool.base") && matches!(settled, Value::Intrinsic(Prim::Truthful, _)) { return Err(self.table.single("ext.builtin.bool.base").unwrap_or("").to_owned().into()); }
        if let Value::Blueprint(class) = settled {
            if Self::sealed(&class) {
                let qualified = if class.shared.borrow().iter().any(|(word, bit)| word == "\0buffer_allocator" && bit.is_true()) {
                    let home = Self::own_entry(&class, self.detail("module")).map(|v| v.bare()).unwrap_or_default();
                    format!("{home}.{}", class.name)
                } else { match class.name.as_str() {
                    "ProxyType" | "CallableProxyType" if self.rules.weak_members => "weakref.".to_owned() + &class.name,
                    _ => class.name.clone(),
                } };
                return Err(format!("TypeError: type '{qualified}' is not an acceptable base type").into());
            }
            return Ok(class);
        }
        if let Value::OctetKind { changeable, .. } = &settled {
            let word = self.octet_kind_word(*changeable).to_owned();
            if self.table.spells("ext.stmt.class.builtin", &word) { return Ok(self.native_kind(&word)); }
        }
        if self.spells_property_kind(&settled) { return Ok(self.property_blueprint()); }
        if let Value::Intrinsic(operation, word) = settled {
            if operation == Prim::SortOf { return Ok(self.builder_blueprint()); }
            if operation == Prim::Truthful {return Err(self.rules.words_ext_builtin_bool_base.first().map(String::as_str).unwrap_or("TypeError: bases must be types").to_owned().into());}
            if operation != Prim::Truthful && (self.table.spells("ext.stmt.class.builtin", &word) || (!self.rules.words_ext_stmt_class_builder.is_empty() && matches!(operation, Prim::ClassWork(9..=11)))) { return Ok(self.native_kind(&word)); }
        }
        if !self.rules.words_ext_stmt_class_builder.is_empty() {
            let word = parent.kind_it_names().map(Rc::from).or_else(|| self.kind_spelling(parent));
            if let Some(word) = word {
                if self.table.spells("ext.stmt.class.builtin", &word) || matches!(self.table.prims.get(word.as_ref()), Some(Prim::ClassWork(9..=11))) { return Ok(self.native_kind(&word)); }
            }
        }
        Err("TypeError: bases must be types".to_owned().into())
    }
    pub(super) fn class_from_type(&mut self,values:Vec<Value>)->Res {
        let (args, options) = self.open_arguments(values)?;
        let mut values: Vec<Value> = args.iter().map(Value::settled).collect();
        if values.len() != 3 && !options.is_empty() { return Err("TypeError: type() takes 1 or 3 arguments".to_owned().into()); }
        let options: Vec<Value> = options.into_iter().map(|(word, value)| Value::Couple(Rc::new((Value::text(&word), value)))).collect();
        if values.len() == 3 {
            self.checked_type_name(&values[0])?;
            if let Some(native) = Self::underlying(&values[2]) {
                if matches!(native.settled(), Value::Dict(_)) {
                    let namespace = values[2].clone();
                    let keys_method = self.read_class_member(namespace.clone(), "keys", false)?;
                    let keys_view = self.apply_class_member(keys_method, Vec::new())?;
                    let mut rows = Vec::new();
                    for name in self.object_members(&keys_view)? {
                        let held = self.prim(Prim::At, "", &[namespace.clone(), name.clone()])?;
                        rows.push((name, held));
                    }
                    values[2] = Value::Dict(Rc::new(rows.into()));
                } else { values[2] = native.settled(); }
            }
            else {
                let rows = match &values[2] {
                    Value::Thing(t) if t.blueprint().name == "frozendict" => t.holds.borrow().iter().find(|entry| entry.0 == "_rows").map(|entry| entry.1.settled()),
                    _ => None,
                };
                if let Some(rows) = rows { values[2] = rows; }
            }
        }
        // A namespace read in is a thing like any other, but the
        // reference knows it by the one word every namespace shares and
        // not by the name that namespace goes by.
        if values.len()==1 && self.namespace_holding(&values[0]).is_some() {return Ok(self.kind_named_after(&values[0]));}
        if let [Value::Thing(t)]=values.as_slice() {
            if self.property_kind.as_ref().is_some_and(|kind|Rc::ptr_eq(kind,&t.blueprint())) { return Ok(Value::Intrinsic(Prim::ClassWork(11),Rc::from(t.blueprint().name.as_str()))); }
            return Ok(Value::Blueprint(t.blueprint().clone()));
        }
        if values.len()==1 && self.is_async_generator(&values[0]) { return Ok(Value::Blueprint(self.native_kind("async_generator"))); }
        // A routine and a method are of kinds the table does not name,
        // so each takes the word the reference gives its kind; an
        // intrinsic word read as a class is of the kind builder's kind.
        if let [Value::Routine(_)|Value::Bound(..)|Value::Method(..)]=values.as_slice(){return Ok(self.kind_named_after(&values[0]));}
        // A method or a data member read off a native kind's own word
        // is of the descriptor kind CPython gives it.
        if let [Value::Wrapped(tag,_)]=values.as_slice(){if *tag != 8 { return Ok(self.kind_named_after(&values[0])); }}
        if values.len()==1 && self.kind_spelling(&values[0]).is_some() {return Ok(self.kind_builder_word());}
        if matches!(values.as_slice(), [Value::Wrapped(..)]) { return Ok(self.kind_named_after(&values[0])); }
        // A class is of the kind that built it: the metaclass named for
        // it or for a class it is built on, and otherwise the kind
        // primitive itself, under whatever word the table spells it by.
        if values.len()==1 {if let Value::Blueprint(b)=&values[0]{
            if let Some(builder)=Self::builder_over(b){return Ok(Value::Blueprint(builder));}
            return Ok(self.kind_builder_word());}}
        if values.len()==3 {
            let mut parts=vec![self.kind_builder_word()];parts.extend(values.iter().cloned());parts.extend(options.iter().cloned());
            let class=self.class_of_parts(parts)?;
            let root=self.builder_blueprint();
            values.extend(options);
            self.finish_class_construction(&class,&root,values)?;
            return Ok(class);
        }
        if values.len()==1 {
            let word=self.table.prims.iter().find(|(_,p)|**p==Prim::SortOf).map(|(w,_)|w.to_string()).unwrap_or_default();
            return Ok(self.prim(Prim::SortOf,&word,&values)?);
        }
        Err("TypeError: type() takes 1 or 3 arguments".to_owned().into())
    }
    /// A class whose metaclass keeps the entry for one of these
    /// questions answers it itself: the entry is read from the
    /// metaclass bound to the class, called with the value, and its
    /// word is taken. A class no metaclass of its own built says
    /// nothing, and the plain reading stands, so the everyday question
    /// costs no more than a look through the constants.
    pub(super) fn builder_answers(&mut self,choice:&Value,given:&Value,class_only:bool)->Result<Option<bool>,Escape>{
        let Value::Blueprint(class)=choice else{return Ok(None);};
        let Some(builder)=Self::builder_over(class) else{return Ok(None);};
        let Some(key)=self.rules.specials.get(if class_only{77}else{76}).cloned() else{return Ok(None);};
        let Some(entry)=self.inherited_entry(&builder,&key) else{return Ok(None);};
        let bound=self.member_binding(entry,Some(choice.clone()),builder)?;
        // A kind word asked about stands before the hook as the kind's
        // own class, the way the reference hands the type itself over.
        let given=match given {
            Value::Wrapped(8,parts) => match parts.first() { Some(Value::Text(word)) => Value::Blueprint(self.native_kind(word)), _ => given.clone() },
            Value::Intrinsic(_, word) if self.table.prims.get(word.as_ref()).copied().map_or(false,|op| Self::names_a_kind(&op)) => Value::Blueprint(self.native_kind(word)),
            Value::OctetKind { changeable, .. } => { let word=self.octet_kind_word(*changeable).to_owned(); Value::Blueprint(self.native_kind(&word)) }
            other => other.clone(),
        };
        let told=self.apply_class_member(bound,vec![given])?;
        Ok(Some(told.is_true()))
    }
    /// The intrinsic words that name a kind of value rather than a piece
    /// of work. Only such a word stands for a class where `issubclass`
    /// and `isinstance` want one; every other intrinsic is as much a
    /// refusal there as a number is.
    pub(super) fn names_a_kind(op:&Prim)->bool { op.names_a_kind() }
    /// The word a value names a kind by, where it names one at all.
    pub(super) fn kind_spelling(&self,value:&Value)->Option<Rc<str>>{
        let Value::Wrapped(8,parts)=value else{return None};
        let Value::Text(word)=&parts[0] else{return None};
        self.table.prims.get(word.as_ref()).filter(|op|Self::names_a_kind(op)).map(|_|word.clone())
    }
    /// Whether a value stands for a kind rather than being one of a
    /// kind: a blueprint, an intrinsic word naming a kind, one of the
    /// two octet kinds, or the worth a plain kind is told by. Each of
    /// these is itself of the kind primitive's kind.
    pub(super) fn stands_for_a_kind(&self,value:&Value)->bool{
        matches!(value,Value::Blueprint(_)|Value::OctetKind{..}|Value::KindOf(_))
            ||matches!(value,Value::Intrinsic(op,_) if op.names_a_kind())
            ||self.kind_spelling(value).is_some()
    }
    /// A union accepts kinds, another union's alternatives, or Nil;
    /// ordinary tuples remain ordinary tuple values.
    pub(super) fn union_member(&self,value:&Value)->bool{
        matches!(value,Value::Nil)
            ||self.stands_for_a_kind(value)
            ||matches!(value,Value::Thing(t) if Self::native_word(&t.blueprint()).as_deref() == Some("Union"))
    }
    /// A kind or union supplies the operator; Nil by itself does not.
    pub(super) fn union_anchor(&self,value:&Value)->bool{
        self.stands_for_a_kind(value)
            ||matches!(value,Value::Thing(t) if Self::native_word(&t.blueprint()).as_deref() == Some("Union"))
    }
    /// The kind primitive read as a worth: what the kind of a kind is.
    pub(super) fn kind_builder_word(&self)->Value{
        match self.table.prims.iter().find(|(_,p)|**p==Prim::SortOf) {
            Some((word,_))=>Value::Intrinsic(Prim::SortOf,Rc::from(word.as_str())),
            None=>Value::Nil,
        }
    }
    /// The blueprint standing for a kind the table has no word of its
    /// own for, named as the reference names that kind. It is built
    /// once and kept, so two askings answer with the very same one.
    pub(super) fn kind_named_after(&mut self,value:&Value)->Value{
        let concrete_module = match value.settled() {
            Value::Thing(object) => {
                let owner = object.blueprint();
                (Self::native_beneath(&owner).as_deref() == Some("module")).then_some(owner)
            }
            _ => None,
        };
        if let Some(owner) = concrete_module { return Value::Blueprint(owner); }
        let word=if matches!(value, Value::Intrinsic(Prim::Textual(_), name) if name.contains('.')) { "method_descriptor".to_owned() } else if self.is_async_generator(value) { String::from("async_generator") } else { match self.namespace_holding(value) {Some(_)=>String::from("module"),None=>value.kind_word()} };
        // A table spelling that very kind answers with its intrinsic
        // word, so that the kind asked for and the kind answered with
        // are one value: `type(enumerate(r)) is enumerate`.
        match self.table.prims.get(word.as_str()).filter(|op|Self::names_a_kind(op)) {
            Some(op)=>Value::Intrinsic(*op,Rc::from(word.as_str())),
            None=>Value::Blueprint(self.native_kind(&word)),
        }
    }
    /// Whether a value is a class at all: one the program laid out, or
    /// an intrinsic word naming a kind.
    fn counts_as_class(&self,value:&Value)->bool{
        matches!(value,Value::Blueprint(_))||self.kind_spelling(value).is_some()
    }
    /// One kind lies under another only where it is that same kind, or
    /// where it is the flag kind, which lies under the whole-number kind
    /// because a flag counts as a number here.
    fn kind_under(&self,under:&str,over:&str)->bool{
        under==over
            || (self.table.prims.get(under)==Some(&Prim::Truthful) && self.table.prims.get(over)==Some(&Prim::AsInt))
    }
    /// Whether a value is of an intrinsic kind. A walk answers by the
    /// name its kind is told by, which is the word that made it.
    pub(super) fn kind_covers(&self,op:&Prim,word:&str,value:&Value)->bool{
        match op {
            Prim::AsInt=>matches!(value,Value::Small(_)|Value::Huge(_)|Value::Flag(_)),
            Prim::AsText=>matches!(value,Value::Text(_) | Value::Unpaired(_)),
            Prim::AsReal=>matches!(value,Value::Frac(r) if r.places.is_some()),
            Prim::Listed=>matches!(value,Value::Vector(_)),
            Prim::SortOf=>self.stands_for_a_kind(value),
            Prim::Dictionary=>matches!(value,Value::Dict(_)|Value::Attributes(_)),
            Prim::Tupling=>matches!(value,Value::Tuple(_)),
            Prim::Uniques=>matches!(value,Value::Set(_))&&!value.set_sealed(),
            Prim::Unchanging=>value.set_sealed(),
            Prim::Truthful=>matches!(value,Value::Flag(_)),
            Prim::ComplexMade=>matches!(value,Value::Complex(_)),
            Prim::Span=>matches!(value,Value::Progression(_)),
            Prim::SpanOf=>matches!(value,Value::Span(_)),
            Prim::Octets(which)=>matches!(value,Value::Octets{changeable,..} if *changeable==(*which==1)),
            Prim::ClassWork(9 | 10) => match value { Value::Wrapped(tag, _) => *tag == if *op == Prim::ClassWork(9) { 4 } else { 5 }, _ => false },
            Prim::Numbered|Prim::Zipped|Prim::Mapped|Prim::Filtered|Prim::Backwards=>value.kind_word()==word,
            _=>false,
        }
    }
    /// The refusal for a subject or a kind that is no class. Where a
    /// language spells no words of its own for it, the class refusal
    /// stands, as it stood before any were spelled.
    pub(super) fn not_a_class(&self,key:&str)->Escape{
        let told=self.core_complaint(key,"");
        if told.is_empty(){self.class_unready()}else{told.into()}
    }
    /// The entry a value keeps under a name where it keeps one, a
    /// missing entry read as the plain absence the reference reads and
    /// any other fault passed on as it stands.
    fn member_if_any(&mut self,value:&Value,key:&str)->Result<Option<Value>,Escape>{
        if key.is_empty(){return Ok(None);}
        match self.read_class_member(value.clone(),key,false){
            Ok(found)=>Ok(Some(found.settled())),
            Err(escape)=>{
                // A reading that raised parks the value while words stand
                // in for it; those words must not be taken for an absent
                // entry, and the parked value is lifted again here.
                if let Some(away)=self.got_away.take(){return Err(away);}
                if self.missing_member_escape(&escape){return Ok(None);}
                Err(escape)
            }
        }
    }
    /// The `__bases__` a value lies under where it keeps a tuple of
    /// them, the reference's own reading of a class a value may stand
    /// for without being a class itself. An entry that is absent, or is
    /// no tuple, ends the walk with nothing.
    fn value_bases(&mut self,value:&Value)->Result<Option<Vec<Value>>,Escape>{
        let key=self.detail("bases").to_owned();
        Ok(match self.member_if_any(value,&key)?{
            Some(Value::Tuple(parts))=>Some(parts.to_vec()),
            _=>None,
        })
    }
    /// The class a value names through its own `__class__`, whether or
    /// not that is the class it was made by.
    fn value_class_named(&mut self,value:&Value)->Result<Option<Value>,Escape>{
        let key=self.detail("kind").to_owned();
        self.member_if_any(value,&key)
    }
    /// The kind word a value names where it names one: an intrinsic
    /// word, the blueprint standing for a kind, or an octet kind.
    fn kind_name(&self,value:&Value)->Option<String>{
        match value{
            Value::Intrinsic(op,word) if op.names_a_kind()=>Some(word.to_string()),
            Value::Wrapped(8,parts)=>match &parts[0]{Value::Text(word)=>Some(word.to_string()),_=>None},
            Value::Blueprint(b)=>Self::native_word(b),
            Value::OctetKind{changeable,..}=>Some(self.octet_kind_word(*changeable).to_owned()),
            _=>None,
        }
    }
    /// Whether two values are the very one thing, which is how a
    /// `__bases__` line is followed: the same blueprint, the same thing,
    /// the same intrinsic under whatever spelling, or the same worth.
    fn same_thing(&self,a:&Value,b:&Value)->bool{
        match (a,b){
            (Value::Blueprint(x),Value::Blueprint(y))=>Rc::ptr_eq(x,y),
            (Value::Thing(x),Value::Thing(y))=>Rc::ptr_eq(x,y),
            (Value::Intrinsic(x,xw),Value::Intrinsic(y,yw))=>x==y&&xw==yw,
            (Value::Nil,Value::Nil)=>true,
            _=>match (self.kind_name(a),self.kind_name(b)){
                (Some(one),Some(two))=>one==two,
                _=>a.equals(b),
            },
        }
    }
    /// The reference's walk along a `__bases__` line from `derived`
    /// towards the very `wanted`: one base is stepped along and does not
    /// grow the count, two or more are each walked, and the whole is
    /// guarded so an endless line raises as the reference's own does.
    fn bases_walk(&mut self,mut derived:Value,wanted:&Value)->Result<bool,Escape>{
        loop{
            if self.same_thing(&derived,wanted){return Ok(true);}
            let Some(bases)=self.value_bases(&derived)? else{return Ok(false);};
            if bases.is_empty(){return Ok(false);}
            if bases.len()==1{derived=bases.into_iter().next().unwrap();continue;}
            self.deeper()?;
            let mut answer=Ok(false);
            for base in bases.iter(){
                match self.bases_walk(base.clone(),wanted){
                    Ok(true)=>{answer=Ok(true);break;}
                    Ok(false)=>{}
                    Err(escape)=>{answer=Err(escape);break;}
                }
            }
            self.standing-=1;
            return answer;
        }
    }
    /// A value that is no class of its own but keeps the question's own
    /// entry answers it, as the reference asks before falling back on
    /// class lines; this is how a stub kind carries its own checking.
    fn member_answers(&mut self,choice:&Value,given:&Value,class_only:bool)->Result<Option<bool>,Escape>{
        let Value::Thing(thing)=choice else{return Ok(None);};
        let holder=thing.blueprint();
        let Some(key)=self.rules.specials.get(if class_only{77}else{76}).cloned() else{return Ok(None);};
        let Some(entry)=self.inherited_entry(&holder,&key) else{return Ok(None);};
        let bound=self.member_binding(entry,Some(choice.clone()),holder)?;
        let told=self.apply_class_member(bound,vec![given.clone()])?;
        Ok(Some(told.is_true()))
    }
    /// The kind word a value reports, where it reports one: a class the
    /// program laid out and the native kind beneath it, a native word,
    /// an octet kind, or the NoneType a bare `None` stands for.
    fn reported_kind_word(&self, value:&Value) -> Option<String> {
        match value {
            Value::Blueprint(b) => Self::native_word(b).or_else(|| Self::native_beneath(b)),
            Value::Intrinsic(op, word) if op.names_a_kind() => Some(word.to_string()),
            Value::Wrapped(8, parts) => match &parts[0] { Value::Text(word) => Some(word.to_string()), _ => None },
            Value::OctetKind { changeable, .. } => Some(self.octet_kind_word(*changeable).to_owned()),
            Value::Nil => Some("NoneType".to_owned()),
            _ => None,
        }
    }
    /// Whether a kind a value reports through `__class__` lies beneath
    /// the kind asked after, in whatever shape each came.
    fn reported_stands_beneath(&self, reported:&Value, wanted:&Value) -> bool {
        if let (Value::Blueprint(a), Value::Blueprint(b)) = (reported, wanted) {
            if Self::fault_descends(a, b) || Self::ancestry_includes(a, b) { return true; }
        }
        match (self.reported_kind_word(reported), self.reported_kind_word(wanted)) {
            (Some(under), Some(over)) => self.kind_under(&under, &over),
            _ => false,
        }
    }
    pub(super) fn namespace_has_kind(&self, subject: &Value, requested: &Rc<Blueprint>) -> bool {
        if self.namespace_holding(subject).is_none() { return false; }
        let configured = self.table.strings("ext.system.module.kind");
        let [path, field] = configured else { return false };
        if let Some(Value::Thing(module)) = self.imported.get(path) {
            for (key, value) in module.holds.borrow().iter() {
                if key == field {
                    if let Value::Blueprint(kind) = value.settled() {
                        return Rc::ptr_eq(&kind, requested);
                    }
                }
            }
        }
        false
    }
    pub(super) fn is_beneath(&mut self,subject:&Value,choice:&Value,class_only:bool)->Result<bool,Escape>{
        if let Value::Mutable(cell, _) | Value::Shared(cell) = subject { let held = cell.borrow().clone(); return self.is_beneath(&held, choice, class_only); }
        if let Value::Mutable(cell, _) | Value::Shared(cell) = choice { let held = cell.borrow().clone(); return self.is_beneath(subject, &held, class_only); }
        // Plain numeric values cannot override __class__ or type hooks.
        // Preserve the native membership rule without wrapping and repeating
        // the full descriptor/metaclass walk for their numeric builtin kinds.
        if !class_only && self.names_in_calls {
            if let (Value::Small(_) | Value::Huge(_) | Value::Flag(_) | Value::Frac(_),
                Value::Intrinsic(operation @ (Prim::AsInt | Prim::AsReal | Prim::Truthful), word)) = (subject, choice) {
                return Ok(self.kind_covers(operation, word, subject));
            }
        }
        // Legacy method wrappers are instances of the same canonical wrapper kinds.
        if !class_only && self.names_in_calls {
            if matches!(subject, Value::Wrapped(4, _)) && self.spells_staticmethod_kind(choice)
                || matches!(subject, Value::Wrapped(5, _)) && self.spells_classmethod_kind(choice) {
                return Ok(true);
            }
        }
        if matches!(choice, Value::Thing(t) if Self::native_word(&t.blueprint()).as_deref() == Some("Union")) {
            let kinds = self.read_class_member(choice.clone(), "__args__", true)?;
            return self.is_beneath(subject, &kinds, class_only);
        }
        if matches!(choice, Value::Thing(alias) if alias.blueprint().name == "GenericAlias") {
            return Err("TypeError: isinstance() argument 2 cannot be a parameterized generic".to_owned().into());
        }
        if let Some(told)=self.builder_answers(choice,subject,class_only)?{return Ok(told);}
        // A side that is still a cell is asked about as whatever the
        // cell is keeping.
        if let Value::Shared(cell) | Value::Mutable(cell, _) = subject { let inner = cell.borrow().clone(); return self.is_beneath(&inner, choice, class_only); }
        if let Value::Shared(cell) | Value::Mutable(cell, _) = choice { let inner = cell.borrow().clone(); return self.is_beneath(subject, &inner, class_only); }
        // The run’s own module objects answer to the native module kind.
        if !class_only && self.namespace_holding(&subject.settled()).is_some() {
            if let (Value::Blueprint(actual), Value::Blueprint(expected)) = (self.kind_named_after(&subject.settled()), choice.settled()) {
                return Ok(Self::ancestry_includes(&actual, &expected));
            }
        }
        if let Some(told)=self.member_answers(choice,subject,class_only)?{return Ok(told);}
        // The byte kinds are values in their own right rather than
        // intrinsic words, so each is asked after under its own word.
        if let Value::OctetKind { changeable, .. } = subject { let word=self.octet_kind_word(*changeable).to_owned(); return self.is_beneath(&Value::Wrapped(8, Rc::new(vec![Value::text(&word)]).into()), choice, class_only); }
        if let Value::OctetKind { changeable, .. } = choice { let word=self.octet_kind_word(*changeable).to_owned(); return self.is_beneath(subject, &Value::Wrapped(8, Rc::new(vec![Value::text(&word)]).into()), class_only); }
        if let Value::Intrinsic(task, spelling) = subject {
            if task.names_a_kind() || !self.names_in_calls {
                return self.is_beneath(&Self::wrap(8, vec![Value::text(spelling)]), choice, class_only);
            }
            if class_only { return Err(self.not_a_class("core.issubclass.subject")); }
            let actual_class = self.kind_named_after(subject);
            return self.is_beneath(&actual_class, choice, true);
        }
        // The property and the two method wrappers name a class of
        // their own where they are asked after as one: a wrapper of
        // their kind, or a class built upon them, answers to them.
        if self.spells_property_kind(choice) { let kind=self.property_blueprint(); return self.is_beneath(subject, &Value::Blueprint(kind), class_only); }
        if self.spells_classmethod_kind(choice) { let kind=self.classmethod_blueprint(); return self.is_beneath(subject, &Value::Blueprint(kind), class_only); }
        if self.spells_staticmethod_kind(choice) { let kind=self.staticmethod_blueprint(); return self.is_beneath(subject, &Value::Blueprint(kind), class_only); }
        if let Value::Intrinsic(_, word) = choice { return self.is_beneath(subject, &Value::Wrapped(8, Rc::new(vec![Value::text(word)]).into()), class_only); }
        // The kind asked after must be a class wherever it is asked, and
        // each of the two has its own words, as the reference has.
        let amiss=if class_only{"core.issubclass.amiss"}else{"core.isinstance.amiss"};
        match choice {
            Value::Tuple(options)=>{
                // A tuple of kinds is walked member by member, and the
                // walk is guarded so a tuple nested without end raises
                // as the reference's own reading does.
                let members: Vec<Value> = options.to_vec();
                self.deeper()?;
                let mut answer=Ok(false);
                for option in members.iter(){
                    match self.is_beneath(subject,option,class_only){
                        Ok(true)=>{answer=Ok(true);break;}
                        Ok(false)=>{}
                        Err(escape)=>{answer=Err(escape);break;}
                    }
                }
                self.standing-=1;
                answer
            },
            // A union built by `|` carries a bare `Nil` for the
            // `NoneType` member, the very value `None` itself is, so
            // a chained union reads it back this way rather than
            // needing `type(None)`.
            Value::Nil=>Ok(if class_only{matches!(subject,Value::KindOf(Kind::Nothing))}else{matches!(subject,Value::Nil)}),
            Value::Blueprint(c)=>{
                if !class_only && self.namespace_has_kind(subject, c) { return Ok(true); }
                // What is asked about must be a class wherever a class
                // is what is asked about; a subject that keeps its own
                // `__bases__` may stand as one all the same.
                if class_only && !self.counts_as_class(subject){
                    if self.value_bases(subject)?.is_some(){return self.bases_walk(subject.clone(),choice);}
                    return Err(self.not_a_class("core.issubclass.subject"));
                }
                // Everything lies under the class everything lies under.
                if Rc::ptr_eq(c,&self.common_ancestor()) {
                    if !self.table.has_any("ext.stmt.class.builder") { return Ok(true); }
                    return Ok(match subject {
                        Value::Blueprint(actual) if class_only => Self::ancestry_includes(actual,c),
                        Value::Thing(object) if !class_only => Self::ancestry_includes(&object.blueprint(),c),
                        _ => true,
                    });
                }
                match (class_only, subject, Self::native_word(c)) {
                    (false, Value::Thing(_), _) => {},
                    (false, _, Some(word)) => return Ok(subject.kind_word()==word),
                    _ => {},
                }
                if !class_only {
                    if let Value::Blueprint(held)=subject {
                        let builder=Self::builder_over(held).unwrap_or_else(||self.builder_blueprint());
                        return Ok(Self::ancestry_includes(&builder,c));
                    }
                }
                let b=match subject{Value::Blueprint(b) if class_only=>Some(b),Value::Thing(t) if !class_only=>Some(&t.blueprint()),_=>None};
                if b.is_some_and(|actual| (!class_only && Rc::ptr_eq(actual,c)) || Self::fault_descends(actual,c)||Self::ancestry_includes(actual,c)){return Ok(true);}
                // A thing not of the kind may still name a class beneath
                // it through its own `__class__`, which is read here and
                // may raise, as the reference asks it. It is the class the
                // thing was made by that this second look must differ
                // from, so a proxy naming the very kind asked after is of
                // it.
                if !class_only {
                    if let Value::Thing(thing) = subject {
                        // A module already answers by the kind it was
                        // made as; its own reader is not walked.
                        if self.namespace_holding(subject).is_some() { return Ok(false); }
                        if let Some(Value::Blueprint(held))=self.value_class_named(subject)? {
                            if !Rc::ptr_eq(&held,&thing.blueprint()) {
                                return Ok(Self::fault_descends(&held,c)||Self::ancestry_includes(&held,c));
                            }
                        }
                    }
                }
                Ok(false)
            }
            Value::Wrapped(8,names)=>{
                let Value::Text(word)=&names[0] else{return Err(self.not_a_class(amiss));};
                let Some(op)=self.table.prims.get(word.as_ref()).copied().filter(Self::names_a_kind) else{return Err(self.not_a_class(amiss));};
                if class_only{
                    if let Value::Blueprint(b)=subject{
                        let inherited=if op==Prim::SortOf { std::iter::once(b).chain(b.ancestry.borrow().iter()).any(|base|self.builds_classes(base)) }
                            else { Self::native_among(b,word) };
                        return Ok(inherited);
                    }
                    if let Some(under)=self.kind_spelling(subject){return Ok(self.kind_under(&under,word));}
                    // A subject that stands as no class still may stand
                    // as one through the `__bases__` it keeps.
                    if self.value_bases(subject)?.is_some(){return self.bases_walk(subject.clone(),choice);}
                    return Err(self.not_a_class("core.issubclass.subject"));
                }
                // A thing of a blueprint standing on the kind is of the
                // kind; one standing on several is of each, and a thing
                // not of it may still name a kind through `__class__`.
                if let Value::Thing(t)=subject{
                    if Self::native_among(&t.blueprint(),word) { return Ok(true); }
                    // A module already answers by the kind it was made
                    // as; its own reader is not walked for `__class__`.
                    if self.namespace_holding(subject).is_some() { return Ok(false); }
                    if let Some(reported)=self.value_class_named(subject)? {
                        if !self.same_thing(&reported, &Value::Blueprint(t.blueprint())) {
                            return Ok(self.reported_stands_beneath(&reported,choice));
                        }
                    }
                    return Ok(false);
                }
                Ok(self.kind_covers(&op,word,subject))
            },
            _=>{
                // The kind may name no class of its own and still stand
                // as one through its `__bases__`; and the subject may
                // stand as one the same way.
                if self.value_bases(choice)?.is_some(){
                    if class_only{
                        if self.value_bases(subject)?.is_none(){return Err(self.not_a_class("core.issubclass.subject"));}
                        return self.bases_walk(subject.clone(),choice);
                    }
                    let Some(named)=self.value_class_named(subject)? else{return Ok(false);};
                    return self.bases_walk(named,choice);
                }
                if class_only && !self.counts_as_class(subject){return Err(self.not_a_class("core.issubclass.subject"));}
                Err(self.not_a_class(amiss))
            }
        }
    }
    /// The words for a question handed the wrong count of arguments:
    /// they name the question, the count it wants and the count it got.
    fn wrong_count(&self,name:&str,wanted:usize,given:usize)->Escape {
        let parts=self.table.strings("ext.builtin.core.arity.exact");
        if parts.len()<3 {return self.class_unready();}
        format!("{}{}{}{}{}{}",parts[0],name,parts[1],wanted,parts[2],given).into()
    }
    /// The word this language spells one of the class tools with, the
    /// one a question handed the wrong count of arguments names itself
    /// by.
    fn class_tool_word(&self,op:u8)->String {
        let target=match op {
            0=>Prim::Belongs, 2=>Prim::CallableValue, 3=>Prim::GetMember,
            4=>Prim::SetMember, 5=>Prim::DropMember, 6=>Prim::HasAttribute, 7=>Prim::MembersOf,
            other=>Prim::ClassWork(other),
        };
        self.table.prims.iter().find(|(_,p)|**p==target).map(|(w,_)|w.to_string()).unwrap_or_default()
    }
    pub(super) fn work_on_class(&mut self,op:u8,mut values:Vec<Value>)->Res {

        if op == 13 { return self.class_from_function(values); }
        if op == 14 { return self.check_class_builtin(); }
        if op == 15 { return self.prepared_class_book(values.remove(0)); }
        if op == 16 { return self.dispatch_class_builder(values); }
        if op == 18 { return self.class_namespace_read(values); }
        if op == 26 {
            let (values, named) = self.open_arguments(values)?;
            if !named.is_empty() { return Err(String::from("TypeError: sys._clear_type_descriptors() takes no keyword arguments").into()); }
            if values.len() != 1 { return Err(format!("TypeError: sys._clear_type_descriptors() takes exactly one argument ({} given)", values.len()).into()); }
            let target = values[0].settled();
            let class = if let Value::Blueprint(class) = target { class } else {
                if self.stands_for_a_kind(&target) { return Err(String::from("TypeError: argument is immutable").into()); }
                return Err(format!("TypeError: _clear_type_descriptors() argument must be type, not {}", target.kind_word()).into());
            };
            if class.type_names.borrow().is_none() || Self::sealed(&class) { return Err(String::from("TypeError: argument is immutable").into()); }
            // The kind described no longer offers a weak reference, so a
            // class made from its namespace may name one for itself.
            class.weak_slot.set(Some(false));
            let mut namespace = class.shared.borrow_mut();
            namespace.retain(|entry| !matches!(entry.0.as_str(), "__dict__" | "__weakref__"));
            return Ok(Value::Nil);
        }
        if op == 19 || op == 20 { return self.class_namespace_write(values, op == 20); }
        if matches!(op, 3|4|5|6) {
            if let Some(name) = values.get_mut(1) {
                if let Some(text @ Value::Text(_)) = Self::underlying(name) { *name = text; }
            }
        }
        if matches!(op,3..=6) && values.len()>=2 {
            if let (Value::Thing(owner),Value::Unpaired(_))=(&values[0],&values[1]) {
                let mut entries=self.attribute_entries(owner);
                let found=entries.iter().position(|(key,_)|key.equals(&values[1]));
                if op==6{return Ok(Value::Flag(found.is_some()))}
                if op==3 {if let Some(at)=found{return Ok(entries[at].1.clone())}if let Some(default)=values.get(2){return Ok(default.clone())}}
                if op==4 && values.len()==3 {match found{Some(at)=>entries[at].1=values[2].clone(),None=>entries.push((values[1].clone(),values[2].clone()))}self.attribute_restore(owner,entries);return Ok(Value::Nil)}
                if op==5 {if let Some(at)=found{entries.remove(at);self.attribute_restore(owner,entries);return Ok(Value::Nil)}}
            }
        }
        if matches!(op, 24 | 25) && values.len() == 1 {
            let function = Self::wrap(44, vec![values[0].settled()]);
            return if op == 24 { Ok(function) } else { self.apply_class_member(function, vec![Value::Small(1)]) };
        }
        if op == 23 {
            let (mut positional, options) = self.open_arguments(values)?;
            if !options.is_empty() { return Err(String::from("TypeError: _typing._idfunc() takes no keyword arguments").into()); }
            if positional.len() != 1 { return Err(format!("TypeError: _typing._idfunc() takes exactly one argument ({} given)", positional.len()).into()); }
            return Ok(positional.remove(0));
        }
        if op == 21 && values.is_empty() { return Ok(Value::Blueprint(self.native_kind("SimpleNamespace"))); }
        if op == 12 && values.len() == 1 {
            let compact = match &values[0] {
                Value::Thing(t) => Self::native_beneath(&t.blueprint()).is_none()
                    && self.allowed_slot(&t.blueprint(), self.detail("namespace"))
                    && t.holds.borrow().iter().all(|entry| entry.0 != "\0dictionary" && entry.0 != "\0wide-attributes"),
                _ => false,
            };
            return Ok(Value::Flag(compact));
        }

        if op<=1 && values.len()==2{return Ok(Value::Flag(self.is_beneath(&values[0],&values[1],op==1)?));}
        // Both questions want two arguments and name themselves where
        // they are handed another count of them.
        if op<=1 {
            return Err(self.wrong_count(&self.class_tool_word(op),2,values.len()));
        }
        if op==2 && values.len()==1{return Ok(Value::Flag(matches!(&values[0],Value::Routine(_)|Value::Bound(..)|Value::Method(..)|Value::Blueprint(_)|Value::Intrinsic(..)|Value::OctetKind {..}|Value::Member(..)|Value::TextCall {..})||matches!(&values[0],Value::Wrapped(tag,_) if matches!(tag,0..=4|8..=12|14|31|33|34|36|44..=48|50..=57|59|60|70..=74|77..=79|81..=85|122|200..=204|132|133|134|135|136))||matches!(&values[0],Value::Thing(t) if self.inherited_entry(&t.blueprint(),self.detail("call")).is_some())));}
        // getattr and hasattr want the receiver and a name, and take a
        // name of any kind but a string only to say so.
        if (op==3||op==6)&&values.len()>=2{
            // A name of a kind standing on text is asked after as that text.
            let spelled=match &values[1]{Value::Thing(_)=>Self::underlying(&values[1]).map(|under|under.settled()).filter(|under|matches!(under,Value::Text(_))),_=>None};
            let spelled=spelled.unwrap_or_else(||values[1].clone());
            // A text keeping a lone half of a surrogate pair still names
            // a member; it is asked after as the stand-in text such a
            // row reads as elsewhere, no member's name keeping one.
            let spelled=match &spelled{Value::Unpaired(numbers)=>Value::text(&Value::category_text(numbers)),other=>other.clone()};
            let Value::Text(key)=&spelled else{return Err(self.core_complaint("core.attribute.name",&values[1].kind_word()).into());};
            // Asking whether a name is there, or reading it with something
            // to fall back on, does not wake a namespace's own answerer.
            self.asking_presence = op==6 || values.len()==3;
            let read = self.read_class_member(values[0].clone(),key,false);
            self.asking_presence = false;
            return match read {
                // A member read by name reads through the cell a namespace
                // keeps it in, as the program's own member read does.
                Ok(v)=>Ok(if op==6{Value::Flag(true)}else if self.table.has_any("ext.builtin.exceptions.syntax"){let keep=match &v{Value::Shared(cell)=>matches!(&*cell.borrow(),Value::Vector(_)|Value::Set(_)|Value::SetCursor{..}|Value::Dict(_)|Value::Octets{..}),_=>false};if keep{v}else{match v{Value::Shared(cell)=>cell.borrow().clone(),held=>held}}}else{match v{Value::Shared(cell)=>cell.borrow().clone(),held=>held}}),
                Err(escape) if self.missing_member_escape(&escape)=>{
                    if op == 6 || values.len() == 3 {
                        self.sought_in_vain.take();
                        Ok(if op == 6 { Value::Flag(false) } else { values[2].clone() })
                    } else { Err(self.explain_absence(escape, &values[0], key)) }
                }
                failed=>failed,
            };
        }
        if op==3 {return Err(self.wrong_count(&self.class_tool_word(3),2,values.len()));}
        if op==6 {return Err(self.wrong_count(&self.class_tool_word(6),2,values.len()));}
        if (op==4&&values.len()==3)||(op==5&&values.len()==2){
            let written=match &values[1]{Value::Unpaired(numbers)=>Value::text(&Value::category_text(numbers)),other=>other.clone()};
            let Value::Text(key)=&written else{return Err(self.core_complaint("core.attribute.name",&values[1].kind_word()).into());};return self.alter_class_member(values[0].clone(),key,values.get(2).cloned(),false);
        }
        if op==4 {return Err(self.wrong_count(&self.class_tool_word(4),3,values.len()));}
        if op==5 {return Err(self.wrong_count(&self.class_tool_word(5),2,values.len()));}
        if op==7&&values.len()==1{
            let key=self.detail("namespace").to_owned();
            return match self.read_class_member(values[0].clone(),&key,false) {
                Err(escape) if self.missing_member_escape(&escape) => Err(self.core_complaint("core.vars", "").into()),
                result => result,
            };
        }
        if op==8&&values.len()==1{
            if let Value::Blueprint(kind) = &values[0] {
                if let Some(builder)=Self::builder_over(kind) {
                    if let Some(word)=self.rules.specials.get(75).cloned() {
                        if let Some(method)=self.inherited_entry(&builder,&word) {
                            let bound=self.member_binding(method,Some(values[0].clone()),builder)?;
                            let answer=self.apply_class_member(bound,Vec::new())?;
                            let names=self.object_members(&answer)?;
                            let ordered=self.arranged(names,&Value::Nil,false).map_err(Escape::from)?;
                            return Ok(Value::Vector(crate::tuples::Sequence::plain(ordered)));
                        }
                    }
                }
            }
            if let Value::Thing(module) = &values[0] {
                let blueprint=module.blueprint();
                let module_kind=Self::native_beneath(&blueprint).as_deref() == Some("module");
                let class_directory=self.rules.specials.get(75)
                    .and_then(|word| self.inherited_entry(&blueprint,word)).is_some();
                if (module_kind || self.namespace_holding(&values[0]).is_some()) && !class_directory {
                    let namespace_key = self.detail("namespace").to_owned();
                    let dictionary = self.read_class_member(values[0].clone(), &namespace_key, false)?;
                    let map = Self::underlying(&dictionary).unwrap_or(dictionary).settled();
                    let entries = match map {
                        Value::Attributes(owner) => self.attribute_entries(&owner),
                        Value::Dict(pairs) => pairs.iter().map(|entry| (entry.0.clone(), entry.1.clone())).collect(),
                        _ => return Err("TypeError: <module>.__dict__ is not a dictionary".to_owned().into()),
                    };
                    if let Some(word)=self.rules.specials.get(75) {
                        if let Some((_,method))=entries.iter().find(|(key,_)| key.bare()==*word) {
                            let answer=self.apply_class_member(method.clone(),Vec::new())?;
                            let names=self.object_members(&answer)?;
                            let ordered=self.arranged(names,&Value::Nil,false).map_err(Escape::from)?;
                            return Ok(Value::Vector(crate::tuples::Sequence::plain(ordered)));
                        }
                    }
                    let keys = entries.into_iter().map(|entry| entry.0).collect();
                    let ordered = self.arranged(keys, &Value::Nil, false).map_err(Escape::from)?;
                    return Ok(Value::Vector(crate::tuples::Sequence::plain(ordered)));
                }
            }
            // A thing with a directory method of its own answers with it,
            // and the names it gives are set in order.
            if matches!(&values[0],Value::Thing(_)){
                if let Some(answer)=self.ask_special(&values[0],75,&[])?{
                    let names=self.object_members(&answer)?;
                    let ordered=self.arranged(names,&Value::Nil,false).map_err(Escape::from)?;
                    return Ok(Value::Vector(crate::tuples::Sequence::plain(ordered)));
                }
            }
            return self.ordinary_directory(&values[0]);
        }
        if op==8 {return Err(self.wrong_count(&self.class_tool_word(8),1,values.len()));}
        // With the protocol spelled, a property is a thing of the
        // property blueprint; without it, the older wrapper.
        if op==11&&self.protocol_spelled(){let kind=self.property_blueprint();return self.construct_ordered(kind,values);}
        if (9..=11).contains(&op)&&!values.is_empty(){return Ok(Self::wrap(op-5,values));}
        Err(self.class_unready())
    }
    pub(super) fn next_ancestor_call(&mut self,receiver:Value,declared:&str,key:&str,mut args:Vec<Value>)->Res {
        let class=match &receiver{Value::Thing(t)=>t.blueprint().clone(),Value::Blueprint(b)=>b.clone(),_=>return Err(self.class_unready())};
        let named_here=|a:&Self,b:&Rc<Blueprint>|b.name==declared||b.type_names.borrow().as_ref().map(|names| names.declared.clone()).or_else(|| Self::own_entry(b,a.detail("qualified"))).map_or(false,|v| (if b.type_names.borrow().is_some() { v.type_text() } else { Self::underlying(&v).unwrap_or(v) }).bare()==declared);
        let mut chain=Self::resolution_order(&class);
        let mut start=chain.iter().position(|b|named_here(self,b));
        // A method of a metaclass is written in the metaclass, so the
        // forebears it reaches past are the metaclass's own, not those
        // of the class it was handed.
        if start.is_none() {
            if let Some(builder)=Self::builder_over(&class) {
                chain=Self::resolution_order(&builder);
                start=chain.iter().position(|b|named_here(self,b));
            }
        }
        let start=start.ok_or_else(||self.class_unready())?;
        for b in &chain[start+1..]{
            // The blueprint every metaclass is built on: it lays a
            // class out and makes a thing of one, as the kind primitive
            // plainly does.
            if self.builds_classes(b) {
                // The kind primitive's own building: a name, the
                // parents and a namespace become a class, remembering
                // the metaclass handed to it as the one that built it.
                if self.rules.words_ext_stmt_class_constructor.first().map(String::as_str)==Some(key) {
                    let mut values=vec![receiver.clone()];values.extend(args);
                    return self.initialise_type_object(values);
                }
                if key==self.detail("allocate") {return self.class_of_parts(args);}
                if key==self.detail("call") {
                    let Value::Blueprint(made)=&receiver else{return Err(self.class_unready())};
                    // What was spread is opened first, so a class taking
                    // nothing is not handed an empty spread.
                    let (positional,named)=self.open_arguments(args)?;
                    let mut values=positional;
                    values.extend(named.into_iter().map(|(k,v)|Value::Couple(Rc::new((Value::text(&k),v)))));
                    return self.construct_plainly(made.clone(),values);
                }
                continue;
            }
            // The native kind a blueprint stands on makes the thing, takes
            // its constructing in silence, and answers the kind's methods
            // through the worth the thing keeps.
            if let Some(word)=Self::native_word(b) {
                if key==self.detail("allocate"){return self.apply_class_member(Self::wrap(14,vec![Value::text(&word)]),args);}
                if word == "module" && self.rules.specials.get(1).is_some_and(|name| name == key) { return self.describe_module(receiver); }
                if self.rules.words_ext_stmt_class_constructor.first().map(String::as_str)==Some(key){
                    if word == "module" { return self.fill_module(receiver, args); }
                    return match Self::underlying(&receiver) {
                        Some(under) if matches!(under.settled(), Value::Set(_) | Value::Vector(_) | Value::Dict(_)) => {
                            let (positional, named) = self.open_arguments(args)?;
                            self.value_member(&under, key, positional, named)
                        }
                        _ => Ok(Value::Nil),
                    };
                }
                // A value working is taken only where the worth answers
                // to it: `__hash__`, for one, spells the working a slice
                // answers and a text does not, so a text asked through
                // its forebear falls to the kind's own member below
                // rather than to a working that would refuse it.
                if let (Some(under),Some(operation))=(Self::underlying(&receiver),Self::kind_method_named(self.table,key)) {
                    if crate::members::answers_to(&under,&operation) {
                        let (given,named)=self.open_arguments(args)?;
                        return self.value_member(&under,&operation,given,named);
                    }
                }
                // A member the kind carries that no value working names,
                // such as `__hash__` or `__len__`, is read off the worth
                // the thing holds and worked there, as the base's own
                // reading of the name gives it.
                if let Some(under)=Self::underlying(&receiver) {
                    if let Some(member)=self.attribute(&under,key) {
                        return self.apply_class_member(member,args);
                    }
                }
                if let Some(member) = Self::own_entry(b, key) {
                    let bound = self.member_binding(member, Some(receiver.clone()), class.clone())?;
                    return self.apply_class_member(bound, args);
                }
                continue;
            }
            if let Some(entry) = Self::own_entry(b, key) {
                if key == self.detail("allocate") { return self.apply_class_member(entry, args); }
                let method = self.member_binding(entry, Some(receiver.clone()), class.clone())?;
                return self.apply_class_member(method, args);
            }
            if key==self.detail("allocate") && self.is_fault_kind(b) {
                let Some((Value::Blueprint(cls), rest)) = args.split_first() else { return Err(self.class_unready()) };
                return Ok(self.native_fault_new(b.clone(), cls.clone(), rest.to_vec())?);
            }
            if self.rules.words_ext_stmt_class_constructor.first().map(String::as_str) == Some(key) && self.is_fault_kind(b) {
                if let Value::Thing(t) = &receiver {
                    let (mut plain, names) = self.open_arguments(args)?;
                    plain.extend(names.into_iter().map(|(word, item)| Value::Couple(Rc::new((Value::text(&word), item)))));
                    return self.fault_method(t.clone(), key, &plain);
                }
            }
            // A fault's making, asked of a base, is the root's making.
            if self.is_fault_kind(b) && key == self.detail("allocate") {
                return self.apply_class_member(Self::wrap(1, Vec::new()), args);
            }
            if b.name==self.detail("root") && key==self.detail("allocate"){let f=self.read_class_member(Value::Blueprint((*b).clone()),key,true)?;return self.apply_class_member(f,args);}
            if b.name==self.detail("root") {
                let builtin=self.read_class_member(Value::Blueprint((*b).clone()),key,true)?;
                args.insert(0,receiver.clone());return self.apply_class_member(builtin,args);
            }
        }
        Err(self.absent_attribute(&receiver,key))
    }
    /// The operation a method word of a native kind stands for, where
    /// the table spells such a method by that word.
    fn kind_method_named(table:&Table,word:&str)->Option<String> {
        crate::table::BUILTIN_LABELS.iter().find(|(label,prim)|*prim==Prim::ValueMethod&&table.spells(label,word))
            .and_then(|(label,_)|label.strip_prefix("ext.builtin.method."))
            .map(str::to_owned)
    }
}

impl<'a> Machine<'a> {
    fn namespace_action(&mut self, kept: &[Value], input: Vec<Value>) -> Res {
        let operation = match kept.first() { Some(Value::Small(n)) => *n, _ => return Err(self.class_unready()) };
        let (mut positional, named) = self.open_arguments(input)?;
        if positional.is_empty() { return Err(self.class_unready()); }
        let receiver = positional.remove(0);
        if operation == 0 {
            if let Value::Blueprint(of) = receiver {
                if Self::native_beneath(&of).as_deref() == Some("SimpleNamespace") {
                    self.made += 1;
                    return Ok(Value::Thing(Rc::new(Thing { reclassified: RefCell::new(None), of,
                        holds: RefCell::new(Vec::new()), turn: self.made })));
                }
            }
            return Err(self.class_unready());
        }
        let object = match &receiver {
            Value::Thing(t) if Self::native_beneath(&t.blueprint()).as_deref() == Some("SimpleNamespace") => t.clone(),
            _ => return Err(self.class_unready()),
        };
        match operation {
            1 => {
                if positional.len() > 1 { return Err(format!("TypeError: {} expected at most 1 argument, got {}", object.blueprint().name, positional.len()).into()); }
                if !positional.is_empty() {
                    let converted = self.prim(Prim::Dictionary, "dict", &positional)?;
                    if let Value::Dict(rows) = converted.settled() {
                        for (key, value) in rows.iter() {
                            match key {
                                Value::Text(name) => { self.alter_class_member(receiver.clone(), name, Some(value.clone()), true)?; }
                                _ => return Err(String::from("TypeError: keywords must be strings").into()),
                            }
                        }
                    } else { return Err(self.class_unready()); }
                }
                for (name, value) in named { self.alter_class_member(receiver.clone(), &name, Some(value), true)?; }
                Ok(Value::Nil)
            }
            2 => {
                if !positional.is_empty() || !named.is_empty() { return Err(self.class_unready()); }
                let title = match Self::native_word(&object.blueprint()) { Some(_) => "namespace".into(), None => object.blueprint().name.clone() };
                let marker = "\0namespace-rendering";
                if object.holds.borrow().iter().any(|(name, _)| name == marker) { return Ok(Value::text(&format!("{title}(...)"))); }
                object.holds.borrow_mut().push((marker.into(), Value::Nil));
                let render = (|| {
                    let dictionary = self.read_class_member(receiver.clone(), &self.detail("namespace").to_owned(), true)?;
                    let mut pieces = Vec::new();
                    for name in self.gathered_members(&dictionary)? {
                        if let Value::Text(text) = name {
                            if text.is_empty() || text.starts_with('\0') { continue; }
                            let value = self.read_class_member(receiver.clone(), &text, true)?;
                            let shown = self.prim(Prim::Quoted, "repr", &[value])?;
                            pieces.push(text.to_string() + "=" + &shown.bare());
                        }
                    }
                    Ok(Value::text(&(title + "(" + &pieces.join(", ") + ")")))
                })();
                object.holds.borrow_mut().retain(|(name, _)| name != marker);
                render
            }
            3 | 4 => {
                if positional.len() != 1 || !named.is_empty() { return Err(self.class_unready()); }
                let belongs = matches!(positional[0].settled(), Value::Thing(t) if Self::native_beneath(&t.blueprint()).as_deref() == Some("SimpleNamespace"));
                if !belongs { return Ok(Value::Refusal(Rc::from("NotImplemented"))); }
                let key = self.detail("namespace").to_owned();
                let left = self.read_class_member(receiver, &key, true)?.settled();
                let right = self.read_class_member(positional.remove(0), &key, true)?.settled();
                self.prim_values(if operation == 3 { Prim::Eq } else { Prim::Ne }, "", &[left, right]).map_err(Escape::from)
            }
            5 => {
                if !positional.is_empty() || !named.is_empty() { return Err(self.class_unready()); }
                let state = self.read_class_member(receiver, &self.detail("namespace").to_owned(), true)?;
                Ok(Value::tuple(vec![Value::Blueprint(object.blueprint().clone()), Value::tuple(Vec::new()), state]))
            }
            6 => {
                if !positional.is_empty() { return Err(String::from("TypeError: __replace__() takes no positional arguments").into()); }
                let new = self.construct_ordered(object.blueprint().clone(), Vec::new())?;
                if !matches!(&new, Value::Thing(t) if Self::native_beneath(&t.blueprint()).as_deref() == Some("SimpleNamespace")) { return Err(self.class_unready()); }
                let key = self.detail("namespace").to_owned();
                let dictionary = self.read_class_member(receiver.clone(), &key, true)?;
                for key in self.gathered_members(&dictionary)? {
                    if let Value::Text(name) = key {
                        let held = self.read_class_member(receiver.clone(), &name, true)?;
                        self.alter_class_member(new.clone(), &name, Some(held), true)?;
                    }
                }
                for (name, held) in named { self.alter_class_member(new.clone(), &name, Some(held), true)?; }
                Ok(new)
            }
            _ => Err(self.class_unready()),
        }
    }
}

impl<'a> Machine<'a> {
    fn alias_action(&mut self, mode: &[Value], arguments: Vec<Value>) -> Res {
        let mut values = arguments.into_iter();
        let subject = values.next().ok_or_else(|| self.class_unready())?.settled();
        let mut rest: Vec<_> = values.collect();
        let action = match mode[0] { Value::Small(n) => n, _ => return Err(self.class_unready()) };
        if action == 0 {
            if rest.len() != 2 { return Err(String::from("TypeError: GenericAlias expected 2 arguments").into()); }
            let Value::Blueprint(of) = subject else { return Err(self.class_unready()); };
            let tail = rest.pop().unwrap();
            let parameters = if matches!(tail.settled(), Value::Tuple(_)) { tail } else { Value::tuple(vec![tail]) };
            let mut free: Vec<Value> = Vec::new();
            for parameter in self.object_members(&parameters)? {
                let variables = if matches!(parameter.settled(), Value::Thing(ref item) if matches!(Self::native_word(&item.blueprint()).as_deref(), Some("TypeVar" | "ParamSpec" | "TypeVarTuple"))) {
                    vec![parameter]
                } else {
                    match self.read_class_member(parameter, "__parameters__", false) {
                        Ok(group) => self.object_members(&group)?,
                        Err(escape) if self.missing_member_escape(&escape) => Vec::new(),
                        Err(escape) => return Err(escape),
                    }
                };
                for variable in variables { if !free.iter().any(|prior| prior.selfsame(&variable)) { free.push(variable); } }
            }
            self.made += 1;
            let holds = vec![("__origin__".to_owned(), rest.remove(0)), ("__args__".to_owned(), parameters),
                ("__unpacked__".to_owned(), Value::Flag(false)), ("__parameters__".to_owned(), Value::tuple(free))];
            return Ok(Value::Thing(Rc::new(Thing { reclassified: RefCell::new(None), of, holds: RefCell::new(holds), turn: self.made })));
        }
        let parent = self.read_class_member(subject.clone(), "__origin__", true)?;
        match action {
            5 if rest.is_empty() => {
                let parameters = self.read_class_member(subject, "__args__", true)?;
                let kind = self.native_kind("GenericAlias");
                let unpacked = self.construct_ordered(kind, vec![parent, parameters])?;
                let Value::Thing(alias) = &unpacked else { return Err(self.class_unready()); };
                let mut fields = alias.holds.borrow_mut();
                if let Some((_, flag)) = fields.iter_mut().find(|(name, _)| name == "__unpacked__") { *flag = Value::Flag(true); }
                drop(fields);
                let entries = Rc::new(vec![unpacked]).into();
                Ok(Self::cursor_value_walked(crate::data::IteratorKind::Stored { entries, next: 0 }, Some(Rc::from("generic_alias_iterator"))))
            }
            2 => self.apply_class_member(parent, rest),
            3 if rest.len() == 1 => Ok(Value::tuple(vec![parent])),
            1 => {
                let tuple = self.read_class_member(subject.clone(), "__args__", true)?;
                let mut names = Vec::new();
                for parameter in self.gathered_members(&tuple)? {
                    let name = if matches!(parameter, Value::Nil) { String::from("None") }
                        else { match parameter.kind_it_names() { Some(name) => name, None => self.prim(Prim::Quoted, "repr", &[parameter])?.bare() } };
                    names.push(name);
                }
                let prefix = parent.kind_it_names().unwrap_or_else(|| parent.bare());
                let unpacked = self.read_class_member(subject, "__unpacked__", true)?;
                let prefix = if matches!(unpacked, Value::Flag(true)) { String::from("*") + &prefix } else { prefix };
                Ok(Value::text(&(prefix + "[" + &names.join(", ") + "]")))
            }
            4 if rest.len() == 1 => {
                let other = rest.remove(0);
                if !matches!(other.settled(), Value::Thing(t) if Self::native_beneath(&t.blueprint()).as_deref() == Some("GenericAlias")) { return Ok(Value::Refusal(Rc::from("NotImplemented"))); }
                let a = self.read_class_member(subject, "__args__", true)?;
                let b = self.read_class_member(other.clone(), "__args__", true)?;
                let other_parent = self.read_class_member(other, "__origin__", true)?;
                let same = self.prim_values(Prim::Eq, "", &[parent, other_parent])?;
                if !self.object_truth(&same)? { return Ok(Value::Flag(false)); }
                self.prim_values(Prim::Eq, "", &[a, b]).map_err(Escape::from)
            }
            _ => Err(self.class_unready()),
        }
    }
}

impl<'a> Machine<'a> {
    pub(super) fn combined_types(&mut self, pair: &[Value]) -> Value {
        let mut row: Vec<Value> = Vec::new();
        for operand in pair {
            let mut additions = vec![operand.clone()];
            if let Value::Thing(t) = operand {
                if Self::native_word(&t.blueprint()).as_deref() == Some("Union") {
                    if let Some((_, Value::Tuple(parts))) = t.holds.borrow().iter().find(|entry| entry.0 == "__args__") { additions = parts.to_vec(); }
                }
            } else if matches!(operand, Value::Nil) { additions = vec![Value::Blueprint(self.native_kind("NoneType"))]; }
            for candidate in additions { if row.iter().all(|held| !held.equals(&candidate)) { row.push(candidate); } }
        }
        match row.len() {
            1 => row.remove(0),
            _ => {
                self.made += 1;
                let of = self.native_kind("Union");
                Value::Thing(Rc::new(Thing { reclassified: RefCell::new(None), of, turn: self.made,
                    holds: RefCell::new(vec![("__args__".to_owned(), Value::tuple(row))]) }))
            }
        }
    }
}

impl<'a> Machine<'a> {
    /// Whether the blueprint is the parent class or stands beneath it:
    /// the kind word it or one of its forebears was made with is the
    /// parent class's own.
    pub(super) fn parent_kind_descended(b:&Rc<Blueprint>)->bool {
        std::iter::once(b).chain(b.ancestry.borrow().iter()).any(|base| Self::native_word(base).as_deref() == Some("super"))
    }

    /// Whether two blueprints are one for the parent walk's purposes:
    /// the very blueprint, or two standings of one builtin kind, whose
    /// blueprint is made in more than one place.
    pub(super) fn parent_same_class(a:&Rc<Blueprint>,b:&Rc<Blueprint>)->bool {
        Rc::ptr_eq(a,b) || matches!((Self::native_word(a),Self::native_word(b)),(Some(x),Some(y)) if x==y)
    }

    /// The name the parent class's refusals give a value's kind: the
    /// blueprint's own name for a thing of one, the kind word for a
    /// plain value, and the cell's name for a cell, as the reference
    /// names them in this very complaint.
    pub(super) fn parent_tp_name(&self,value:&Value)->String {
        if let Some(word) = self.kind_spelling(value) { return word.to_string(); }
        match value {
            Value::Thing(t) => t.blueprint().name.clone(),
            Value::Blueprint(b) => b.name.clone(),
            other => other.kind_word(),
        }
    }

    /// How the parent thing keeps a class it names: the kind word the
    /// program reads for a builtin kind, the blueprint for any other.
    fn parent_public(&self,b:&Rc<Blueprint>)->Value {
        Self::native_word(b).and_then(|word| self.kind_by_word(&word)).unwrap_or_else(|| Value::Blueprint(b.clone()))
    }

    /// The blueprint a kept form names: a blueprint as it stands, or
    /// the blueprint of a builtin kind word, the two forms the parent
    /// thing keeps its classes in.
    fn parent_blueprint_of(&mut self,held:Option<Value>)->Option<Rc<Blueprint>> {
        match held {
            Some(Value::Blueprint(b)) => Some(b),
            Some(Value::Intrinsic(op, word)) if Self::names_a_kind(&op) => Some(self.native_kind(word.as_ref())),
            _ => None,
        }
    }

    /// The blueprint the first of the parent class's arguments names: a
    /// blueprint as it stands, the blueprint of a builtin kind word, and
    /// otherwise the refusal the reference words the same way.
    fn parent_type_arg(&mut self,given:&Value)->Result<Rc<Blueprint>,Escape> {
        match given.settled() {
            Value::Blueprint(b) => Ok(b.clone()),
            Value::Intrinsic(op, word) if Self::names_a_kind(&op) => Ok(self.native_kind(word.as_ref())),
            other => Err(format!("TypeError: super() argument 1 must be a type, not {}", other.kind_word()).into()),
        }
    }

    /// The blueprint a parent thing answers as: the value given itself
    /// for a class beneath the type, the value's own blueprint for a
    /// thing of one, and otherwise the refusal the reference words the
    /// same way.
    fn parent_check(&mut self,owner:&Rc<Blueprint>,receiver:&Value)->Result<Rc<Blueprint>,Escape> {
        let stands = |actual:&Rc<Blueprint>| Self::resolution_order(actual).iter().any(|base| Self::parent_same_class(base, owner));
        if let Value::Blueprint(given) = receiver {
            if stands(given) { return Ok(given.clone()); }
            // A class the type given builds is one of its things: the
            // builder answers for it, as its own kind does in the
            // reference.
            if let Some(maker) = Self::builder_over(given) {
                if stands(&maker) { return Ok(maker); }
            }
        }
        if let Value::Thing(t) = receiver {
            let actual = t.blueprint();
            if stands(&actual) { return Ok(actual); }
        }
        if let Value::Intrinsic(op, word) = receiver {
            if Self::names_a_kind(op) {
                let actual = self.native_kind(word.as_ref());
                if stands(&actual) { return Ok(actual); }
            }
        }
        let spelled = Self::underlying(receiver).and_then(|worth| self.kind_spelling(&worth).map(|word| word.to_string()))
            .unwrap_or_else(|| receiver.kind_word());
        if !spelled.is_empty() {
            let actual = self.native_kind(&spelled);
            if stands(&actual) { return Ok(actual); }
        }
        // A proxy's own `__class__` may speak where its own blueprint
        // cannot: read through the ordinary reading, and where the
        // answer is a type beneath the one given, it is the class the
        // thing answers as.
        let kind_word = self.detail("kind").to_owned();
        match self.read_class_member(receiver.clone(), &kind_word, false) {
            Ok(exposed) => {
                if let Ok(shown) = self.parent_type_arg(&exposed) {
                    if stands(&shown) { return Ok(shown); }
                }
            }
            Err(escape) if self.missing_member_escape(&escape) => {}
            Err(escape) => return Err(escape),
        }
        let (which, spelled) = match receiver {
            Value::Blueprint(given) => ("type", given.name.clone()),
            Value::Intrinsic(op, word) if Self::names_a_kind(op) => ("type", word.to_string()),
            other => ("instance of", self.parent_tp_name(other)),
        };
        Err(format!("TypeError: super(type, obj): obj ({} {}) is not an instance or subtype of type ({}).", which, spelled, owner.name).into())
    }

    /// A thing of the parent class made of the class it looks past and
    /// the thing it stands on, the reference's own checks worded as it
    /// words them: too many arguments, a first that is no type, and a
    /// second the type has no hold on.
    pub(super) fn parent_constructed(&mut self,class:Rc<Blueprint>,given:Vec<Value>)->Res {
        // A thing of the parent class is built the way any call of a
        // class builds a thing: the allocation the class names, then
        // the constructing it answers with, handed the arguments given.
        let made = self.parent_allocated(&class, given.clone())?;
        let Value::Thing(t) = &made else { return Ok(made) };
        if !Self::parent_kind_descended(&t.blueprint()) { return Ok(made); }
        let constructor = self.table.single("ext.stmt.class.constructor").unwrap_or_default().to_owned();
        if let Some(init) = self.inherited_entry(&t.blueprint(), &constructor) {
            if matches!(&init, Value::Routine(_) | Value::Bound(..)) {
                let bound = self.member_binding(init, Some(made.clone()), t.blueprint().clone())?;
                let answer = self.apply_class_member(bound, given)?;
                if !matches!(answer, Value::Nil) { return Err(format!("TypeError: __init__() should return None, not '{}'", answer.kind_word()).into()); }
                return Ok(made);
            }
            let mut fresh = vec![made.clone()];
            fresh.extend(given);
            self.apply_class_member(init, fresh)?;
        }
        Ok(made)
    }

    /// The allocation a call of the parent class or a class beneath it
    /// runs: a `__new__` written out, called with the class and the
    /// arguments given, or a bare thing of the class otherwise.
    fn parent_allocated(&mut self,class:&Rc<Blueprint>,given:Vec<Value>)->Res {
        let allocate = self.detail("allocate").to_owned();
        if let Some(newer @ (Value::Routine(_) | Value::Bound(..))) = self.inherited_entry(class, &allocate) {
            let bound = self.member_binding(newer, None, class.clone())?;
            let mut fresh = vec![Value::Blueprint(class.clone())];
            fresh.extend(given);
            return self.apply_class_member(bound, fresh);
        }
        self.made += 1;
        Ok(Value::Thing(Rc::new(Thing { reclassified: RefCell::new(None), of: class.clone(), holds: RefCell::new(Vec::new()), turn: self.made })))
    }

    /// A parent call with no argument written: the frame gave the class
    /// and the thing, and only the parent class itself is filled from
    /// them at once; a class beneath it is built as any call of it
    /// builds a thing, its own constructing, where one is written out,
    /// running with no argument at all.
    pub(super) fn parent_framed(&mut self,class:Rc<Blueprint>,owner:Rc<Blueprint>,receiver:Value)->Res {
        let made = self.parent_allocated(&class, Vec::new())?;
        let Value::Thing(t) = &made else { return Ok(made) };
        if !Self::parent_kind_descended(&t.blueprint()) { return Ok(made); }
        if Self::native_word(&class).as_deref() == Some("super") {
            self.parent_fill(t, owner, receiver)?;
            return Ok(made);
        }
        // A class beneath the parent class is built as any call of it
        // builds a thing, its own constructing running with no
        // argument at all; the parent class's own init, where it is
        // the one answered, reads the frame the call stands in.
        let constructor = self.table.single("ext.stmt.class.constructor").unwrap_or_default().to_owned();
        if let Some(init) = self.inherited_entry(&t.blueprint(), &constructor) {
            if matches!(&init, Value::Routine(_) | Value::Bound(..)) {
                let bound = self.member_binding(init, Some(made.clone()), t.blueprint().clone())?;
                let answer = self.apply_class_member(bound, Vec::new())?;
                if !matches!(answer, Value::Nil) { return Err(format!("TypeError: __init__() should return None, not '{}'", answer.kind_word()).into()); }
                return Ok(made);
            }
            self.apply_class_member(init, vec![made.clone()])?;
        }
        Ok(made)
    }

    /// The three names a parent thing keeps, filled from the class it
    /// looks past and the thing it stands on, each class kept in the
    /// form the program reads it.
    fn parent_fill(&mut self,t:&Rc<Thing>,owner:Rc<Blueprint>,receiver:Value)->Res {
        let mut answers_as = Value::Nil;
        if !matches!(receiver, Value::Nil) {
            let checked = self.parent_check(&owner, &receiver)?;
            answers_as = self.parent_public(&checked);
        }
        let shown_owner = self.parent_public(&owner);
        let mut holds = t.holds.borrow_mut();
        for (key, value) in [("__thisclass__", shown_owner), ("__self__", receiver), ("\0objtype", answers_as)] {
            match holds.iter_mut().find(|(kept, _)| kept == key) {
                Some((_, place)) => *place = value,
                None => holds.push((key.to_owned(), value)),
            }
        }
        Ok(Value::Nil)
    }

    /// The parent class's own constructing called by name: the thing is
    /// measured against the arguments once more and made over, the way
    /// the reference lets its constructing run again on a thing made.
    fn parent_initialised(&mut self,given:Vec<Value>)->Res {
        let (positional, named) = self.open_arguments(given)?;
        let plain: Vec<Value> = positional.into_iter().map(|value| value.settled()).collect();
        let Some(subject) = plain.first() else { return Err("TypeError: descriptor '__init__' of 'super' object needs an argument".to_owned().into()); };
        if !matches!(subject, Value::Thing(instance) if Self::parent_kind_descended(&instance.blueprint())) {
            let receiver_kind = self.class_from_type(vec![subject.clone()])?;
            let received = receiver_kind.kind_it_names().ok_or_else(|| self.class_unready())?;
            return Err(format!("TypeError: descriptor '__init__' requires a 'super' object but received a '{received}'").into());
        }
        let Value::Thing(t) = subject else { unreachable!() };
        if !named.is_empty() { return Err("TypeError: super() takes no keyword arguments".to_owned().into()); }
        if plain.len() > 3 { return Err(format!("TypeError: super() expected at most 2 arguments, got {}", plain.len() - 1).into()); }
        // Reached with nothing but the thing, the constructing reads
        // its class and first argument from the frame it was called in.
        let (owner, receiver) = match plain.get(1) {
            Some(first) => (self.parent_type_arg(first)?, plain.get(2).cloned().unwrap_or(Value::Nil)),
            None => self.parent_frame_args()?,
        };
        self.parent_fill(t, owner, receiver)?;
        Ok(Value::Nil)
    }

    /// The class and the first argument a no-argument parent init reads
    /// from the frame the call stands in: the routine running now, the
    /// worth its first parameter holds, and the class cell it closes
    /// over, each absence said the way the reference says it.
    fn parent_frame_args(&mut self) -> Result<(Rc<Blueprint>, Value), Escape> {
        // A call with no frame of its own to read from -- the
        // program's outermost text has no first argument to give.
        let Some(program) = self.frames_named.last().cloned() else { return Err("RuntimeError: super(): no arguments".to_owned().into()); };
        if program.formals.is_empty() { return Err("RuntimeError: super(): no arguments".to_owned().into()); }
        let mut environment = None;
        if let Some(trace) = &self.active_trace {
            let held = trace.holds.borrow();
            if let Some((_, Value::Bound(_, env))) = held.iter().find(|(name, _)| name == "\0environment") {
                environment = Some(env.clone());
            }
        }
        let mut first = Value::Unset;
        if let Some(env) = &environment {
            let at = program.formal_slots.first().copied().unwrap_or(0);
            first = env.cells.borrow()[at].settled();
        }
        if matches!(first, Value::Unset) { return Err("RuntimeError: super(): arg[0] deleted".to_owned().into()); }
        let Some(env) = &environment else { return Err("RuntimeError: super(): bad __class__ cell".to_owned().into()); };
        // The class cell the routine closes over, wherever it was
        // taken from: the names it reaches and how far.
        let Some((up, at)) = program.reaching.iter().find_map(|held| held.ident.starts_with("#completed_class").then_some((held.up, held.at)))
            .or_else(|| program.idents.iter().position(|word| word.starts_with("#completed_class")).map(|at| (0, at)))
        else { return Err("RuntimeError: super(): __class__ cell not found".to_owned().into()); };
        let held = crate::exec::ascend(env, up).cells.borrow()[at].settled();
        if matches!(held, Value::Unset) { return Err("RuntimeError: super(): empty __class__ cell".to_owned().into()); }
        let Value::Blueprint(owner) = held else {
            return Err(format!("RuntimeError: super(): __class__ is not a type ({})", self.parent_tp_name(&held)).into());
        };
        Ok((owner, first))
    }

    /// The member a parent reading finds: the line of the blueprint the
    /// thing answers as, walked from just after the blueprint given,
    /// each member found tied to the thing the reading stands on — or
    /// left loose where that thing is itself a class, the very reading
    /// the class would take of it. Nothing where the walk finds no
    /// member, so the caller's own reading answers instead.
    fn parent_walk(&mut self,owner:&Rc<Blueprint>,receiver:&Value,actual:Rc<Blueprint>,key:&str)->Res<Option<Value>> {
        let loose = matches!(receiver, Value::Blueprint(r) if Self::parent_same_class(r, &actual));
        let mut passed = false;
        let root = self.common_ancestor();
        let lookup_order = Self::resolution_order(&actual);
        for base in &lookup_order {
            if passed {
                if Self::own_entry(base, key).is_none() && key == self.detail("allocate") {
                    if let Some(native) = Self::native_word(base).filter(|word| word != self.detail("root")) {
                        return Ok(Some(Self::wrap(14, vec![Value::text(&native)])));
                    }
                }
                if let Some(entry) = Self::own_entry(base, key) {
                    // The subclass hook is a class method: it ties to
                    // the class the thing answers as, loose or not.
                    let receiver = if key == self.detail("subclass") && loose { Some(Value::Blueprint(actual.clone())) }
                        else if key == self.detail("allocate") || loose { None } else { Some(receiver.clone()) };
                    return Ok(Some(self.member_binding(entry, receiver, actual.clone())?));
                }
                // The common ancestor supplies slots at its own MRO position.
                if Rc::ptr_eq(&root, base) {
                    if let Some(slot) = self.from_the_root(key, true, &actual) {
                        let answer = if key == self.detail("subclass") {
                            Self::wrap(3, vec![slot, Value::Blueprint(actual.clone())])
                        } else if loose || key == self.detail("allocate") { slot }
                        else { Self::wrap(3, vec![slot, receiver.clone()]) };
                        return Ok(Some(answer));
                    }
                    continue;
                }
                if Self::native_word(base).is_some() || self.is_fault_kind(base) {
                    if self.table.single("ext.stmt.class.constructor") == Some(key) {
                        // The constructing of a native forebear runs the
                        // way a parent's call of the method spelled out
                        // runs it, worth and all.
                        return Ok(Some(Self::wrap(126, vec![receiver.clone(), Value::text(&owner.name), Value::text(key)])));
                    }
                    if let Some(member @ Value::Member(..)) = Self::underlying(receiver).and_then(|worth| self.attribute(&worth, key)) {
                        return Ok(Some(member));
                    }
                }
                if Self::own_entry(base, self.detail("module")).is_none() {
                    let prototype = Self::native_word(base).and_then(|word| self.kind_by_word(&word)).unwrap_or_else(|| Value::Blueprint(base.clone()));
                    let inherited = self.read_class_member(prototype, key, false);
                    match inherited {
                        Ok(found) => {
                            // A classmethod found ties to the class even
                            // where the reading stands loose; anything
                            // else loose comes back as it lies.
                            let bind = (!loose || key == self.detail("subclass")) && matches!(&found, Value::Wrapped(tag, _) if [2, 10, 11, 12, 19, 30, 36, 60, 71, 73].contains(tag));
                            return Ok(Some(if bind { Self::wrap(3, vec![found, receiver.clone()]) } else { found }));
                        }
                        Err(escape) if self.missing_member_escape(&escape) => (),
                        Err(escape) => return Err(escape),
                    }
                }
                // Native slots are descriptors on their defining kind;
                // bind that descriptor before considering the root.
                if Self::native_word(base).is_some() {
                    if let Some(slot) = self.carried_by_kind(&Value::Blueprint(base.clone()), key) {
                        return Ok(Some(if loose { slot } else { Self::wrap(3, vec![slot, receiver.clone()]) }));
                    }
                }
                if self.table.single("ext.stmt.class.constructor")==Some(key) {
                    if Self::native_word(base).as_deref().and_then(|word|self.table.prims.get(word)).is_some_and(|op|matches!(op,Prim::Listed|Prim::Dictionary)) {
                        if let Some(entry)=self.inherited_entry(base,key) { return Ok(Some(self.member_binding(entry,Some(receiver.clone()),actual.clone())?)); }
                    }
                }
                // A member a native forebear carries without keeping an
                // entry of its own, `__hash__` or `__eq__` among them, is
                // answered off the worth the thing holds, which is what
                // the base's own reading of that name gives.
                if Self::native_beneath(base).is_some() {
                    if let Some(worth) = Self::underlying(receiver) {
                        if let Some(member) = self.attribute(&worth, key) { return Ok(Some(member)); }
                    }
                }
            }
            passed |= Self::parent_same_class(base, owner);
        }
        Ok(None)
    }
}
