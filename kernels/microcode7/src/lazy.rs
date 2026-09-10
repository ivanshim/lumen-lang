// A suspended walk releases its borrow before it enters any user routine.
// A routine may thereby ask another walk, or the very same one, to go on.

impl<'a> Machine<'a> {
    fn walk_word(&self, label: &str) -> String {
        self.table.single(label).unwrap_or_default().to_owned()
    }

    fn walk_type(&self, subject: &Value) -> String {
        use Value::*;
        let plain = collection_read(subject);
        let position = match plain {
            Lazy(handle) => return handle.borrow().title.clone(),
            Thing(object) => return object.of.name.clone(),
            Small(_) | Huge(_) => 6,
            Frac(_) => 7,
            Flag(_) => 8,
            Text(_) => 9,
            Vector(_) => 10,
            Dict(_) => 11,
            _ => 12,
        };
        self.table.strings("ext.op.iterator.kind").get(position).cloned().unwrap_or_default()
    }

    fn no_walk(&self, label: &str, value: &Value) -> String {
        self.argument_fault(label, Some(&self.walk_type(value)))
    }

    fn suspend(&self, title: String, work: crate::data::PendingWalk) -> Value {
        let extent = match &work {
            crate::data::PendingWalk::Places(source, _, _) => match collection_read(source) {
                Value::Dict(pairs) => Some(pairs.len()),
                Value::Window(view) => Some(crate::data::window_members(&view.0, view.1).len()),
                _ => None,
            },
            _ => None,
        };
        Value::Lazy(Rc::new(RefCell::new(crate::data::LazyWalk {
            title, work, saved: None, finished: false, extent,
        })))
    }

    fn walk_message(&mut self, receiver: &Value, label: &str, arguments: Vec<Value>) -> Res<Option<Value>> {
        if let Value::Thing(object) = collection_read(receiver) {
            if let Some(program) = object.of.program(&self.walk_word(label)).cloned() {
                let mut supplied = vec![Value::Thing(object)];
                supplied.extend(arguments);
                return self.invoke(program, self.outermost.clone(), supplied).map(Some);
            }
        }
        Ok(None)
    }

    fn walk_call(&mut self, callable: &Value, mut arguments: Vec<Value>) -> Res<Value> {
        let value = collection_read(callable);
        match value {
            Value::Bound(program, environment) => self.invoke(program, environment, arguments),
            Value::Routine(program) => self.invoke(program, self.outermost.clone(), arguments),
            Value::Method(program, receiver) => {
                arguments.insert(0, Value::Thing(receiver));
                self.invoke(program, self.outermost.clone(), arguments)
            }
            Value::Native(operation, spelling) => self.prim(operation, &spelling, &arguments).map_err(Escape::Error),
            other => self.walk_message(&other, "ext.op.iterator.call", arguments)?
                .ok_or_else(|| Escape::Error(self.walk_word("ext.op.iterator.unready"))),
        }
    }

    fn start_walk(&mut self, subject: &Value) -> Res<Value> {
        use crate::data::PendingWalk;
        let content = collection_read(subject);
        if let Value::Lazy(_) = &content { return Ok(content); }
        if let Some(replacement) = self.walk_message(&content, "ext.op.iterator.give", vec![])? {
            let answer = collection_read(&replacement);
            let has_next = match &answer {
                Value::Lazy(_) => true,
                Value::Thing(object) => object.of.program(&self.walk_word("ext.op.iterator.next")).is_some(),
                _ => false,
            };
            return if has_next { Ok(answer) } else { Err(self.no_walk("ext.op.iterator.unnextable", &answer).into()) };
        }
        let title_index = match &content {
            Value::Vector(_) | Value::List(_) | Value::Tuple(_) | Value::Set(_) => Some(0),
            Value::Text(_) => Some(1),
            Value::Progression(_) => Some(2),
            Value::Dict(_) | Value::Window(_) => Some(3),
            Value::Thing(o) if o.of.program(&self.walk_word("ext.op.iterator.item")).is_some() => Some(4),
            _ => None,
        };
        match title_index {
            None => Err(self.no_walk("ext.op.iterator.unwalkable", &content).into()),
            Some(index) => Ok(self.suspend(self.table.strings("ext.op.iterator.kind")[index].clone(), PendingWalk::Places(subject.clone(), BigInt::from(0), false))),
        }
    }

