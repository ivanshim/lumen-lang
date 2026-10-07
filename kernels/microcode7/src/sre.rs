// CPython 3b564385e4c9 Modules/_sre provides these instruction rules; PSF License.
use crate::data::Value;
use num_traits::ToPrimitive;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

type CachedSubject = (Rc<str>, Rc<Vec<u32>>);
thread_local! {
    static RECENT_INPUTS: RefCell<VecDeque<CachedSubject>> = const { RefCell::new(VecDeque::new()) };
}
fn characters_for_regex(value: &Value) -> Result<Rc<Vec<u32>>, String> {
    match value.settled() {
        Value::Text(source) => RECENT_INPUTS.with(|cache| {
            let mut rows = cache.borrow_mut();
            if let Some(index) = rows.iter().position(|row| Rc::ptr_eq(&row.0, &source)) {
                let item = rows.remove(index).unwrap();
                let numbers = item.1.clone();
                rows.push_front(item);
                return Ok(numbers);
            }
            let numbers: Rc<Vec<u32>> = Rc::new(source.chars().map(|letter| letter as u32).collect());
            rows.push_front((source, numbers.clone()));
            rows.truncate(2);
            Ok(numbers)
        }),
        other => other.character_numbers().map(Rc::new).ok_or_else(|| String::from("TypeError: SRE input must be text")),
    }
}

