// Walks held between calls. The state is copied before a call, so that
// the called routine may itself ask a walk for another member.

impl<'a> Engine<'a> {
    fn cursor_words(&self, label: &str) -> &[String] {
        self.lang.iterator_words.get(label).map(Vec::as_slice).unwrap_or(&[])
    }

    fn cursor_word(&self, label: &str) -> String {
        self.cursor_words(label).first().cloned().unwrap_or_default()
    }

    fn cursor_kind(&self, value: &Value) -> String {
        let held = collection_contents(value);
        let kinds = self.cursor_words("ext.op.iterator.kind");
        let at = match held {
            Value::Cursor(c) => return c.borrow().name.clone(),
            Value::Object(o) => return o.class.name.clone(),
            Value::Small(_) | Value::Huge(_) => 6,
            Value::Real(_) | Value::Frac(_) => 7,
            Value::Flag(_) => 8,
            Value::Text(_) => 9,
            Value::Array(_) => 10,
            Value::Map(_) => 11,
            _ => 12,
        };
        kinds.get(at).cloned().unwrap_or_default()
    }

    fn cursor_fault(&self, label: &str, value: &Value) -> String {
        Self::named_fault(self.cursor_words(label), &self.cursor_kind(value))
    }

    fn cursor_make(&self, way: u8, name: String, sources: Vec<Value>, function: Value, sentinel: Value, at: BigInt) -> Value {
        Value::Cursor(Rc::new(RefCell::new(crate::value::Cursor {
            way, name, sources, function, sentinel, at, ended: false, waiting: None,
        })))
    }

    fn cursor_method(&mut self, value: &Value, label: &str, mut args: Vec<Value>) -> Flow<Option<Value>> {
        let Value::Object(o) = collection_contents(value) else { return Ok(None) };
        let Some(method) = o.class.method(&self.cursor_word(label)).cloned() else { return Ok(None) };
        args.insert(0, Value::Object(o));
        self.invoke(&method, args)?;
        Ok(Some(self.drop_top()?))
    }

    fn cursor_call(&mut self, function: &Value, args: Vec<Value>) -> Flow<Value> {
        if let Some(answer) = self.cursor_method(function, "ext.op.iterator.call", args.clone())? { return Ok(answer); }
        let count = args.len();
        self.data.extend(args);
        self.data.push(collection_contents(function));
        self.perform(&Action::Invoke(Rc::from("")), count + 1)?;
        self.drop_top()
    }

    fn cursor_from(&mut self, value: &Value) -> Flow<Value> {
        let held = collection_contents(value);
        if matches!(held, Value::Cursor(_)) { return Ok(held); }
        if let Some(given) = self.cursor_method(&held, "ext.op.iterator.give", Vec::new())? {
            let given = collection_contents(&given);
            if matches!(&given, Value::Cursor(_)) || matches!(&given, Value::Object(o) if o.class.method(&self.cursor_word("ext.op.iterator.next")).is_some()) { return Ok(given); }
            return Err(self.cursor_fault("ext.op.iterator.unnextable", &given).into());
        }
        let kind = match &held {
            Value::Array(_) => 0,
            Value::Text(_) => 1,
            Value::Counted(_) => 2,
            Value::Map(_) => 3,
            Value::Object(o) if o.class.method(&self.cursor_word("ext.op.iterator.item")).is_some() => 4,
            _ => return Err(self.cursor_fault("ext.op.iterator.unwalkable", &held).into()),
        };
        let name = self.cursor_words("ext.op.iterator.kind")[kind].clone();
        Ok(self.cursor_make(0, name, vec![value.clone()], Value::Null, Value::Null, BigInt::from(0)))
    }

    fn cursor_stopped(&self, fault: &Fault, sequence: bool) -> bool {
        let matches = |name: &str| name == self.cursor_word("ext.op.iterator.stop")
            || sequence && name == self.cursor_word("ext.op.iterator.end");
        match fault {
            Fault::Thrown(Value::Object(o)) => matches(&o.class.name),
            Fault::Thrown(Value::Class(c)) => matches(&c.name),
            Fault::Thrown(Value::Text(t)) => matches(t),
            Fault::Note(t) => matches(t),
            _ => false,
        }
    }