    fn end_of_walk(&self, escape: &Escape, indexed: bool) -> bool {
        let title = match escape {
            Escape::Error(words) => words.as_str(),
            Escape::Thrown(Value::Text(words)) => words.as_ref(),
            Escape::Thrown(Value::Thing(object)) => &object.of.name,
            Escape::Thrown(Value::Blueprint(class)) => &class.name,
            _ => return false,
        };
        self.table.spells("ext.op.iterator.stop", title) || indexed && self.table.spells("ext.op.iterator.end", title)
    }

    fn take_walk(&mut self, iterator: &Value) -> Res<Option<Value>> {
        use crate::data::PendingWalk::*;
        let value = collection_read(iterator);
        let Value::Lazy(shared) = &value else {
            let asked = self.walk_message(&value, "ext.op.iterator.next", vec![]);
            return match asked {
                Ok(None) => Err(self.no_walk("ext.op.iterator.unnextable", &value).into()),
                Err(e) if self.end_of_walk(&e, false) => Ok(None),
                answer => answer,
            };
        };
        if shared.borrow().finished { return Ok(None); }
        if let Some(saved) = shared.borrow_mut().saved.take() { return Ok(Some(saved)); }
        let work = shared.borrow().work.clone();
        let result = match work {
            Stepping(object) => self.take_walk(&object)?,
            Places(sequence, index, backwards) => {
                if index < BigInt::from(0) { None } else {
                    let plain = collection_read(&sequence);
                    let length = match &plain {
                        Value::Dict(pairs) => Some(pairs.len()),
                        Value::Window(view) => Some(crate::data::window_members(&view.0, view.1).len()),
                        _ => None,
                    };
                    if length != shared.borrow().extent {
                        shared.borrow_mut().extent = Some(usize::MAX);
                        return Err(self.walk_word("ext.op.iterator.changed").into());
                    }
                    let found = match plain {
                        Value::Vector(items) | Value::List(items) | Value::Tuple(items) | Value::Set(items) => index.to_usize().and_then(|n| items.get(n).cloned()),
                        Value::Dict(pairs) => index.to_usize().and_then(|n| pairs.get(n).map(|p| p.0.clone())),
                        Value::Text(text) => index.to_usize().and_then(|n| text.chars().nth(n)).map(|c| Value::text(&String::from(c))),
                        Value::Window(view) => index.to_usize().and_then(|at| crate::data::window_members(&view.0, view.1).get(at).cloned()),
                        Value::Progression(p) => if index < p.count() { Some(Value::from_big(&p.first + &index * &p.stride)) } else { None },
                        object => match self.walk_message(&object, "ext.op.iterator.item", vec![Value::from_big(index.clone())]) {
                            Err(e) if self.end_of_walk(&e, true) => None,
                            other => other?,
                        },
                    };
                    shared.borrow_mut().work = Places(sequence, index + if backwards { -1 } else { 1 }, backwards);
                    found
                }
            }
            Calls(function, sentinel) => match self.walk_call(&function, vec![]) {
                Err(e) if self.end_of_walk(&e, false) => None,
                other => { let found = other?; if found.equals(&sentinel) { None } else { Some(found) } }
            },
            Filter(predicate, input) => loop {
                match self.take_walk(&input)? {
                    None => break None,
                    Some(item) => {
                        let test = if matches!(predicate, Value::Nil) { item.clone() } else { self.walk_call(&predicate, vec![item.clone()])? };
                        if self.stands_true(&test) { break Some(item); }
                    }
                }
            },
            Number(input, count) => match self.take_walk(&input)? {
                None => None,
                Some(item) => {
                    shared.borrow_mut().work = Number(input, &count + 1);
                    Some(Value::Tuple(Rc::new(vec![Value::from_big(count), item])))
                }
            },
            Map(function, inputs) => {
                let mut gathered = Vec::new();
                for source in &inputs {
                    let Some(item) = self.take_walk(source)? else { shared.borrow_mut().finished = true; return Ok(None) };
                    gathered.push(item);
                }
                match self.walk_call(&function, gathered) { Ok(v) => Some(v), Err(e) if self.end_of_walk(&e, false) => None, Err(e) => return Err(e) }
            }
            Zip(inputs, strict) => {
                if inputs.is_empty() { None } else {
                    let mut row = Vec::new();
                    for (column, input) in inputs.iter().enumerate() {
                        if let Some(item) = self.take_walk(input)? { row.push(item); continue; }
                        shared.borrow_mut().finished = true;
                        if strict {
                            if column != 0 { return Err(self.argument_fault("ext.builtin.zip.short", Some(&(column + 1).to_string())).into()); }
                            for (number, rest) in inputs.iter().enumerate().skip(1) {
                                if self.take_walk(rest)?.is_some() { return Err(self.argument_fault("ext.builtin.zip.long", Some(&(number + 1).to_string())).into()); }
                            }
                        }
                        return Ok(None);
                    }
                    Some(Value::Tuple(Rc::new(row)))
                }
            }
        };
        shared.borrow_mut().finished = result.is_none();
        Ok(result)
    }

