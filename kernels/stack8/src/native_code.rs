
impl<'a> Engine<'a> {
    fn native_root_run(&mut self, original: &Rc<Routine>, args: &[Value]) -> Res<Value> {
        if let Some(opcode) = original.invalid_native_opcode() { return Err(format!("SystemError: unknown opcode {opcode}")); }
        let book = |value: Option<&Value>| -> Res<Option<Rc<RefCell<Value>>>> {
            match value {
                None | Some(Value::Null) => Ok(None),
                Some(Value::Bond(cell) | Value::Collection(cell, _)) if matches!(&*cell.borrow(), Value::Map(_)) => Ok(Some(cell.clone())),
                _ => Err("TypeError: globals and locals must be dictionaries".into()),
            }
        };
        let outer = book(args.get(1))?.unwrap_or_else(|| self.outer_book_made());
        let near = book(args.get(2))?;
        let tokens = original.source_tokens.clone();
        let offset = self.registry.idents.len();
        let mut registry = crate::compile::Registry::with_outside_slots(offset);
        registry.program_bound.extend(self.lang.builtins.keys().cloned());
        registry.globe = Some(Value::Bond(outer.clone()));
        registry.future_bits = original.image_origin.2 & !0x307;
        registry.value_only = original.image_origin.2 & 1 != 0;
        registry.interactive = original.image_origin.2 & 2 != 0;
        registry.allow_top_level_await = original.image_origin.2 & 4 != 0;
        registry.optimize = (original.image_origin.2 >> 8) & 3;
        let rebuilt = crate::compile::compile_from(&tokens, self.lang, &mut registry, original.image_origin.1, original.written_in.clone())?;
        let mut program = rebuilt.replacing_constants(&original.code_constants)?;
        program.location_bytes = original.location_bytes.clone();
        program.exception_image = original.exception_image.clone();
        let names = registry.idents[offset..].to_vec();
        for name in &names { self.registry.slot(&format!("\0names:{offset}:{name}")); }
        self.world.resize(self.registry.idents.len(), Value::Blank);
        let (near, outer) = near.map_or((outer.clone(), None), |near| (near, Some(outer)));
        let builtins = registry.born.clone().unwrap_or_else(|| self.native_dict());
        self.text_books.push(TextBook { builtins, near, outer, from: offset, upto: offset + names.len(), declared: registry.declared_outer });
        let previous = self.reading_in.replace(self.text_books.len() - 1);
        let file = original.written_in.clone().unwrap_or_else(|| Rc::from("<string>"));
        let result = self.text_finished(&Rc::new(program), &file, false);
        self.reading_in = previous;
        self.world[offset..offset + names.len()].fill(Value::Blank);
        result
    }

    fn native_code_subject(value: &Value) -> Res<Rc<Routine>> {
        match value.contents() {
            Value::Object(object) if Self::own_kind(&object.class_now()).as_deref() == Some("code") => {
                let retained = object.fields.borrow().iter().find(|(name, _)| name == "\0native_code").map(|(_, value)| value.clone());
                retained.as_ref().ok_or_else(|| "TypeError: expected native code metadata".to_string()).and_then(Self::native_code_subject)
            }
            Value::Routine(body) => Ok(body),
            Value::Adapter(parts) if parts.0 == 7 => match parts.1.first() {
                Some(Value::Routine(body)) => Ok(body.clone()),
                _ => Err("TypeError: expected code".into()),
            },
            _ => Err("TypeError: expected a function or code object".into()),
        }
    }

    fn native_code_load(&mut self, image: &Value) -> Res<Value> {
        let Value::Bytes(bytes, false, _) = image.contents() else { return Err("TypeError: native code image must be bytes".into()) };
        let bytes = bytes.borrow();
        if bytes.is_empty() { return Err("ValueError: empty native code image".into()); }
        let payload: serde_json::Value = serde_json::from_slice(&bytes[1..]).map_err(|_| "ValueError: invalid native code image")?;
        let parts = payload.as_array().filter(|parts| parts.len() == 3).ok_or("ValueError: invalid native code image")?;
        let source = parts[0].as_str().ok_or("ValueError: invalid native code image")?;
        let end = parts[1].as_u64().and_then(|at| usize::try_from(at).ok()).ok_or("ValueError: invalid native code boundary")?;
        let origin: (usize, u32, i64) = serde_json::from_value(parts[2].clone()).map_err(|_| "ValueError: invalid native code origin")?;
        let tokens = crate::lex::image_tokens(source)?;
        let prior = (self.registry.future_bits, self.registry.value_only, self.registry.interactive,
            self.registry.allow_top_level_await, self.registry.optimize);
        self.registry.future_bits = origin.2 & !0x307;
        self.registry.value_only = origin.2 & 1 != 0;
        self.registry.interactive = origin.2 & 2 != 0;
        self.registry.allow_top_level_await = origin.2 & 4 != 0;
        self.registry.optimize = (origin.2 >> 8) & 3;
        let compiled = crate::compile::compile(&tokens, self.lang, &mut self.registry, origin.1);
        (self.registry.future_bits, self.registry.value_only, self.registry.interactive,
            self.registry.allow_top_level_await, self.registry.optimize) = prior;
        self.world.resize(self.registry.idents.len(), Value::Blank);
        let root = compiled?;
        fn locate(body: &Rc<Routine>, end: usize, origin: &(usize, u32, i64)) -> Option<Rc<Routine>> {
            if body.source_end == end && &body.image_origin == origin { return Some(body.clone()); }
            for word in body.instrs.iter() {
                match word {
                    Instr::Const(Value::Routine(child)) => if let Some(found) = locate(child, end, origin) { return Some(found); },
                    Instr::Act(Action::Forge(plan), _) => {
                        for (_, child) in &plan.methods { if let Some(found) = locate(child, end, origin) { return Some(found); } }
                    }
                    _ => {}
                }
            }
            None
        }
        let template = locate(&root, end, &origin).ok_or("ValueError: native code boundary does not name a routine")?;
        let mut body = (*template).clone();
        body.instruction_image = Some(image.clone());
        Ok(self.routine_code(&Rc::new(body)))
    }