    fn cursor_next(&mut self, value: &Value) -> Flow<Option<Value>> {
        let value = collection_contents(value);
        let Value::Cursor(cell) = &value else {
            return match self.cursor_method(&value, "ext.op.iterator.next", Vec::new()) {
                Ok(Some(v)) => Ok(Some(v)),
                Ok(None) => Err(self.cursor_fault("ext.op.iterator.unnextable", &value).into()),
                Err(e) if self.cursor_stopped(&e, false) => Ok(None),
                Err(e) => Err(e),
            };
        };
        let state = cell.borrow().clone();
        if state.ended { return Ok(None); }
        if state.waiting.is_some() { return Ok(cell.borrow_mut().waiting.take()); }
        let answer = match state.way {
            0 | 6 => {
                if state.at < BigInt::from(0) { None } else {
                    let source = collection_contents(&state.sources[0]);
                    let at = state.at.to_usize();
                    let found = match &source {
                        Value::Array(a) => at.and_then(|n| a.get(n).cloned()),
                        Value::Map(m) => at.and_then(|n| m.get(n).map(|(k, _)| k.clone())),
                        Value::Text(t) => at.and_then(|n| t.chars().nth(n).map(|c| Value::text(&c.to_string()))),
                        Value::Counted(r) => r.at(state.at.clone()),
                        _ => match self.cursor_method(&source, "ext.op.iterator.item", vec![Value::of_big(state.at.clone())]) {
                            Ok(answer) => answer,
                            Err(e) if self.cursor_stopped(&e, true) => None,
                            Err(e) => return Err(e),
                        },
                    };
                    cell.borrow_mut().at += if state.way == 6 { -1 } else { 1 };
                    found
                }
            }
            1 => match self.cursor_call(&state.function, Vec::new()) {
                Ok(v) if v.equals(&state.sentinel) => None,
                Ok(v) => Some(v),
                Err(e) if self.cursor_stopped(&e, false) => None,
                Err(e) => return Err(e),
            },
            2 | 4 => {
                let mut args = Vec::new();
                for (at, source) in state.sources.iter().enumerate() {
                    match self.cursor_next(source)? {
                        Some(v) => args.push(v),
                        None => {
                            cell.borrow_mut().ended = true;
                            if state.way == 4 && self.truth(&state.sentinel) {
                                if at > 0 { return Err(Self::named_fault(self.cursor_words("ext.builtin.zip.short"), &(at + 1).to_string()).into()); }
                                for (later, other) in state.sources.iter().enumerate().skip(1) {
                                    if self.cursor_next(other)?.is_some() { return Err(Self::named_fault(self.cursor_words("ext.builtin.zip.long"), &(later + 1).to_string()).into()); }
                                }
                            }
                            return Ok(None);
                        }
                    }
                }
                if args.is_empty() { None }
                else if state.way == 2 { Some(self.cursor_call(&state.function, args)?) }
                else { Some(Value::array(args)) }
            }
            3 => loop {
                let Some(v) = self.cursor_next(&state.sources[0])? else { break None };
                let selected = if matches!(state.function, Value::Null) { self.truth(&v) }
                    else { let answer = self.cursor_call(&state.function, vec![v.clone()])?; self.truth(&answer) };
                if selected { break Some(v); }
            },
            5 => match self.cursor_next(&state.sources[0])? {
                None => None,
                Some(v) => { cell.borrow_mut().at += 1; Some(Value::array(vec![Value::of_big(state.at), v])) }
            },
            _ => return Err(self.cursor_word("ext.op.iterator.unready").into()),
        };
        if answer.is_none() { cell.borrow_mut().ended = true; }
        Ok(answer)
    }

    fn cursor_builtin(&mut self, builtin: Builtin, name: &str, args: &[Value]) -> Flow<Value> {
        let amiss = || self.lang.call_amiss.first().cloned().unwrap_or_default();
        if builtin == Builtin::Next {
            if !(1..=2).contains(&args.len()) { return Err(amiss().into()); }
            return self.cursor_next(&args[0])?.or_else(|| args.get(1).cloned()).ok_or_else(|| self.cursor_word("ext.op.iterator.stop").into());
        }
        if builtin == Builtin::Iterate {
            return match args {
                [value] => self.cursor_from(value),
                [function, sentinel] => Ok(self.cursor_make(1, self.cursor_words("ext.op.iterator.kind")[5].clone(), Vec::new(), function.clone(), sentinel.clone(), BigInt::from(0))),
                _ => Err(amiss().into()),
            };
        }
        let (way, sources, function, sentinel, mut at) = match builtin {
            Builtin::MapLazy if args.len() >= 2 => (2, &args[1..], args[0].clone(), Value::Null, BigInt::from(0)),
            Builtin::FilterLazy if args.len() == 2 => (3, &args[1..], args[0].clone(), Value::Null, BigInt::from(0)),
            Builtin::ZipLazy => {
                let strict = args.last().and_then(|v| if let Value::Tie(p) = v { Some(p.1.clone()) } else { None });
                let end = args.len() - usize::from(strict.is_some());
                (4, &args[..end], Value::Null, strict.unwrap_or(Value::Flag(false)), BigInt::from(0))
            }
            Builtin::EnumerateLazy if (1..=2).contains(&args.len()) => (5, &args[..1], Value::Null, Value::Null, args.get(1).map(Value::as_big).transpose()?.unwrap_or_else(|| BigInt::from(0))),
            Builtin::ReverseLazy if args.len() == 1 => (6, &args[..1], Value::Null, Value::Null, BigInt::from(0)),
            _ => return Err(amiss().into()),
        };
        let mut held = Vec::new();
        if way == 6 {
            if let Some(answer) = self.cursor_method(&sources[0], "ext.op.iterator.reverse", Vec::new())? { return Ok(answer); }
            let source = collection_contents(&sources[0]);
            at = match &source {
                Value::Array(a) => BigInt::from(a.len()),
                Value::Text(s) => BigInt::from(s.chars().count()),
                Value::Counted(r) => r.length(),
                Value::Object(_) => self.cursor_method(&source, "ext.op.iterator.length", Vec::new())?.ok_or_else(|| self.cursor_word("ext.op.iterator.unready"))?.as_big()?,
                _ => return Err(self.cursor_word("ext.op.iterator.unready").into()),
            } - 1;
            held.push(sources[0].clone());
        } else { for source in sources { held.push(self.cursor_from(source)?); } }
        Ok(self.cursor_make(way, name.to_string(), held, function, sentinel, at))
    }
}
