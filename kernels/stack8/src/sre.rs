// Bytecode semantics from CPython 3b564385e4c9 Modules/_sre/{sre.c,sre_lib.h}; PSF License.
use crate::value::Value;
use num_traits::ToPrimitive;
use std::rc::Rc;

#[derive(Clone)]
struct Repeat { body: usize, tail: usize, min: usize, max: usize, count: usize, before: usize }
#[derive(Clone)]
struct State { pc: usize, pos: usize, marks: Vec<i64>, last: i64, loops: Vec<Repeat> }
struct Machine<'a> { code: &'a [u32], text: &'a [u32], end: usize }

fn lower(n: u32, ascii: bool) -> u32 {
    if ascii { return if (65..=90).contains(&n) { n + 32 } else { n }; }
    char::from_u32(n).and_then(|c| c.to_lowercase().next()).map_or(n, u32::from)
}
fn word(n: u32, wide: bool) -> bool {
    n == 95 || if wide { char::from_u32(n).is_some_and(|c| crate::unicode::bits(c) & 512 != 0 || general_category(n).starts_with('L')) }
    else { n < 128 && char::from_u32(n).is_some_and(|c| c.is_ascii_alphanumeric()) }
}
fn category(kind: u32, n: u32) -> bool {
    let c = char::from_u32(n);
    let b = c.map_or(0, crate::unicode::bits);
    let alpha = general_category(n).starts_with('L');
    let space = (28..=31).contains(&n) || c.is_some_and(char::is_whitespace);
    let yes = match kind & !1 {
        0 => (48..=57).contains(&n), 2 => [9,10,11,12,13,32].contains(&n),
        4 | 8 => word(n, false), 6 => n == 10,
        10 => b & 2 != 0, 12 => space, 14 => word(n, true),
        16 => [10,13,11,12,28,29,30,133,0x2028,0x2029].contains(&n),
        18 => alpha, 20 => b & 128 != 0, 22 => b & 64 != 0,
        24 => b & 512 != 0, 26 => b & 1 != 0,
        28 => alpha || b & 512 != 0, 30 => b & 4 != 0, 32 => b & 8 != 0,
        34 => b & 256 != 0,
        36 => b & 16 != 0, 38 => b & 32 != 0,
        40 => b & 64 != 0 && alpha,
        42 => b & 512 != 0 && !alpha,
        44 => alpha && b & 32 != 0,
        46 => b & 512 != 0 && !alpha && b & 4 != 0,
        48 => b & 512 != 0 && !alpha && b & (2|4) == 0,
        50 => b & 32 != 0 && b & 1 == 0,
        52 => space && !(n <= 31 || (127..=159).contains(&n)),
        54 => category(52, n) && n != 8232 && n != 8233,
        56 => b & 1 == 0 && !category(52, n),
        58 => category(56, n) && !(n <= 31 || (127..=159).contains(&n) || (0xd800..=0xdfff).contains(&n)
            || (0xe000..=0xf8ff).contains(&n) || (0xf0000..=0xffffd).contains(&n) || (0x100000..=0x10fffd).contains(&n)) && b & 32 == 0,
        60 => !category(58, n),
        62 => category(54, n) || n == 9,
        64 => !space && !(n <= 31 || (127..=159).contains(&n) || (0xd800..=0xdfff).contains(&n)) && !category(58, n),
        66 => (category(64, n) || category(62, n)) && !(n <= 31 || (127..=159).contains(&n)),
        _ => false,
    };
    yes != (kind & 1 != 0)
}
impl Machine<'_> {
    fn at(&self, kind: u32, p: usize) -> bool {
        let preceding = p.checked_sub(1).and_then(|i| self.text.get(i)).copied();
        let following = if p < self.end { self.text.get(p).copied() } else { None };
        match kind {
            0 | 2 => p == 0, 1 => p == 0 || preceding == Some(10),
            5 => p == self.end || p + 1 == self.end && following == Some(10),
            6 => p == self.end || following == Some(10), 7 => p == self.end,
            3 | 4 | 8 | 9 | 10 | 11 => {
                let wide = kind >= 10;
                let different = preceding.is_some_and(|v| word(v, wide)) != following.is_some_and(|v| word(v, wide));
                different != matches!(kind, 4 | 9 | 11)
            }
            _ => false,
        }
    }
    fn charset(&self, mut pc: usize, n: u32, other: u32) -> bool {
        let mut answer = false;
        let mut positive = true;
        while pc < self.code.len() {
            let op = self.code[pc]; pc += 1;
            match op {
                0 => return !positive,
                21 => { positive = !positive; }
                16 => { answer |= self.code.get(pc).is_some_and(|&v| v == n || v == other); pc += 1; }
                22 | 42 => {
                    if let (Some(&a), Some(&z)) = (self.code.get(pc), self.code.get(pc+1)) {
                        answer |= a <= n && n <= z || a <= other && other <= z;
                        if op == 42 { if let Some(up) = char::from_u32(n).and_then(|c| c.to_uppercase().next()) { answer |= a <= up as u32 && up as u32 <= z; } }
                    }
                    pc += 2;
                }
                8 => { answer |= self.code.get(pc).is_some_and(|&k| category(k, n)); pc += 1; }
                9 => { answer |= [n, other].into_iter().any(|ch| ch < 256 && self.code.get(pc + ch as usize / 32).is_some_and(|v| v & (1 << (ch % 32)) != 0)); pc += 8; }
                10 => {
                    let Some(&blocks) = self.code.get(pc) else { return false }; pc += 1;
                    if n < 65536 {
                        if let Some(&packed) = self.code.get(pc + (n as usize >> 8) / 4) {
                            let block = (packed >> (((n >> 8) % 4) * 8)) & 255;
                            answer |= self.code.get(pc + 64 + block as usize * 8 + (n as usize & 255) / 32).is_some_and(|v| v & (1 << (n % 32)) != 0);
                        }
                    }
                    pc += 64 + blocks as usize * 8;
                }
                _ => return false,
            }
            if answer { return positive; }
        }
        false
    }
    fn run(&self, mut s: State, stop: Option<usize>, full: bool, advance: Option<usize>, depth: usize) -> Result<Option<State>, String> {
        if depth > 512 { return Err("RecursionError: regular expression assertion nesting exceeded".into()); }
        let mut pending = Vec::new();
        'backtrack: loop {
            loop {
                if stop == Some(s.pc) { return Ok(Some(s)); }
                let Some(&op) = self.code.get(s.pc) else { return Err("RuntimeError: invalid SRE code".into()) };
                let arg = self.code.get(s.pc+1).copied().unwrap_or(0) as usize;
                let n = self.text.get(s.pos).copied().filter(|_| s.pos < self.end);
                match op {
                    0 => break,
                    1 => { if (!full || s.pos == self.end) && advance != Some(s.pos) { return Ok(Some(s)); } break; }
                    14 | 15 => s.pc += 1 + arg,
                    17 => {
                        if arg >= s.marks.len() { return Err("RuntimeError: invalid SRE code".into()); }
                        s.marks[arg] = s.pos as i64;
                        if arg % 2 == 1 { s.last = (arg / 2 + 1) as i64; }
                        s.pc += 2;
                    }
                    6 => { if !self.at(arg as u32, s.pos) { break; } s.pc += 2; }
                    7 => {
                        let mut branch = s.pc + 1;
                        let mut choices = Vec::new();
                        while let Some(&skip) = self.code.get(branch) {
                            if skip == 0 { break; }
                            let mut candidate = s.clone(); candidate.pc = branch + 1; choices.push(candidate);
                            branch += skip as usize;
                        }
                        let Some(first) = choices.first().cloned() else { break };
                        pending.extend(choices.into_iter().skip(1).rev()); s = first;
                    }
                    4 | 5 => {
                        let back = self.code.get(s.pc+2).copied().unwrap_or(0) as usize;
                        let tested = if s.pos >= back {
                            let mut inner = s.clone(); inner.pc += 3; inner.pos -= back;
                            self.run(inner, None, false, None, depth+1)?
                        } else { None };
                        if op == 4 {
                            let Some(found) = tested else { break }; s.marks = found.marks; s.last = found.last;
                        } else if tested.is_some() { break; }
                        s.pc += arg + 1;
                    }
                    27 => {
                        let mut inner = s.clone(); inner.pc += 2;
                        let Some(found) = self.run(inner, None, false, None, depth+1)? else { break };
                        s.pos = found.pos; s.marks = found.marks; s.last = found.last; s.pc += arg + 1;
                    }
                    12 => {
                        let captured = s.marks.get(arg*2).zip(s.marks.get(arg*2+1)).is_some_and(|(&a,&b)| a >= 0 && b >= a);
                        s.pc += if captured { 3 } else { 1 + self.code.get(s.pc+2).copied().unwrap_or(0) as usize };
                    }
                    11 | 30 | 34 | 38 => {
                        let Some((&a,&b)) = s.marks.get(arg*2).zip(s.marks.get(arg*2+1)) else { break };
                        if a < 0 || b < a || s.pos + (b-a) as usize > self.end { break; }
                        let same = (a as usize..b as usize).zip(s.pos..).all(|(i,j)| {
                            if op == 11 { self.text[i] == self.text[j] }
                            else { lower(self.text[i], op != 38) == lower(self.text[j], op != 38) }
                        });
                        if !same { break; } s.pos += (b-a) as usize; s.pc += 2;
                    }
                    24 | 26 | 29 => {
                        let min = self.code.get(s.pc+2).copied().unwrap_or(0) as usize;
                        let max = self.code.get(s.pc+3).copied().unwrap_or(0) as usize;
                        let mut ends = vec![s.clone()];
                        let mut inner = s.clone(); inner.pc += 4;
                        while ends.len()-1 < max {
                            let Some(mut next) = self.run(inner.clone(), None, false, None, depth+1)? else { break };
                            if next.pos == inner.pos { break; }
                            next.pc = inner.pc; inner = next.clone(); ends.push(next);
                        }
                        if ends.len()-1 < min { break; }
                        let tail = s.pc + 1 + arg;
                        let mut choices: Vec<State> = ends.into_iter().skip(min).map(|mut v| { v.pc = tail; v }).collect();
                        if op != 26 { choices.reverse(); }
                        let first = choices.remove(0);
                        if op != 29 { pending.extend(choices.into_iter().rev()); }
                        s = first;
                    }
                    28 => {
                        let min = self.code.get(s.pc+2).copied().unwrap_or(0) as usize;
                        let max = self.code.get(s.pc+3).copied().unwrap_or(0) as usize;
                        let boundary = s.pc + 1 + arg;
                        let body = s.pc + 4;
                        let mut count = 0;
                        while count < max {
                            let mut inner = s.clone(); inner.pc = body;
                            let Some(found) = self.run(inner, Some(boundary), false, None, depth+1)? else { break };
                            count += 1;
                            let unmoved = found.pos == s.pos;
                            s.pos = found.pos; s.marks = found.marks; s.last = found.last;
                            if unmoved && count >= min { break; }
                        }
                        if count < min { break; } s.pc = boundary + 1;
                    }
                    23 => {
                        let rep = Repeat { body: s.pc+4, tail: s.pc+1+arg,
                            min: self.code.get(s.pc+2).copied().unwrap_or(0) as usize,
                            max: self.code.get(s.pc+3).copied().unwrap_or(0) as usize,
                            count: 0, before: usize::MAX };
                        s.pc = rep.tail; s.loops.push(rep);
                    }
                    18 | 19 => {
                        let Some(mut r) = s.loops.pop() else { return Err("RuntimeError: invalid SRE repeat".into()); };
                        let grow = r.count < r.max && (r.before != s.pos || r.count < r.min);
                        let enough = r.count >= r.min;
                        let mut tail = s.clone(); tail.pc += 1;
                        r.count += 1; r.before = s.pos;
                        let mut more = s.clone(); more.pc = r.body; more.loops.push(r);
                        if !enough { if !grow { break; } s = more; }
                        else if !grow { s = tail; }
                        else if op == 18 { pending.push(tail); s = more; }
                        else { pending.push(more); s = tail; }
                    }
                    2 | 3 | 8 | 13 | 16 | 20 | 31 | 32 | 33 | 35 | 36 | 37 | 39 | 40 | 41 => {
                        let Some(mut ch) = n else { break };
                        if matches!(op,31..=33 | 35..=37 | 39..=41) { ch = lower(ch, op < 39); }
                        let accepted = match op {
                            2 => ch != 10, 3 => true, 8 => category(arg as u32, ch),
                            16 | 32 | 40 => ch == arg as u32,
                            36 => ch == lower(arg as u32, true),
                            20 | 33 | 41 => ch != arg as u32,
                            37 => ch != lower(arg as u32, true),
                            35 => self.charset(s.pc+2, ch, if (97..=122).contains(&ch) { ch-32 } else { ch }),
                            _ => self.charset(s.pc+2, ch, ch),
                        };
                        if !accepted { break; }
                        s.pos += 1;
                        s.pc += if matches!(op,13 | 31 | 35 | 39) { arg+1 } else if op == 2 || op == 3 { 1 } else { 2 };
                    }
                    _ => return Err(format!("RuntimeError: unsupported SRE opcode {op}")),
                }
            }
            if let Some(next) = pending.pop() { s = next; continue 'backtrack; }
            return Ok(None);
        }
    }
}

