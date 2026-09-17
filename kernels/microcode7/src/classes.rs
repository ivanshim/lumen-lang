// A class keeps its direct forebears and the order in which their own
// members are sought. Calls and member wrappers use that order alike.
use super::*;

impl<'a> Machine<'a> {
    pub(super) fn detail(&self,key:&str)->&str {self.table.single(&format!("ext.stmt.class.detail.{key}")).unwrap_or("")}
    pub(super) fn has_class_order(&self)->bool {!self.detail("root").is_empty()}
    fn class_unready(&self)->Escape {self.detail("unready").to_owned().into()}
    pub(super) fn common_ancestor(&mut self)->Rc<Blueprint> {
        if self.ancestor.is_none() {
            let title=self.detail("root").to_owned();
            self.ancestor=Some(Rc::new(Blueprint {presentation:Some(format!("<class '{title}'>")),name:title,
                parents:Vec::new(),ancestry:Vec::new(),under:None,answers:Vec::new(),fields:Vec::new(),
                reaches:Vec::new(),methods:Vec::new(),constants:Vec::new(),shared:RefCell::new(Vec::new())}));
        }
        self.ancestor.as_ref().unwrap().clone()
    }
    /// The blueprint standing for a native kind, made once for each word
    /// asked for. A thing of a blueprint beneath it keeps a worth of that
    /// kind among what it holds, under a name no program can write.
    pub(super) fn native_kind(&mut self,word:&str)->Rc<Blueprint> {
        if let Some((_,kind))=self.native_kinds.iter().find(|(w,_)|w==word){return kind.clone();}
        let root=self.common_ancestor();
        let kind=Rc::new(Blueprint {presentation:Some(format!("<class '{word}'>")),name:word.to_owned(),
            parents:vec![root.clone()],ancestry:vec![root.clone()],under:Some(root),answers:Vec::new(),fields:Vec::new(),
            reaches:Vec::new(),methods:Vec::new(),constants:vec![("\0native".to_owned(),Value::text(word))],shared:RefCell::new(Vec::new())});
        self.native_kinds.push((word.to_owned(),kind.clone()));
        kind
    }
    fn native_word(b:&Blueprint)->Option<String> {b.constants.iter().find(|(k,_)|k=="\0native").map(|(_,v)|v.bare())}
    /// The value whose kind a directory should describe: an empty value
    /// of the kind a native kind word names, or the value itself where
    /// it is one of a native kind. Nothing for a blueprint of a class's
    /// own or a thing of one, which list their own members instead.
    fn directory_stand_in(&self,value:&Value)->Option<Value> {
        if let Value::Intrinsic(word)=value { return self.kind_stand_in(word); }
        if let Value::OctetKind{changeable,..}=value { let word=self.octet_kind_word(*changeable).to_owned(); return self.kind_stand_in(&word); }
        if let Value::Blueprint(b)=value { return Self::native_word(b).and_then(|word|self.kind_stand_in(&word)); }
        if matches!(value,Value::Thing(_)) { return None; }
        if self.native_directory(value).is_empty() { return None; }
        Some(value.clone())
    }
    pub(super) fn native_beneath(b:&Blueprint)->Option<String> {
        std::iter::once(b).chain(b.ancestry.iter().map(Rc::as_ref)).find_map(Self::native_word)
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
            reaches:Vec::new(),methods:Vec::new(),constants:Vec::new(),shared:RefCell::new(Vec::new())});
        self.builder_kind=Some(kind.clone());
        kind
    }
    fn builds_classes(&self,b:&Rc<Blueprint>)->bool {self.builder_kind.as_ref().map_or(false,|k|Rc::ptr_eq(k,b))}
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
    fn builder_for(&mut self,asked:Option<Value>,parents:&[Rc<Blueprint>])->Result<Option<Rc<Blueprint>>,Escape> {
        let named=match asked.map(|v|v.settled()) {
            None=>None,
            // The kind primitive names the plainest builder there is,
            // which is no metaclass of its own; so does its blueprint.
            Some(Value::Intrinsic(word)) if self.table.prims.get(word.as_ref())==Some(&Prim::SortOf)=>None,
            Some(Value::Blueprint(b)) if self.builds_classes(&b)=>None,
            Some(Value::Blueprint(b))=>Some(b),
            Some(other)=>return Err(self.core_complaint("core.uncallable",&other.kind_word()).into()),
        };
        let handed_down=parents.iter().find_map(|p|Self::builder_over(p));
        Ok(match (named,handed_down) {
            (Some(a),Some(b))=>Some(if b.ancestry.iter().any(|c|Rc::ptr_eq(c,&a)){b}else{a}),
            (a,b)=>a.or(b),
        })
    }
    /// The native worth a thing keeps, as it is kept: a row or a map
    /// stays in its cell, so that what is done to it through the thing
    /// is done to what the thing holds.
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
    fn thing_over_native(&mut self,class:Rc<Blueprint>,word:&str,given:Vec<Value>)->Res {
        let Some(op)=self.table.prims.get(word).copied()else{return Err(self.class_unready());};
        let (positional,named)=self.open_arguments(given)?;
        let made=if named.is_empty(){self.prim(op,word,&positional)?}else{self.core_primitive(op,word,positional,named)?};
        let kept=match made.settled(){
            held @ (Value::Vector(_)|Value::Dict(_))=>Value::Mutable(Rc::new(RefCell::new(held)),true),
            other=>other,
        };
        self.made+=1;
        Ok(Value::Thing(Rc::new(Thing{of:class,holds:RefCell::new(vec![("\0underlying".to_owned(),kept)]),turn:self.made})))
    }
    pub(super) fn build_class_value(&mut self,title:String,mut parents:Vec<Rc<Blueprint>>,entries:Vec<(String,Value)>)->Res {
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
                let listed=Value::Tuple(Rc::new(parents.iter().cloned().map(Value::Blueprint).collect()));
                let pairs:Vec<(Value,Value)>=entries.iter().filter(|(_,v)|!matches!(v,Value::Unset))
                    .map(|(k,v)|(Value::text(k),v.clone())).collect();
                let namespace=Value::Mutable(Rc::new(RefCell::new(Value::Dict(Rc::new(pairs)))),true);
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
        self.assemble_class(title,parents,entries,builder,handed)
    }
    /// A class laid out from what the kind primitive is given: the
    /// metaclass to remember, the name, the parents and the namespace,
    /// any further keyword being kept for a forebear's subclass hook.
    pub(super) fn class_of_parts(&mut self,values:Vec<Value>)->Res {
        let (plain,named)=self.open_arguments(values)?;
        if plain.len()<4 {return Err(self.class_unready());}
        let by=match &plain[0]{Value::Blueprint(m) if !self.builds_classes(m)=>Some(m.clone()),_=>None};
        let name=plain[1].bare();
        let mut ancestors=Vec::new();
        let (Value::Tuple(listed)|Value::Vector(listed))=plain[2].settled() else{return Err(self.class_unready())};
        for p in listed.iter(){let Value::Blueprint(a)=p else{return Err(self.class_unready())};ancestors.push(a.clone());}
        if ancestors.is_empty(){ancestors.push(self.common_ancestor());}
        let Value::Dict(pairs)=plain[3].settled() else{return Err(self.class_unready())};
        let body=pairs.iter().map(|(k,v)|(k.bare(),v.clone())).collect();
        let keywords=named.into_iter().map(|(k,v)|Value::Couple(Rc::new((Value::text(&k),v)))).collect();
        self.assemble_class(name,ancestors,body,by,keywords)
    }
    /// The class itself, laid out from its name, its parents, its
    /// entries and the metaclass it is to remember. This is the building
    /// the kind primitive does, which a metaclass reaches through its
    /// forebears once it has made a namespace of its own.
    pub(super) fn assemble_class(&mut self,title:String,parents:Vec<Rc<Blueprint>>,mut entries:Vec<(String,Value)>,
        builder:Option<Rc<Blueprint>>,handed:Vec<Value>)->Res {
        // A place only an arm of a conditional writes to may stay
        // unwritten. Nothing stands in it, and the class is given no
        // entry for it: a name a conditional never bound is no member.
        entries.retain(|(_,v)|!matches!(v,Value::Unset));
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
        let module=self.detail("main");
        if entries.iter().all(|(k,_)|k!=self.detail("module")){entries.push((self.detail("module").into(),Value::text(module)));}
        let shown=entries.iter().find(|(k,_)|k==self.detail("qualified")).map_or(title.clone(),|(_,v)|v.bare());
        let class=Rc::new(Blueprint {presentation:Some(format!("<class '{module}.{shown}'>")),name:title,
            under:parents.first().cloned(),parents,ancestry:ranks,answers:vec![],fields:vec![],reaches:vec![],
            methods:vec![],constants:builder.map(|m|vec![("\0metaclass".to_owned(),Value::Blueprint(m))]).unwrap_or_default(),
            shared:RefCell::new(entries)});
        self.name_slots(&class)?;
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
        else if !handed.is_empty(){return Err(self.class_unready());}
        Ok(Value::Blueprint(class))
    }
    fn protocol_spelled(&self)->bool {!self.detail("descriptor.get").is_empty()}
    /// A blueprint's slots become entries of it: each a descriptor
    /// keeping the slot's worth in the thing under the slot's name and
    /// the blueprint that declared it, so a descendant declaring the
    /// same slot keeps its own.
    fn name_slots(&mut self,class:&Rc<Blueprint>)->Result<(),Escape> {
        if !self.protocol_spelled(){return Ok(());}
        let Some(declared)=Self::own_entry(class,self.detail("slots")) else{return Ok(())};
        let words=match declared.settled(){Value::Tuple(items)|Value::Vector(items)=>items.as_ref().clone(),alone=>vec![alone]};
        for word in words {
            let Value::Text(name)=word else{return Err(self.class_unready())};
            if name.as_ref()==self.detail("namespace"){continue;}
            if Self::own_entry(class,&name).is_some(){return Err(self.class_unready());}
            let descriptor=Self::wrap(32,vec![Value::Text(name.clone()),Value::Blueprint(class.clone())]);
            class.shared.borrow_mut().push((name.to_string(),descriptor));
        }
        Ok(())
    }
    /// The key a slot's worth is kept under in a thing. A thing not of
    /// the declaring blueprint's line has no such key, and is told so.
    fn slot_key(&self,thing:&Value,parts:&[Value])->Result<String,Escape> {
        let (Value::Thing(t),Some(Value::Blueprint(declared)))=(thing,parts.get(1)) else{return Err(self.class_unready())};
        let name=parts[0].bare();
        let of_line=Rc::ptr_eq(&t.of,declared)||t.of.ancestry.iter().any(|b|Rc::ptr_eq(b,declared));
        if !of_line {
            let words=self.table.strings("ext.stmt.class.detail.descriptor.foreign");
            if words.len()!=4{return Err(self.class_unready());}
            return Err(format!("{}{name}{}{}{}{}{}",words[0],words[1],declared.name,words[2],t.of.name,words[3]).into());
        }
        Ok(format!("\0slot:{name}:{:p}",Rc::as_ptr(declared)))
    }
    fn slot_value(&self,thing:&Value,parts:&[Value])->Res {
        let key=self.slot_key(thing,parts)?;
        let Value::Thing(t)=thing else{return Err(self.class_unready())};
        let held=t.holds.borrow().iter().find(|(k,_)|*k==key).map(|(_,v)|v.clone());
        held.ok_or_else(||self.absent_attribute(thing,&parts[0].bare()))
    }
    fn slot_change(&self,thing:&Value,parts:&[Value],replacement:Option<Value>)->Res {
        let key=self.slot_key(thing,parts)?;
        let Value::Thing(t)=thing else{return Err(self.class_unready())};
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
        for part in ["property.fget","property.fset","property.fdel","doc"] {
            entries.push((self.detail(part).to_owned(),Self::wrap(58,vec![Value::text(Self::accessor_key(part))])));
        }
        let kind=Rc::new(Blueprint {presentation:Some(format!("<class '{title}'>")),name:title,
            parents:vec![root.clone()],ancestry:vec![root.clone()],under:Some(root),answers:Vec::new(),fields:Vec::new(),
            reaches:Vec::new(),methods:Vec::new(),constants:Vec::new(),shared:RefCell::new(entries)});
        self.property_kind=Some(kind.clone());
        kind
    }
    /// The keys a property keeps its accessors under, out of reach of
    /// any name a program can write.
    fn accessor_key(part:&str)->&'static str {
        match part {"property.fget"=>"\0fget","property.fset"=>"\0fset","property.fdel"=>"\0fdel","doc"=>"\0doc",_=>"\0name"}
    }
    /// Whether a value stands for the property builtin read as a class:
    /// its word, before anything else was bound to that name.
    pub(super) fn spells_property_kind(&self,value:&Value)->bool {
        let word=match value {Value::Intrinsic(w)=>w.as_ref(),Value::Wrapped(8,parts)=>match parts.first(){Some(Value::Text(t))=>t.as_ref(),_=>return false},_=>return false};
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
        let of=match about {Value::Thing(t)=>t.of.name.clone(),Value::Blueprint(b)=>b.name.clone(),other=>other.bare()};
        format!("{}{title}{}{of}{}",words[0],words[1],words[2]).into()
    }
    /// What a property's kept accessor shows: itself; or, for the first
    /// string, the one given, else the getter's own.
    fn accessor_shown(&self,property:&Thing,key:&str)->Value {
        if let Some(v)=Self::kept_accessor(property,key){return v;}
        if key=="\0doc" {
            if let Some(Value::Routine(getter)|Value::Bound(getter,_))=Self::kept_accessor(property,"\0fget") {
                return getter.doc.as_ref().map_or(Value::Nil,|d|Value::text(d));
            }
        }
        Value::Nil
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
                Ok(Value::Thing(Rc::new(Thing{of:property.of.clone(),holds:RefCell::new(holds),turn:self.made})))
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
        match member {Value::Thing(t)=>self.inherited_entry(&t.of,word),_=>None}
    }
    fn through_descriptor(&mut self,member:&Value,hook:Value,values:Vec<Value>)->Res {
        let Value::Thing(t)=member else{return Err(self.class_unready())};
        let bound=self.member_binding(hook,Some(member.clone()),t.of.clone())?;
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
    fn missing_member_escape(&self,escape:&Escape)->bool {
        let Some(kind)=self.detail("attribute.amiss").split(':').next().filter(|k|!k.is_empty()) else{return false};
        match escape {
            Escape::Error(words)=>words.split(':').next()==Some(kind),
            Escape::Thrown(Value::Thing(t))=>t.of.name==kind||t.of.ancestry.iter().any(|b|b.name==kind),
            _=>false,
        }
    }
    fn own_entry(b:&Blueprint,key:&str)->Option<Value> {
        let stored=b.shared.borrow().iter().find(|(n,_)|n==key).map(|(_,v)|v.clone());
        stored.or_else(||b.methods.iter().find(|(n,_)|n==key).map(|(_,p)|Value::Routine(p.clone())))
            .or_else(||b.constants.iter().find(|(n,_)|n==key).map(|(_,v)|v.clone()))
    }
    pub(super) fn inherited_entry(&self,b:&Blueprint,key:&str)->Option<Value> {
        std::iter::once(b).chain(b.ancestry.iter().map(Rc::as_ref)).find_map(|c|Self::own_entry(c,key))
    }
    fn wrap(tag:u8,items:Vec<Value>)->Value {Value::Wrapped(tag,Rc::new(items))}
    pub(super) fn apply_class_member(&mut self,f:Value,mut values:Vec<Value>)->Res {
        match f {
            Value::Bound(code,environment)=>self.invoke(code,environment,values),
            Value::Routine(code)=>self.invoke(code,self.outermost.clone(),values),
            Value::Method(code,thing)=>{values.insert(0,Value::Thing(thing));self.invoke(code,self.outermost.clone(),values)},
            // A thing called stands on its own call member, which may
            // be a thing again. Reaching through one makes no frame, so
            // the step is counted among the calls standing all the
            // same: a thing whose call member is a thing of its own
            // kind is refused at the depth the table allows, as any
            // call that never comes back is.
            Value::Thing(t)=>{let called=self.inherited_entry(&t.of,self.detail("call")).ok_or_else(||self.class_unready())?;self.deeper()?;values.insert(0,Value::Thing(t));let answer=self.apply_class_member(called,values);self.standing-=1;answer},
            Value::Blueprint(c)=>self.construct_ordered(c,values),
            Value::Wrapped(tag,kept)=>{
                match tag {
                    0=>Ok(kept[0].clone()),
                    1 if values.len()==1=>{if let Some(Value::Blueprint(c))=values.first(){self.made+=1;Ok(Value::Thing(Rc::new(Thing{of:c.clone(),holds:RefCell::new(vec![]),turn:self.made})))}else{Err(self.class_unready())}},
                    2 if values.len()==1=>Ok(Value::Nil),
                    // The maker of a native kind: the blueprint to make a
                    // thing of, then what the kind's primitive takes.
                    14 if !values.is_empty()=>{
                        let Value::Blueprint(c)=values.remove(0) else{return Err(self.class_unready());};
                        let word=kept[0].bare();
                        self.thing_over_native(c,&word,values)
                    }
                    3=>{values.insert(0,kept[1].clone());self.apply_class_member(kept[0].clone(),values)},
                    4|8=>self.apply_class_member(kept[0].clone(),values),
                    // The root's formatting of a thing to a specification.
                    59 if values.len()==2=>{
                        let Value::Text(spec)=&values[1] else{return Err(self.class_unready())};
                        let spec=spec.to_string();
                        self.thing_in_spec(&values[0],&spec).map(|shown|Value::text(&shown)).map_err(Escape::from)
                    }
                    13 if values.len()==1=>{
                        let Value::Wrapped(6, property)=&kept[0] else{return Err(self.class_unready());};
                        let mut parts=property.as_ref().clone();
                        parts.resize(2, Value::Nil); parts[1]=values.remove(0);
                        Ok(Self::wrap(6,parts))
                    }
                    10|11|12 if values.len()>=2=>{
                        let subject=values[0].clone();
                        let Value::Text(key)=&values[1]else{return Err(self.class_unready());};
                        if tag==10 {self.read_class_member(subject,key,true)}
                        else {self.alter_class_member(subject,key,if tag==11{values.get(2).cloned()}else{None},true)}
                    }
                    // A binding member's reader, called as the program
                    // calls it: with the thing, or nothing and the class.
                    31 if matches!(values.len(),1|2)=>{
                        let receiver=match &values[0]{Value::Nil=>None,other=>Some(other.clone())};
                        let owner=match (values.get(1),&receiver) {
                            (Some(Value::Blueprint(b)),_)=>b.clone(),
                            (_,Some(Value::Thing(t)))=>t.of.clone(),
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
                match op {Prim::ClassWork(k)=>self.work_on_class(k,values),Prim::SortOf=>self.class_from_type(values),_=>Ok(self.prim(op,&word,&values)?)}
            }
            _=>Err(self.class_unready()),
        }
    }
    /// Making a thing of a class. A class built by a metaclass is called
    /// through that metaclass's own call, which says what comes of it.
    pub(super) fn construct_ordered(&mut self,class:Rc<Blueprint>,given:Vec<Value>)->Res {
        if let Some(builder)=Self::builder_over(&class) {
            if let Some(f)=self.inherited_entry(&builder,self.detail("call")) {
                let mut values=vec![Value::Blueprint(class)];values.extend(given);
                return self.apply_class_member(f,values);
            }
        }
        self.construct_plainly(class,given)
    }
    /// The making itself, as the kind primitive does it: the class
    /// allocates a thing and constructs it.
    pub(super) fn construct_plainly(&mut self,class:Rc<Blueprint>,given:Vec<Value>)->Res {
        let native=Self::native_beneath(&class);
        let allocator=self.inherited_entry(&class,self.detail("allocate"));
        // A metaclass called outright builds a class, the way the kind
        // primitive does, from a name, parents and a namespace.
        if allocator.is_none() && class.ancestry.iter().any(|b|self.builds_classes(b)) {
            let mut values=vec![Value::Blueprint(class.clone())];values.extend(given);
            return self.class_of_parts(values);
        }
        let created=match (allocator,&native) {
            (Some(allocator),_)=>{let mut args=vec![Value::Blueprint(class.clone())];args.extend(given.clone());self.apply_class_member(allocator,args)?},
            (None,Some(word))=>self.thing_over_native(class.clone(),word,given.clone())?,
            (None,None)=>{self.made+=1;Value::Thing(Rc::new(Thing{of:class.clone(),holds:RefCell::new(Vec::new()),turn:self.made}))},
        };
        if let Value::Thing(thing)=&created {
            let belongs=Rc::ptr_eq(&thing.of,&class)||thing.of.ancestry.iter().any(|c|Rc::ptr_eq(c,&class));
            if belongs {
                let constructor=self.table.single("ext.stmt.class.constructor").and_then(|word|self.inherited_entry(&thing.of,word));
                if let Some(f)=constructor {
                    let bound=self.member_binding(f,Some(created.clone()),thing.of.clone())?;
                    if !matches!(self.apply_class_member(bound,given)?,Value::Nil){return Err(self.class_unready());}
                }else if !given.is_empty()&&native.is_none(){return Err(self.class_unready());}
            }
        }
        Ok(created)
    }
    fn absent_attribute(&self,value:&Value,member:&str)->Escape {
        // A namespace and a kind go by their own name, in words of
        // their own, as CPython names them.
        let named=self.member_named_missing(value,member);
        if !named.is_empty(){return named.into();}
        // A thing and a blueprint go by their own name; anything else
        // by the name its kind goes under.
        let kind;
        let name=match value{Value::Thing(t)=>t.of.name.as_str(),Value::Blueprint(b)=>b.name.as_str(),other=>{kind=other.kind_word();&kind}};
        let parts=self.table.strings("ext.stmt.class.detail.attribute.amiss");
        if parts.len()<3{return self.class_unready();}
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
        match &entry {
            Value::Wrapped(4,items)=>return Ok(items[0].clone()),
            Value::Wrapped(5,items)=>return Ok(Self::wrap(3,vec![items[0].clone(),Value::Blueprint(owner)])),
            Value::Wrapped(6,items) if receiver.is_some()=>return self.apply_class_member(items[0].clone(),vec![receiver.unwrap()]),
            Value::Wrapped(32,items) if receiver.is_some()=>return self.slot_value(&receiver.unwrap(),items),
            // A working of the property blueprint, reached through a
            // property, is tied to it; a kept accessor reads at once.
            Value::Wrapped(50..=57,_) if receiver.is_some()=>return Ok(Self::wrap(3,vec![entry.clone(),receiver.unwrap()])),
            Value::Wrapped(58,items)=>return Ok(match receiver {Some(Value::Thing(t))=>self.accessor_shown(&t,&items[0].bare()),_=>entry}),
            _=>{}
        }
        // A member whose blueprint furnishes a reader answers through it,
        // told the thing -- nothing, for a read on the class -- and the
        // class the read came through.
        if let Some(reader)=self.protocol_entry(&entry,"descriptor.get") {
            return self.through_descriptor(&entry,reader,vec![receiver.unwrap_or(Value::Nil),Value::Blueprint(owner)]);
        }
        match receiver {
            Some(Value::Thing(t))=>match entry {Value::Routine(code)=>Ok(Value::Method(code,t)),Value::Bound(..)=>Ok(Self::wrap(3,vec![entry,Value::Thing(t)])),_=>Ok(entry)},
            Some(other) if matches!(entry,Value::Routine(_)|Value::Bound(..))=>Ok(Self::wrap(3,vec![entry,other])),
            _=>Ok(entry),
        }
    }
    fn routine_storage(&mut self, code: &Value) -> usize {
        match self.routine_members.iter().position(|(candidate, _)| candidate.equals(code)) {
            Some(found) => found,
            None => {
                let of = self.common_ancestor(); self.made += 1;
                let holder = Rc::new(Thing { of, turn: self.made, holds: RefCell::new(Vec::new()) });
                self.routine_members.push((code.clone(), holder));
                self.routine_members.len() - 1
            }
        }
    }
    fn member_map(entries:&[(String,Value)])->Value {
        let pairs=entries.iter().map(|(key,value)|(Value::text(key),value.clone())).collect();
        Value::Dict(Rc::new(pairs))
    }
    /// A read that ends with the member missing -- the blueprint's own
    /// reading hook having said so, or a property's getter, or nothing
    /// found -- goes to the fallback reader before it is reported. A
    /// direct read, the root's own, has none.
    pub(super) fn read_class_member(&mut self,value:Value,key:&str,direct:bool)->Res {
        let sought=self.seek_class_member(value.clone(),key,direct);
        if direct{return sought;}
        let (Err(escape),Value::Thing(t))=(&sought,&value) else{return sought};
        if !self.missing_member_escape(escape){return sought;}
        let Some(fallback)=self.table.single("ext.stmt.class.reader").and_then(|n|self.inherited_entry(&t.of,n)) else{return sought};
        let bound=self.member_binding(fallback,Some(value.clone()),t.of.clone())?;
        self.apply_class_member(bound,vec![Value::text(key)])
    }
    fn seek_class_member(&mut self,value:Value,key:&str,direct:bool)->Res {
        if matches!(&value,Value::Wrapped(6,_)) && self.table.spells("ext.stmt.class.property.setter",key) {return Ok(Self::wrap(13,vec![value]));}
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
        // A native kind's word read as a class: its maker, and its name.
        if let Value::Intrinsic(word)=&value {
            if self.table.spells("ext.stmt.class.builtin",word) {
                if key==self.detail("allocate"){return Ok(Self::wrap(14,vec![Value::text(word)]));}
                if key==self.detail("name")||self.table.spells("ext.builtin.class.name",key){return Ok(Value::text(word));}
                // Read as a class the kind stands on the root and on
                // nothing further, which is the whole of its line.
                if key==self.detail("mro")||key==self.detail("order"){
                    let listed=key==self.detail("order");
                    let word=word.to_string();
                    let kind=self.native_kind(&word);
                    let root=self.common_ancestor();
                    let line=Value::Tuple(Rc::new(vec![Value::Blueprint(kind),Value::Blueprint(root)]));
                    return Ok(if listed {Self::wrap(0,vec![line])} else {line});
                }
            }
        }
        if let Value::Blueprint(b)=&value {
            if key==self.detail("name"){return Ok(Value::text(&b.name));}
            if key==self.detail("qualified"){return Ok(self.inherited_entry(b,key).unwrap_or_else(||Value::text(&b.name)));}
            if key==self.detail("namespace"){
                // A blueprint standing for a native kind keeps no
                // entries of its own; what it names are the ones a
                // value of that kind answers to.
                if let Some(word)=Self::native_word(b) {
                    let named=self.kind_member_names(&word);
                    let pairs=named.iter().map(|n|(Value::text(n),Value::text(&format!("<slot wrapper '{n}' of '{word}' objects>")))).collect();
                    return Ok(Value::Dict(Rc::new(pairs)));
                }
                return Ok(Self::member_map(&b.shared.borrow()));
            }
            if key==self.detail("bases"){return Ok(Value::Tuple(Rc::new(b.parents.iter().map(|p|Value::Blueprint(p.clone())).collect())));}
            if key==self.detail("mro")||key==self.detail("order"){
                let mut all=Vec::new();all.push(value.clone());all.extend(b.ancestry.iter().map(|p|Value::Blueprint(p.clone())));
                let result=Value::Tuple(Rc::new(all));return Ok(if key==self.detail("order"){Self::wrap(0,vec![result])}else{result});
            }
            if let Some(found)=self.inherited_entry(b,key){return self.member_binding(found,None,b.clone());}
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
                let tag=if key==self.detail("allocate"){1}else if self.table.single("ext.stmt.class.constructor")==Some(key)||key==self.detail("subclass"){2}
                    else if key==self.detail("get"){10}else if key==self.detail("set"){11}else if key==self.detail("remove"){12}else{255};
                if tag!=255{return Ok(Self::wrap(tag,Vec::new()));}
            }
        }else if let Value::Thing(t)=&value {
            if !direct {if let Some(reader)=self.inherited_entry(&t.of,self.detail("get")){return self.apply_class_member(reader,vec![value.clone(),Value::text(key)]);}}
            if key==self.detail("kind"){return Ok(Value::Blueprint(t.of.clone()));}
            if key==self.detail("namespace"){
                // A blueprint naming its slots without the namespace among them has things without one.
                if !self.allowed_slot(&t.of,key){return Err(self.absent_attribute(&value,key));}
                return Ok(Value::Attributes(t.clone()));
            }
            let from_class=self.inherited_entry(&t.of,key);
            // An entry that takes writes is heard before what the thing
            // holds itself; every other entry after.
            if from_class.as_ref().map_or(false,|e|self.writes_too(e)){return self.member_binding(from_class.unwrap(),Some(value.clone()),t.of.clone());}
            let own=t.holds.borrow().iter().find(|(k,_)|k==key).map(|(_,v)|v.clone());
            if let Some(v)=own{return Ok(v);}
            if let Some(v)=from_class{return self.member_binding(v,Some(value.clone()),t.of.clone());}
            // The worth a thing keeps answers for the methods of its kind.
            if let Some(under)=Self::underlying(&value) {
                if let Some(operation)=Self::kind_method_named(self.table,key){
                    // The parts of a complex number are members read and
                    // not methods called, as on the number itself.
                    if matches!(operation.as_str(),"real"|"imag") {
                        if let Value::Complex(pair)=under.settled() {
                            return Ok(crate::complex::decimal_value(if operation=="real"{pair.0}else{pair.1}));
                        }
                    }
                    return Ok(Value::Member(Rc::new(under),operation));
                }
                // A row of bytes answers to the methods its kind keeps,
                // which the worth beneath the thing carries out.
                if matches!(under.settled(),Value::Octets{..}) {
                    if let Some(working)=self.octet_member(key) {
                        return Ok(Value::Member(Rc::new(under),working.to_string()));
                    }
                }
                // A set answers some of its methods with primitives that
                // take the receiver first, so one read through the thing
                // is tied to the worth it keeps.
                if matches!(self.table.prims.get(key),Some(Prim::SetCall(1..=17))) {
                    return Ok(Self::wrap(3,vec![Value::text(key),under]));
                }
            }
        }else if let Value::Method(code,t)=&value {
            if key==self.detail("receiver"){return Ok(Value::Thing(t.clone()));}
            if key==self.detail("function"){return Ok(Value::Routine(code.clone()));}
            return self.read_class_member(Value::Routine(code.clone()),key,true);
        }else if let Value::Routine(code)|Value::Bound(code,_)=&value {
            if let Some((_,members))=self.routine_members.iter().find(|(f,_)|f.equals(&value)){
                if let Some((_,v))=members.holds.borrow().iter().find(|(k,_)|k==key){return Ok(v.clone());}
            }
            if key==self.detail("name"){return Ok(Value::text(&code.ident));}
            if key==self.detail("qualified"){let qualified=code.qualification.clone();return Ok(Value::text(&qualified));}
            if key==self.detail("doc"){return Ok(code.doc.as_ref().map_or(Value::Nil,|d|Value::text(d)));}
            if key==self.detail("module"){return Ok(Value::text(self.detail("main")));}
            if key==self.detail("code"){return Ok(Self::wrap(7,vec![value.clone()]));}
            if key==self.detail("namespace"){let index=self.routine_storage(&value);return Ok(Value::Attributes(self.routine_members[index].1.clone()));}
            if key==self.detail("defaults"){
                let mut defaults=Vec::new();
                if let Value::Bound(_,env)=&value {for slot in &code.carried{if let Some(i)=code.formal_slots.iter().position(|at|at==slot){if code.taking.as_ref().map_or(true,|rules|matches!(rules[i],'b'|'p')){defaults.push(env.cells.borrow()[*slot].clone());}}}}
                if defaults.is_empty() && !code.local_defaults.is_empty(){return Err(self.class_unready());}
                return Ok(if defaults.is_empty(){Value::Nil}else{Value::Tuple(Rc::new(defaults))});
            }
        }else if let Value::Wrapped(tag,items)=&value {
            if (*tag==4||*tag==5)&&key==self.detail("function"){return Ok(items[0].clone());}
            // A method bound to its thing answers for the thing and the
            // function by the table's words, and for anything else as
            // the function itself would: a method of a class formed in
            // a function is bound this way, and its name, its full name
            // and what it says of itself are the function's.
            if *tag==3 {
                if key==self.detail("receiver"){return Ok(items[1].clone());}
                if key==self.detail("function"){return Ok(items[0].clone());}
                return self.read_class_member(items[0].clone(),key,true);
            }
            if *tag==7 {
                if let Value::Routine(p)|Value::Bound(p,_)=&items[0] {
                    if key==self.detail("argcount"){return Ok(Value::Small(p.taking.as_ref().map_or(p.formals.len(),|rules|rules.iter().filter(|r|matches!(r,'b'|'p')).count()) as i64));}
                    if key==self.detail("varnames"){return Ok(Value::Tuple(Rc::new(p.idents.iter().filter(|s|!s.starts_with('#')).map(|s|Value::text(s)).collect())));}
                }
            }
        }
        // A namespace may hold a routine, under the table's word for it,
        // that answers for names the namespace has not.
        if let (false, Value::Thing(t), Some(word)) = (self.asking_presence, &value, self.table.single("ext.system.module.getattr")) {
            let answerer = t.holds.borrow().iter().find(|(n, _)| n == word).map(|(_, held)| match held { Value::Shared(cell) => cell.borrow().clone(), other => other.settled() });
            if let Some(routine @ (Value::Routine(_) | Value::Bound(..))) = answerer {
                return self.apply_class_member(routine, vec![Value::text(key)]);
            }
        }
        Err(self.absent_attribute(&value,key))
    }
    fn allowed_slot(&self,b:&Blueprint,key:&str)->bool {
        let Some(slots)=Self::own_entry(b,self.detail("slots"))else{return Self::native_word(b).is_none();};
        let matching=|x:&Value|matches!(x,Value::Text(s) if s.as_ref()==key||s.as_ref()==self.detail("namespace"));
        if match slots{Value::Tuple(s)|Value::Vector(s)=>s.iter().any(matching),v=>matching(&v)}{return true;}
        b.parents.iter().any(|p|p.name!=self.detail("root")&&self.allowed_slot(p,key))
    }
    fn change_entry(entries:&mut Vec<(String,Value)>,key:&str,replacement:Option<Value>)->bool {
        if let Some(i)=entries.iter().position(|(k,_)|k==key){
            if let Some(v)=replacement{entries[i].1=v;}else{entries.remove(i);}true
        }else if let Some(v)=replacement{entries.push((key.to_owned(),v));true}else{false}
    }
    pub(super) fn alter_class_member(&mut self,subject:Value,key:&str,replacement:Option<Value>,direct:bool)->Res {
        let writing=replacement.is_some();
        let success=match &subject {
            Value::Thing(t)=>{
                if self.is_fault_kind(&t.of) && self.table.single("ext.builtin.exceptions.args") == Some(key) {
                    if let Some(supplied) = replacement.as_ref() {
                        let sequence = match supplied.settled() {
                            Value::Arguments(items) | Value::Tuple(items) | Value::Vector(items) => Value::Arguments(items),
                            _ => return Err(self.argument_fault("ext.builtin.exceptions.unready", None).into()),
                        };
                        let mut storage = t.holds.borrow_mut();
                        Self::change_entry(&mut storage, key, Some(sequence.clone()));
                        Self::change_entry(&mut storage, "\0raised-values", Some(sequence));
                        return Ok(Value::Nil);
                    }
                }
                if !direct{
                    let hook=self.detail(if replacement.is_some(){"set"}else{"remove"});
                    if let Some(f)=self.inherited_entry(&t.of,hook){let mut args=vec![subject.clone(),Value::text(key)];args.extend(replacement);return self.apply_class_member(f,args);}
                    if let Some(Value::Wrapped(6,property))=self.inherited_entry(&t.of,key){
                        if let (Some(setter),Some(v))=(property.get(1),replacement.clone()){return self.apply_class_member(setter.clone(),vec![subject.clone(),v]);}
                        return Err(self.absent_attribute(&subject,key));
                    }
                }
                // An entry that takes writes takes this one: a slot keeps
                // the worth, a property's kept accessor will not have it,
                // and any other asks its blueprint's writer or remover.
                if let Some(entry)=self.inherited_entry(&t.of,key) {
                    match &entry {
                        Value::Wrapped(32,parts)=>return self.slot_change(&subject,parts,replacement),
                        Value::Wrapped(58,_)=>return Err(self.detail("property.readonly").to_owned().into()),
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
                // The thing's own namespace handed back to it, an entry
                // having been put in through it, is already in place.
                if key==self.detail("namespace") {
                    if let Some(Value::Attributes(view))=&replacement {if Rc::ptr_eq(view,t){return Ok(Value::Nil);}}
                }
                if key==self.detail("kind")||key==self.detail("namespace"){return Err(self.class_unready());}
                // A loaded namespace keeps each binding in a cell its own
                // code reads through; a new value goes into the cell.
                if self.imported.values().any(|held| matches!(held, Value::Thing(space) if Rc::ptr_eq(space, t))) {
                    let link = t.holds.borrow().iter().find(|(k, _)| k == key).and_then(|(_, held)| match held { Value::Shared(link) => Some(link.clone()), _ => None });
                    if let (Some(link), Some(v)) = (link, replacement.clone()) { *link.borrow_mut() = v; return Ok(Value::Nil); }
                }
                if replacement.is_some()&&!self.allowed_slot(&t.of,key){false}else{Self::change_entry(&mut t.holds.borrow_mut(),key,replacement)}
            }
            Value::Blueprint(b)=>{
                if ["name","qualified","kind","bases","mro","namespace","order"].iter().any(|part|key==self.detail(part)){return Err(self.class_unready());}
                Self::change_entry(&mut b.shared.borrow_mut(),key,replacement)
            },
            Value::Routine(_)|Value::Bound(..)=>{
                if key == self.detail("namespace") {
                    let at = self.routine_storage(&subject);
                    if let Some(Value::Attributes(storage)) = &replacement {
                        if Rc::ptr_eq(storage, &self.routine_members[at].1) { return Ok(Value::Nil); }
                    }
                }
                if key==self.detail("code")||key==self.detail("defaults")||key==self.detail("namespace"){return Err(self.class_unready());}
                let index=self.routine_storage(&subject);
                Self::change_entry(&mut self.routine_members[index].1.holds.borrow_mut(),key,replacement)
            }
            _=>false,
        };
        if success{return Ok(Value::Nil);}
        // A thing whose blueprint names the entries it holds, and a
        // value of a builtin kind, have nowhere to put a new entry.
        let nowhere=match &subject {
            Value::Thing(_)=>writing,
            Value::Blueprint(_)|Value::Routine(_)|Value::Bound(..)=>false,
            _=>true,
        };
        if nowhere{return Err(self.unwritable_attribute(&subject,key));}
        Err(self.absent_attribute(&subject,key))
    }
    pub(super) fn class_from_type(&mut self,values:Vec<Value>)->Res {
        let values: Vec<Value> = values.iter().map(Value::settled).collect();
        if values.len()==1 {if let Value::Thing(t)=&values[0]{return Ok(Value::Blueprint(t.of.clone()));}}
        // A class is of the kind that built it: the metaclass named for
        // it or for a class it is built on, and otherwise the kind
        // primitive itself, under whatever word the table spells it by.
        if values.len()==1 {if let Value::Blueprint(b)=&values[0]{
            if let Some(builder)=Self::builder_over(b){return Ok(Value::Blueprint(builder));}
            return Ok(self.table.prims.iter().find(|(_,p)|**p==Prim::SortOf).map_or(Value::Nil,|(word,_)|Value::Intrinsic(Rc::from(word.as_str()))));}}
        if let [Value::Text(title),sequence,Value::Dict(entries)]=values.as_slice(){
            let bases=match sequence{Value::Vector(v)|Value::Tuple(v)=>v,_=>return Err(self.class_unready())};
            let mut parents=Vec::new();for c in bases.iter(){if let Value::Blueprint(b)=c{parents.push(b.clone());}else{return Err(self.class_unready());}}
            let mut own=Vec::new();for (k,v) in entries.iter(){if let Value::Text(key)=k{own.push((key.to_string(),v.clone()));}else{return Err(self.class_unready());}}
            return self.build_class_value(title.to_string(),parents,own);
        }
        Err(self.class_unready())
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
    pub(super) fn names_a_kind(op:&Prim)->bool {
        matches!(op,Prim::AsInt|Prim::AsText|Prim::AsReal|Prim::SortOf|Prim::Listed|Prim::Dictionary
            |Prim::Tupling|Prim::Uniques|Prim::Truthful|Prim::ComplexMade|Prim::Octets(0|1)|Prim::Span
            |Prim::Numbered|Prim::Zipped|Prim::Mapped|Prim::Filtered|Prim::Backwards|Prim::SpanOf
            |Prim::ClassWork(9..=11))
    }
    /// The word a value names a kind by, where it names one at all.
    pub(super) fn kind_spelling(&self,value:&Value)->Option<Rc<str>>{
        let Value::Wrapped(8,parts)=value else{return None};
        let Value::Text(word)=&parts[0] else{return None};
        self.table.prims.get(word.as_ref()).filter(|op|Self::names_a_kind(op)).map(|_|word.clone())
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
            Prim::AsText=>matches!(value,Value::Text(_)),
            Prim::AsReal=>matches!(value,Value::Frac(r) if r.places.is_some()),
            Prim::Listed=>matches!(value,Value::Vector(_)),
            Prim::SortOf=>matches!(value,Value::Blueprint(_)|Value::Intrinsic(_)|Value::OctetKind{..}|Value::KindOf(_))||self.kind_spelling(value).is_some(),
            Prim::Dictionary=>matches!(value,Value::Dict(_)),
            Prim::Tupling=>matches!(value,Value::Tuple(_)),
            Prim::Uniques=>matches!(value,Value::Set(_)),
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
    fn is_beneath(&mut self,subject:&Value,choice:&Value,class_only:bool)->Result<bool,Escape>{
        if let Some(told)=self.builder_answers(choice,subject,class_only)?{return Ok(told);}
        // The byte kinds are values in their own right rather than
        // intrinsic words, so each is asked after under its own word.
        if let Value::OctetKind { changeable, .. } = subject { let word=self.octet_kind_word(*changeable).to_owned(); return self.is_beneath(&Value::Wrapped(8, Rc::new(vec![Value::text(&word)])), choice, class_only); }
        if let Value::OctetKind { changeable, .. } = choice { let word=self.octet_kind_word(*changeable).to_owned(); return self.is_beneath(subject, &Value::Wrapped(8, Rc::new(vec![Value::text(&word)])), class_only); }
        if let Value::Intrinsic(word) = subject { return self.is_beneath(&Value::Wrapped(8, Rc::new(vec![Value::text(word)])), choice, class_only); }
        if let Value::Intrinsic(word) = choice { return self.is_beneath(subject, &Value::Wrapped(8, Rc::new(vec![Value::text(word)])), class_only); }
        // The kind asked after must be a class wherever it is asked, and
        // each of the two has its own words, as the reference has.
        let amiss=if class_only{"core.issubclass.amiss"}else{"core.isinstance.amiss"};
        match choice {
            Value::Tuple(options)|Value::Vector(options)=>{for option in options.iter(){if self.is_beneath(subject,option,class_only)?{return Ok(true);}}Ok(false)},
            Value::Blueprint(c)=>{
                // What is asked about must be a class wherever a class
                // is what is asked about.
                if class_only && !self.counts_as_class(subject){return Err(self.not_a_class("core.issubclass.subject"));}
                // Everything lies under the class everything lies under.
                if c.name==self.detail("root"){return Ok(true);}
                let b=match subject{Value::Blueprint(b) if class_only=>Some(b),Value::Thing(t) if !class_only=>Some(&t.of),_=>None};
                Ok(b.map_or(false,|b|Rc::ptr_eq(b,c)||b.ancestry.iter().any(|a|Rc::ptr_eq(a,c))))
            }
            Value::Wrapped(8,names)=>{
                let Value::Text(word)=&names[0] else{return Err(self.not_a_class(amiss));};
                let Some(op)=self.table.prims.get(word.as_ref()).copied().filter(Self::names_a_kind) else{return Err(self.not_a_class(amiss));};
                if class_only{
                    if let Value::Blueprint(b)=subject{return Ok(Self::native_beneath(b).as_deref()==Some(word.as_ref()));}
                    let Some(under)=self.kind_spelling(subject) else{return Err(self.not_a_class("core.issubclass.subject"));};
                    return Ok(self.kind_under(&under,word));
                }
                // A thing of a blueprint standing on the kind is of the kind.
                if let Value::Thing(t)=subject{return Ok(Self::native_beneath(&t.of).as_deref()==Some(word.as_ref()));}
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
    pub(super) fn work_on_class(&mut self,op:u8,values:Vec<Value>)->Res {
        if op<=1 && values.len()==2{return Ok(Value::Flag(self.is_beneath(&values[0],&values[1],op==1)?));}
        // Both questions want two arguments and name themselves where
        // they are handed another count of them.
        if op<=1 {
            let word=self.table.prims.iter().find(|(_,p)|**p==Prim::ClassWork(op)).map(|(w,_)|w.to_string()).unwrap_or_default();
            return Err(self.wrong_count(&word,2,values.len()));
        }
        if op==2 && values.len()==1{return Ok(Value::Flag(matches!(&values[0],Value::Routine(_)|Value::Bound(..)|Value::Method(..)|Value::Blueprint(_))||matches!(&values[0],Value::Wrapped(tag,_) if matches!(tag,0..=4|8..=12|31|33|34|50..=57|59))||matches!(&values[0],Value::Thing(t) if self.inherited_entry(&t.of,self.detail("call")).is_some())));}
        if (op==3||op==6)&&values.len()>=2{
            let Value::Text(key)=&values[1]else{return Err(self.class_unready());};
            // Asking whether a name is there, or reading it with something
            // to fall back on, does not wake a namespace's own answerer.
            self.asking_presence = op==6 || values.len()==3;
            let read = self.read_class_member(values[0].clone(),key,false);
            self.asking_presence = false;
            return match read {
                // A member read by name reads through the cell a namespace
                // keeps it in, as the program's own member read does.
                Ok(v)=>Ok(if op==6{Value::Flag(true)}else{match v{Value::Shared(cell)=>cell.borrow().clone(),held=>held}}),
                Err(escape) if self.missing_member_escape(&escape)=>{
                    if op==6{Ok(Value::Flag(false))}else if values.len()==3{Ok(values[2].clone())}else{Err(escape)}
                }
                failed=>failed,
            };
        }
        if (op==4&&values.len()==3)||(op==5&&values.len()==2){
            let Value::Text(key)=&values[1]else{return Err(self.class_unready());};return self.alter_class_member(values[0].clone(),key,values.get(2).cloned(),false);
        }
        if op==7&&values.len()==1{let key=self.detail("namespace").to_owned();return self.read_class_member(values[0].clone(),&key,true);}
        if op==8&&values.len()==1{
            // A thing with a directory method of its own answers with it,
            // and the names it gives are set in order.
            if matches!(&values[0],Value::Thing(_)){
                if let Some(answer)=self.ask_special(&values[0],75,&[])?{
                    let mut names=self.object_members(&answer)?;
                    names.sort_by_key(|name|name.bare());
                    return Ok(Value::Vector(Rc::new(names)));
                }
            }
            // A native kind, named as the kind itself or held as a value
            // of one, answers the members a value of that kind has: the
            // methods of the kind and the special names its mark answers.
            if let Some(sample)=self.directory_stand_in(&values[0]) {
                let named=self.native_directory(&sample);
                return Ok(Value::Vector(Rc::new(named.iter().map(|word|Value::text(word)).collect())));
            }
            let mut names=Vec::new();
            let class=match &values[0]{Value::Thing(t)=>{names.extend(t.holds.borrow().iter().filter(|(k,_)|!k.starts_with('\0')).map(|(k,_)|k.clone()));Some(&t.of)},Value::Blueprint(b)=>Some(b),_=>None};
            if let Some(b)=class{for c in std::iter::once(b).chain(b.ancestry.iter()){names.extend(c.shared.borrow().iter().map(|(k,_)|k.clone()));}}
            else if let Some((_,attrs))=self.routine_members.iter().find(|(f,_)|f.equals(&values[0])){names.extend(attrs.holds.borrow().iter().map(|(k,_)|k.clone()));}
            names.sort_unstable();names.dedup();return Ok(Value::Vector(Rc::new(names.iter().map(|s|Value::text(s)).collect())));
        }
        // With the protocol spelled, a property is a thing of the
        // property blueprint; without it, the older wrapper.
        if op==11&&self.protocol_spelled(){let kind=self.property_blueprint();return self.construct_ordered(kind,values);}
        if (9..=11).contains(&op)&&!values.is_empty(){return Ok(Self::wrap(op-5,values));}
        Err(self.class_unready())
    }
    pub(super) fn next_ancestor_call(&mut self,receiver:Value,declared:&str,key:&str,mut args:Vec<Value>)->Res {
        let class=match &receiver{Value::Thing(t)=>t.of.clone(),Value::Blueprint(b)=>b.clone(),_=>return Err(self.class_unready())};
        let named_here=|a:&Self,b:&Rc<Blueprint>|b.name==declared||Self::own_entry(b,a.detail("qualified")).map_or(false,|v|v.bare()==declared);
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
                if self.table.single("ext.stmt.class.constructor")==Some(key){return Ok(Value::Nil);}
                if let (Some(under),Some(operation))=(Self::underlying(&receiver),Self::kind_method_named(self.table,key)) {
                    let (given,named)=self.open_arguments(args)?;
                    return self.value_member(&under,&operation,given,named);
                }
                continue;
            }
            if let Some(f)=Self::own_entry(b,key){if key!=self.detail("allocate"){args.insert(0,receiver.clone());}return self.apply_class_member(f,args);}
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
