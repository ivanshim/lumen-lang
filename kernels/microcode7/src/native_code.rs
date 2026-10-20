
fn image_children(node: &Form) -> Vec<&Form> {
    match node {
        Form::Write(_, child) | Form::Tie(_, child) | Form::OnLine(_, _, child) | Form::Located(_, _, child)
        | Form::Muted(child) | Form::Silenced(child) | Form::ShareItem(_, child) | Form::ShareField(child, _)
        | Form::ShareOwn(child, _) | Form::ShareCalled(child) | Form::ForgetCalled(child) | Form::ReadyCalled(child)
        | Form::Called(child) | Form::CellOrSaid(_, _, _, child) | Form::HeldEither(_, _, _, child) => vec![child],
        Form::Apply(callee, values) => {
            let mut children: Vec<_> = values.iter().collect();
            if let Callee::Code(value) = callee { children.push(value); }
            children
        }
        Form::Dyad { a, b, .. } => [a, b].into_iter().filter_map(|value| match value { Input::Form(child) => Some(child.as_ref()), _ => None }).collect(),
        Form::Cycle { test, body, step, otherwise, .. } => {
            let mut children = vec![test.as_ref(), body.as_ref()];
            children.extend(step.as_deref()); children.extend(otherwise.as_deref()); children
        }
        Form::Attempt { body, clauses, last, otherwise, .. } => {
            let mut children = vec![body.as_ref()];
            for clause in clauses { children.push(&clause.body); if let Some(choices) = &clause.choices { children.extend(choices); } }
            children.extend(last.as_deref()); children.extend(otherwise.as_deref()); children
        }
        Form::Class { values, .. } => values.iter().collect(),
        Form::Assert { condition, message } => vec![condition, message],
        Form::Fits { value, kinds, .. } => std::iter::once(value.as_ref()).chain(kinds).collect(),
        Form::SharePlace(_, keys) => keys.iter().collect(),
        Form::ShareWithin(value, keys) => std::iter::once(value.as_ref()).chain(keys).collect(),
        Form::ForgetWithin(a, b) | Form::TieCalled(a, b) | Form::CallWrite(a, b) => vec![a, b],
        _ => Vec::new(),
    }
}

fn image_body(target: &Value) -> Result<Rc<Routine>, String> {
    match target.settled() {
        Value::Thing(instance) if Machine::native_word(&instance.blueprint()).as_deref() == Some("code") => {
            let saved = instance.holds.borrow().iter().find_map(|(name, value)| (name == "\0native_code").then(|| value.clone()));
            image_body(saved.as_ref().ok_or("TypeError: expected native code metadata")?)
        }
        Value::Routine(routine) | Value::Bound(routine, _) => Ok(routine),
        Value::Wrapped(7, payload) => match payload.first() {
            Some(Value::Routine(routine)) => Ok(routine.clone()),
            _ => Err(String::from("TypeError: expected code")),
        },
        _ => Err(String::from("TypeError: expected a function or code object")),
    }
}

fn find_image_routine(routine: &Rc<Routine>, boundary: usize, entry: &(usize, u32, i64)) -> Option<Rc<Routine>> {
    if routine.body_boundary == boundary && &routine.compilation_entry == entry { return Some(routine.clone()); }
    fn search(node: &Form, boundary: usize, entry: &(usize, u32, i64)) -> Option<Rc<Routine>> {
        if let Form::Const(Value::Routine(child)) = node {
            if let Some(found) = find_image_routine(child, boundary, entry) { return Some(found); }
        }
        if let Form::Class { plan, .. } = node {
            for (_, method) in &plan.methods {
                if let Some(found) = find_image_routine(method, boundary, entry) { return Some(found); }
            }
        }
        for child in image_children(node) { if let Some(found) = search(child, boundary, entry) { return Some(found); } }
        None
    }
    search(&routine.body, boundary, entry)
}