    fn finish_walk(&mut self, operation: Prim, supplied: &[Value]) -> Res<Value> {
        let valid = match operation {
            Prim::Tupled | Prim::Unique | Prim::Dictionary => supplied.len() < 2,
            Prim::JoinedWalk => supplied.len() == 2,
            Prim::Least | Prim::Greatest => supplied.len() > 0,
            Prim::Total => supplied.len() == 1 || supplied.len() == 2,
            _ => supplied.len() == 1,
        };
        if !valid { return Err(self.argument_fault("ext.syntax.call.amiss", None).into()); }
        if operation == Prim::Represented {
            return match supplied { [value] => Ok(Value::text(&self.quoted_remainder(value)?)), _ => Err(self.argument_fault("ext.syntax.call.amiss", None).into()) };
        }

        if supplied.is_empty() {
            return match operation {
                Prim::Unique => Ok(Value::Set(Rc::new(vec![]))),
                Prim::Tupled => Ok(Value::Tuple(Rc::new(vec![]))),
                Prim::Dictionary => Ok(Value::Dict(Rc::new(vec![]))),
                _ => Err(self.argument_fault("ext.syntax.call.amiss", None).into()),
            };
        }
        if operation == Prim::Dictionary && supplied.len() == 1 {
            if let Value::Dict(_) = collection_read(&supplied[0]) { return Ok(collection_read(&supplied[0])); }
        }
        let many = matches!(operation, Prim::Least | Prim::Greatest) && supplied.len() > 1;
        let argument = if many { Value::Vector(Rc::new(supplied.to_vec())) } else if operation == Prim::JoinedWalk { supplied[1].clone() } else { supplied[0].clone() };
        let iterator = self.start_walk(&argument)?;
        let mut contents = vec![];
        let mut sum = supplied.get(1).cloned().unwrap_or(Value::Small(0));
        loop {
            let Some(member) = self.take_walk(&iterator)? else { break };
            match operation {
                Prim::SomeTrue if self.stands_true(&member) => return Ok(Value::Flag(true)),
                Prim::EveryTrue if !self.stands_true(&member) => return Ok(Value::Flag(false)),
                Prim::Total => {
                    let numeric = |x: Value| if let Value::Flag(yes) = x { Value::Small(yes as i64) } else { x };
                    sum = math::compute(Calc::Plus, &numeric(sum), &numeric(member)).ok_or_else(|| self.walk_word("ext.builtin.sum.non_number"))??;
                }
                _ => contents.push(member),
            }
        }
        match operation {
            Prim::Total => Ok(sum),
            Prim::SomeTrue => Ok(Value::Flag(false)),
            Prim::EveryTrue => Ok(Value::Flag(true)),
            Prim::Unique => {
                let mut seen: Vec<Value> = Vec::new();
                for member in contents {
                    if matches!(collection_read(&member), Value::Vector(_) | Value::List(_) | Value::Dict(_) | Value::Set(_)) { return Err(self.walk_word("ext.op.iterator.unready").into()); }
                    if seen.iter().all(|prior| !prior.equals(&member)) { seen.push(member); }
                }
                Ok(Value::Set(Rc::new(seen)))
            }
            Prim::Tupled => Ok(Value::Tuple(Rc::new(contents))),
            Prim::Ordered | Prim::Least | Prim::Greatest => {
                let mut ordered: Vec<Value> = Vec::new();
                for value in contents {
                    let mut position = ordered.len();
                    for (at, prior) in ordered.iter().enumerate() {
                        if self.prim(Prim::Lt, "", &[value.clone(), prior.clone()])?.is_true() { position = at; break; }
                    }
                    ordered.insert(position, value);
                }
                match operation {
                    Prim::Ordered => Ok(Value::List(Rc::new(ordered))),
                    Prim::Least if !ordered.is_empty() => Ok(ordered.remove(0)),
                    Prim::Greatest if !ordered.is_empty() => Ok(ordered.pop().unwrap()),
                    _ => Err(self.walk_word("ext.op.iterator.unready").into()),
                }
            }
            Prim::JoinedWalk => {
                let delimiter = supplied.first().map(collection_read);
                let Some(Value::Text(delimiter)) = delimiter else { return Err(self.walk_word("ext.op.iterator.unready").into()) };
                let mut words = Vec::new();
                for value in contents {
                    if let Value::Text(word) = collection_read(&value) { words.push(word.to_string()); }
                    else { return Err(self.walk_word("ext.op.iterator.unready").into()); }
                }
                Ok(Value::text(&words.join(&delimiter)))
            }
            Prim::Dictionary => {
                let mut associations: Vec<(Value, Value)> = Vec::new();
                for row in contents {
                    let pair = self.gathered_members(&row)?;
                    if pair.len() != 2 { return Err(self.walk_word("ext.op.iterator.unready").into()); }
                    if let Some(at) = associations.iter().position(|(key, _)| key.equals(&pair[0])) { associations[at].1 = pair[1].clone(); }
                    else { associations.push((pair[0].clone(), pair[1].clone())); }
                }
                Ok(Value::Dict(Rc::new(associations)))
            }
            _ => Err(self.walk_word("ext.op.iterator.unready").into()),
        }
    }