fn folded(value: u32, narrow: bool) -> u32 {
    match (narrow, value) {
        (true, 65..=90) => value + 32,
        (true, _) => value,
        (false, _) => char::from_u32(value).and_then(|ch| ch.to_lowercase().next()).map_or(value, |ch| ch as u32),
    }
}
fn has_property(value: u32, mask: u16) -> bool { char::from_u32(value).is_some_and(|ch| crate::unicode::property(ch, mask)) }
fn is_word(value: u32, unicode: bool) -> bool {
    if value == 95 { return true; }
    if unicode { alphabetic(value) || has_property(value, 512) }
    else { matches!(value, 48..=57 | 65..=90 | 97..=122) }
}
fn in_category(tag: u32, value: u32) -> bool {
    let result = match tag / 2 {
        0 => (48..=57).contains(&value), 1 => matches!(value, 9..=13 | 32),
        2 | 4 => is_word(value, false), 3 => value == 10,
        5 => has_property(value, 2), 6 => white(value),
        7 => is_word(value, true), 8 => matches!(value, 10..=13 | 28..=30 | 133 | 8232 | 8233),
        9 => alphabetic(value), 10 => has_property(value, 128),
        11 => has_property(value, 64), 12 => has_property(value, 512),
        13 => has_property(value, 1), 14 => alphabetic(value) || has_property(value, 512),
        15 => has_property(value, 4), 16 => has_property(value, 8),
        17 => has_property(value, 256), 18 => has_property(value, 16),
        19 => has_property(value, 32),
        20 => has_property(value, 64) && alphabetic(value),
        21 => has_property(value, 512) && !alphabetic(value),
        22 => alphabetic(value) && has_property(value, 32),
        23 => in_category(42, value) && has_property(value, 4),
        24 => in_category(42, value) && !has_property(value, 2 | 4),
        25 => has_property(value, 32) && !has_property(value, 1),
        26 => white(value) && !control(value),
        27 => in_category(52, value) && !matches!(value, 8232 | 8233),
        28 => !has_property(value, 1) && !in_category(52, value),
        29 => in_category(56, value) && !control(value) && !surrogate(value) && !private(value) && !has_property(value, 32),
        30 => !in_category(58, value),
        31 => value == 9 || in_category(54, value),
        32 => !white(value) && !control(value) && !surrogate(value) && !in_category(58, value),
        33 => !control(value) && (in_category(64, value) || in_category(62, value)),
        _ => false,
    };
    if tag % 2 == 0 { result } else { !result }
}
#[derive(Clone)]
struct Cycle { entry: usize, exit: usize, low: usize, high: usize, taken: usize, position: Option<usize> }
#[derive(Clone)]
struct Cursor { instruction: usize, offset: usize, captures: Vec<i64>, recent: i64, cycles: Vec<Cycle> }
struct Regex<'a> { program: &'a [u32], subject: &'a [u32], limit: usize }
impl Regex<'_> {
    fn viable(&self, offset: usize) -> bool {
        let words = self.program;
        let header = words.get(..5);
        let Some(header) = header.filter(|entry| entry[0] == 14) else { return true };
        let after = header[1] as usize + 1;
        if after < 5 || after >= words.len() { return true; }
        let required = header[3] as usize;
        if required > self.limit.saturating_sub(offset) { return false; }
        if required == 0 { return true; }
        match header[2] {
            bits if bits & 1 != 0 && after >= 7 => {
                let count = words[5] as usize;
                let Some(prefix) = words.get(7..7usize.saturating_add(count)).filter(|_| count <= after - 7) else { return true };
                prefix.iter().enumerate().all(|(index, number)| self.subject.get(offset.saturating_add(index)) == Some(number))
            }
            bits if bits & 4 != 0 => self.subject.get(offset).is_some_and(|&number| self.contains(5, number, number)),
            _ => true,
        }
    }
    fn instruction(&self, at: usize) -> u32 { self.program.get(at).copied().unwrap_or(0) }
    fn boundary(&self, tag: u32, at: usize) -> bool {
        if tag < 3 { return at == 0 || tag == 1 && self.subject.get(at.wrapping_sub(1)) == Some(&10); }
        if (5..=7).contains(&tag) {
            return at == self.limit || tag != 7 && self.subject.get(at) == Some(&10) && (tag == 6 || at+1 == self.limit);
        }
        let unicode = tag == 10 || tag == 11;
        let left = at > 0 && self.subject.get(at-1).is_some_and(|&n| is_word(n, unicode));
        let right = at < self.limit && is_word(self.subject[at], unicode);
        (left != right) ^ matches!(tag, 4 | 9 | 11)
    }
    fn contains(&self, mut here: usize, value: u32, companion: u32) -> bool {
        let mut polarity = true;
        let mut found = false;
        while here < self.program.len() {
            let operation = self.instruction(here); here += 1;
            match operation {
                0 => return !polarity,
                21 => { polarity ^= true; }
                8 => { found |= in_category(self.instruction(here), value); here += 1; }
                16 => { found |= value == self.instruction(here) || companion == self.instruction(here); here += 1; }
                22 | 42 => {
                    let a = self.instruction(here); let z = self.instruction(here+1); here += 2;
                    found |= (a..=z).contains(&value) || (a..=z).contains(&companion);
                    if operation == 42 {
                        let capital = char::from_u32(value).and_then(|ch| ch.to_uppercase().next());
                        found |= capital.is_some_and(|ch| (a..=z).contains(&(ch as u32)));
                    }
                }
                9 => { for ch in [value, companion] { if ch < 256 { found |= self.instruction(here+ch as usize/32) & (1u32 << (ch%32)) != 0; } } here += 8; }
                10 => {
                    let count = self.instruction(here) as usize; here += 1;
                    if value <= 65535 {
                        let index = self.instruction(here+(value as usize/256)/4) >> (((value/256)%4)*8) & 255;
                        let bitmap = here+64+index as usize*8+(value as usize%256)/32;
                        found |= self.instruction(bitmap) & (1u32 << (value%32)) != 0;
                    }
                    here += 64+count*8;
                }
                _ => return false,
            }
            if found { return polarity; }
        }
        false
    }
    fn follow(&self, start: Cursor, terminal: Option<usize>, entire: bool, forbidden: Option<usize>, nesting: usize) -> Result<Option<Cursor>, String> {
        if nesting > 512 { return Err(String::from("RecursionError: regular expression assertion nesting exceeded")); }
        let mut alternatives = vec![start];
        while let Some(mut current) = alternatives.pop() {
            loop {
                let ip = current.instruction;
                if terminal == Some(ip) { return Ok(Some(current)); }
                if ip >= self.program.len() { return Err(String::from("RuntimeError: invalid SRE code")); }
                let tag = self.instruction(ip);
                let operand = self.instruction(ip+1) as usize;
                match tag {
                    0 => break,
                    1 => {
                        if forbidden != Some(current.offset) && (!entire || current.offset == self.limit) { return Ok(Some(current)); }
                        break;
                    }
                    14 | 15 => current.instruction = ip + operand + 1,
                    17 => {
                        let Some(capture) = current.captures.get_mut(operand) else { return Err(String::from("RuntimeError: invalid SRE code")); };
                        *capture = current.offset as i64;
                        if operand & 1 != 0 { current.recent = (operand/2+1) as i64; }
                        current.instruction += 2;
                    }
                    6 => { if !self.boundary(operand as u32, current.offset) { break; } current.instruction += 2; }
                    7 => {
                        let mut arm = ip+1; let mut branches = Vec::new();
                        while self.instruction(arm) != 0 {
                            let mut choice = current.clone(); choice.instruction = arm+1; branches.push(choice);
                            arm += self.instruction(arm) as usize;
                        }
                        branches.reverse();
                        let Some(first) = branches.pop() else { break };
                        alternatives.extend(branches); current = first;
                    }
                    4 | 5 | 27 => {
                        let lookback = if tag == 27 { 0 } else { self.instruction(ip+2) as usize };
                        let trial = if current.offset < lookback { None } else {
                            let mut probe = current.clone(); probe.offset -= lookback; probe.instruction += if tag == 27 { 2 } else { 3 };
                            self.follow(probe, None, false, None, nesting+1)?
                        };
                        if tag == 5 { if trial.is_some() { break; } }
                        else {
                            let Some(matched) = trial else { break };
                            current.captures = matched.captures; current.recent = matched.recent;
                            if tag == 27 { current.offset = matched.offset; }
                        }
                        current.instruction = ip+operand+1;
                    }
                    12 => {
                        let group = operand*2;
                        let present = current.captures.get(group).zip(current.captures.get(group+1)).is_some_and(|(&a,&b)| a >= 0 && a <= b);
                        current.instruction = ip + if present { 3 } else { self.instruction(ip+2) as usize+1 };
                    }
                    11 | 30 | 34 | 38 => {
                        let group = operand*2;
                        let Some((&from,&until)) = current.captures.get(group).zip(current.captures.get(group+1)) else { break };
                        if from < 0 || until < from { break; }
                        let width = (until-from) as usize;
                        if current.offset+width > self.limit { break; }
                        let a = &self.subject[from as usize..until as usize];
                        let b = &self.subject[current.offset..current.offset+width];
                        let equal = if tag == 11 { a == b } else { a.iter().zip(b).all(|(&x,&y)| folded(x, tag != 38) == folded(y, tag != 38)) };
                        if !equal { break; }
                        current.offset += width; current.instruction += 2;
                    }
                    23 => {
                        let cycle = Cycle { entry: ip+4, exit: ip+operand+1, low: self.instruction(ip+2) as usize,
                            high: self.instruction(ip+3) as usize, taken: 0, position: None };
                        current.instruction = cycle.exit; current.cycles.push(cycle);
                    }
                    18 | 19 => {
                        let Some(mut cycle) = current.cycles.pop() else { return Err(String::from("RuntimeError: invalid SRE repeat")); };
                        let satisfied = cycle.taken >= cycle.low;
                        let another = cycle.taken < cycle.high && (cycle.position != Some(current.offset) || !satisfied);
                        let mut leave = current.clone(); leave.instruction += 1;
                        cycle.taken += 1; cycle.position = Some(current.offset);
                        let mut again = current.clone(); again.instruction = cycle.entry; again.cycles.push(cycle);
                        match (satisfied, another, tag) {
                            (false, false, _) => break,
                            (false, true, _) => current = again,
                            (true, false, _) => current = leave,
                            (true, true, 18) => { alternatives.push(leave); current = again; }
                            _ => { alternatives.push(again); current = leave; }
                        }
                    }
                    26 => {
                        let low = self.instruction(ip + 2) as usize;
                        let high = self.instruction(ip + 3) as usize;
                        let mut trial = current.clone();
                        let mut taken = 0;
                        while taken <= high {
                            trial.instruction = ip + operand + 1;
                            if taken >= low {
                                let attempt = self.follow(trial.clone(), terminal, entire, forbidden, nesting + 1)?;
                                if attempt.is_some() { return Ok(attempt); }
                            }
                            if taken >= high { break; }
                            let previous = trial.offset;
                            trial.instruction = ip + 4;
                            match self.follow(trial, None, false, None, nesting + 1)? {
                                Some(next) if next.offset != previous => trial = next,
                                _ => break,
                            }
                            taken += 1;
                        }
                        break;
                    }
                    24 | 28 | 29 => {
                        let least = self.instruction(ip+2) as usize;
                        let most = self.instruction(ip+3) as usize;
                        let boundary = ip+operand+1;
                        let body = ip+4;
                        let possessive = tag >= 28;
                        let mut choices = vec![current.clone()];
                        let mut count = 0;
                        while count < most {
                            let mut step = current.clone(); step.instruction = body;
                            let Some(next) = self.follow(step, if tag == 28 { Some(boundary) } else { None }, false, None, nesting+1)? else { break };
                            let empty = next.offset == current.offset;
                            current = next; count += 1;
                            if !possessive { choices.push(current.clone()); }
                            if empty && count >= least { break; }
                        }
                        if count < least { break; }
                        if possessive { current.instruction = boundary + usize::from(tag == 28); }
                        else {
                            choices.drain(..least);
                            for option in &mut choices { option.instruction = boundary; }
                            current = choices.pop().unwrap(); alternatives.extend(choices);
                        }
                    }
                    2 | 3 | 8 | 13 | 16 | 20 | 31..=33 | 35..=37 | 39..=41 => {
                        if current.offset >= self.limit { break; }
                        let mut value = self.subject[current.offset];
                        if tag >= 31 { value = folded(value, tag < 39); }
                        let allowed = match tag {
                            2 => value != 10, 3 => true,
                            8 => in_category(operand as u32, value),
                            16 | 32 | 40 => value == operand as u32,
                            36 => value == folded(operand as u32, true),
                            20 | 33 | 41 => value != operand as u32,
                            37 => value != folded(operand as u32, true),
                            35 => self.contains(ip+2, value, match value { 97..=122 => value-32, _ => value }),
                            _ => self.contains(ip+2, value, value),
                        };
                        if !allowed { break; }
                        current.offset += 1;
                        current.instruction += match tag { 2 | 3 => 1, 13 | 31 | 35 | 39 => operand+1, _ => 2 };
                    }
                    _ => return Err(format!("RuntimeError: unsupported SRE opcode {tag}")),
                }
            }
        }
        Ok(None)
    }
}
pub fn invoke(input: &[Value]) -> Result<Value, String> {
    fn integer(v: &Value) -> Result<i64, String> {
        match v {
            Value::Small(whole) => Ok(*whole),
            Value::Flag(truth) => Ok(if *truth { 1 } else { 0 }),
            other => other.as_big()?.to_i64().ok_or_else(|| String::from("OverflowError: regular expression code size limit exceeded")),
        }
    }
    let action = input.first().map(integer).transpose()?.unwrap_or(-1);
    if action == 10 {
        let [_, text, snapshot, count, directory, selection] = input else {
            return Err(String::from("TypeError: invalid SRE capture argument count"));
        };
        let Value::Tuple(choices) = selection.settled() else { return Err(String::from("TypeError: SRE groups must be a tuple")); };
        for choice in choices.iter() {
            match choice {
                Value::Small(_) | Value::Huge(_) | Value::Flag(_) | Value::Text(_) => (),
                _ => return Ok(Value::Nil),
            }
        }
        let Value::Vector(saved) = snapshot.settled() else { return Err(String::from("TypeError: invalid SRE match state")); };
        if saved.get(2).is_none() { return Err(String::from("TypeError: invalid SRE match state")); }
        let Value::Vector(offsets) = saved[2].settled() else { return Err(String::from("TypeError: invalid SRE capture marks")); };
        let Value::Dict(labels) = directory.settled() else { return Err(String::from("TypeError: invalid SRE group names")); };
        let characters = characters_for_regex(text)?;
        let maximum = integer(count)?;
        if usize::try_from(maximum).map_or(true, |size| size > offsets.len() / 2) {
            return Err(String::from("TypeError: invalid SRE capture marks"));
        }
        let mut values = Vec::new();
        let mut cursor = 0;
        while cursor < choices.len() {
            let chosen = &choices[cursor];
            let number = match chosen {
                Value::Text(word) => {
                    let position = labels.pairs().iter().position(|pair| matches!(&pair.0, Value::Text(key) if key == word))
                        .ok_or_else(|| String::from("IndexError: no such group"))?;
                    integer(&labels.pairs()[position].1)
                }
                other => integer(other),
            }.map_err(|_| String::from("IndexError: no such group"))?;
            if !(0..=maximum).contains(&number) { return Err(String::from("IndexError: no such group")); }
            let range = if number == 0 { (integer(&saved[0])?, integer(&saved[1])?) }
                else { let at = number as usize * 2; (integer(&offsets[at-2])?, integer(&offsets[at-1])?) };
            values.push(if range.0 == -1 || range.1 < range.0 { Value::Nil }
                else {
                    let letters = characters.get(range.0 as usize..range.1 as usize)
                        .ok_or_else(|| String::from("TypeError: invalid SRE capture position"))?;
                    Value::characters(letters.to_vec())
                });
            cursor += 1;
        }
        let answer = match values.len() { 1 => values.remove(0), _ => Value::tuple(values) };
        return Ok(Value::tuple(vec![answer]));
    }
    if action == 9 {
        let [_, source, first, last] = input else {
            return Err(String::from("TypeError: invalid SRE bounds argument count"));
        };
        let maximum = characters_for_regex(source)?.len() as i64;
        let position = |number: &Value| -> Result<Value, String> {
            let whole = integer(number).map_err(|_| String::from("OverflowError: Python int too large to convert to C ssize_t"))?;
            Ok(Value::Small(whole.max(0).min(maximum)))
        };
        return Ok(Value::tuple(vec![position(first)?, position(last)?]));
    }
    if action == 6 { return Ok(Value::text(character_category(integer(&input[1])? as u32))); }
    if action == 5 {
        return match input.get(1) {
            Some(Value::Text(label)) => crate::unicode::name_value(label).map(|word| Value::text(&word)).ok_or_else(|| String::from("KeyError: undefined character name")),
            _ => Err(String::from("TypeError: argument must be str")),
        };
    }
    if action == 11 {
        let supplied = input.get(1).ok_or_else(|| String::from("TypeError: normalize() argument 2 must be str"))?;
        let codes = supplied.type_text().character_numbers().ok_or_else(|| {
            let kind = match supplied.settled() { Value::Nil => "None".to_string(), other => other.kind_word() };
            format!("TypeError: normalize() argument 2 must be str, not {kind}")
        })?;
        // A stowed half is passed through as it stands; only the run
        // of real characters around it is folded.
        let mut folded: Vec<u32> = Vec::with_capacity(codes.len());
        let mut plain = String::new();
        for number in codes {
            if let Some(letter) = char::from_u32(number) { plain.push(letter); continue; }
            folded.extend(crate::unicode::collated(&plain).chars().map(|c| c as u32));
            plain.clear();
            folded.push(number);
        }
        folded.extend(crate::unicode::collated(&plain).chars().map(|c| c as u32));
        return Ok(Value::characters(folded));
    }
    if action == 12 {
        let point = u32::try_from(integer(&input[1])?).map_err(|_| String::from("OverflowError: Python int too large to convert to C unsigned long"))?;
        return Ok(Value::text(&crate::unicode::decomposition(point)));
    }
    if !matches!(action, 0 | 7 | 8) {
        let value = u32::try_from(integer(&input[1])?).map_err(|_| String::from("OverflowError: Python int too large to convert to C unsigned long"))?;
        return match action {
            1 | 2 => Ok(Value::Small(folded(value, action == 1) as i64)),
            3 => Ok(Value::Flag(matches!(value, 65..=90 | 97..=122))),
            _ => Ok(Value::Flag(has_property(value, 16))),
        };
    }
    let required = if action == 7 { 6 } else { 8 };
    if input.len() != required { return Err(String::from("TypeError: invalid SRE matcher argument count")); }
    let Value::Vector(raw) = &input[1] else { return Err(String::from("TypeError: SRE code must be a list")); };
    let program: Vec<u32> = raw.iter().map(|v| integer(v).and_then(|n| u32::try_from(n).map_err(|_| String::from("OverflowError: regular expression code size limit exceeded")))).collect::<Result<_, _>>()?;
    let subject = characters_for_regex(&input[2])?;
    let begin = integer(&input[3])?.max(0) as usize;
    let limit = (integer(&input[4])?.max(0) as usize).min(subject.len());
    let groups = integer(&input[5])?.max(0) as usize;
    let mode = if action == 0 { integer(&input[6])? } else { 2 };
    let skip_empty = action == 0 && integer(&input[7])? != 0;
    if begin > limit { return Ok(if action == 7 { vector(Vec::new()) } else { Value::Nil }); }
    let regex = Regex { program: &program, subject: &subject, limit };
    if matches!(action, 7 | 8) {
        let permitted = if action == 8 { integer(&input[6])? } else { 0 };
        let inserted = if action == 8 { input[7].character_numbers().ok_or_else(|| String::from("TypeError: replacement must be text"))? } else { vec![] };
        let mut output = Vec::new();
        let mut until = begin;
        let mut replacements = 0;
        let mut rows = Vec::new();
        let mut scan = begin;
        let mut omit = false;
        loop {
            if permitted != 0 && replacements >= permitted { break; }
            let origin = scan;
            let mut matched = None;
            while scan <= limit {
                if !regex.viable(scan) { scan += 1; continue; }
                let state = Cursor { instruction: 0, offset: scan, captures: vec![-1; groups * 2], recent: -1, cycles: vec![] };
                let avoid = (omit && scan == origin).then_some(origin);
                if let Some(result) = regex.follow(state, None, false, avoid, 0)? {
                    matched = Some(result);
                    break;
                }
                scan += 1;
            }
            let Some(result) = matched else { break };
            let slice = |bounds: &[i64]| {
                if bounds[0] == -1 || bounds[1] < bounds[0] { Value::text("") }
                else { Value::characters(subject[bounds[0] as usize..bounds[1] as usize].to_vec()) }
            };
            if action == 8 {
                output.extend_from_slice(&subject[until..scan]);
                output.extend_from_slice(&inserted);
                until = result.offset;
                replacements += 1;
            } else {
            let item = if groups == 0 {
                Value::characters(subject[scan..result.offset].to_vec())
            } else if groups == 1 { slice(&result.captures[..2]) }
            else { Value::Tuple(crate::tuples::Sequence::plain(result.captures.chunks_exact(2).map(slice).collect())) };
            rows.push(item);
            }
            omit = result.offset == scan;
            scan = result.offset;
        }
        if action == 8 {
            output.extend_from_slice(&subject[until..limit]);
            return Ok(Value::tuple(vec![Value::characters(output), Value::Small(replacements)]));
        }
        return Ok(vector(rows));
    }
    let mut position = begin;
    while position <= limit {
        // Anchored operations enter the matcher without search's INFO filter.
        if mode == 2 && !regex.viable(position) {
            position += 1;
            continue;
        }
        let cursor = Cursor { instruction: 0, offset: position, captures: vec![-1; groups*2], recent: -1, cycles: vec![] };
        let forbid = if skip_empty && position == begin { Some(begin) } else { None };
        if let Some(m) = regex.follow(cursor, None, mode == 1, forbid, 0)? {
            return Ok(vector(vec![Value::Small(position as i64), Value::Small(m.offset as i64),
                vector(m.captures.into_iter().map(Value::Small).collect()), Value::Small(m.recent)]));
        }
        if mode != 2 { break; }
        position += 1;
    }
    Ok(Value::Nil)
}