impl<'a> Machine<'a> {
    fn perform_native_root(&mut self, code: &Rc<Routine>, arguments: &[Value]) -> Result<Value, String> {
        if let Some(operation) = code.invalid_image_operation() { return Err(format!("SystemError: unknown opcode {operation}")); }
        fn dictionary(value: Option<&Value>) -> Result<Option<Rc<RefCell<Value>>>, String> {
            match value {
                None | Some(Value::Nil) => Ok(None),
                Some(Value::Shared(cell) | Value::Mutable(cell, _)) if matches!(&*cell.borrow(), Value::Dict(_)) => Ok(Some(cell.clone())),
                _ => Err(String::from("TypeError: globals and locals must be dictionaries")),
            }
        }
        let global = dictionary(arguments.get(1))?.unwrap_or_else(|| self.book_about(true));
        let local = dictionary(arguments.get(2))?;
        let scanned = code.lexical_origin.clone();
        let beginning = self.idents.len();
        let prior: Vec<_> = (0..beginning).map(|index| format!("\0prior/{index}")).collect();
        let shadowed: Vec<_> = self.table.prims.keys().cloned().collect();
        let flags = code.compilation_entry.2;
        let mut built = crate::build::build_text(&scanned, self.table, &prior, code.compilation_entry.1,
            code.written_in.clone(), Some(Value::Shared(global.clone())), None, None,
            flags & 1 != 0, &shadowed, flags & 4 != 0, flags & 2 != 0,
            (flags >> 8) & 3, flags & !0x307).map_err(|(message, _, _)| message)?;
        let mut updated = built.program.with_literals(&code.literals)?;
        updated.locations = code.locations.clone();
        updated.encoded_handlers = code.encoded_handlers.clone();
        built.program = Rc::new(updated);
        let fresh = &built.globals[beginning..];
        self.idents.extend(fresh.iter().map(|word| format!("\0names/{beginning}/{word}")));
        self.outermost.cells.borrow_mut().resize(self.idents.len(), Value::Unset);
        let (near, outer) = match local { Some(local) => (local, Some(global)), None => (global, None) };
        let builtins = built.program.born.clone().unwrap_or_else(|| Value::Mutable(self.natives_kept(), true));
        self.readings.push(Namebook { builtins, near, outer, from: beginning, upto: beginning + fresh.len(), declared: built.outer_aliases.clone() });
        let saved = self.reading_now.replace(self.readings.len() - 1);
        let file = code.written_in.as_deref().unwrap_or("<string>");
        let result = self.text_concluded(&built, file, usize::from(flags & 1 != 0), false);
        self.reading_now = saved;
        self.outermost.cells.borrow_mut()[beginning..beginning + fresh.len()].fill(Value::Unset);
        result
    }

    fn load_code_image(&mut self, encoded: &Value) -> Result<Value, String> {
        let Value::Octets { cell, changeable: false, .. } = encoded.settled() else { return Err("TypeError: native code image must be bytes".to_owned()) };
        let data = cell.borrow();
        let payload = data.get(1..).filter(|_| !data.is_empty()).ok_or("ValueError: empty native code image")?;
        let parsed: serde_json::Value = serde_json::from_slice(payload).map_err(|_| "ValueError: invalid native code image")?;
        let pair = parsed.as_array().filter(|pair| pair.len() == 3).ok_or("ValueError: invalid native code image")?;
        let serialized = pair[0].as_str().ok_or("ValueError: invalid native code image")?;
        let boundary = pair[1].as_u64().and_then(|n| n.try_into().ok()).ok_or("ValueError: invalid native code boundary")?;
        let entry: (usize, u32, i64) = serde_json::from_value(pair[2].clone()).map_err(|_| "ValueError: invalid native code origin")?;
        let scanned = crate::scan::restore_tokens(serialized)?;
        let compiled = crate::build::build_from_image(&scanned, self.table, &self.idents, entry.1, entry.2)?;
        for name in &compiled.globals {
            if !self.idents.contains(name) { self.idents.push(name.clone()); }
        }
        self.outermost.cells.borrow_mut().resize(self.idents.len(), Value::Unset);
        let selected = find_image_routine(&compiled.program, boundary, &entry).ok_or("ValueError: native code boundary does not name a routine")?;
        let mut reconstructed = selected.as_ref().clone();
        reconstructed.encoded_body = Some(encoded.clone());
        Ok(Value::Wrapped(7, Rc::new(vec![Value::Routine(Rc::new(reconstructed))]).into()))
    }

