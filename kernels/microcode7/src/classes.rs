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
    pub(super) fn build_class_value(&mut self,title:String,mut parents:Vec<Rc<Blueprint>>,mut entries:Vec<(String,Value)>)->Res {
        if parents.is_empty(){parents.push(self.common_ancestor());}
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
        let module=self.detail("main");
        if entries.iter().all(|(k,_)|k!=self.detail("module")){entries.push((self.detail("module").into(),Value::text(module)));}
        let shown=entries.iter().find(|(k,_)|k==self.detail("qualified")).map_or(title.clone(),|(_,v)|v.bare());
        let class=Rc::new(Blueprint {presentation:Some(format!("<class '{module}.{shown}'>")),name:title,
            under:parents.first().cloned(),parents,ancestry:ranks,answers:vec![],fields:vec![],reaches:vec![],
            methods:vec![],constants:vec![],shared:RefCell::new(entries)});
        let hook=class.ancestry.iter().find_map(|b|Self::own_entry(b,self.detail("subclass")));
        if let Some(f)=hook {
            if matches!(&f,Value::Wrapped(5,_)){let bound=self.member_binding(f,None,class.clone())?;self.apply_class_member(bound,Vec::new())?;}
            else{self.apply_class_member(f,vec![Value::Blueprint(class.clone())])?;}
        }
        Ok(Value::Blueprint(class))
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
            Value::Thing(t)=>{let called=self.inherited_entry(&t.of,self.detail("call")).ok_or_else(||self.class_unready())?;values.insert(0,Value::Thing(t));self.apply_class_member(called,values)},
            Value::Blueprint(c)=>self.construct_ordered(c,values),
            Value::Wrapped(tag,kept)=>{
                match tag {
                    0=>Ok(kept[0].clone()),
                    1 if values.len()==1=>{if let Some(Value::Blueprint(c))=values.first(){self.made+=1;Ok(Value::Thing(Rc::new(Thing{of:c.clone(),holds:RefCell::new(vec![]),turn:self.made})))}else{Err(self.class_unready())}},
                    2 if values.len()==1=>Ok(Value::Nil),
                    3=>{values.insert(0,kept[1].clone());self.apply_class_member(kept[0].clone(),values)},
                    4|8=>self.apply_class_member(kept[0].clone(),values),
                    10|11|12 if values.len()>=2=>{
                        let subject=values[0].clone();
                        let Value::Text(key)=&values[1]else{return Err(self.class_unready());};
                        if tag==10 {self.read_class_member(subject,key,true)}
                        else {self.alter_class_member(subject,key,if tag==11{values.get(2).cloned()}else{None},true)}
                    }
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
    pub(super) fn construct_ordered(&mut self,class:Rc<Blueprint>,given:Vec<Value>)->Res {
        let created=match self.inherited_entry(&class,self.detail("allocate")) {
            Some(allocator)=>{let mut args=vec![Value::Blueprint(class.clone())];args.extend(given.clone());self.apply_class_member(allocator,args)?},
            None=>{self.made+=1;Value::Thing(Rc::new(Thing{of:class.clone(),holds:RefCell::new(Vec::new()),turn:self.made}))},
        };
        if let Value::Thing(thing)=&created {
            let belongs=Rc::ptr_eq(&thing.of,&class)||thing.of.ancestry.iter().any(|c|Rc::ptr_eq(c,&class));
            if belongs {
                let constructor=self.table.single("ext.stmt.class.constructor").and_then(|word|self.inherited_entry(&thing.of,word));
                if let Some(f)=constructor {
                    let bound=self.member_binding(f,Some(created.clone()),thing.of.clone())?;
                    if !matches!(self.apply_class_member(bound,given)?,Value::Nil){return Err(self.class_unready());}
                }else if !given.is_empty(){return Err(self.class_unready());}
            }
        }
        Ok(created)
    }
    fn absent_attribute(&self,value:&Value,member:&str)->Escape {
        let name=match value{Value::Thing(t)=>t.of.name.as_str(),Value::Blueprint(b)=>b.name.as_str(),_=>"function"};
        let parts=self.table.strings("ext.stmt.class.detail.attribute.amiss");
        if parts.len()<3{return self.class_unready();}
        format!("{}{name}{}{member}{}",parts[0],parts[1],parts[2]).into()
    }
    fn member_binding(&mut self,entry:Value,receiver:Option<Value>,owner:Rc<Blueprint>)->Res {
        match &entry {
            Value::Wrapped(4,items)=>return Ok(items[0].clone()),
            Value::Wrapped(5,items)=>return Ok(Self::wrap(3,vec![items[0].clone(),Value::Blueprint(owner)])),
            Value::Wrapped(6,items) if receiver.is_some()=>return self.apply_class_member(items[0].clone(),vec![receiver.unwrap()]),
            _=>{}
        }
        if let Some(Value::Thing(t))=receiver {
            match entry {Value::Routine(code)=>Ok(Value::Method(code,t)),Value::Bound(..)=>Ok(Self::wrap(3,vec![entry,Value::Thing(t)])),_=>Ok(entry)}
        }else{Ok(entry)}
    }
    fn member_map(entries:&[(String,Value)])->Value {
        let pairs=entries.iter().map(|(key,value)|(Value::text(key),value.clone())).collect();
        Value::Dict(Rc::new(pairs))
    }
    pub(super) fn read_class_member(&mut self,value:Value,key:&str,direct:bool)->Res {
        if let Value::Blueprint(b)=&value {
            if key==self.detail("name"){return Ok(Value::text(&b.name));}
            if key==self.detail("qualified"){return Ok(self.inherited_entry(b,key).unwrap_or_else(||Value::text(&b.name)));}
            if key==self.detail("namespace"){return Ok(Self::member_map(&b.shared.borrow()));}
            if key==self.detail("bases"){return Ok(Value::Tuple(Rc::new(b.parents.iter().map(|p|Value::Blueprint(p.clone())).collect())));}
            if key==self.detail("mro")||key==self.detail("order"){
                let mut all=Vec::new();all.push(value.clone());all.extend(b.ancestry.iter().map(|p|Value::Blueprint(p.clone())));
                let result=Value::Tuple(Rc::new(all));return Ok(if key==self.detail("order"){Self::wrap(0,vec![result])}else{result});
            }
            if let Some(found)=self.inherited_entry(b,key){return self.member_binding(found,None,b.clone());}
            {
                let tag=if key==self.detail("allocate"){1}else if self.table.single("ext.stmt.class.constructor")==Some(key)||key==self.detail("subclass"){2}
                    else if key==self.detail("get"){10}else if key==self.detail("set"){11}else if key==self.detail("remove"){12}else{255};
                if tag!=255{return Ok(Self::wrap(tag,Vec::new()));}
            }
        }else if let Value::Thing(t)=&value {
            if !direct {if let Some(reader)=self.inherited_entry(&t.of,self.detail("get")){return self.apply_class_member(reader,vec![value.clone(),Value::text(key)]);}}
            if key==self.detail("kind"){return Ok(Value::Blueprint(t.of.clone()));}
            if key==self.detail("namespace"){return Ok(Self::member_map(&t.holds.borrow()));}
            let from_class=self.inherited_entry(&t.of,key);
            if matches!(&from_class,Some(Value::Wrapped(6,_))){return self.member_binding(from_class.unwrap(),Some(value.clone()),t.of.clone());}
            let own=t.holds.borrow().iter().find(|(k,_)|k==key).map(|(_,v)|v.clone());
            if let Some(v)=own{return Ok(v);}
            if let Some(v)=from_class{return self.member_binding(v,Some(value.clone()),t.of.clone());}
            if !direct {let fallback=self.table.single("ext.stmt.class.reader").and_then(|n|self.inherited_entry(&t.of,n));if let Some(f)=fallback{return self.apply_class_member(f,vec![value.clone(),Value::text(key)]);}}
        }else if let Value::Method(code,t)=&value {
            if key==self.detail("receiver"){return Ok(Value::Thing(t.clone()));}
            if key==self.detail("function"){return Ok(Value::Routine(code.clone()));}
            return self.read_class_member(Value::Routine(code.clone()),key,true);
        }else if let Value::Routine(code)|Value::Bound(code,_)=&value {
            if let Some((_,members))=self.routine_members.iter().find(|(f,_)|f.equals(&value)){
                if let Some((_,v))=members.iter().find(|(k,_)|k==key){return Ok(v.clone());}
            }
            if key==self.detail("name"){return Ok(Value::text(&code.ident));}
            if key==self.detail("qualified"){let qualified=code.qualification.clone();return Ok(Value::text(&qualified));}
            if key==self.detail("doc"){return Ok(code.doc.as_ref().map_or(Value::Nil,|d|Value::text(d)));}
            if key==self.detail("module"){return Ok(Value::text(self.detail("main")));}
            if key==self.detail("code"){return Ok(Self::wrap(7,vec![value.clone()]));}
            if key==self.detail("namespace"){return Ok(self.routine_members.iter().find(|(f,_)|f.equals(&value)).map_or_else(||Self::member_map(&[]),|(_,m)|Self::member_map(m)));}
            if key==self.detail("defaults"){
                let mut defaults=Vec::new();
                if let Value::Bound(_,env)=&value {for slot in &code.carried{if let Some(i)=code.formal_slots.iter().position(|at|at==slot){if code.taking.as_ref().map_or(true,|rules|matches!(rules[i],'b'|'p')){defaults.push(env.cells.borrow()[*slot].clone());}}}}
                if defaults.is_empty() && !code.local_defaults.is_empty(){return Err(self.class_unready());}
                return Ok(if defaults.is_empty(){Value::Nil}else{Value::Tuple(Rc::new(defaults))});
            }
        }else if let Value::Wrapped(tag,items)=&value {
            if (*tag==4||*tag==5)&&key==self.detail("function"){return Ok(items[0].clone());}
            if *tag==3 {
                if key==self.detail("receiver"){return Ok(items[1].clone());}
                if key==self.detail("function"){return Ok(items[0].clone());}
            }
            if *tag==7 {
                if let Value::Routine(p)|Value::Bound(p,_)=&items[0] {
                    if key==self.detail("argcount"){return Ok(Value::Small(p.taking.as_ref().map_or(p.formals.len(),|rules|rules.iter().filter(|r|matches!(r,'b'|'p')).count()) as i64));}
                    if key==self.detail("varnames"){return Ok(Value::Tuple(Rc::new(p.idents.iter().filter(|s|!s.starts_with('#')).map(|s|Value::text(s)).collect())));}
                }
            }
        }
        Err(self.absent_attribute(&value,key))
    }
    fn allowed_slot(&self,b:&Blueprint,key:&str)->bool {
        let Some(slots)=Self::own_entry(b,self.detail("slots"))else{return true;};
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
        let success=match &subject {
            Value::Thing(t)=>{
                if !direct{
                    let hook=self.detail(if replacement.is_some(){"set"}else{"remove"});
                    if let Some(f)=self.inherited_entry(&t.of,hook){let mut args=vec![subject.clone(),Value::text(key)];args.extend(replacement);return self.apply_class_member(f,args);}
                    if let Some(Value::Wrapped(6,property))=self.inherited_entry(&t.of,key){
                        if let (Some(setter),Some(v))=(property.get(1),replacement.clone()){return self.apply_class_member(setter.clone(),vec![subject.clone(),v]);}
                        return Err(self.absent_attribute(&subject,key));
                    }
                }
                if key==self.detail("kind")||key==self.detail("namespace"){return Err(self.class_unready());}
                if replacement.is_some()&&!self.allowed_slot(&t.of,key){false}else{Self::change_entry(&mut t.holds.borrow_mut(),key,replacement)}
            }
            Value::Blueprint(b)=>{
                if ["name","qualified","kind","bases","mro","namespace","order"].iter().any(|part|key==self.detail(part)){return Err(self.class_unready());}
                Self::change_entry(&mut b.shared.borrow_mut(),key,replacement)
            },
            Value::Routine(_)|Value::Bound(..)=>{
                if key==self.detail("code")||key==self.detail("defaults")||key==self.detail("namespace"){return Err(self.class_unready());}
                let index=self.routine_members.iter().position(|(f,_)|f.equals(&subject)).unwrap_or_else(||{self.routine_members.push((subject.clone(),vec![]));self.routine_members.len()-1});
                Self::change_entry(&mut self.routine_members[index].1,key,replacement)
            }
            _=>false,
        };
        if success{Ok(Value::Nil)}else{Err(self.absent_attribute(&subject,key))}
    }
    pub(super) fn class_from_type(&mut self,values:Vec<Value>)->Res {
        if values.len()==1 {if let Value::Thing(t)=&values[0]{return Ok(Value::Blueprint(t.of.clone()));}}
        if let [Value::Text(title),sequence,Value::Dict(entries)]=values.as_slice(){
            let bases=match sequence{Value::Vector(v)|Value::Tuple(v)=>v,_=>return Err(self.class_unready())};
            let mut parents=Vec::new();for c in bases.iter(){if let Value::Blueprint(b)=c{parents.push(b.clone());}else{return Err(self.class_unready());}}
            let mut own=Vec::new();for (k,v) in entries.iter(){if let Value::Text(key)=k{own.push((key.to_string(),v.clone()));}else{return Err(self.class_unready());}}
            return self.build_class_value(title.to_string(),parents,own);
        }
        Err(self.class_unready())
    }
    fn is_beneath(&self,subject:&Value,choice:&Value,class_only:bool)->Result<bool,Escape>{
        match choice {
            Value::Tuple(options)|Value::Vector(options)=>{for option in options.iter(){if self.is_beneath(subject,option,class_only)?{return Ok(true);}}Ok(false)},
            Value::Blueprint(c)=>{
                if c.name==self.detail("root"){
                    if !class_only||matches!(subject,Value::Blueprint(_)){return Ok(true);}
                    if let Value::Wrapped(8,parts)=subject{if let Value::Text(n)=&parts[0]{return Ok(matches!(self.table.prims.get(n.as_ref()),Some(Prim::AsInt|Prim::AsText|Prim::AsReal|Prim::Listed|Prim::SortOf)));}}
                    return Err(self.class_unready());
                }
                let b=match subject{Value::Blueprint(b) if class_only=>Some(b),Value::Thing(t) if !class_only=>Some(&t.of),_=>None};
                Ok(b.map_or(false,|b|Rc::ptr_eq(b,c)||b.ancestry.iter().any(|a|Rc::ptr_eq(a,c))))
            }
            Value::Wrapped(8,names)=>{
                let Value::Text(word)=&names[0] else{return Err(self.class_unready());};
                if class_only{
                    if !matches!(self.table.prims.get(word.as_ref()),Some(Prim::AsInt|Prim::AsText|Prim::AsReal|Prim::Listed|Prim::SortOf)){return Err(self.class_unready());}
                    return match subject {Value::Blueprint(_)=>Ok(false),Value::Wrapped(8,other)=>Ok(other[0].equals(&names[0])),_=>Err(self.class_unready())};
                }
                match self.table.prims.get(word.as_ref()){
                    Some(Prim::AsInt)=>Ok(matches!(subject,Value::Small(_)|Value::Huge(_)|Value::Flag(_))),
                    Some(Prim::AsText)=>Ok(matches!(subject,Value::Text(_))),
                    Some(Prim::AsReal)=>Ok(matches!(subject,Value::Frac(r) if r.places.is_some())),
                    Some(Prim::Listed)=>Ok(matches!(subject,Value::Vector(_))),
                    Some(Prim::SortOf)=>Ok(matches!(subject,Value::Blueprint(_))),
                    _=>Err(self.class_unready()),
                }
            },
            _=>Err(self.class_unready()),
        }
    }
    pub(super) fn work_on_class(&mut self,op:u8,values:Vec<Value>)->Res {
        if op<=1 && values.len()==2{return Ok(Value::Flag(self.is_beneath(&values[0],&values[1],op==1)?));}
        if op==2 && values.len()==1{return Ok(Value::Flag(matches!(&values[0],Value::Routine(_)|Value::Bound(..)|Value::Method(..)|Value::Blueprint(_))||matches!(&values[0],Value::Wrapped(tag,_) if matches!(tag,0..=4|8..=12))||matches!(&values[0],Value::Thing(t) if self.inherited_entry(&t.of,self.detail("call")).is_some())));}
        if (op==3||op==6)&&values.len()>=2{
            let Value::Text(key)=&values[1]else{return Err(self.class_unready());};
            return match self.read_class_member(values[0].clone(),key,false){
                Ok(v)=>Ok(if op==6{Value::Flag(true)}else{v}),
                Err(Escape::Error(words)) if words.starts_with(self.detail("attribute.amiss"))=>{
                    if op==6{Ok(Value::Flag(false))}else if values.len()==3{Ok(values[2].clone())}else{Err(words.into())}
                }
                failed=>failed,
            };
        }
        if (op==4&&values.len()==3)||(op==5&&values.len()==2){
            let Value::Text(key)=&values[1]else{return Err(self.class_unready());};return self.alter_class_member(values[0].clone(),key,values.get(2).cloned(),false);
        }
        if op==7&&values.len()==1{let key=self.detail("namespace").to_owned();return self.read_class_member(values[0].clone(),&key,true);}
        if op==8&&values.len()==1{
            let mut names=Vec::new();
            let class=match &values[0]{Value::Thing(t)=>{names.extend(t.holds.borrow().iter().map(|(k,_)|k.clone()));Some(&t.of)},Value::Blueprint(b)=>Some(b),_=>None};
            if let Some(b)=class{for c in std::iter::once(b).chain(b.ancestry.iter()){names.extend(c.shared.borrow().iter().map(|(k,_)|k.clone()));}}
            else if let Some((_,attrs))=self.routine_members.iter().find(|(f,_)|f.equals(&values[0])){names.extend(attrs.iter().map(|(k,_)|k.clone()));}
            names.sort_unstable();names.dedup();return Ok(Value::Vector(Rc::new(names.iter().map(|s|Value::text(s)).collect())));
        }
        if (9..=11).contains(&op)&&!values.is_empty(){return Ok(Self::wrap(op-5,values));}
        Err(self.class_unready())
    }
    pub(super) fn next_ancestor_call(&mut self,receiver:Value,declared:&str,key:&str,mut args:Vec<Value>)->Res {
        let class=match &receiver{Value::Thing(t)=>&t.of,Value::Blueprint(b)=>b,_=>return Err(self.class_unready())};
        let chain=std::iter::once(class).chain(class.ancestry.iter()).collect::<Vec<_>>();
        let start=chain.iter().position(|b|b.name==declared||Self::own_entry(b,self.detail("qualified")).map_or(false,|v|v.bare()==declared)).ok_or_else(||self.class_unready())?;
        for b in &chain[start+1..]{
            if let Some(f)=Self::own_entry(b,key){if key!=self.detail("allocate"){args.insert(0,receiver.clone());}return self.apply_class_member(f,args);}
            if b.name==self.detail("root") && key==self.detail("allocate"){let f=self.read_class_member(Value::Blueprint((*b).clone()),key,true)?;return self.apply_class_member(f,args);}
            if b.name==self.detail("root") {
                let builtin=self.read_class_member(Value::Blueprint((*b).clone()),key,true)?;
                args.insert(0,receiver.clone());return self.apply_class_member(builtin,args);
            }
        }
        Err(self.absent_attribute(&receiver,key))
    }
}