    fn native_code_analysis(&self, target: &Value) -> Res<Value> {
        let body = Self::native_code_subject(target)?;
        let mut globals = std::collections::BTreeSet::new();
        let mut attrs = std::collections::BTreeSet::new();
        let global_cell = |cell: &crate::code::Cell| cell.near.is_empty() && !cell.free
            && !cell.ident.starts_with('#') && !body.local_names.iter().any(|name| name == cell.ident.as_ref());
        // A member-pipe fallback belongs to the attribute operation.
        let mut member_calls = std::collections::HashSet::new();
        for (pc, word) in body.instrs.iter().enumerate() {
            let Instr::Act(Action::HasMember(member), _) = word else { continue; };
            let Some(Instr::Skip(fallback)) = body.instrs.get(pc + 1) else { continue; };
            let Some(Instr::Skip(finish)) = fallback.checked_sub(1).and_then(|at| body.instrs.get(at)) else { continue; };
            if let Some(at) = finish.checked_sub(1) {
                if matches!(body.instrs.get(at), Some(Instr::Act(Action::Builtin(_, spelling), _)) if spelling == member) {
                    member_calls.insert(at);
                } else if matches!(body.instrs.get(at), Some(Instr::Act(Action::Invoke(spelling), _)) if spelling == member) {
                    if let Some(load) = at.checked_sub(1) {
                        if matches!(body.instrs.get(load), Some(Instr::Read(cell)) if cell.ident == *member) { member_calls.insert(load); }
                    }
                }
            }
        }
        let mut reached = vec![false; body.instrs.len()];
        let mut pending = vec![0usize];
        while let Some(pc) = pending.pop() {
            if pc >= reached.len() || reached[pc] { continue; }
            reached[pc] = true;
            match &body.instrs[pc] {
                Instr::Depart { to, .. } => pending.push(*to),
                Instr::Act(Action::Builtin(Builtin::Raise, _), _) => {},
                Instr::Attempt(plan) => {
                    pending.extend([plan.body.0, plan.after]);
                    pending.extend(plan.clauses.iter().map(|clause| clause.body.0));
                    pending.extend(plan.otherwise.map(|span| span.0));
                    pending.extend(plan.last.map(|span| span.0));
                }
                word => {
                    pending.push(pc + 1);
                    match word { Instr::Skip(to) | Instr::SkipCmp { to, .. } | Instr::Guard(to) => pending.push(*to), _ => {} }
                }
            }
        }
        let words: Vec<_> = body.instrs.iter().enumerate().filter(|(pc, word)| reached[*pc]
            && !matches!(word, Instr::Line(_) | Instr::Location(..))).collect();
        let mut only_none = true;
        for (at, (pc, word)) in words.iter().enumerate() {
            match word {
                Instr::Read(cell) | Instr::Write(cell) | Instr::Glance(cell) if global_cell(cell) && !member_calls.contains(pc) => { globals.insert(cell.ident.to_string()); }
                Instr::Const(Value::Native(_, name)) if self.lang.builtins.contains_key(name.as_ref()) => { globals.insert(name.to_string()); }
                Instr::Act(Action::Builtin(kind, name), _) if !matches!(kind, Builtin::Raise) && self.lang.builtins.contains_key(name.as_ref()) && !member_calls.contains(pc) => { globals.insert(name.to_string()); }
                Instr::Act(Action::HasMember(name) | Action::Grab(name) | Action::Plant(name), _) => { attrs.insert(name.to_string()); }
                Instr::Dyad { a, b, .. } | Instr::SkipCmp { a, b, .. } => {
                    for operand in [a, b] { if let crate::code::Operand::Cell(cell) = operand {
                        if global_cell(cell) { globals.insert(cell.ident.to_string()); }
                    } }
                }
                Instr::Depart { to, cycle: None } if *to == body.instrs.len() => {
                    // Direct returns carry their value on the operand stack.
                    if !matches!(at.checked_sub(1).and_then(|prior| words.get(prior).map(|(_, word)| *word)), Some(Instr::Const(Value::Null))) { only_none = false; }
                }
                Instr::Write(cell) if cell.ident.as_ref() == "#result" => {
                    let returning = matches!(words.get(at + 1).map(|(_, word)| *word), Some(Instr::Depart { .. }))
                        || matches!(words.get(at + 1).map(|(_, word)| *word), Some(Instr::Read(cell)) if cell.ident.as_ref() == "#result");
                    if returning && !matches!(at.checked_sub(1).and_then(|prior| words.get(prior).map(|(_, word)| *word)), Some(Instr::Const(Value::Null))) { only_none = false; }
                }
                _ => {}
            }
        }
        let names = |set: std::collections::BTreeSet<String>| Value::tuple(set.into_iter().map(|name| Value::text(&name)).collect());
        Ok(Value::tuple(vec![names(globals), names(attrs), Value::Flag(only_none)]))
    }
}