pub fn call(args: &[Value]) -> Result<Value, String> {
    let number = |i: usize| -> Result<i64, String> {
        args.get(i).and_then(|v| match v {
            Value::Small(n) => Some(*n), Value::Flag(flag) => Some(i64::from(*flag)),
            _ => v.as_big().ok().and_then(|n| n.to_i64()),
        }).ok_or_else(|| "TypeError: SRE argument must be an integer".into())
    };
    let op = number(0)?;
    if op == 6 {
        return Ok(Value::text(general_category(number(1)? as u32)));
    }
    if op == 5 {
        let Some(Value::Text(name)) = args.get(1) else { return Err("TypeError: argument must be str".into()); };
        return crate::unicode::named_text(name).map(|text| Value::text(&text)).ok_or_else(|| "KeyError: undefined character name".into());
    }
    if !matches!(op, 0 | 7 | 8) {
        let n = u32::try_from(number(1)?).map_err(|_| "OverflowError: Python int too large to convert to C unsigned long".to_string())?;
        return Ok(if op == 1 || op == 2 { Value::Small(lower(n, op == 1) as i64) }
        else { Value::Flag(char::from_u32(n).is_some_and(|c| if op == 3 { c.is_ascii_alphabetic() } else { crate::unicode::bits(c) & 16 != 0 })) });
    }
    if args.len() != if op == 7 { 6 } else { 8 } { return Err("TypeError: invalid SRE matcher argument count".into()); }
    let Value::Array(items) = args[1].contents() else { return Err("TypeError: SRE code must be a list".into()) };
    let mut code = Vec::with_capacity(items.len());
    for item in items.iter() {
        let word = match item {
            Value::Small(n) => *n, Value::Flag(flag) => i64::from(*flag),
            _ => item.as_big()?.to_i64().ok_or("OverflowError: regular expression code size limit exceeded")?,
        };
        code.push(u32::try_from(word).map_err(|_| "OverflowError: regular expression code size limit exceeded".to_string())?);
    }
    let text = args[2].contents().text_codes().ok_or_else(|| "TypeError: SRE input must be text".to_string())?;
    let start = number(3)?.max(0) as usize;
    let end = (number(4)?.max(0) as usize).min(text.len());
    let groups = number(5)?.max(0) as usize;
    let mode = if op != 0 { 2 } else { number(6)? };
    let advance = op == 0 && number(7)? != 0;
    if start > end { return Ok(if op == 7 { Value::array(Vec::new()) } else { Value::Null }); }
    let machine = Machine { code: &code, text: &text, end };
    if op == 7 || op == 8 {
        let limit = if op == 8 { number(6)? } else { 0 };
        let replacement = if op == 8 { args[7].contents().text_codes().ok_or_else(|| "TypeError: replacement must be text".to_string())? } else { Vec::new() };
        let mut substituted = Vec::new();
        let mut copied = start;
        let mut substitutions = 0;
        let mut matches = Vec::new();
        let mut next = start;
        let mut must_advance = false;
        while next <= end && (limit == 0 || substitutions < limit) {
            let mut found = None;
            for at in next..=end {
                let state = State { pc: 0, pos: at, marks: vec![-1; groups * 2], last: -1, loops: Vec::new() };
                if let Some(done) = machine.run(state, None, false, if must_advance && at == next { Some(next) } else { None }, 0)? {
                    found = Some((at, done));
                    break;
                }
            }
            let Some((at, done)) = found else { break };
            let capture = |begin: i64, finish: i64| {
                if begin < 0 || finish < begin { Value::text("") }
                else { Value::from_codes(text[begin as usize..finish as usize].to_vec()) }
            };
            if op == 8 {
                substituted.extend_from_slice(&text[copied..at]);
                substituted.extend_from_slice(&replacement);
                copied = done.pos;
                substitutions += 1;
            } else { matches.push(match groups {
                0 => capture(at as i64, done.pos as i64),
                1 => capture(done.marks[0], done.marks[1]),
                _ => Value::Tuple(Rc::new(done.marks.chunks_exact(2).map(|pair| capture(pair[0], pair[1])).collect::<Vec<_>>()).into()),
            }); }
            next = done.pos;
            must_advance = next == at;
        }
        if op == 8 {
            substituted.extend_from_slice(&text[copied..end]);
            return Ok(Value::Tuple(Rc::new(vec![Value::from_codes(substituted), Value::Small(substitutions)]).into()));
        }
        return Ok(Value::array(matches));
    }
    for p in start..=if mode == 2 { end } else { start } {
        let initial = State { pc: 0, pos: p, marks: vec![-1; groups*2], last: -1, loops: Vec::new() };
        if let Some(result) = machine.run(initial, None, mode == 1, if advance && p == start { Some(start) } else { None }, 0)? {
            let marks = Value::array(result.marks.into_iter().map(Value::Small).collect());
            return Ok(Value::array(vec![Value::Small(p as i64), Value::Small(result.pos as i64), marks, Value::Small(result.last)]));
        }
    }
    Ok(Value::Null)
}

fn general_category(n: u32) -> &'static str {
    use std::sync::OnceLock;
    static RANGES: OnceLock<Vec<(u32, u32, &'static str)>> = OnceLock::new();
    let ranges = RANGES.get_or_init(|| {
        let mut result = Vec::new();
        let mut first = 0;
        for line in include_str!("../../../unicode-data/UnicodeData.txt").lines() {
            let mut fields = line.split(';');
            let Some(cp) = fields.next().and_then(|v| u32::from_str_radix(v, 16).ok()) else { continue };
            let name = fields.next().unwrap_or("");
            let gc = fields.next().unwrap_or("Cn");
            if name.ends_with(", First>") { first = cp; }
            else if name.ends_with(", Last>") { result.push((first, cp, gc)); }
            else { result.push((cp, cp, gc)); }
        }
        result
    });
    let at = ranges.partition_point(|row| row.1 < n);
    ranges.get(at).filter(|row| row.0 <= n).map_or("Cn", |row| row.2)
}
