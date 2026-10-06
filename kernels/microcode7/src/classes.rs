// A class keeps its direct forebears and the order in which their own
// members are sought. Calls and member wrappers use that order alike.
use super::*;

impl<'a> Machine<'a> {
    /// The type-method row of the table keeps the word that lists the
    /// classes made directly on a class in its second place.
    pub(super) fn is_offspring_word(&self, key: &str) -> bool {
        self.table.strings("ext.stmt.class.detail.order").get(1).is_some_and(|word| word == key)
    }
    fn offspring_listing(&self, parent: &Rc<Blueprint>) -> Value {
        Self::wrap(190, vec![Value::Blueprint(parent.clone())])
    }
    pub(super) fn detail(&self, key: &str) -> &str {
        match key {
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
                parents:Vec::new(),ancestry:Vec::new(),under:None,answers:Vec::new(),fields:Vec::new(),
                reaches:Vec::new(),methods:Vec::new(),constants:Vec::new(),shared:RefCell::new(Vec::new()),weak_slot:Cell::new(None),has_slot_storage: false, sealed:Cell::new(false), type_names: std::cell::RefCell::new(None)}));
        }
        self.ancestor.as_ref().unwrap().clone()
    }
    /// The blueprint standing for a native kind, made once for each word
    /// asked for. A thing of a blueprint beneath it keeps a worth of that
    /// kind among what it holds, under a name no program can write.
    pub(super) fn native_kind(&mut self,word:&str)->Rc<Blueprint> {
        if let Some((_,kind))=self.native_kinds.iter().find(|(w,_)|w==word){return kind.clone();}
        let mut root=self.common_ancestor();
        if self.table.prims.get(word)==Some(&Prim::Truthful) {
            let integer=self.table.prims.iter().find(|(_,p)|**p==Prim::AsInt).map(|(w,_)|w.to_string());
            if let Some(integer)=integer {root=self.native_kind(&integer);}
        }
        let mut ranks=vec![root.clone()];ranks.extend_from_slice(&root.ancestry);
        let mut protocols = if matches!(self.table.prims.get(word), Some(Prim::Zipped | Prim::Mapped | Prim::Filtered)) {
            let names = self.table.strings("ext.stmt.class.special");
            [(15usize, 4i64), (16, 3)].into_iter().filter_map(|(slot, operation)| names.get(slot).map(|name| (name.clone(), Self::wrap(120, vec![Value::Small(operation)])))).collect()
        } else { Vec::new() };
        if word == "Union" {
            protocols.push(("__repr__".to_owned(), Self::wrap(127, Vec::new())));
            protocols.push((self.detail("getitem").to_owned(), Self::wrap(5, vec![Self::wrap(77, vec![Value::text("#union")])])));
        }
        if word == "GenericAlias" {
            protocols.push(("__iter__".to_owned(), Self::wrap(125, vec![Value::Small(5)])));
            let names = [self.detail("allocate"), "__repr__", "__call__", "__mro_entries__", "__eq__"];
            protocols.extend(names.iter().enumerate().map(|(i, n)| (n.to_string(), Self::wrap(125, vec![Value::Small(i as i64)]))));
            protocols.push(("__or__".to_owned(), Self::wrap(125, vec![Value::Small(6)])));
            protocols.push(("__ror__".to_owned(), Self::wrap(125, vec![Value::Small(7)])));
        }
        if word == "SimpleNamespace" {
            let init = self.table.single("ext.stmt.class.constructor").unwrap_or_default();
            let operations = [self.detail("allocate"), init, "__repr__", "__eq__", "__ne__", "__reduce__", "__replace__"];
            protocols.extend(operations.iter().enumerate().map(|(operation, name)|
                (name.to_string(), Self::wrap(123, vec![Value::Small(operation as i64)]))));
            protocols.push(("__hash__".into(), Value::Nil));
        }
        let title = match word { "TypeVar" | "ParamSpec" | "TypeVarTuple" | "TypeAliasType" | "Generic" | "NoDefaultType" | "ParamSpecArgs" | "ParamSpecKwargs" => format!("typing.{word}"), "SimpleNamespace" | "GenericAlias" => format!("types.{word}"), "Union" => String::from("typing.Union"), _ => word.to_owned() };
        let kind=Rc::new(Blueprint {presentation:Some(format!("<class '{title}'>")),name:word.to_owned(),
            parents:vec![root.clone()],ancestry:ranks,under:Some(root),answers:Vec::new(),fields:Vec::new(),
            reaches:Vec::new(),methods:Vec::new(),constants:vec![("\0native".to_owned(),Value::text(word))],shared:RefCell::new(protocols),weak_slot:Cell::new(None),has_slot_storage: false, sealed:Cell::new(false), type_names: std::cell::RefCell::new(None)});
        if self.table.has_any("ext.stmt.class.builder") && matches!(self.table.prims.get(word), Some(Prim::ClassWork(9..=10))) {
            kind.shared.borrow_mut().push((self.detail("descriptor.get").to_owned(), Self::wrap(78, Vec::new())));
            if self.table.prims.get(word) == Some(&Prim::ClassWork(9)) {
                kind.shared.borrow_mut().push((self.detail("call").to_owned(), Self::wrap(79, Vec::new())));
            }
        }
        if word == "module" && !self.detail("kind").is_empty() {
            if let Some(key) = self.table.single("ext.stmt.class.constructor") {
                kind.shared.borrow_mut().push((key.to_owned(), Self::wrap(2, vec![Value::text("module")])));
            }
        }
        if word == "module" && !self.detail("kind").is_empty() {
            let dictionary = self.detail("namespace").to_owned();
            kind.shared.borrow_mut().push((dictionary.clone(), Self::wrap(32,
                vec![Value::text(&dictionary), Value::Blueprint(kind.clone()), Value::Small(-1)])));
            if let Some(represent) = self.table.strings("ext.stmt.class.special").get(1) {
                kind.shared.borrow_mut().push((represent.clone(), Self::wrap(60, vec![Value::text(word), Value::text(represent)])));
            }
        }
        self.native_kinds.push((word.to_owned(),kind.clone()));
        if matches!(word, "function" | "builtin_function_or_method" | "method" | "method_descriptor" | "wrapper_descriptor" | "type" | "NoneType") {
            for (at, key) in self.table.strings("ext.stmt.class.special").iter().enumerate() {
                if at == 8 || at == 17 && word != "NoneType" { kind.shared.borrow_mut().push((key.clone(), self.kind_entry(word, key))); }
            }
        }
        if matches!(word, "int" | "float" | "str" | "tuple" | "bytes" | "bytearray" | "dict" | "set" | "frozenset" | "complex" | "list") {
            let allocation = self.detail("allocate").to_string();
            kind.shared.borrow_mut().push((allocation, Self::wrap(14, vec![Value::text(word)])));
        }
        if matches!(word, "bytes" | "bytearray") {
            kind.shared.borrow_mut().push(("__buffer__".to_owned(), Value::Intrinsic(Prim::ValueMethod, Rc::from(format!("{word}.__buffer__")))));
            if word == "bytearray" { kind.shared.borrow_mut().push(("__release_buffer__".to_owned(), Value::Intrinsic(Prim::ValueMethod, Rc::from("bytearray.__release_buffer__")))); }
        }
        if let Some(representative) = self.kind_stand_in(word) {
            let owner = Value::Blueprint(kind.clone());
            let known = self.native_directory(&representative);
            let descriptors = known.into_iter().filter(|name|
                !["real", "imag", "numerator", "denominator", "start", "stop", "step", "__dir__", "__reduce_ex__"].contains(&name.as_str())
                && !(word == "generator" && name == "__setstate__")
                && !(self.table.prims.get(word) == Some(&Prim::Unchanging) && self.table.single("ext.stmt.class.constructor") == Some(name.as_str()))
                && !kind.shared.borrow().iter().any(|(key, _)| key == name))
                .filter_map(|name| self.carried_by_kind(&owner, &name).map(|entry| (name, entry))).collect::<Vec<_>>();
            kind.shared.borrow_mut().extend(descriptors);
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
    pub(super) fn ordinary_directory(&self, item: &Value) -> Value {
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
            return Value::Vector(crate::tuples::Sequence::plain(names.iter().map(|s|Value::text(s)).collect()));
        }
            if matches!(item, Value::Wrapped(62, _)) {
                let mut names = Vec::new();
                names.extend(self.table.strings("ext.stmt.async.generator.methods").iter().skip(3).cloned());
                names.extend([15, 16].iter().filter_map(|index| self.table.strings("ext.stmt.class.special").get(*index).cloned()));
                for label in ["ext.stmt.yield.send", "ext.stmt.yield.throw", "ext.stmt.yield.close"] {
                    names.extend(self.table.strings(label).iter().cloned());
                }
                names.sort(); names.dedup();
                return Value::Vector(crate::tuples::Sequence::plain(names.iter().map(|name| Value::text(name)).collect()));
            }
        // A walk over a routine's own body lists the walking pair it
        // answers to (through the mark below) and the few names a
        // language gives it for stepping it by hand.
        if let Value::Generator(_)=item {
            let mut names=self.native_directory(item);
                if self.is_async_generator(item) {
                    names.extend(self.table.strings("ext.stmt.async.generator.methods").iter().take(3).cloned());
                    names.extend(self.table.strings("ext.stmt.async.generator.fields").iter().cloned());
                    names.sort(); names.dedup();
                    return Value::Vector(crate::tuples::Sequence::plain(names.iter().map(|s|Value::text(s)).collect()));
                }
            for label in ["ext.stmt.yield.close","ext.stmt.yield.send","ext.stmt.yield.throw","ext.stmt.yield.running"] {
                if let Some(w)=self.table.strings(label).first() { names.push(w.clone()); }
            }
            for word in [14, 15, 21, 24].iter().filter_map(|i| self.table.strings("ext.builtin.exceptions.traceback").get(*i).cloned()).filter(|word| !word.is_empty()) {
                if !word.is_empty() { names.push(word); }
            }
            names.sort();names.dedup();
            return Value::Vector(crate::tuples::Sequence::plain(names.iter().map(|s|Value::text(s)).collect()));
        }
        // A native kind, named as the kind itself or held as a value
        // of one, answers the members a value of that kind has: the
        // methods of the kind and the special names its mark answers.
        if let Some(sample)=self.directory_stand_in(item) {
            let named=self.native_directory(&sample);
            return Value::Vector(crate::tuples::Sequence::plain(named.iter().map(|word|Value::text(word)).collect()));
        }
        let mut names=Vec::new();
        let class=match item{Value::Thing(t)=>{names.extend(t.holds.borrow().iter().filter(|(k,_)|!k.starts_with('\0')).map(|(k,_)|k.clone()));Some(&t.blueprint())},Value::Blueprint(b)=>Some(b),_=>None};
        if let Some(b)=class {
            for c in std::iter::once(b).chain(b.ancestry.iter()) {
                names.extend(c.shared.borrow().iter().filter(|(k,_)|!k.starts_with('\0')).map(|(k,_)|k.clone()));
                if let Some(word) = Self::native_word(c) {
                    if let Some(example) = self.kind_stand_in(&word) { names.extend(self.native_directory(&example)); }
                }
            }
        }
        else{names.extend(self.routine_holding_names(item));}
        if let Some(class) = class.filter(|_| !matches!(item, Value::Thing(_)) || !names.iter().any(|name| name == self.detail("kind"))) {
            for part in ["kind", "get", "set", "remove", "allocate", "subclass"] {
                let name = self.detail(part); if !name.is_empty() { names.push(name.to_owned()); }
            }
            if let Some(name) = self.table.single("ext.stmt.class.constructor") { names.push(name.to_owned()); }
            if self.allowed_slot(class, self.detail("namespace")) { names.push(self.detail("namespace").to_owned()); }
            if self.table.has_any("ext.builtin.weak.get") && self.admits_weak(class) { names.push("__weakref__".to_owned()); }
            names.extend(self.table.strings("ext.stmt.class.detail.root.members").iter().cloned());
            names.extend(self.table.strings("ext.stmt.class.special").get(72).cloned());
        }
        names.sort_unstable();names.dedup();Value::Vector(crate::tuples::Sequence::plain(names.iter().map(|s|Value::text(s)).collect()))
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
        std::iter::once(b).chain(b.ancestry.iter().map(Rc::as_ref)).find_map(Self::native_word)
    }

    /// Whether a blueprint stands on the native kind named, through any
    /// of its line: a blueprint written with several native parents
    /// stands on each of them, and answers to any asked after.
    pub(super) fn native_among(b:&Blueprint,word:&str)->bool {
        std::iter::once(b).chain(b.ancestry.iter().map(Rc::as_ref))
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
            parents:vec![root.clone()],ancestry:vec![root.clone()],under:Some(root),answers:Vec::new(),fields:Vec::new(),
            reaches:Vec::new(),methods:Vec::new(),constants:Vec::new(),shared:RefCell::new(Vec::new()),weak_slot:Cell::new(None),has_slot_storage: false, sealed:Cell::new(false), type_names: std::cell::RefCell::new(None)});
        self.builder_kind=Some(kind.clone());
        for at in [8, 17] { if let Some(key) = self.table.strings("ext.stmt.class.special").get(at) {
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
        std::iter::once(b).chain(b.ancestry.iter().map(Rc::as_ref)).find_map(Self::named_builder)
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
                if Rc::ptr_eq(current,&candidate)||current.ancestry.iter().any(|p|Rc::ptr_eq(p,&candidate)){continue;}
                if !candidate.ancestry.iter().any(|p|Rc::ptr_eq(p,current)) {
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
            "NotImplementedType"=>Some((Value::Refusal(Rc::from(self.table.strings("ext.stmt.class.special.declined").first().map_or("NotImplemented",String::as_str))),"NotImplementedType")),
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
        let made=if self.table.has_any("ext.stmt.class.builder") && matches!(op, Prim::ClassWork(9..=10)) {
            let mut values = positional;
            values.extend(named.into_iter().map(|(key, value)| Value::Couple(Rc::new((Value::text(&key), value)))));
            let Prim::ClassWork(operation) = op else { unreachable!() };
            self.work_on_class(operation, values)?
        } else if self.table.has_any("ext.stmt.class.builder") && op == Prim::Dictionary { self.core_primitive(op,word,positional,named)? } else if named.is_empty(){self.prim(op,word,&positional)?}else{self.core_primitive(op,word,positional,named)?};
        let made = if word == "str" { Self::underlying(&made).unwrap_or(made) } else { made };
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
                if let Some(begun)=self.table.single("ext.stmt.class.constructor").and_then(|w|self.inherited_entry(&m,w)) {
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
        if !(self.builds_classes(&explicit)||explicit.ancestry.iter().any(|p|self.builds_classes(p))) {
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
    fn reject_type_surrogates(&mut self, text: &Value) -> Result<(), Escape> {
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
            if Rc::ptr_eq(class, target) || class.ancestry.iter().any(|ancestor| Rc::ptr_eq(ancestor, target)) { return true; }
            let address = Rc::as_ptr(class);
            if seen.contains(&address) { return false; }
            seen.push(address);
            if class.fields.iter().any(|(key, held)| key == "\0also-under"
                && matches!(held, Value::Blueprint(other) if visit(other, target, seen))) { return true; }
            if class.parents.iter().any(|base| visit(base, target, seen)) { return true; }
            class.under.as_ref().is_some_and(|base| visit(base, target, seen))
        }
        if Rc::ptr_eq(class, target) || class.ancestry.iter().any(|ancestor| Rc::ptr_eq(ancestor, target)) { return true; }
        visit(class, target, &mut Vec::new())
    }
    fn visible_blueprint(&self,class:Rc<Blueprint>)->Value {
        if self.builds_classes(&class){return self.kind_builder_word();}
        if let Some(word)=Self::native_word(&class) {
            if let Some(op)=self.table.prims.get(word.as_str()).copied().filter(Self::names_a_kind) {
                return Value::Intrinsic(op,Rc::from(word));
            }
        }
        Value::Blueprint(class)
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
                if Rc::ptr_eq(&candidate,&previous)||previous.ancestry.iter().any(|p|Rc::ptr_eq(p,&candidate)){continue;}
                if !candidate.ancestry.iter().any(|p|Rc::ptr_eq(p,&previous)) {
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
        if self.table.has_any("ext.system.module.cache") {
            for (key, value) in &entries {
                if key == "__abc_tpflags__" && matches!(value.settled(), Value::Small(bits) if bits & 96 == 96) {
                    return Err(format!("TypeError: type {title} has both Py_TPFLAGS_SEQUENCE and Py_TPFLAGS_MAPPING set").into());
                }
            }
        }
        let mut queues=Vec::new();
        for base in &parents {
            let mut queue=Vec::with_capacity(base.ancestry.len()+1);
            queue.push(base.clone());queue.extend_from_slice(&base.ancestry);queues.push(queue);
        }
        queues.push(parents.clone());
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
        // One thing cannot keep worths of two native kinds.
        let mut natives:Vec<String>=ranks.iter().filter_map(|b|Self::native_word(b)).collect();
        natives.dedup();
        if natives.len()>1 {return Err(self.table.single("ext.stmt.class.layout").unwrap_or(self.detail("unready")).to_owned().into());}
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
        let native_name = self.table.single("ext.stmt.class.native.name")
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
            under:primary,parents,ancestry:ranks,answers:vec![],fields:vec![],reaches:vec![],
            methods:vec![],constants:fixed,
            shared:RefCell::new(entries),weak_slot:Cell::new(None),has_slot_storage: storage, sealed:Cell::new(false), type_names: RefCell::new(type_names)});
        if let Some(cell) = class_cell { let word = self.detail("cell.contents").to_owned(); self.alter_class_member(cell, &word, Some(Value::Blueprint(class.clone())), true)?; }

        if self.table.has_any("ext.builtin.weak.get") { crate::ghost::note(crate::ghost::Ghost::Blueprint(Rc::downgrade(&class))); }
        self.name_slots(&class)?;
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
        let hook=class.ancestry.iter().find_map(|b|Self::own_entry(b,self.detail("subclass")));
        if let Some(f)=hook {
            if matches!(&f,Value::Wrapped(5,_)){let bound=self.member_binding(f,None,class.clone())?;self.apply_class_member(bound,handed)?;}
            else{let mut given=vec![Value::Blueprint(class.clone())];given.extend(handed);self.apply_class_member(f,given)?;}
        }
        // What was handed over with no hook to take it is refused.
        else if !handed.is_empty(){return Err(format!("TypeError: {}.__init_subclass__() takes no keyword arguments", class.name).into());}
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
            if self.table.has_any("ext.builtin.weak.get") && self.admits_weak(class) && class.parents.iter().all(|parent| !self.admits_weak(parent)) {
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
                class.ancestry.iter().any(|b|!Rc::ptr_eq(b,&root)&&Self::native_word(b).is_none()&&Self::own_entry(b,&slots_word).is_none())
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
        let of_line=Rc::ptr_eq(&t.blueprint(),declared)||t.blueprint().ancestry.iter().any(|b|Rc::ptr_eq(b,declared));
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
        if parts[0].bare() == "__weakref__" && self.table.has_any("ext.builtin.weak.get") {
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
        if parts[0].bare() == "__weakref__" && self.table.has_any("ext.builtin.weak.get") { return Err(format!("AttributeError: attribute '__weakref__' of '{}' objects is not writable", t.blueprint().name).into()); }
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
        if let Some(word)=self.table.single("ext.stmt.class.constructor"){entries.push((word.to_owned(),Self::wrap(56,Vec::new())));}
        for part in ["property.fget","property.fset","property.fdel","doc","property.is_abstract"] {
            entries.push((self.detail(part).to_owned(),Self::wrap(58,vec![Value::text(Self::accessor_key(part))])));
        }
        let kind=Rc::new(Blueprint {presentation:Some(format!("<class '{title}'>")),name:title,
            parents:vec![root.clone()],ancestry:vec![root.clone()],under:Some(root),answers:Vec::new(),fields:Vec::new(),
            reaches:Vec::new(),methods:Vec::new(),constants:Vec::new(),shared:RefCell::new(entries),weak_slot:Cell::new(None),has_slot_storage: false, sealed:Cell::new(false), type_names: std::cell::RefCell::new(None)});
        self.property_kind=Some(kind.clone());
        kind
    }
    /// The keys a property keeps its accessors under, out of reach of
    /// any name a program can write.
    fn accessor_key(part:&str)->&'static str {
        match part {"property.fget"=>"\0fget","property.fset"=>"\0fset","property.fdel"=>"\0fdel","doc"=>"\0doc","property.is_abstract"=>"\0abstract",_=>"\0name"}
    }
    /// Whether a value stands for the property builtin read as a class:
    /// its word, before anything else was bound to that name.
    pub(super) fn spells_property_kind(&self,value:&Value)->bool {
        let word=match value {Value::Intrinsic(_,w)=>w.as_ref(),Value::Wrapped(8,parts)=>match parts.first(){Some(Value::Text(t))=>t.as_ref(),_=>return false},_=>return false};
        self.table.prims.get(word)==Some(&Prim::ClassWork(11))
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
        let of=match about {Value::Thing(t)=>t.blueprint().name.clone(),Value::Blueprint(b)=>if self.detail("name").is_empty(){b.name.clone()}else{Self::builder_over(b).map_or_else(||b.name.clone(),|builder|builder.type_names.borrow().as_ref().map_or_else(||builder.name.clone(),|names|names.short.type_text().bare()))},other=>other.bare()};
        format!("{}{title}{}{of}{}",words[0],words[1],words[2]).into()
    }
    /// What a property's kept accessor shows: itself; or, for the first
    /// string, the one given, else the getter's own.
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
        }
        Ok(Value::Nil)
    }
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
                let mut holds:Vec<(String,Value)>=property.holds.borrow().iter().filter(|(k,_)|k!=key&&k!="\0name").cloned().collect();
                holds.push((key.to_owned(),accessor.clone()));
                self.made+=1;
                Ok(Value::Thing(Rc::new(Thing{reclassified: RefCell::new(None), of:property.blueprint().clone(),holds:RefCell::new(holds),turn:self.made})))
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
                let mut holds=property.holds.borrow_mut();
                for (key,v) in ["\0fget","\0fset","\0fdel","\0doc"].into_iter().zip(taken){holds.push((key.to_owned(),v));}
                Ok(Value::Nil)
            }
            57=>{
                let [_,name]=rest else{return Err(self.class_unready())};
                Self::change_entry(&mut property.holds.borrow_mut(),"\0name",Some(name.clone()));
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
        let bound=self.member_binding(hook,Some(member.clone()),t.blueprint().clone())?;
        self.apply_class_member(bound,values)
    }
    /// A member that takes writes speaks before a thing's own entries;
    /// one that only reads gives way to them.
    fn writes_too(&self,member:&Value)->bool {
        if let Value::Wrapped(tag,_)=member {return matches!(tag,6|32|58);}
        self.protocol_entry(member,"descriptor.set").is_some()||self.protocol_entry(member,"descriptor.delete").is_some()
    }
    /// Whether an escape tells of a member that is not there, in words
    /// or as a raised thing of that kind.
    pub(super) fn missing_member_escape(&self,escape:&Escape)->bool {
        let Some(kind)=self.detail("attribute.amiss").split(':').next().filter(|k|!k.is_empty()) else{return false};
        match escape {
            Escape::Error(words)=>words.split(':').next()==Some(kind),
            Escape::Thrown(Value::Thing(t))=>t.blueprint().name==kind||t.blueprint().ancestry.iter().any(|b|b.name==kind),
            _=>false,
        }
    }
    fn own_entry(b:&Blueprint,key:&str)->Option<Value> {
        if let Some((_, held)) = b.shared.borrow().iter().find(|(word, _)| word == key) {
            return (!matches!(held.settled(), Value::Unset)).then(|| held.clone());
        }
        b.methods.iter().find(|(n,_)|n==key).map(|(_,p)|Value::Routine(p.clone()))
            .or_else(||b.constants.iter().find(|(n,_)|n==key).map(|(_,v)|v.clone()))
    }
    pub(super) fn native_declares_protocol(&self, spelling: &str, key: &str) -> bool {
        self.table.strings("ext.stmt.class.detail.native.protocols").chunks_exact(2)
            .any(|pair| pair[0] == spelling && pair[1].split_whitespace().any(|entry| entry == key))
    }
    pub(super) fn allocation_changed(&self, class: &Blueprint) -> bool {
        if self.table.strings("ext.stmt.class.detail.native.protocols").is_empty() { return false; }
        let key = self.detail("allocate");
        for parent in std::iter::once(class).chain(class.ancestry.iter().map(Rc::as_ref)) {
            if let Some(value) = Self::own_entry(parent, key) { return !matches!(value, Value::Wrapped(14, _)); }
            if Self::native_word(parent).is_some_and(|word| self.native_declares_protocol(&word, key)) { return false; }
        }
        false
    }
    pub(super) fn inherited_entry(&self,b:&Blueprint,key:&str)->Option<Value> {
        std::iter::once(b).chain(b.ancestry.iter().map(Rc::as_ref)).find_map(|parent| {
            Self::own_entry(parent, key).or_else(|| {
                if !self.table.has_class_order { return None; }
                let spelling = Self::native_word(parent)?;
                if self.table.strings("ext.stmt.class.special").get(8).is_some_and(|hash| hash == key) {
                    match spelling.as_str() { "dict" | "set" | "bytearray" | "list" => return Some(Value::Nil), _ => {} }
                }
                if !self.native_declares_protocol(&spelling, key) { return None; }
                let sample = self.kind_stand_in(&spelling)?;
                let protocols = self.table.strings("ext.stmt.class.special");
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
    pub(super) fn apply_class_member(&mut self,f:Value,mut values:Vec<Value>)->Res {
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
            Value::Intrinsic(Prim::ClassWork(op), _) if self.table.has_any("ext.stmt.class.builder") => self.work_on_class(op, values),
            Value::Intrinsic(Prim::SortOf, _) if self.table.has_any("ext.stmt.class.builder") => self.class_from_type(values),
            Value::Intrinsic(operation, name) => {
                let (mut input, keywords) = self.open_arguments(values)?;
                if let Some(answer) = self.builtin_names(operation, &name, &mut input, keywords)? { return Ok(answer); }
                let result = self.prim(operation, &name, &input);
                if let Some(raised) = self.got_away.take() { return Err(raised); }
                Ok(result?)
            }
            // A method tied to a value of a native kind, reached as a
            // value of its own and then called.
            Value::Member(receiver,operation)=>{
                if self.table.strings("ext.stmt.class.detail.native.protocols").is_empty() {
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
            Value::Wrapped(tag,kept)=>{
                match tag {
                    130 => {
                        if !values.is_empty() { return Err(String::from("TypeError: function takes no arguments").into()); }
                        Ok(self.prim(Prim::Weigh, "eval", &kept)?)
                    }
                    // The classes standing directly on the one kept, read
                    // anew whenever the member is called.
                    190 => {
                        if !values.is_empty() { return Err(format!("TypeError: {}.__subclasses__() takes no arguments ({} given)", match kept.first() { Some(Value::Blueprint(owner)) => owner.name.clone(), _ => String::new() }, values.len()).into()); }
                        let Some(Value::Blueprint(parent)) = kept.first() else { return Err(self.class_unready()) };
                        let seen = crate::ghost::offspring_of(parent).into_iter().map(|child| self.visible_blueprint(child)).collect::<Vec<_>>();
                        Ok(Value::Vector(crate::tuples::Sequence::plain(seen)))
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
                    125 => self.alias_action(&kept, values),
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
                    77 if kept.first().is_some_and(|mark| mark.bare() == "#union") && values.len() == 2 => {
                        let operands = match values[1].settled() { Value::Tuple(row) => row.to_vec(), single => vec![single] };
                        if operands.is_empty() { return Err(String::from("TypeError: Cannot take a Union of no types.").into()); }
                        let namespace = self.load_namespace("typing")?;
                        let check = self.read_class_member(namespace, "_type_check", false)?;
                        let mut joined: Option<Value> = None;
                        for operand in operands {
                            let vetted = self.apply_class_member(check.clone(), vec![operand, Value::text("Union[arg, ...]: each arg must be a type.")])?;
                            joined = Some(match joined { None => vetted, Some(so_far) => self.combined_types(&[so_far, vetted]) });
                        }
                        Ok(joined.expect("one operand at least"))
                    }
                    77 => match values.as_slice() {
                        [owner @ Value::Blueprint(_), _] if self.table.has_any("ext.stmt.type_params.open") => Ok(owner.clone()),
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
                    1 if !values.is_empty()=>{
                        let Some(Value::Blueprint(c))=values.first() else{return Err(self.class_unready())};
                        if self.is_fault_kind(c) { return Ok(self.make_fault(c.clone(), values[1..].to_vec(), Value::Nil)); }
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
                        let key = self.table.single("ext.stmt.class.constructor").unwrap_or_default().to_owned();
                        self.fault_method(receiver, &key, &values)
                    }
                    2 if values.len()==1=>Ok(Value::Nil),
                    2 if values.len()>1=>{
                        let Some(Value::Thing(t))=values.first() else{return Err(self.class_unready())};
                        self.root_turns_away(&t.blueprint(),'i')?;
                        Ok(Value::Nil)
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
                    143 => {
                        if !values.is_empty() { return Err(String::from("TypeError: this code object takes no arguments").into()); }
                        self.text_performed(true, "eval", &kept[..2]).map_err(|message| self.got_away.take().unwrap_or(Escape::Error(message)))
                    }
                    43 => {

                        self.apply_class_member(kept[0].clone(), values)
                    }
                    72 => Ok(Value::Mutable(Rc::new(RefCell::new(Value::Dict(Rc::new(Vec::new().into())))), true)),
                    36=>{
                        if kept.get(1).is_some_and(|owner|owner.bare()!=self.detail("root")) {
                            if let Some(native)=values.first().and_then(Self::underlying){values[0]=native.settled();}
                        }
                        let named=kept[0].bare();self.root_answers(&named,values)
                    }
                    // The maker of a native kind: the blueprint to make a
                    // thing of, then what the kind's primitive takes.
                    14 if !values.is_empty()=>{
                        let target = values.remove(0);
                        let word=kept[0].bare();
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
                        let Value::Blueprint(c)=target else{return Err(self.class_unready());};
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
                    3 | 132=>{values.insert(0,kept[1].clone());if self.table.has_any("ext.stmt.class.builder"){self.apply_held(kept[0].clone(),values)}else{self.apply_class_member(kept[0].clone(),values)}},
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
                            if word == "module" && self.table.strings("ext.stmt.class.special").get(1).is_some_and(|name| name == &entry) {
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
                        let held = values.remove(0);
                        let subject = if matches!(held.settled(), Value::Thing(_)) { held.settled() } else { held.keep(false) };
                        // A thing of a class standing on the very kind
                        // this word names answers as its worth would,
                        // since the loose entry is the kind's own and
                        // not the class's: `set.union(s, ...)` for `s`
                        // a subclass of `set` works upon what `s` keeps
                        // of a set.
                        if word == "module" && self.table.strings("ext.stmt.class.special").get(1).is_some_and(|key| key == &entry) {
                            if !values.is_empty() { return Err(format!("TypeError: expected 0 arguments, got {}", values.len()).into()); }
                            return self.describe_module(subject);
                        }
                        let receiver=match &subject {
                            Value::Thing(t) if Self::native_beneath(&t.blueprint()).as_deref()==Some(word.as_str()) =>
                                Self::underlying(&subject).unwrap_or_else(||subject.clone()),
                            _=>subject.clone(),
                        };
                        let subscription = self.table.strings("ext.stmt.class.special").get(11).map_or(false, |slot| slot == &entry);
                        if subscription && word == "dict" && values.len() == 1 {
                            if let Value::Thing(instance) = &subject {
                                if let Value::Dict(store) = receiver.settled() {
                                    let location = self.map_locate(&store, Some(&store), &values[0])?.0;
                                    if let Some(index) = location { return Ok(store[index].1.clone()); }
                                    let missing = self.table.single("ext.stmt.class.missing").unwrap_or("");
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
                            let slots = self.table.strings("ext.stmt.class.special");
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
                    4 if self.table.has_any("ext.stmt.class.builder") => self.apply_held(kept[0].clone(), values),
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
                        let held = Self::underlying(descriptor).ok_or_else(|| self.class_unready())?;
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
                    10|11|12 if values.len()>=2=>{
                        let subject=values[0].clone();
                        let Value::Text(key)=&values[1]else{return Err("TypeError: attribute name must be string".to_string().into());};
                        if tag==10 {self.read_class_member(subject,key,true)}
                        else {self.alter_class_member(subject,key,if tag==11{values.get(2).cloned()}else{None},true)}
                    }
                    // A binding member's reader, called as the program
                    // calls it: with the thing, or nothing and the class.
                    31 if matches!(values.len(),1|2)=>{
                        let receiver=match &values[0]{Value::Nil=>None,other=>Some(other.clone())};
                        let owner=match (values.get(1),&receiver) {
                            (Some(Value::Blueprint(b)),_)=>b.clone(),
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
        entries.push(("_idfunc".into(), Value::Intrinsic(Prim::ClassWork(22), Rc::from("_idfunc"))));
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
        self.imported.insert(String::from("_typing"), space.clone());
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
        if Self::native_word(&class).is_some() && matches!(class.name.as_str(), "TypeVar" | "ParamSpec" | "TypeVarTuple" | "TypeAliasType" | "NoDefaultType") {
            return self.make_type_parameter(class, given);
        }

        if Self::native_word(&class).as_deref() == Some("function") {
            if matches!(given.first().map(Value::settled), Some(Value::Thing(code)) if self.table.single("ext.builtin.compile.kind") == Some(code.blueprint().name.as_str())) {
                if given.len() != 2 || !matches!(given[1].settled(), Value::Dict(_)) { return Err(String::from("TypeError: function() requires code and globals").into()); }
                return Ok(Self::wrap(130, given));
            }
        }
        if (class.name == "FunctionType" || Self::native_word(&class).as_deref() == Some("function")) && self.table.has_any("ext.builtin.exceptions.traceback") && matches!(given.first().map(Value::settled), Some(Value::Wrapped(7, _))) {
            let (positional, keywords) = self.open_arguments(given)?;
            let mut options = vec![None; 5];
            for (slot, value) in positional.into_iter().enumerate() {
                if slot >= options.len() { return Err(String::from("TypeError: function() takes at most 5 arguments").into()); }
                options[slot] = Some(value);
            }
            for (word, value) in keywords {
                let Some(slot) = ["code", "globals", "name", "argdefs", "closure"].iter().position(|name| *name == word) else {
                    return Err(format!("TypeError: function() got an unexpected keyword argument '{word}'").into());
                };
                if options[slot].replace(value).is_some() { return Err(String::from("TypeError: invalid function arguments").into()); }
            }
            let Some(Value::Wrapped(7, body)) = options[0].as_ref().map(Value::settled) else { return Err(String::from("TypeError: function() argument 'code' must be code").into()); };
            let Some(Value::Routine(origin) | Value::Bound(origin, _)) = body.first() else { return Err(String::from("TypeError: function() argument 'code' must be code").into()); };
            let Some(globals) = options[1].clone() else { return Err(String::from("TypeError: function() missing required argument 'globals'").into()); };
            if !matches!(globals.settled(), Value::Dict(_)) { return Err(String::from("TypeError: function() argument 'globals' must be dict").into()); }
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
            fresh.globe = Some(globals);
            fresh.born = Some(self.builtins_here());
            let source = Rc::new(fresh);
            let base = shared_room.or_else(|| layers.first().cloned()).unwrap_or_else(|| self.outermost.clone());
            let (source, room) = if let Some(Value::Tuple(defaults)) = options[3].as_ref().map(Value::settled) {
                let (adjusted, kept) = self.respared(&source, &base, base.clone(), Some(defaults.as_ref().clone()), None);
                (Rc::new(adjusted), kept)
            } else { (source, base) };
            let callable = Value::Bound(source, room);

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
            if Rc::ptr_eq(&builder,called)||builder.ancestry.iter().any(|p|Rc::ptr_eq(p,called)) {
                let hook=self.table.single("ext.stmt.class.constructor").and_then(|key|self.inherited_entry(&builder,key));
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
        let native = match Self::native_beneath(&class) {
            Some(word) if word == "Generic" && self.table.has_any("ext.stmt.type_params.open") => None,
            other => other,
        };
        let allocator=self.inherited_entry(&class,self.detail("allocate")).filter(|value| {
            !matches!(value, Value::Wrapped(14, _))
                || Self::own_entry(&class, self.detail("allocate")).is_some()
        });
        // A metaclass called outright builds a class, the way the kind
        // primitive does, from a name, parents and a namespace.
        let metaclass=self.builds_classes(&class)||class.ancestry.iter().any(|b|self.builds_classes(b));
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
                    Some(Prim::Dictionary) if self.table.single("ext.stmt.class.constructor").and_then(|key| self.inherited_entry(&class, key)).is_some() => Vec::new(),
                    Some(Prim::AsReal) if self.table.single("ext.stmt.class.constructor").and_then(|key| self.inherited_entry(&class, key)).is_some() => {
                        self.open_arguments(given.clone())?.0.into_iter().take(1).collect()
                    },
                    // A mutable container takes its members in its own
                    // `__init__`; where the class writes one, the kind
                    // primitive is asked only to allocate the empty thing
                    // and the class's method is handed the arguments.
                    Some(Prim::Listed | Prim::Dictionary) if self.table.single("ext.stmt.class.constructor").and_then(|key| self.inherited_entry(&class, key)).is_some() => Vec::new(),
                    Some(Prim::Filtered) if self.table.single("ext.stmt.class.constructor").and_then(|key| self.inherited_entry(&class, key)).is_some() => self.open_arguments(given.clone())?.0,
                    Some(Prim::Unchanging | Prim::Tupling) if self.table.single("ext.stmt.class.constructor").and_then(|key| self.inherited_entry(&class, key)).is_some() => self.open_arguments(given.clone())?.0,
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
                if self.table.single("ext.builtin.exceptions.traceback").is_some() { crate::ghost::anchor(thing); }
            }
            if self.table.single("ext.stmt.class.destructor").is_some()
                || self.table.single("ext.stmt.class.finaliser").is_some() {
                self.things.borrow_mut().push(Rc::downgrade(thing));
            }
            let belongs=Rc::ptr_eq(&thing.blueprint(),&class)||thing.blueprint().ancestry.iter().any(|c|Rc::ptr_eq(c,&class));
            if belongs {
                let constructor=self.table.single("ext.stmt.class.constructor").and_then(|word|self.inherited_entry(&thing.blueprint(),word));
                if let Some(f)=constructor {
                    let bound=self.member_binding(f,Some(created.clone()),thing.blueprint().clone())?;
                    if !matches!(self.apply_class_member(bound,given)?,Value::Nil){return Err(self.class_unready());}
                }else if let Some(under @ Value::Set(_)) = Self::underlying(&created).filter(|v| !v.set_sealed()) {
                    let (positional, named) = self.open_arguments(given)?;
                    let key = self.table.single("ext.stmt.class.constructor").unwrap_or_default().to_owned();
                    self.value_member(&under, &key, positional, named)?;
                }else if native.is_none()&&self.inherited_entry(&class,self.detail("allocate")).is_none(){
                    let (positional, keywords) = self.open_arguments(given)?;
                    if !positional.is_empty() || !keywords.is_empty() { self.root_turns_away(&class,'n')?; }
                }
            }
        }
        Ok(created)
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
            Value::Thing(thing) if self.table.has_any("ext.system.module.name") && Self::native_beneath(&thing.blueprint()).as_deref() == Some("module") => {
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
        let args_key = self.table.single("ext.builtin.exceptions.args").unwrap_or("args");
        let old = held.iter().find(|(name, _)| name == "\0raised-values").and_then(|(_, item)| match item { Value::Arguments(items) => Some(items.clone()), _ => None });
        if old.as_ref().is_some_and(|items| items.is_empty() || items.len() == 1 && items[0].bare() == key) {
            let fresh = crate::tuples::Sequence::plain(vec![Value::text(&said)]);
            for (name, item) in held.iter_mut() {
                if name == "\0raised-values" { *item = Value::Arguments(fresh.clone()); }
                if name == args_key { *item = Value::Tuple(fresh.clone()); }
            }
        }
        for (name, item) in held.iter_mut() {
            if self.table.single("ext.builtin.exceptions.name") == Some(name.as_str()) { *item = Value::text(key); }
            if self.table.single("ext.builtin.exceptions.object") == Some(name.as_str()) { *item = value.clone(); }
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
    /// Subscription through a descriptor reads its result. It does not
    /// turn the descriptor stored on the class into a shared field.
    pub(super) fn reads_descriptor(&self, subject: &Value, key: &str) -> bool {
        let Value::Thing(instance) = subject else { return false };
        self.inherited_entry(&instance.blueprint(), key).is_some_and(|entry| {
            matches!(&entry, Value::Wrapped(6 | 32, _) | Value::Adorned(_))
                || self.protocol_entry(&entry, "descriptor.get").is_some()
        })
    }

    pub(super) fn member_binding(&mut self,entry:Value,receiver:Option<Value>,owner:Rc<Blueprint>)->Res {
        if let Value::Wrapped(60, parts) = &entry {
            if parts[0].bare() == "int" {
                let value = receiver.as_ref().cloned().unwrap_or(Value::Blueprint(owner.clone()));
                if let Some(integer) = self.integer_attribute(&value, &parts[1].bare()) { return Ok(integer); }
            }
            if parts[0].bare() == "dict" && self.table.spells("ext.builtin.method.fromkeys", &parts[1].bare()) {
                return Ok(Value::Member(Rc::new(Value::Blueprint(owner)), parts[1].bare()));
            }
        }
        match &entry {
            Value::Wrapped(134, ref parts) if parts[0].bare() == "normal_pdf" && receiver.is_some() => return Ok(Self::wrap(3, vec![entry.clone(), receiver.unwrap()])),
            Value::Wrapped(2, items) if !items.is_empty() && receiver.is_some() => return Ok(Self::wrap(3, vec![entry.clone(), receiver.unwrap()])),
            Value::Wrapped(120, _) if receiver.is_some() => return Ok(Self::wrap(3, vec![entry.clone(), receiver.unwrap()])),
            Value::Wrapped(133 | 60 | 120 | 123 | 125 | 127, _) if receiver.is_some() => return Ok(Self::wrap(3, vec![entry.clone(), receiver.unwrap()])),
            Value::Wrapped(4,items)=>return Ok(items[0].clone()),
            Value::Wrapped(5,items)=>return Ok(Self::wrap(3,vec![items[0].clone(),Value::Blueprint(owner)])),
            Value::Wrapped(6,items) if receiver.is_some()=>return self.apply_class_member(items[0].clone(),vec![receiver.unwrap()]),
            Value::Wrapped(32,items) if receiver.is_some()=>return self.slot_value(&receiver.unwrap(),items),
            // A working of the property blueprint, reached through a
            // property, is tied to it; a kept accessor reads at once.
            Value::Wrapped(60, _) if receiver.is_some()=>return Ok(Self::wrap(3,vec![entry.clone(),receiver.unwrap()])),
            Value::Wrapped(36 | 50..=57 | 78..=79,_) if receiver.is_some()=>return Ok(Self::wrap(3,vec![entry.clone(),receiver.unwrap()])),
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
        if let Some(word)=self.table.single("ext.system.module.builtins").map(str::to_owned) {
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
            if source_code.flags!=code.flags {
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
        let builds=self.table.single("ext.stmt.class.constructor").map_or(false,|word|self.inherited_entry(class,word).is_some());
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
    /// A member every class and thing has from the root: one the table
    /// names among the root's members, or, read off a thing, one of the
    /// hooks for making, constructing, reading, writing, removing and
    /// formatting, left loose to be bound to it.
    fn from_the_root(&self,key:&str,of_a_thing:bool,class:&Blueprint)->Option<Value> {
        if key.is_empty(){return None;}
        if self.table.strings("ext.stmt.class.detail.root.members").iter().any(|word|word==key) {
            let owner = Self::native_beneath(class).unwrap_or_else(|| self.detail("root").to_owned());
            let cache_key = format!("{owner} member {key}");
            let mut cache = self.loose_members.borrow_mut();
            return Some(cache.entry(cache_key).or_insert_with(|| Self::wrap(36,vec![Value::text(key),Value::text(&owner)])).clone());
        }
        if !of_a_thing{return None;}
        let tag=if key==self.detail("allocate"){1}
            else if self.table.single("ext.stmt.class.constructor")==Some(key)||key==self.detail("subclass"){2}
            else if key==self.detail("get"){10}else if key==self.detail("set"){11}else if key==self.detail("remove"){12}
            else if self.table.strings("ext.stmt.class.special").get(72).map_or(false,|word|word==key){59}
            else{return None};
        Some(Self::wrap(tag, if key == self.detail("subclass") { vec![Value::text(key)] } else { Vec::new() }))
    }
    /// The root's own answer for one of its members, the value it
    /// answers about coming first.
    fn root_answers(&mut self,named:&str,values:Vec<Value>)->Res {
        let which=self.table.strings("ext.stmt.class.detail.root.members").iter().position(|word|word==named);
        if which == Some(9) && values.len() != 1 { return Err(self.method_fault("arguments").into()); }
        let not_mine=Value::Refusal(Rc::from(self.table.single("ext.stmt.class.special.declined").unwrap_or("NotImplemented")));
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
            Some(9)=>self.ordinary_directory(&first),
            Some(10)=>Self::held_as_state(&first),
            Some(11|12)=>{
                if which == Some(12) {
                    match self.ask_special(&first, 79, &[])? {
                        None => {},
                        Some(reduction) => return Ok(reduction),
                    }
                }
                self.reduction_permitted(&first)?;
                let kind=self.class_from_type(vec![first.clone()])?;
                let state = if let Value::Thing(object) = &first {
                    let name = self.table.strings("ext.stmt.class.detail.root.members").get(10).cloned().unwrap_or_default();
                    if let Some(getter) = self.inherited_entry(&object.blueprint(), &name) {
                        let bound = self.member_binding(getter, Some(first.clone()), object.blueprint())?;
                        self.apply_class_member(bound, Vec::new())?
                    } else { Self::held_as_state(&first) }
                } else { Self::held_as_state(&first) };
                Value::tuple(vec![kind,Value::tuple(Vec::new()),state])
            }
            Some(13)=>Value::Small(match &first{Value::Thing(t) if t.blueprint().name!=self.detail("root")=>24,_=>16}),
            _=>return Err(self.class_unready()),
        })
    }
    /// What a thing holds of its own as a dictionary, or nothing where
    /// it holds nothing.
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
        let word=self.table.single("ext.stmt.class.annotations").unwrap_or_default().to_owned();
        if let Some(own)=Self::own_entry(b,&word){return Ok(own);}
        let title = self.table.strings("ext.stmt.class.detail.code.fields").get(10).cloned().unwrap_or_default();
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
                else { Value::Intrinsic(Prim::AsInt, Rc::from(self.table.single("builtin.to_int")?)) };
            return Some(Value::Member(Rc::new(owner), "integer_from_bytes".into()));
        }
        let at = words.iter().take(3).position(|word| word == key)?;
        if at == 2 { return Some(Value::Member(Rc::new(value.clone()), "integer_size".into())); }
        if !is_type { return None; }
        let slot = if at == 1 { 4 } else if derived { 6 } else { 3 };
        words[slot].parse().ok().map(Value::Small)
    }
    pub(super) fn read_class_member(&mut self,value:Value,key:&str,direct:bool)->Res {
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

        if let Value::Member(owner, operation) = &value {
            if key == self.detail("receiver") { return Ok(owner.as_ref().clone()); }
            let public_format = (operation == "float_getformat").then(|| self.table.single("ext.builtin.method.getformat")).flatten();
            if key == self.detail("qualified") {
                return Ok(Value::text(&public_format.map(str::to_owned).unwrap_or_else(|| [owner.kind_word(), operation.clone()].join("."))));
            }
            if key == self.detail("name") {
                return Ok(Value::text(public_format.and_then(|word| word.rsplit('.').next()).unwrap_or(operation)));
            }
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

        let value = if self.table.has_any("ext.stmt.class.builder") { value.settled() } else { value };
        if self.table.has_any("ext.builtin.weak.get") {
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
                        let belongs = Rc::ptr_eq(b, defining) || b.ancestry.iter().any(|parent| Rc::ptr_eq(parent, defining));
                        if belongs { b.clone() } else { Self::builder_over(b).unwrap_or_else(|| b.clone()) }
                    }
                    _ => return Err(self.class_unready()),
                };
                let mut passed = false;
                for base in std::iter::once(&actual).chain(actual.ancestry.iter()) {
                    if passed {
                        if key == self.detail("allocate") {
                            if let Some(native) = Self::native_word(base).filter(|word| word != self.detail("root")) {
                                return Ok(Self::wrap(14, vec![Value::text(&native)]));
                            }
                        }
                        if let Some(entry) = Self::own_entry(base, key) {
                            let receiver = if key == self.detail("allocate") { None } else { Some(instance.clone()) };
                            return self.member_binding(entry, receiver, actual.clone());
                        }
                        if Self::native_word(base).is_some() || self.is_fault_kind(base) {
                            let maker = key == self.detail("allocate") || self.table.single("ext.stmt.class.constructor") == Some(key);
                            let working = Self::underlying(instance).and_then(|worth| self.attribute(&worth, key));
                            if maker || matches!(working, Some(Value::Member(..))) {
                                return Ok(Self::wrap(73, vec![instance.clone(), Value::text(&defining.name), Value::text(key)]));
                            }
                        }
                        if Self::own_entry(base, self.detail("module")).is_none() {
                            let prototype = Self::native_word(base).and_then(|word| self.kind_by_word(&word)).unwrap_or_else(|| Value::Blueprint(base.clone()));
                            let inherited = self.read_class_member(prototype, key, false);
                            match inherited {
                                Ok(found) => {
                                    let bind = matches!(&found, Value::Wrapped(tag, _) if [2, 10, 11, 12, 19, 30, 60, 71].contains(tag));
                                    return Ok(if bind { Self::wrap(3, vec![found, instance.clone()]) } else { found });
                                }
                                Err(escape) if self.missing_member_escape(&escape) => (),
                                Err(escape) => return Err(escape),
                            }
                        }
                        // Native slots are descriptors on their defining kind;
                        // bind that descriptor before considering the root.
                        if Self::native_word(base).is_some() {
                            if let Some(slot) = self.carried_by_kind(&Value::Blueprint(base.clone()), key) {
                                return Ok(Self::wrap(3, vec![slot, instance.clone()]));
                            }
                        }
                        if self.table.single("ext.stmt.class.constructor")==Some(key) {
                            if Self::native_word(base).as_deref().and_then(|word|self.table.prims.get(word)).is_some_and(|op|matches!(op,Prim::Listed|Prim::Dictionary)) {
                                if let Some(entry)=self.inherited_entry(base,key) { return self.member_binding(entry,Some(instance.clone()),actual.clone()); }
                            }
                        }
                        // A member a native forebear carries without
                        // keeping an entry of its own, `__hash__` or
                        // `__eq__` among them, is answered off the worth
                        // the thing holds, which is what the base's own
                        // reading of that name gives.
                        if Self::native_beneath(base).is_some() {
                            if let Some(worth) = Self::underlying(&instance) {
                                if let Some(member) = self.attribute(&worth, key) { return Ok(member); }
                            }
                        }
                    }
                    passed |= Rc::ptr_eq(base, defining);
                }
                // The walk coming to the common ancestor, the root
                // answers with its own members as a plain read of a
                // thing does: the maker every blueprint stands on, its
                // beginning among them.
                if let Some(root)=self.from_the_root(key,true,&actual) {
                    if key==self.detail("allocate"){return Ok(root);}
                    let bound=if key==self.detail("subclass"){Value::Blueprint(actual.clone())}else{instance.clone()};
                    return Ok(Self::wrap(3,vec![root,bound]));
                }
                return Err(self.absent_attribute(&value, key));
            }
        }
        let sought=self.seek_class_member(value.clone(),key,direct);
        if direct{return sought;}
        let Err(escape) = &sought else { return sought };
        if !self.missing_member_escape(escape) { return sought; }
        if let Value::Blueprint(kind) = &value {
            if let Some(builder) = Self::builder_over(kind) {
                if let Some(entry) = self.table.single("ext.stmt.class.reader").and_then(|key| self.inherited_entry(&builder, key)) {
                    let bound = self.member_binding(entry, Some(value.clone()), builder)?;
                    return self.apply_class_member(bound, vec![Value::text(key)]).map_err(|escaped| self.explain_absence(escaped, &value, key));
                }
            }
            return sought;
        }
        let Value::Thing(t) = &value else { return sought };
        let Some(fallback)=self.table.single("ext.stmt.class.reader").and_then(|n|self.inherited_entry(&t.blueprint(),n)) else{return sought};
        self.sought_in_vain.take();
        let bound=self.member_binding(fallback,Some(value.clone()),t.blueprint().clone())?;
        self.apply_class_member(bound,vec![Value::text(key)]).map_err(|escaped| self.explain_absence(escaped, &value, key))
    }
    fn seek_class_member(&mut self,value:Value,key:&str,direct:bool)->Res {
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
        if let Value::Wrapped(60, parts) = &value {
            if let [Value::Text(kind), Value::Text(word)] = parts.as_slice() {
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
            let names = self.table.strings("ext.builtin.exceptions.traceback");
            if names.get(1).map_or(false, |n| n == key) { return Ok(Value::Small(link.location as i64)); }
            if names.get(2).map_or(false, |n| n == key) { return Ok(link.following.clone()); }
            if names.get(3).map_or(false, |n| n == key) { return Ok(Value::Thing(link.activation.clone())); }
            if names.get(26).map_or(false, |n| n == key) { return Ok(Value::Small(link.instruction)); }
            for (index, offset) in [(16, link.extent.map_or(Some(link.location), |x| Some(x.2))), (17, link.extent.map(|x| x.1)), (18, link.extent.map(|x| x.3))] {
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
                let mut names = self.kind_stand_in(&word).map_or_else(Vec::new, |sample| self.native_directory(&sample));
                if word.as_ref() == "type" { let slots = self.table.strings("ext.stmt.class.special"); names.extend([8, 17].iter().filter_map(|at| slots.get(*at).cloned())); }
                if word.as_ref() == "dict" { names.extend(self.table.strings("ext.builtin.method.fromkeys").iter().cloned()); }
                let mut pairs = names.into_iter().filter_map(|name| {
                    self.carried_by_kind(&value, &name).map(|descriptor| (Value::text(&name), descriptor))
                }).collect::<Vec<_>>();
                if matches!(word.as_ref(), "int" | "float" | "str" | "tuple" | "bytes" | "bytearray" | "dict" | "set" | "frozenset" | "complex" | "list") {
                    pairs.push((Value::text(self.detail("allocate")), Self::wrap(14, vec![Value::text(&word)])));
                }
                return Ok(Value::Window(Rc::new(Value::Dict(Rc::new(pairs.into()))),'m'));
            }
        }
        // Routines, wrapped routines and slots are members that bind, and
        // read as such; a slot writes and removes besides.
        if self.protocol_spelled() {
            let binds=matches!(&value,Value::Routine(_)|Value::Bound(..))||matches!(&value,Value::Wrapped(4|5|32,_));
            if binds && key==self.detail("descriptor.get"){return Ok(Self::wrap(31,vec![value]));}
            if let Value::Wrapped(32,parts)=&value {
                if key==self.detail("descriptor.set"){return Ok(Self::wrap(33,parts.as_ref().clone()));}
                if key==self.detail("descriptor.delete"){return Ok(Self::wrap(34,parts.as_ref().clone()));}
            }
        }
        if matches!(&value, Value::Intrinsic(Prim::SortOf, _))
            || (self.table.has_any("ext.stmt.class.builder") && (matches!(&value, Value::Blueprint(b) if self.builds_classes(b) || Self::native_word(b).is_some_and(|word| self.table.prims.get(&word) == Some(&Prim::SortOf)))
                || self.kind_spelling(&value).is_some_and(|word| self.table.prims.get(word.as_ref()) == Some(&Prim::SortOf)))) {
            if self.table.single("ext.stmt.class.constructor")==Some(key){return Ok(Self::wrap(73,Vec::new()));}
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
        if matches!(&value,Value::Intrinsic(op,_) if !Self::names_a_kind(op)) {
            if let Some(member)=self.attribute(&value,key) { return Ok(member); }
        }
        if let Value::Intrinsic(op,word)=&value {
            if *op == Prim::AsReal && self.table.strings("ext.builtin.method.from_number").iter().any(|spelling| spelling.rsplit('.').next() == Some(key)) {
                return Ok(Value::Member(Rc::new(value.clone()), String::from("float_from_number")));
            }
            if *op == Prim::ComplexMade && self.table.strings("ext.builtin.method.from_number").iter().any(|spelling| spelling.rsplit('.').next() == Some(key)) {
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
                if key==self.detail("allocate"){return Ok(Self::wrap(14,vec![Value::text(word)]));}
                if key==self.detail("name")||self.table.spells("ext.builtin.class.name",key){return Ok(Value::text(word));}
                // The flags of the kind read as a class: a native kind
                // is never sealed and its values are no collector's,
                // so only the standing bits answer here.
                if key==self.detail("flags") {
                    let kind=self.native_kind(word);
                    let mut bits=512|1024;
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
                if self.is_offspring_word(key){let kind=self.native_kind(word);return Ok(self.offspring_listing(&kind));}
                if key==self.detail("mro")||key==self.detail("order"){
                    let listed=key==self.detail("order");
                    let word=word.to_string();
                    let kind=if *op==Prim::SortOf {self.builder_blueprint()} else {self.native_kind(&word)};
                    let mut ranks=vec![self.visible_blueprint(kind.clone())];ranks.extend(kind.ancestry.iter().cloned().map(|p|self.visible_blueprint(p)));
                    let line=Value::tuple(ranks);
                    return Ok(if listed {Self::wrap(0,vec![line])} else {line});
                }
            }
            // Everything a value of the kind answers to the kind itself
            // carries, standing loose: the value worked upon is the
            // first the entry is handed when it is called.
            if let Some(carried)=self.carried_by_kind(&value,key) { return Ok(carried); }
            let native_class = self.native_kind(&word);
            if let Some(inherited) = self.from_the_root(key, false, &native_class) { return Ok(inherited); }
        }
        if self.table.spells("ext.stmt.class.builtin", "bytes") && (key == "__buffer__" || key == "__release_buffer__") {
            let provider = match &value {
                Value::OctetKind { changeable, .. } | Value::Octets { changeable, .. } => key == "__buffer__" || *changeable,
                Value::Blueprint(class) => Self::native_word(class).or_else(|| Self::native_beneath(class)).as_deref().is_some_and(|word| word == "bytearray" || key == "__buffer__" && word == "bytes"),
                _ => false,
            };
            if provider {
                return Ok(match value {
                    Value::Blueprint(ref class) => {
                        let word = Self::native_word(class).or_else(|| Self::native_beneath(class)).unwrap_or_default();
                        Value::Intrinsic(Prim::ValueMethod, Rc::from(format!("{word}.{key}")))
                    },
                    Value::OctetKind { changeable, .. } => Value::Intrinsic(Prim::ValueMethod, Rc::from(format!("{}.{key}", self.octet_kind_word(changeable)))),
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
            let builtin_origin = Self::native_word(b).is_some()
                || (b.under.is_none() && b.name == self.detail("root"));
            if builtin_origin && !key.is_empty() && key == self.detail("module") {
                return Ok(Value::text(self.builtin_module()));
            }
            if key == self.detail("flags") {
                // The seal takes the base-standing bit off and puts the
                // unchangeable one on.
                let mut bits = if Self::sealed(b) { 512 | 256 } else { 512 | 1024 };
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
            if (Self::native_word(b).as_deref() == Some("float") || Self::native_beneath(b).as_deref() == Some("float")) && self.table.strings("ext.builtin.method.from_number").iter().any(|spelling| spelling.rsplit('.').next() == Some(key)) {
                return Ok(Value::Member(Rc::new(value.clone()), String::from("float_from_number")));
            }
            if (Self::native_word(b).as_deref() == Some("complex") || Self::native_beneath(b).as_deref() == Some("complex")) && self.table.strings("ext.builtin.method.from_number").iter().any(|spelling| spelling.rsplit('.').next() == Some(key)) {
                return Ok(Value::Member(Rc::new(value.clone()), String::from("complex_from_number")));
            }
            if Self::native_beneath(b).as_deref() == Some("float") && key == "__getformat__" {
                return Ok(Value::Member(Rc::new(value.clone()), String::from("float_getformat")));
            }
            if Self::native_beneath(b).as_deref() == Some("float") && key == "fromhex" {
                return Ok(Value::Member(Rc::new(value.clone()), String::from("float_fromhex")));
            }
            if key == self.detail("module") && self.table.has_any("ext.stmt.class.detail.module") {
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
                    return Ok(Self::wrap(14, vec![Value::text(&word)]));
                }
                if key==self.detail("module") { return Ok(Value::text(if matches!(word.as_str(), "SimpleNamespace" | "GenericAlias") { "types" } else if word == "Union" { "typing" } else { self.builtin_module() })); }
                if key==self.detail("qualified") { return Ok(Value::text(&word)); }
                if key=="__getformat__" && word=="float" {
                    return Ok(Value::Member(Rc::new(value.clone()), "float_getformat".to_owned()));
                }
            }
            if key==self.detail("name"){let names=b.type_names.borrow();return Ok(names.as_ref().map_or_else(|| Value::text(&b.name), |names| names.short.clone()));}
            if key==self.detail("qualified"){return Ok(b.type_names.borrow().as_ref().map(|names| names.full.clone()).unwrap_or_else(|| self.inherited_entry(b,key).unwrap_or_else(||Value::text(&b.name))));}
            if key==self.detail("namespace"){
                let title = self.table.strings("ext.stmt.class.detail.code.fields").get(10).cloned().unwrap_or_default();
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
            if self.is_offspring_word(key){return Ok(self.offspring_listing(b));}
            if key==self.detail("mro")||key==self.detail("order"){
                let mut all=Vec::new();all.push(value.clone());all.extend(b.ancestry.iter().map(|p|self.visible_blueprint(p.clone())));
                let result=Value::tuple(all);return Ok(if key==self.detail("order"){Self::wrap(0,vec![result])}else{result});
            }
            if self.table.strings("ext.stmt.class.detail.code.fields").get(10).is_some_and(|word| word == key) {
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
            if self.table.single("ext.stmt.class.annotations")==Some(key){return self.blueprint_annotations(b);}
            if let Some(found)=self.inherited_entry(b,key){return self.member_binding(found,None,b.clone());}
            if let Some(size) = self.integer_attribute(&value, key) { return Ok(size); }
            // A blueprint standing on a native kind reads that kind's
            // own class method too, bound to the blueprint, so that
            // `dictlike.fromkeys` reaches `dict.fromkeys` and hands
            // back a dictlike.
            if self.table.spells("ext.builtin.method.fromkeys", key) {
                if let Some(base) = std::iter::once(b).chain(b.ancestry.iter()).find(|base| Self::native_word(base).is_some()) {
                    if self.carried_by_kind(&Value::Blueprint(base.clone()), key).is_some() {
                        return Ok(Value::Member(Rc::new(value.clone()), key.to_owned()));
                    }
                }
            }
            if let Some(entry)=self.carried_by_kind(&value,key){return Ok(entry);}
            if Self::native_word(b).is_none() && !self.table.spells("ext.builtin.method.fromkeys", key) {
                for ancestor in &b.ancestry {
                    if Self::native_word(ancestor).is_some() {
                        if let Some(method) = self.carried_by_kind(&Value::Blueprint(ancestor.clone()), key) { return Ok(method); }
                        break;
                    }
                }
            }
            if key == self.detail("allocate") {
                let mut cache = self.loose_members.borrow_mut();
                return Ok(cache.entry("root:allocation".into()).or_insert_with(|| Self::wrap(1, Vec::new())).clone());
            }
            // A class reads what the metaclass that built it holds as
            // well, each entry bound to the class itself, as a thing's
            // method is bound to the thing.
            if let Some(builder)=Self::builder_over(b) {
                if let Some(found)=self.inherited_entry(&builder,key){return self.member_binding(found,Some(value.clone()),builder);}
            }
            // The formatting every blueprint has from the root: a thing
            // and a specification, answered as the format builtin would.
            if self.table.strings("ext.stmt.class.special").get(72).map_or(false,|word|word==key){return Ok(Self::wrap(59,Vec::new()));}
            {
                let tag=if key==self.detail("allocate")&&(self.builds_classes(b)||b.ancestry.iter().any(|p|self.builds_classes(p))){70}else if key==self.detail("allocate"){1}else if self.table.single("ext.stmt.class.constructor")==Some(key)&&(self.builds_classes(b)||b.ancestry.iter().any(|p|self.builds_classes(p))){73}else if self.table.single("ext.stmt.class.constructor")==Some(key)||key==self.detail("subclass"){2}
                    else if key==self.detail("get"){10}else if key==self.detail("set"){11}else if key==self.detail("remove"){12}else{255};
                if tag!=255{return Ok(Self::wrap(tag, if key == self.detail("subclass") { vec![Value::text(key)] } else { Vec::new() }));}
            }
            if let Some(root)=self.from_the_root(key,false,b){return Ok(root);}
        }else if let Value::Thing(t)=&value {
            if self.namespace_holding(&value).is_some() || Self::native_beneath(&t.blueprint()).as_deref() == Some("module") {
                let title = self.table.strings("ext.stmt.class.detail.code.fields").get(10).cloned().unwrap_or_default();
                let dictionary = t.holds.borrow().iter().find(|entry| entry.0 == "\0dictionary").map(|entry| entry.1.clone());
                let lookup = |wanted: &str| {
                    dictionary.as_ref().and_then(|value| if let Value::Dict(entries) = value.settled() {
                        entries.iter().find(|entry| entry.0.bare() == wanted).map(|entry| entry.1.settled())
                    } else { None }).or_else(|| t.holds.borrow().iter().find(|entry| entry.0 == wanted).map(|entry| entry.1.settled()))
                };
                let own = lookup(key);
                if key == title { return Ok(own.unwrap_or(Value::Nil)); }
                if self.table.single("ext.stmt.class.annotations") == Some(key) {
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
            if key == self.detail("kind") {
                if self.namespace_holding(&value).is_some() { return Ok(self.kind_named_after(&value)); }
                match self.inherited_entry(&t.blueprint(), key) {
                    Some(entry) => return self.member_binding(entry, Some(value.clone()), t.blueprint().clone()),
                    None => return Ok(Value::Blueprint(t.blueprint().clone())),
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
                if !self.allowed_slot(&t.blueprint(),key){return Err(self.absent_attribute(&value,key));}
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
            if self.table.strings("ext.stmt.class.detail.root.members").get(9).is_some_and(|word| word == key) {
                let root = Self::wrap(36, vec![Value::text(key)]);
                return Ok(Self::wrap(3, vec![root, value.clone()]));
            }
            // The worth a thing keeps answers for the methods of its kind.
            let native=Self::underlying(&value);
            // A mapping subclass's subscript member stays bound to the
            // thing itself, so a key the mapping does not hold reaches
            // its `__missing__` the way a plain subscript does.
            if self.table.strings("ext.stmt.class.special").get(11).map_or(false, |word| word == key)
                && native.as_ref().map_or(false, |under| matches!(under.settled(), Value::Dict(_))) {
                return Ok(Value::Member(Rc::new(value.clone()), key.to_owned()));
            }
            if let Some(set)=native.as_ref().filter(|v|matches!(v.settled(),Value::Set(_))) {
                let protocol = self.table.strings("ext.stmt.class.special").iter().position(|word| word == key);
                if protocol == Some(79) || protocol == Some(81) {
                    return Ok(Value::Member(Rc::new(value.clone()), key.to_owned()));
                }
                if let Some(member)=self.attribute(&set.settled(),key) { return Ok(member); }
            }
            if let Some(text @ Value::Text(_)) = &native {
                if let Some(member) = self.attribute(text, key) { return Ok(member); }
            }
            if let Some(size) = self.integer_attribute(&value, key) { return Ok(size); }
            if let Some(root)=self.from_the_root(key,true,&t.blueprint()) {
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
                let named_slot = self.table.strings("ext.stmt.class.special").iter().position(|word| word == key);
                let container_member = named_slot.map_or(false, |index| index >= 10 && index <= 14);
                if self.native_member(&under,key) && (container_member || matches!(under.settled(), Value::Complex(_))) {
                    if named_slot == Some(11) && matches!(under.settled(), Value::Dict(_)) {
                        return Ok(Self::wrap(3, vec![Self::wrap(60, vec![Value::text("dict"), Value::text(key)]), value.clone()]));
                    }
                    return Ok(Value::Member(Rc::new(under),key.to_owned()));
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
                    return Ok(Value::Member(Rc::new(under),operation));
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
                    let callable = if self.table.has_any("ext.stmt.class.builder") {
                        Value::Intrinsic(operation, Rc::from(key))
                    } else { Value::text(key) };
                    return Ok(Self::wrap(3,vec![callable,under]));
                }
            }
            if self.namespace_holding(&value).is_some() {
                let handler = t.holds.borrow().iter().find_map(|(word, held)| if word == "__getattr__" { Some(held.settled()) } else { None });
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
            let annotations = self.table.strings("ext.stmt.class.annotations");
            if annotations.first().map_or(false, |word| word == key) {
                let label = self.table.strings("ext.stmt.class.detail.code.fields").get(10).cloned().unwrap_or_default();
                let written = self.routine_holding(&value, &label);
                let source = written.or_else(|| code.annotator.clone().map(|a| Value::Bound(a, annotation_room.clone())));
                let contents = if let Some(evaluator) = source.filter(|a| !matches!(a.settled(), Value::Nil)) {
                    self.apply_class_member(evaluator, vec![Value::Small(1)])?
                } else { self.collection_cell(Value::Dict(Rc::new(Vec::new().into()))) };
                let index = self.routine_storage(&value);
                self.routine_members[index].1.holds.borrow_mut().push((format!("\0{key}\0"), contents.clone()));
                return Ok(contents);
            }
            let fields = self.table.strings("ext.stmt.class.detail.code.fields");
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
                if defaults.is_empty() && !code.local_defaults.is_empty(){return Err(self.class_unready());}
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
                let carried=key==self.detail("module")||key==self.detail("qualified")||key==self.detail("name")||key==self.detail("doc")
                    || self.table.strings("ext.stmt.class.annotations").first().map_or(false,|word|word==key)
                    || self.table.strings("ext.stmt.class.detail.code.fields").get(10).is_some_and(|word|word==key);
                if carried { return self.read_class_member(items[0].clone(),key,true); }
            }
            // A method bound to its thing answers for the thing and the
            // function by the table's words, and for anything else as
            // the function itself would: a method of a class formed in
            // a function is bound this way, and its name, its full name
            // and what it says of itself are the function's.
            if matches!(*tag, 3 | 132) {
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
                    let fields = self.table.strings("ext.stmt.class.detail.code.fields");
                    if let Some(index) = fields.iter().position(|word| word == key) {
                        let result = match index {
                            0 => Value::text(if p.ident == "<generator>" { "<genexpr>" } else if p.ident == "<program>" { self.table.strings("ext.builtin.exceptions.traceback").get(10).map_or("<module>", String::as_str) } else { &p.ident }),
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
                    let words = self.table.strings("ext.builtin.exceptions.traceback");
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
        if let (false, Value::Thing(t), Some(word)) = (self.asking_presence, &value, self.table.single("ext.system.module.getattr")) {
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
    /// Two blueprints that declare the very same slots, and stand on
    /// nothing else that declares any, lay their things out alike.
    fn alike_in_slots(&self, one: &Blueprint, other: &Blueprint) -> bool {
        let names = |b: &Blueprint| Self::own_entry(b, self.detail("slots")).map(|held| match held.settled() {
            Value::Tuple(items) | Value::Vector(items) => items.iter().map(|item| item.bare()).collect::<Vec<_>>(),
            single => vec![single.bare()],
        });
        let bare = |b: &Blueprint| b.parents.iter().filter(|p| p.name != self.detail("root")).all(|p| !self.slots_named(p));
        match (names(one), names(other)) {
            (Some(a), Some(b)) => a == b && bare(one) && bare(other),
            _ => false,
        }
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
    fn allowed_slot(&self,b:&Blueprint,key:&str)->bool {
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
        if let Value::Thing(object) = &subject {
            if let Some(kind) = Self::native_word(&object.blueprint()).filter(|kind| matches!(kind.as_str(), "TypeVar" | "ParamSpec" | "TypeVarTuple" | "TypeAliasType" | "NoDefaultType")) {
                if key == "__name__" || kind == "ParamSpec" && key == "__bound__" { return Err(String::from("AttributeError: readonly attribute").into()); }
                let fixed = key == "__default__" || kind != "TypeVarTuple" && ["__bound__", "__constraints__", "__covariant__", "__contravariant__", "__infer_variance__"].contains(&key);
                if fixed || kind == "TypeAliasType" || kind == "NoDefaultType" { return Err(format!("AttributeError: attribute '{key}' of 'typing.{kind}' objects is not writable").into()); }
                if !Self::change_entry(&mut object.holds.borrow_mut(), key, replacement) { return Err(self.absent_attribute(&subject, key)); }
                return Ok(Value::Nil);
            }
        }
        if self.table.has_any("ext.builtin.weak.get") {
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

        if self.table.has_any("ext.stmt.class.detail.name") && !matches!(&subject, Value::Blueprint(_)) {
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
            if self.table.single("ext.stmt.yield.running") == Some(key) {
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
        if self.table.single("ext.builtin.exceptions.traceback.member") == Some(key) {
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
                if self.table.single("ext.builtin.exceptions.suppress") == Some(key) {
                    if replacement.is_none() { return Err(String::from("TypeError: can't delete numeric/char attribute").into()); }
                    if !matches!(replacement.as_ref().map(Value::settled), Some(Value::Flag(_))) {
                        return Err(String::from("TypeError: attribute value type must be bool").into());
                    }
                }
                if replacement.is_none() {
                    if self.table.single("ext.builtin.exceptions.args") == Some(key) { return Err(format!("TypeError: {key} may not be deleted").into()); }
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
                    let evaluator = self.table.strings("ext.stmt.class.detail.code.fields").get(10).cloned().unwrap_or_default();
                    if key == evaluator {
                        match replacement.as_ref() {
                            None => return Err(String::from("TypeError: cannot delete __annotate__ attribute").into()),
                            Some(value) if !matches!(value.settled(), Value::Nil) => {
                                if !self.work_on_class(2, vec![value.clone()])?.is_true() { return Err(String::from("TypeError: __annotate__ must be callable or None").into()); }
                                if let Some(annotation) = self.table.single("ext.stmt.class.annotations") {
                                    t.holds.borrow_mut().retain(|entry| entry.0 != annotation);
                                    let book = t.holds.borrow().iter().find(|entry| entry.0 == "\0dictionary").map(|entry| entry.1.clone());
                                    if let Some(book) = book { Self::write_into_book(&book, annotation, None); }
                                }
                            }
                            _ => {},
                        }
                    } else if self.table.single("ext.stmt.class.annotations") == Some(key) {
                        let book = t.holds.borrow().iter().find(|entry| entry.0 == "\0dictionary").map(|entry| entry.1.clone());
                        if let Some(book) = book { Self::write_into_book(&book, &evaluator, Some(Value::Nil)); }
                        else { Self::change_entry(&mut t.holds.borrow_mut(), &evaluator, Some(Value::Nil)); }
                    }
                }


                if self.is_fault_kind(&t.blueprint()) && self.table.single("ext.builtin.exceptions.args") == Some(key) {
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
                            if place!="\0doc"{return Err(self.detail("property.readonly").to_owned().into());}
                            let mut holds=t.holds.borrow_mut();
                            Self::change_entry(&mut holds,"\0doc",replacement);
                            return Ok(Value::Nil);
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
                if key == self.detail("kind") {
                    match replacement {
                        Some(Value::Blueprint(next)) => {
                            let previous = t.blueprint();
                            let alike = self.alike_in_slots(&previous, &next);
                            let compatible = Self::own_entry(&previous, self.detail("module")).is_some()
                                && Self::own_entry(&next, self.detail("module")).is_some()
                                && Self::native_beneath(&previous) == Self::native_beneath(&next)
                                && (alike || (!self.slots_named(&previous) && !self.slots_named(&next)));
                            if !compatible { return Err("TypeError: __class__ assignment: object layout differs".to_owned().into()); }
                            // The worth a slot keeps stands under the key of
                            // the blueprint that declared it: move it over.
                            if alike {
                                let (was, now) = (format!(":{:p}", Rc::as_ptr(&previous)), format!(":{:p}", Rc::as_ptr(&next)));
                                for entry in t.holds.borrow_mut().iter_mut() {
                                    if entry.0.starts_with("\0slot:") && entry.0.ends_with(&was) { let cut = entry.0.len() - was.len(); entry.0.truncate(cut); entry.0.push_str(&now); }
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
                    let link = t.holds.borrow().iter().find(|(k, _)| k == key).and_then(|(_, held)| match held { Value::Shared(link) => Some(link.clone()), _ => None });
                    if let (Some(link), Some(v)) = (link.as_ref(), replacement.clone()) { *link.borrow_mut() = self.collection_cell(v); return Ok(Value::Nil); }
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
                            *cell.borrow_mut() = self.collection_cell(v);
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
                let annotation_function = self.table.strings("ext.stmt.class.detail.code.fields").get(10).cloned().unwrap_or_default();
                if !annotation_function.is_empty() && key == annotation_function {
                    match replacement.as_ref() {
                        None => return Err(String::from("TypeError: cannot delete __annotate__ attribute").into()),
                        Some(value) if !matches!(value.settled(), Value::Nil) => {
                            if !self.work_on_class(2, vec![value.clone()])?.is_true() { return Err(String::from("TypeError: __annotate__ must be callable or None").into()); }
                            if let Some(annotation) = self.table.single("ext.stmt.class.annotations") { b.shared.borrow_mut().retain(|entry| entry.0 != annotation); }
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
                if self.table.single("ext.stmt.class.annotations") == Some(key) {
                    b.shared.borrow_mut().retain(|entry| entry.0 != "__annotate_func__");
                    let evaluator = self.table.strings("ext.stmt.class.detail.code.fields").get(10).cloned().unwrap_or_default();
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
                let evaluator_key = self.table.strings("ext.stmt.class.detail.code.fields").get(10).cloned().unwrap_or_default();
                if !evaluator_key.is_empty() && key == evaluator_key {
                    let incoming = match replacement { Some(v) => v, None => return Err("TypeError: __annotate__ cannot be deleted".to_owned().into()) };
                    if !matches!(incoming.settled(), Value::Nil) && !self.work_on_class(2, vec![incoming.clone()])?.is_true() {
                        return Err("TypeError: __annotate__ must be callable or None".to_owned().into());
                    }
                    let index = self.routine_storage(&subject);
                    let mut holds = self.routine_members[index].1.holds.borrow_mut();
                    if !matches!(incoming.settled(), Value::Nil) {
                        if let Some(annotation) = self.table.strings("ext.stmt.class.annotations").first() { holds.retain(|(word, _)| word != &format!("\0{annotation}\0")); }
                    }
                    Self::change_entry(&mut holds, &format!("\0{key}\0"), Some(incoming));
                    return Ok(Value::Nil);
                }
                if self.table.strings("ext.stmt.class.annotations").first().map_or(false, |s| s == key) {
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
        if self.table.has_any("ext.stmt.class.builder") && self.spells_property_kind(&settled) { return Ok(self.property_blueprint()); }
        if self.table.has_any("ext.builtin.bool.base") && matches!(settled, Value::Intrinsic(Prim::Truthful, _)) { return Err(self.table.single("ext.builtin.bool.base").unwrap_or("").to_owned().into()); }
        if let Value::Blueprint(class) = settled {
            if Self::sealed(&class) {
                let qualified = match class.name.as_str() {
                    "ProxyType" | "CallableProxyType" if self.table.has_any("ext.builtin.weak.get") => "weakref.".to_owned() + &class.name,
                    _ => class.name.clone(),
                };
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
            if operation == Prim::Truthful {return Err(self.table.single("ext.builtin.bool.base").unwrap_or("TypeError: bases must be types").to_owned().into());}
            if operation != Prim::Truthful && (self.table.spells("ext.stmt.class.builtin", &word) || (self.table.has_any("ext.stmt.class.builder") && matches!(operation, Prim::ClassWork(9..=11)))) { return Ok(self.native_kind(&word)); }
        }
        if self.table.has_any("ext.stmt.class.builder") {
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
        let Some(key)=self.table.strings("ext.stmt.class.special").get(if class_only{77}else{76}).cloned() else{return Ok(None);};
        let Some(entry)=self.inherited_entry(&builder,&key) else{return Ok(None);};
        let bound=self.member_binding(entry,Some(choice.clone()),builder)?;
        let told=self.apply_class_member(bound,vec![given.clone()])?;
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
            ||matches!(value,Value::Thing(t) if matches!(Self::native_word(&t.blueprint()).as_deref(), Some("Union"|"GenericAlias")))
    }
    /// A kind or union supplies the operator; Nil by itself does not.
    pub(super) fn union_anchor(&self,value:&Value)->bool{
        self.stands_for_a_kind(value)
            ||matches!(value,Value::Thing(t) if matches!(Self::native_word(&t.blueprint()).as_deref(), Some("Union"|"GenericAlias")))
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
        if let Value::Thing(thing) = value {
            if Self::native_beneath(&thing.blueprint()).as_deref() == Some("module") { return Value::Blueprint(thing.blueprint()); }
        }
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
    fn is_beneath(&mut self,subject:&Value,choice:&Value,class_only:bool)->Result<bool,Escape>{
        if let Value::Mutable(cell, _) | Value::Shared(cell) = subject { let held = cell.borrow().clone(); return self.is_beneath(&held, choice, class_only); }
        if let Value::Mutable(cell, _) | Value::Shared(cell) = choice { let held = cell.borrow().clone(); return self.is_beneath(subject, &held, class_only); }
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
        if let Value::Tuple(options)|Value::Vector(options)=choice {
            for option in options.iter() { if self.is_beneath(subject,option,class_only)? { return Ok(true); } }
            return Ok(false);
        }
        // The byte kinds are values in their own right rather than
        // intrinsic words, so each is asked after under its own word.
        if let Value::OctetKind { changeable, .. } = subject { let word=self.octet_kind_word(*changeable).to_owned(); return self.is_beneath(&Value::Wrapped(8, Rc::new(vec![Value::text(&word)]).into()), choice, class_only); }
        if let Value::OctetKind { changeable, .. } = choice { let word=self.octet_kind_word(*changeable).to_owned(); return self.is_beneath(subject, &Value::Wrapped(8, Rc::new(vec![Value::text(&word)]).into()), class_only); }
        if let Value::Intrinsic(_, word) = subject { return self.is_beneath(&Value::Wrapped(8, Rc::new(vec![Value::text(word)]).into()), choice, class_only); }
        if let Value::Intrinsic(_, word) = choice { return self.is_beneath(subject, &Value::Wrapped(8, Rc::new(vec![Value::text(word)]).into()), class_only); }
        // The kind asked after must be a class wherever it is asked, and
        // each of the two has its own words, as the reference has.
        let amiss=if class_only{"core.issubclass.amiss"}else{"core.isinstance.amiss"};
        match choice {
            // A union built by `|` carries a bare `Nil` for the
            // `NoneType` member, the very value `None` itself is, so
            // a chained union reads it back this way rather than
            // needing `type(None)`.
            Value::Nil=>Ok(if class_only{matches!(subject,Value::KindOf(Kind::Nothing))}else{matches!(subject,Value::Nil)}),
            Value::Blueprint(c)=>{
                if !class_only && self.namespace_has_kind(subject, c) { return Ok(true); }
                // What is asked about must be a class wherever a class
                // is what is asked about.
                if class_only && !self.counts_as_class(subject){return Err(self.not_a_class("core.issubclass.subject"));}
                // Everything lies under the class everything lies under.
                if Rc::ptr_eq(c,&self.common_ancestor()){return Ok(true);}
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
                Ok(b.map_or(false,|b|Self::fault_descends(b,c)||Self::ancestry_includes(b,c)))
            }
            Value::Wrapped(8,names)=>{
                let Value::Text(word)=&names[0] else{return Err(self.not_a_class(amiss));};
                let Some(op)=self.table.prims.get(word.as_ref()).copied().filter(Self::names_a_kind) else{return Err(self.not_a_class(amiss));};
                if class_only{
                    if let Value::Blueprint(b)=subject{
                        let inherited=if op==Prim::SortOf { std::iter::once(b).chain(b.ancestry.iter()).any(|base|self.builds_classes(base)) }
                            else { Self::native_among(b,word) };
                        return Ok(inherited);
                    }
                    let Some(under)=self.kind_spelling(subject) else{return Err(self.not_a_class("core.issubclass.subject"));};
                    return Ok(self.kind_under(&under,word));
                }
                // A thing of a blueprint standing on the kind is of
                // the kind, and one standing on several is of each.
                if let Value::Thing(t)=subject{return Ok(Self::native_among(&t.blueprint(),word));}
                Ok(self.kind_covers(&op,word,subject))
            },
            _=>{
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
        if op == 22 && values.len() == 1 { return Ok(values.remove(0)); }

        if op == 13 { return self.class_from_function(values); }
        if op == 14 { return self.check_class_builtin(); }
        if op == 15 { return self.prepared_class_book(values.remove(0)); }
        if op == 16 { return self.dispatch_class_builder(values); }
        if op == 18 { return self.class_namespace_read(values); }
        if op == 22 {
            let (values, named) = self.open_arguments(values)?;
            if !named.is_empty() { return Err(String::from("TypeError: sys._clear_type_descriptors() takes no keyword arguments").into()); }
            if values.len() != 1 { return Err(format!("TypeError: sys._clear_type_descriptors() takes exactly one argument ({} given)", values.len()).into()); }
            let target = values[0].settled();
            let class = if let Value::Blueprint(class) = target { class } else {
                if self.stands_for_a_kind(&target) { return Err(String::from("TypeError: argument is immutable").into()); }
                return Err(format!("TypeError: _clear_type_descriptors() argument must be type, not {}", target.kind_word()).into());
            };
            if class.type_names.borrow().is_none() || Self::sealed(&class) { return Err(String::from("TypeError: argument is immutable").into()); }
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
        if op==2 && values.len()==1{return Ok(Value::Flag(matches!(&values[0],Value::Routine(_)|Value::Bound(..)|Value::Method(..)|Value::Blueprint(_)|Value::Intrinsic(..)|Value::OctetKind {..}|Value::Member(..))||matches!(&values[0],Value::Wrapped(tag,_) if matches!(tag,0..=4|8..=12|31|33|34|36|44..=48|50..=57|59|60|70..=74|77..=79|132|133|134))||matches!(&values[0],Value::Thing(t) if self.inherited_entry(&t.blueprint(),self.detail("call")).is_some())));}
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
                    if let Some(word)=self.table.strings("ext.stmt.class.special").get(75).cloned() {
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
                let class_directory=self.table.strings("ext.stmt.class.special").get(75)
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
                    if let Some(word)=self.table.strings("ext.stmt.class.special").get(75) {
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
            return Ok(self.ordinary_directory(&values[0]));
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
        let mut chain=std::iter::once(class.clone()).chain(class.ancestry.iter().cloned()).collect::<Vec<_>>();
        let mut start=chain.iter().position(|b|named_here(self,b));
        // A method of a metaclass is written in the metaclass, so the
        // forebears it reaches past are the metaclass's own, not those
        // of the class it was handed.
        if start.is_none() {
            if let Some(builder)=Self::builder_over(&class) {
                chain=std::iter::once(builder.clone()).chain(builder.ancestry.iter().cloned()).collect();
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
                if self.table.single("ext.stmt.class.constructor")==Some(key) {
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
                if word == "module" && self.table.strings("ext.stmt.class.special").get(1).is_some_and(|name| name == key) { return self.describe_module(receiver); }
                if self.table.single("ext.stmt.class.constructor")==Some(key){
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
            if self.table.single("ext.stmt.class.constructor") == Some(key) && self.is_fault_kind(b) {
                if let Value::Thing(t) = &receiver {
                    let (mut plain, names) = self.open_arguments(args)?;
                    plain.extend(names.into_iter().map(|(word, item)| Value::Couple(Rc::new((Value::text(&word), item)))));
                    return self.fault_method(t.clone(), key, &plain);
                }
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
                        else if matches!(parameter.settled(), Value::Blueprint(_)) {
                            let namespace = self.read_class_member(parameter.clone(), "__module__", false)?.bare();
                            let qualified = self.read_class_member(parameter.clone(), "__qualname__", false)?.bare();
                            if namespace == "builtins" { qualified } else { namespace + "." + &qualified }
                        }
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
            6 | 7 if rest.len() == 1 => {
                let other = rest.remove(0).settled();
                if !self.union_member(&other) { return Ok(Value::Refusal(Rc::from("NotImplemented"))); }
                Ok(if action == 6 { self.combined_types(&[subject, other]) } else { self.combined_types(&[other, subject]) })
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