    fn analyze_code_image(&self, value: &Value) -> Result<Value, String> {
        let program = image_body(value)?;
        let mut global = std::collections::BTreeSet::new();
        let mut attributes = std::collections::BTreeSet::new();
        let mut returns_none = true;
        fn walk(node: &Form, global: &mut std::collections::BTreeSet<String>, attributes: &mut std::collections::BTreeSet<String>, returns_none: &mut bool, builtins: &HashMap<String, Prim, crate::table::FxBuildHasher>, member: Option<&str>) {
            // Only the fallback's outer call implements the member lookup; its arguments remain ordinary expressions.
            if let Form::Apply(Callee::Prim(Prim::Choose, _), branches) = node {
                if let Some(Form::Apply(Callee::Prim(Prim::HasMember, _), operands)) = branches.first() {
                    if let Some(Form::Const(Value::Text(name))) = operands.get(1) {
                        for (index, branch) in branches.iter().enumerate() {
                            walk(branch, global, attributes, returns_none, builtins, if index == 2 { Some(name) } else { None });
                        }
                        return;
                    }
                }
            }
            if member.is_some() {
                if let Form::OnLine(_, _, inner) | Form::Located(_, _, inner) = node {
                    walk(inner, global, attributes, returns_none, builtins, member);
                    return;
                }
            }
            if let (Some(member), Form::Apply(Callee::Code(callee), arguments)) = (member, node) {
                if matches!(callee.as_ref(), Form::Read(address) if address.ident.as_ref() == member) {
                    for argument in arguments { walk(argument, global, attributes, returns_none, builtins, None); }
                    return;
                }
            }
            let named_address = |address: &Address| address.up > 0 && address.fallback.is_none() && !address.ident.starts_with('#');
            match node {
                Form::Read(address) | Form::Take(address) | Form::Glance(address) if named_address(address) => { global.insert(address.ident.to_string()); }
                Form::Const(Value::Intrinsic(_, spelling)) if builtins.contains_key(spelling.as_ref()) => { global.insert(spelling.to_string()); }
                Form::Const(Value::Routine(arm)) if arm.frameless => { walk(&arm.body, global, attributes, returns_none, builtins, member); return; },
                Form::Dyad { a, b, .. } => {
                    for input in [a, b] { if let Input::Address(address) = input { if named_address(address) { global.insert(address.ident.to_string()); } } }
                }
                Form::Apply(Callee::Prim(operation, spelling), arguments) => {
                    if *operation == Prim::Yield {
                        fn null(node: &Form) -> bool {
                            match node { Form::Const(Value::Nil) => true, Form::OnLine(_, _, inner) | Form::Located(_, _, inner) => null(inner), _ => false }
                        }
                        if arguments.first().is_some_and(|argument| !null(argument)) { *returns_none = false; }
                    } else if matches!(operation, Prim::Of | Prim::ReadMember | Prim::WriteMember | Prim::HasMember | Prim::Ask | Prim::Onto) {
                        if let Some(Form::Const(Value::Text(name))) = arguments.get(1) { attributes.insert(name.to_string()); }
                    } else if !matches!(operation, Prim::Seq | Prim::Choose | Prim::Both | Prim::Either | Prim::Leave | Prim::Resume | Prim::Raise) && builtins.contains_key(spelling.as_ref()) && member != Some(spelling.as_ref()) {
                        global.insert(spelling.to_string());
                    }
                }
                _ => {}
            }
            for child in image_children(node) { walk(child, global, attributes, returns_none, builtins, None); }
        }
        walk(&program.body, &mut global, &mut attributes, &mut returns_none, &self.table.prims, None);
        global.retain(|name| !program.locals.contains(name) && !program.cells.contains(name)
            && !program.reaching.iter().any(|address| address.ident.as_ref() == name));
        let strings = |names: std::collections::BTreeSet<String>| Value::tuple(names.iter().map(|name| Value::text(name)).collect());
        Ok(Value::tuple(vec![strings(global), strings(attributes), Value::Flag(returns_none)]))
    }