fn vector(row: Vec<Value>) -> Value { Value::Vector(crate::tuples::Sequence::plain(row)) }

fn control(n: u32) -> bool { n < 32 || (127..160).contains(&n) }
fn surrogate(n: u32) -> bool { (55296..57344).contains(&n) }
fn private(n: u32) -> bool { matches!(n, 57344..=63743 | 983040..=1048573 | 1048576..=1114109) }

fn character_category(number: u32) -> &'static str {
    static CATEGORY: std::sync::OnceLock<Vec<(std::ops::RangeInclusive<u32>, &'static str)>> = std::sync::OnceLock::new();
    let entries = CATEGORY.get_or_init(|| {
        let mut rows = Vec::new();
        let mut opening = None;
        for record in include_str!("../../../unicode-data/UnicodeData.txt").split('\n') {
            let pieces: Vec<_> = record.split(';').take(3).collect();
            if pieces.len() != 3 { continue; }
            let Ok(codepoint) = u32::from_str_radix(pieces[0], 16) else { continue };
            if pieces[1].ends_with(", First>") { opening = Some(codepoint); continue; }
            let start = opening.take().unwrap_or(codepoint);
            rows.push((start..=codepoint, pieces[2]));
        }
        rows
    });
    let found = entries.binary_search_by(|(range, _)| {
        if number < *range.start() { std::cmp::Ordering::Greater }
        else if number > *range.end() { std::cmp::Ordering::Less }
        else { std::cmp::Ordering::Equal }
    });
    found.map_or("Cn", |position| entries[position].1)
}

fn alphabetic(n: u32) -> bool { matches!(character_category(n), "Lu" | "Ll" | "Lt" | "Lm" | "Lo") }
fn white(n: u32) -> bool { matches!(n, 28..=31) || char::from_u32(n).is_some_and(char::is_whitespace) }

// Return the database mapping for one Unicode character, including surrogate halves.
pub fn decompose(args: &[Value]) -> Result<Value, String> {
    let [source] = args else { return Err(format!("TypeError: decomposition() takes exactly one argument ({} given)", args.len())); };
    let invalid = || {
        let kind = match source.settled() { Value::Nil => "None".to_owned(), other => other.kind_word() };
        format!("TypeError: decomposition() argument must be a unicode character, not {kind}")
    };
    let point = match source.type_text() {
        Value::Text(text) => {
            let mut chars = text.chars();
            match (chars.next(), chars.next()) {
                (Some(point), None) => point as u32,
                _ => return Err(invalid()),
            }
        }
        Value::Unpaired(points) if points.len() == 1 => points[0],
        Value::Unpaired(_) => return Err(invalid()),
        _ => return Err(invalid()),
    };
    Ok(Value::text(&crate::unicode::decomposition(point)))
}
