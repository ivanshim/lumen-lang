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
    pub(super) fn form_class(&mut self, name: String, mut bases: Vec<Rc<Class>>, mut members: Vec<(String, Value)>) -> Flow<Value> {
        if bases.is_empty() { bases.push(self.root_class()); }
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
        let module = self.class_word("main").to_string();
        if !members.iter().any(|(n,_)| n == self.class_word("module")) {
            members.push((self.class_word("module").to_string(), Value::text(&module)));
        }
        let display=members.iter().find(|(n,_)|n==self.class_word("qualified")).map(|(_,v)|v.plain()).unwrap_or_else(||name.clone());
        let c = Rc::new(Class { name: name.clone(), outline: Some(format!("<class '{module}.{display}'>")),
            base: bases.first().cloned(), direct: bases, lineage, answers: vec![], fields: vec![], reaches: vec![],
            methods: vec![], constants: vec![], shared: RefCell::new(members) });
        if let Some(hook) = c.lineage.iter().find_map(|b| Self::own_class_value(b, self.class_word("subclass"))) {
            if matches!(&hook,Value::Adapter(w) if w.0==5){let bound=self.bind_class_value(hook,None,c.clone())?;self.class_apply(bound,vec![])?;}
            else{self.class_apply(hook, vec![Value::Class(c.clone())])?;}
        }
        Ok(Value::Class(c))
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
        let object = if let Some(f) = allocation {
            let mut given = vec![Value::Class(c.clone())]; given.extend(args.clone());
            self.class_apply(f,given)?
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
                } else if !args.is_empty() { return Err(self.class_refusal()); }
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
    fn bind_class_value(&mut self, value: Value, subject: Option<Value>, class: Rc<Class>) -> Flow<Value> {
        if let Value::Adapter(w) = &value {
            return match w.0 {
                4 => Ok(w.1[0].clone()),
                5 => Ok(Self::adapter(3,vec![w.1[0].clone(),Value::Class(class)])),
                6 if subject.is_some() => self.class_apply(w.1[0].clone(),vec![subject.unwrap()]),
                _ => Ok(value),
            };
        }
        match (value,subject) {
            (Value::Routine(f),Some(Value::Object(o))) => Ok(Value::Method(o,f)),
            (v,_) => Ok(v),
        }
    }
    pub(super) fn class_get(&mut self, subject: Value, name: &str, plain: bool) -> Flow<Value> {
        if let Value::Adapter(property) = &subject {
            if property.0 == 6 && Lang::spells(&self.lang.property_setter, name) { return Ok(Self::adapter(13, vec![subject])); }
        }
        match &subject {
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
                if name==self.class_word("namespace") {return Ok(Value::Fields(o.clone()));}
                let member=self.class_value(&o.class,name);
                if let Some(Value::Adapter(w))=&member {if w.0==6 {return self.bind_class_value(member.unwrap(),Some(subject.clone()),o.class.clone());}}
                if let Some((_,v))=o.fields.borrow().iter().find(|(n,_)| n==name) {return Ok(v.clone());}
                if let Some(v)=member {return self.bind_class_value(v,Some(subject.clone()),o.class.clone());}
                if !plain {if let Some(f)=self.lang.reader.as_deref().and_then(|n|self.class_value(&o.class,n)){return self.class_apply(f,vec![subject.clone(),Value::text(name)]);}}
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
        let Some(slots)=own else{return true;};
        let allows=|v:&Value|match v {Value::Text(s)=>s.as_ref()==name||s.as_ref()==self.class_word("namespace"),_=>false};
        let fits=match slots {Value::Array(v)|Value::Tuple(v)=>v.iter().any(allows),v=>allows(&v)};
        fits||c.direct.iter().filter(|b|b.name!=self.class_word("root")).any(|b|self.slots_allow(b,name))
    }
    pub(super) fn class_type(&mut self,args:Vec<Value>)->Flow<Value> {
        let args: Vec<Value> = args.iter().map(Value::contents).collect();
        match args.as_slice() {
            [Value::Object(o)]=>Ok(Value::Class(o.class.clone())),
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
                    if !matches!(builtin,Builtin::ToInt|Builtin::ToText|Builtin::AsReal|Builtin::List|Builtin::SortOf){return Err(self.class_refusal());}
                    if let Value::Class(_)=value{return Ok(false);}
                    if let Value::Adapter(other)=value {if other.0==8{return Ok(other.1[0].equals(&w.1[0]));}}
                    return Err(self.class_refusal());
                }
                return Ok(match builtin{Builtin::ToInt=>matches!(value,Value::Small(_)|Value::Huge(_)|Value::Flag(_)),Builtin::ToText=>matches!(value,Value::Text(_)),Builtin::AsReal=>matches!(value,Value::Real(_)),Builtin::List=>matches!(value,Value::Array(_)),Builtin::SortOf=>matches!(value,Value::Class(_)),_=>return Err(self.class_refusal())});
            }}
        }
        Err(self.class_refusal())
    }
    pub(super) fn class_work(&mut self,which:u8,args:Vec<Value>)->Flow<Value> {
        let one=args.first().cloned().unwrap_or(Value::Null);
        match which {
            0|1 if args.len()==2=>Ok(Value::Flag(self.beneath(&one,&args[1],which==1)?)),
            2 if args.len()==1=>Ok(Value::Flag(matches!(one,Value::Class(_)|Value::Routine(_)|Value::Method(..))||matches!(&one,Value::Adapter(w) if matches!(w.0,0..=4|8..=12))||matches!(&one,Value::Object(o) if self.class_value(&o.class,self.class_word("call")).is_some()))),
            3|6 if args.len()>=2=>{let Value::Text(name)=&args[1]else{return Err(self.class_refusal());};match self.class_get(one,name,false){Ok(v)=>Ok(if which==6{Value::Flag(true)}else{match v{Value::Bond(cell)=>cell.borrow().clone(),held=>held}}),Err(Fault::Note(s)) if s.starts_with(self.class_word("attribute.amiss"))=>if which==6{Ok(Value::Flag(false))}else if args.len()==3{Ok(args[2].clone())}else{
                // A module asked by name for a member it has not may answer through its own routine, as it does for a member read in the program.
                if let Value::Object(o)=&args[0]{if let Some(routine)=self.module_reader(o){self.invoke(&routine,vec![Value::text(name)])?;return self.drop_top().map_err(|words|Fault::Note(words));}}
                Err(s.into())},Err(e)=>Err(e)}},
            4|5 if args.len()==if which==4{3}else{2}=>{let Value::Text(n)=&args[1]else{return Err(self.class_refusal());};self.class_write(one,n,args.get(2).cloned(),false)},
            7 if args.len()==1=>{let word=self.class_word("namespace").to_string();self.class_get(one,&word,true)},
            8 if args.len()==1=>{
                let mut names=vec![];let class=match &one{Value::Class(c)=>Some(c),Value::Object(o)=>{names.extend(o.fields.borrow().iter().map(|(n,_)|n.clone()));Some(&o.class)},_=>None};
                if let Some(c)=class {for b in std::iter::once(c).chain(c.lineage.iter()){names.extend(b.shared.borrow().iter().map(|(n,_)|n.clone()));}}
                else if let Some((_,m))=self.function_members.iter().find(|(v,_)|v.equals(&one)){names.extend(m.fields.borrow().iter().map(|(n,_)|n.clone()));}
                names.sort();names.dedup();Ok(Value::array(names.iter().map(|n|Value::text(n)).collect()))
            }
            9..=11 if !args.is_empty()=>Ok(Self::adapter(which-5,args)),
            _=>Err(self.class_refusal()),
        }
    }
    pub(super) fn class_super(&mut self,subject:Value,owner:&str,name:&str,args:Vec<Value>)->Flow<Value> {
        let receiver=match &subject{Value::Object(o)=>o.class.clone(),Value::Class(c)=>c.clone(),_=>return Err(self.class_refusal())};
        let mut sequence=vec![receiver.clone()];sequence.extend(receiver.lineage.iter().cloned());
        let at=sequence.iter().position(|c|c.name==owner || Self::own_class_value(c,self.class_word("qualified")).map_or(false,|v|v.plain()==owner)).ok_or_else(||self.class_refusal())?;
        for c in sequence.iter().skip(at+1) {
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