    pub(super) fn code_handler_image(&self, routine: &Routine) -> Vec<u8> {
        if let Some(Value::Octets { cell, .. }) = &routine.encoded_handlers { return cell.borrow().clone(); }
        fn extent(node: &Form) -> usize {
            let mut listing = Vec::new();
            node.instruction_listing(&mut 0, (0, u32::MAX, u32::MAX, u32::MAX), &mut listing);
            listing.len()
        }
        fn gather(node: &Form, position: &mut usize, handlers: &mut Vec<serde_json::Value>) {
            let start = *position;
            *position += 1;
            if let Form::Attempt { context, async_context, body, clauses, last, otherwise } = node {
                let arms: Vec<_> = clauses.iter().map(|clause| serde_json::json!([
                    clause.classes, clause.choices.as_ref().map(|choices| choices.iter().map(extent).collect::<Vec<_>>()),
                    clause.grouped, clause.takes_all, clause.held.is_some(), extent(&clause.body)
                ])).collect();
                handlers.push(serde_json::json!([start, context.is_some(), async_context, extent(body), arms,
                    last.as_deref().map(extent), otherwise.as_deref().map(extent)]));
            }
            if let Form::Const(Value::Routine(arm)) = node {
                if arm.frameless { gather(&arm.body, position, handlers); }
            }
            for sub in image_children(node) { gather(sub, position, handlers); }
        }
        let mut handlers = Vec::new();
        gather(&routine.body, &mut 0, &mut handlers);
        if handlers.is_empty() { Vec::new() } else { serde_json::json!(handlers).to_string().into_bytes() }
    }

    pub(super) fn native_stack_bound(&self, routine: &Routine) -> i64 {
        if let Some(bound) = routine.stack_bound { return bound; }
        fn retained(node: &Form) -> usize {
            match node {
                Form::Const(Value::Routine(body)) if body.frameless => retained(&body.body),
                Form::Apply(Callee::Prim(Prim::Seq | Prim::Choose | Prim::Both | Prim::Either, _), _) =>
                    image_children(node).into_iter().map(retained).max().unwrap_or(1),
                Form::Apply(callee, values) => {
                    let mut peak = 1;
                    for (saved, value) in values.iter().enumerate() {
                        peak = peak.max(saved.saturating_add(retained(value)));
                    }
                    if let Callee::Code(target) = callee { peak = peak.max(values.len().saturating_add(retained(target))); }
                    peak.max(values.len().saturating_add(1))
                }
                Form::Class { values, .. } | Form::SharePlace(_, values) => {
                    values.iter().enumerate().map(|(held, item)| held.saturating_add(retained(item)))
                        .max().unwrap_or(1).max(values.len())
                }
                Form::Dyad { a, b, .. } => {
                    let input = |part: &Input| match part { Input::Form(child) => retained(child), _ => 1 };
                    input(a).max(input(b).saturating_add(1))
                }
                Form::ForgetWithin(left, right) | Form::TieCalled(left, right) | Form::CallWrite(left, right) =>
                    retained(left).max(retained(right).saturating_add(1)),
                _ => image_children(node).into_iter().map(retained).max().unwrap_or(1),
            }
        }
        retained(&routine.body) as i64
    }
}