    fn lazy_primitive(&mut self, op: Prim, written: &str, values: &[Value]) -> Res<Value> {
        use crate::data::PendingWalk;
        if matches!(op, Prim::KeyWindow | Prim::ValueWindow | Prim::PairWindow) {
            if values.len() != 1 || !matches!(collection_read(&values[0]), Value::Dict(_)) { return Err(self.walk_word("ext.op.iterator.unready").into()); }
            let mode = match op { Prim::KeyWindow => 0, Prim::ValueWindow => 1, _ => 2 };
            return Ok(Value::Window(Rc::new((values[0].clone(), mode, self.table.strings("ext.op.iterator.views")[mode as usize].clone()))));
        }
        if matches!(op, Prim::Tupled | Prim::Unique | Prim::Ordered | Prim::EveryTrue | Prim::Least | Prim::Greatest | Prim::Dictionary | Prim::Represented | Prim::JoinedWalk | Prim::SomeTrue | Prim::Total) { return self.finish_walk(op, values); }
        let wrong = self.argument_fault("ext.syntax.call.amiss", None);
        let title = written.to_owned();
        match op {
            Prim::IteratorOf => match values {
                [source] => self.start_walk(source),
                [function, stop] => Ok(self.suspend(self.table.strings("ext.op.iterator.kind")[5].clone(), PendingWalk::Calls(function.clone(), stop.clone()))),
                _ => Err(wrong.into()),
            },
            Prim::TakeNext if (1..=2).contains(&values.len()) => self.take_walk(&values[0])?
                .or_else(|| values.get(1).cloned()).ok_or_else(|| self.walk_word("ext.op.iterator.stop").into()),
            Prim::Mapped if values.len() >= 2 => {
                let mut inputs = vec![];
                for value in &values[1..] { inputs.push(self.start_walk(value)?); }
                Ok(self.suspend(title, PendingWalk::Map(values[0].clone(), inputs)))
            }
            Prim::Filtered if values.len() == 2 => { let input = self.start_walk(&values[1])?; Ok(self.suspend(title, PendingWalk::Filter(values[0].clone(), input))) }
            Prim::Enumerated if (1..=2).contains(&values.len()) => {
                if values.get(1).map_or(false, |v| !matches!(collection_read(v), Value::Small(_) | Value::Huge(_) | Value::Flag(_))) { return Err(self.walk_word("ext.op.iterator.unready").into()); }
                let index = values.get(1).map(Value::as_big).transpose()?.unwrap_or_else(|| BigInt::from(0));
                let input = self.start_walk(&values[0])?;
                Ok(self.suspend(title, PendingWalk::Number(input, index)))
            }
            Prim::Zipped => {
                let mut strict = false;
                let mut inputs = Vec::new();
                for value in values {
                    if let Value::Couple(pair) = value { strict = self.stands_true(&pair.1); }
                    else { inputs.push(self.start_walk(value)?); }
                }
                Ok(self.suspend(title, PendingWalk::Zip(inputs, strict)))
            }
            Prim::Reversed if values.len() == 1 => {
                if let Some(custom) = self.walk_message(&values[0], "ext.op.iterator.reverse", vec![])? { return Ok(custom); }
                let size = match collection_read(&values[0]) {
                    Value::Vector(v) | Value::List(v) | Value::Tuple(v) => BigInt::from(v.len()),
                    Value::Text(t) => BigInt::from(t.chars().count()),
                    Value::Progression(r) => r.count(),
                    object => self.walk_message(&object, "ext.op.iterator.length", vec![])?.ok_or_else(|| self.walk_word("ext.op.iterator.unready"))?.as_big()?,
                };
                Ok(self.suspend(title, PendingWalk::Places(values[0].clone(), size - 1, true)))
            }
            _ => Err(wrong.into()),
        }
    }
}
