// Operations upon text, admitted only by their words in the definition.
// Bounds count letters; byte places are used only after a bound is settled.

use std::rc::Rc;
use num_traits::ToPrimitive;
use crate::lang::Lang;
use crate::value::{Value, Wording};
use crate::unicode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextOp {
    Splitlines, Partition, Rpartition, Expandtabs, Swapcase, Casefold,
    Capitalize, Title, Istitle, Isidentifier, Isprintable, Isdecimal,
    Isnumeric, Isascii, Removeprefix, Removesuffix, FormatMap, Maketrans,
    Translate, Encode, Join, Split, Rsplit, Strip, Lstrip, Rstrip, Center,
    Ljust, Rjust, Zfill, Count, Find, Rfind, Index, Rindex, Startswith,
    Endswith, Replace, Upper, Lower, Length, Repr,
}

pub fn fault(lang: &Lang, key: &str) -> String {
    lang.text_words.get(&format!("ext.builtin.text.fault.{key}"))
        .and_then(|w| w.first()).cloned().unwrap_or_default()
}

pub fn already_named(lang: &Lang, said: &str) -> bool {
    lang.text_words.get("ext.builtin.text.complaint")
        .map_or(false, |words| words.iter().any(|w| said.starts_with(w)))
}

fn integer(v: &Value, lang: &Lang) -> Result<i64, String> {
    match v {
        Value::Small(n) => Ok(*n),
        Value::Flag(b) => Ok(i64::from(*b)),
        Value::Huge(n) => Ok(n.to_i64().unwrap_or(if n.sign() == num_bigint::Sign::Minus { i64::MIN } else { i64::MAX })),
        _ => Err(fault(lang, "integer")),
    }
}

fn text<'a>(v: &'a Value, lang: &Lang) -> Result<&'a str, String> {
    match v { Value::Text(s) => Ok(s), _ => Err(fault(lang, "string")) }
}

pub fn keywords(op: TextOp, args: &mut Vec<Value>, named: Vec<(String, Value)>, lang: &Lang) -> Result<(), String> {
    let initial = args.len();
    let mut used = Vec::new();
    for (key, value) in named {
        let fits = |tail: &str| lang.text_words.get(&format!("ext.builtin.text.keyword.{tail}"))
            .map_or(false, |words| words.contains(&key));
        let place = match op {
            TextOp::Splitlines if fits("keepends") => 1,
            TextOp::Expandtabs if fits("tabsize") => 1,
            TextOp::Split | TextOp::Rsplit if fits("sep") => 1,
            TextOp::Split | TextOp::Rsplit if fits("maxsplit") => 2,
            TextOp::Encode if fits("encoding") => 1,
            TextOp::Encode if fits("errors") => 2,
            _ => return Err(fault(lang, "arguments")),
        };
        if place < initial || used.contains(&place) { return Err(fault(lang, "arguments")); }
        used.push(place);
        while args.len() <= place { args.push(Value::Null); }
        args[place] = value;
    }
    Ok(())
}

fn white(c: char) -> bool { c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c) }

pub fn quoted(s: &str) -> String {
    let quote = if s.contains('\'') && !s.contains('"') { '"' } else { '\'' };
    let mut result = String::from(quote);
    for c in s.chars() {
        match c {
            '\t' => result.push_str("\\t"), '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"), '\\' => result.push_str("\\\\"),
            c if c == quote => { result.push('\\'); result.push(c); }
            c if unicode::bits(c) & 1 == 0 => {
                let n = c as u32;
                result.push_str(&if n <= 255 { format!("\\x{n:02x}") }
                    else if n <= 65535 { format!("\\u{n:04x}") } else { format!("\\U{n:08x}") });
            }
            c => result.push(c),
        }
    }
    result.push(quote);
    result
}

pub fn row(items: &[String], tuple: bool) -> String {
    let joined = items.iter().map(|s| quoted(s)).collect::<Vec<_>>().join(", ");
    if tuple { format!("({}{})", joined, if items.len() == 1 { "," } else { "" }) }
    else { format!("[{joined}]") }
}

fn repr(v: &Value, words: &Wording) -> String {
    match v {
        Value::Text(s) => quoted(s),
        Value::Words(items, tuple) => row(items, *tuple),
        Value::Array(items) => format!("[{}]", items.iter().map(|x| repr(x, words)).collect::<Vec<_>>().join(", ")),
        Value::Map(pairs) => format!("{{{}}}", pairs.iter().map(|(k,v)| format!("{}: {}", repr(k,words),repr(v,words))).collect::<Vec<_>>().join(", ")),
        _ => v.display(words),
    }
}

fn final_sigma(letters: &[char], at: usize) -> bool {
    let before = letters[..at].iter().rev().find(|c| unicode::bits(**c) & 32 == 0);
    let after = letters[at+1..].iter().find(|c| unicode::bits(**c) & 32 == 0);
    before.map_or(false, |c| unicode::bits(*c) & 16 != 0)
        && !after.map_or(false, |c| unicode::bits(*c) & 16 != 0)
}

fn recase(s: &str, op: TextOp) -> String {
    let letters: Vec<char> = s.chars().collect();
    let mut prior = false;
    let mut answer = String::new();
    for (at, c) in letters.iter().copied().enumerate() {
        let bits = unicode::bits(c);
        let kind = match op {
            TextOp::Casefold => 3, TextOp::Upper => 1, TextOp::Lower => 0,
            TextOp::Capitalize => if at == 0 { 2 } else { 0 },
            TextOp::Title => if prior { 0 } else { 2 },
            _ if bits & 128 != 0 => 1,
            _ if bits & 64 != 0 => 0,
            _ => { answer.push(c); prior = bits & 16 != 0; continue; }
        };
        if kind == 0 && c == 'Σ' && final_sigma(&letters, at) { answer.push('ς'); }
        else { answer.push_str(&unicode::change(c, kind)); }
        prior = bits & 16 != 0;
    }
    answer
}

fn translated_table(args: &[Value], lang: &Lang) -> Result<Value, String> {
    if args.len() == 1 {
        let Value::Map(pairs) = &args[0] else { return Err(fault(lang,"mapping")); };
        let mut table = Vec::new();
        for (key, value) in pairs.iter() {
            let key = match key {
                Value::Text(s) if s.chars().count() == 1 => Value::Small(s.chars().next().unwrap() as i64),
                Value::Text(_) => return Err(fault(lang,"maketrans.key")),
                Value::Small(_) | Value::Huge(_) | Value::Flag(_) => Value::Small(integer(key, lang)?),
                _ => return Err(fault(lang,"maketrans.type")),
            };
            if let Some(at) = table.iter().position(|(k, _): &(Value, Value)| k.equals(&key)) { table[at] = (key, value.clone()); }
            else { table.push((key, value.clone())); }
        }
        return Ok(Value::Map(Rc::new(table)));
    }
    if !(2..=3).contains(&args.len()) { return Err(fault(lang,"arguments")); }
    let from = text(&args[0],lang)?;
    let to = text(&args[1],lang)?;
    if from.chars().count() != to.chars().count() { return Err(fault(lang,"maketrans.length")); }
    let mut pairs = Vec::<(Value,Value)>::new();
    for (c, value) in from.chars().zip(to.chars().map(|c| Value::Small(c as i64)))
        .chain(if args.len() == 3 { text(&args[2],lang)? } else { "" }.chars().map(|c| (c, Value::Null))) {
        let key = Value::Small(c as i64);
        if let Some(at) = pairs.iter().position(|(k,_)| k.equals(&key)) { pairs[at].1 = value; }
        else { pairs.push((key,value)); }
    }
    Ok(Value::Map(Rc::new(pairs)))
}

pub fn run(op: TextOp, name: &str, args: &[Value], lang: &Lang, words: &Wording) -> Result<Value, String> {
    use TextOp::*;
    if op == Maketrans {
        let skip = usize::from(!name.contains('.') && matches!(args.first(), Some(Value::Text(_))) && args.len() > 2);
        return translated_table(&args[skip..],lang);
    }
    if op == Repr {
        if args.len() != 1 { return Err(fault(lang,"arguments")); }
        return Ok(Value::text(&repr(&args[0],words)));
    }
    let Some(Value::Text(source)) = args.first() else { return Err(fault(lang,"receiver")); };
    let params = &args[1..];
    let (least,most) = match op {
        Splitlines | Expandtabs | Split | Rsplit | Strip | Lstrip | Rstrip | Encode => (0, if matches!(op, Split | Rsplit | Encode) { 2 } else { 1 }),
        Partition | Rpartition | Removeprefix | Removesuffix | FormatMap | Translate | Join | Zfill => (1,1),
        Center | Ljust | Rjust => (1,2),
        Count | Find | Rfind | Index | Rindex | Startswith | Endswith => (1,3),
        Replace => (2,3), _ => (0,0),
    };
    if params.len() < least || params.len() > most { return Err(fault(lang,"arguments")); }
    let s: &str = source;
    let number = |at: usize, default: i64| -> Result<i64,String> {
        params.get(at).map_or(Ok(default), |v| integer(v,lang))
    };
    let result = match op {
        Encode => return Err(fault(lang,"encode")),
        Length => Value::Small(s.chars().count() as i64),
        Upper | Lower | Swapcase | Casefold | Capitalize | Title => Value::text(&recase(s,op)),
        Isascii => Value::Flag(s.is_ascii()),
        Isprintable => Value::Flag(s.chars().all(|c| unicode::bits(c) & 1 != 0)),
        Isdecimal | Isnumeric => Value::Flag(!s.is_empty() && s.chars().all(|c| unicode::bits(c) & if op == Isdecimal { 2 } else { 512 } != 0)),
        Isidentifier => {
            let mut chars = s.chars();
            Value::Flag(chars.next().map_or(false, |c| c == '_' || unicode::bits(c)&4 != 0)
                && chars.all(|c| unicode::bits(c)&8 != 0))
        }
        Istitle => {
            let mut prior = false; let mut seen = false; let mut good = true;
            for c in s.chars() {
                let b = unicode::bits(c);
                if b & (64|256) != 0 { if prior { good = false; } prior=true; seen=true; }
                else if b & 128 != 0 { if !prior { good=false; } prior=true; seen=true; }
                else { prior=false; }
            }
            Value::Flag(good && seen)
        }
        Splitlines => {
            let keep = params.first().map(|v| integer(v,lang)).transpose()?.unwrap_or(0) != 0;
            let mut parts = Vec::new(); let mut start=0; let mut chars=s.char_indices().peekable();
            while let Some((at,c))=chars.next() {
                if matches!(c,'\n'|'\r'|'\u{b}'|'\u{c}'|'\u{1c}'|'\u{1d}'|'\u{1e}'|'\u{85}'|'\u{2028}'|'\u{2029}') {
                    let mut end=at+c.len_utf8();
                    if c=='\r' && chars.peek().map_or(false, |(_,c)| *c=='\n') { chars.next(); end+=1; }
                    parts.push(s[start..if keep {end} else {at}].to_string()); start=end;
                }
            }
            if start<s.len() { parts.push(s[start..].to_string()); }
            Value::Words(Rc::new(parts),false)
        }
        Partition | Rpartition => {
            let sep=text(&params[0],lang)?;
            if sep.is_empty() { return Err(fault(lang,"separator")); }
            let at=if op==Partition {s.find(sep)} else {s.rfind(sep)};
            let parts=match at {Some(at)=>vec![s[..at].to_string(),sep.to_string(),s[at+sep.len()..].to_string()],
                None if op==Partition=>vec![s.to_string(),String::new(),String::new()],
                None=>vec![String::new(),String::new(),s.to_string()]};
            Value::Words(Rc::new(parts),true)
        }
        Expandtabs => {
            let size=number(0,8)?.max(0) as usize;
            let mut column=0usize; let mut out=String::new();
            for c in s.chars() {
                if c=='\t' {
                    let pad=if size==0 {0} else {size-column%size};
                    out.try_reserve(pad).map_err(|_| fault(lang,"room"))?;
                    out.extend(std::iter::repeat(' ').take(pad)); column+=pad;
                } else {out.push(c); if c=='\r'||c=='\n' {column=0;} else {column+=1;} }
            }
            Value::text(&out)
        }
        Removeprefix | Removesuffix => {
            let affix=text(&params[0],lang)?;
            Value::text(if op==Removeprefix {s.strip_prefix(affix)} else {s.strip_suffix(affix)}.unwrap_or(s))
        }
        Strip | Lstrip | Rstrip => {
            let chars=match params.first() {None|Some(Value::Null)=>None,Some(v)=>Some(text(v,lang)?)};
            let trim=|c: char| chars.map_or_else(|| white(c), |set| set.contains(c));
            Value::text(match op {Strip=>s.trim_matches(trim),Lstrip=>s.trim_start_matches(trim),_=>s.trim_end_matches(trim)})
        }
        Split | Rsplit => {
            let limit=number(1,-1)?; let cap=if limit<0 {usize::MAX} else {limit as usize};
            let sep=match params.first() {None|Some(Value::Null)=>None,Some(v)=>Some(text(v,lang)?)};
            let backward=op==Rsplit; let mut parts=Vec::new();
            if let Some(sep)=sep {
                if sep.is_empty() {return Err(fault(lang,"separator"));}
                if backward { parts.extend(s.rsplitn(cap.saturating_add(1),sep).map(str::to_string));parts.reverse(); }
                else {parts.extend(s.splitn(cap.saturating_add(1),sep).map(str::to_string));}
            } else {
                let mut rest=if backward {s.trim_end_matches(white)} else {s.trim_start_matches(white)};
                while !rest.is_empty() {
                    if parts.len()==cap {parts.push(rest.to_string());break;}
                    let at=if backward {rest.rfind(white)} else {rest.find(white)};
                    match at {
                        None=>{parts.push(rest.to_string());break;}
                        Some(i) if backward=>{let width=rest[i..].chars().next().unwrap().len_utf8();parts.push(rest[i+width..].to_string());rest=rest[..i].trim_end_matches(white);}
                        Some(i)=>{parts.push(rest[..i].to_string());rest=rest[i..].trim_start_matches(white);}
                    }
                }
                if backward {parts.reverse();}
            }
            Value::Words(Rc::new(parts),false)
        }
        Center | Ljust | Rjust | Zfill => {
            let width=number(0,0)?.max(0) as usize;
            let fill=if op==Zfill {'0'} else {match params.get(1) {None=>' ',Some(v)=>{let f=text(v,lang)?;if f.chars().count()!=1 {return Err(fault(lang,"fill"));} f.chars().next().unwrap()}}};
            let extra=width.saturating_sub(s.chars().count());
            let left=match op {Ljust=>0,Center=>extra/2+(extra&width&1),_=>extra};
            let mut out=String::new();
            out.try_reserve(extra.checked_mul(fill.len_utf8()).and_then(|n| n.checked_add(s.len())).ok_or_else(||fault(lang,"room"))?).map_err(|_|fault(lang,"room"))?;
            let sign=op==Zfill && s.starts_with(['+','-']);
            if sign {out.push(s.chars().next().unwrap());}
            out.extend(std::iter::repeat(fill).take(left));out.push_str(if sign {&s[1..]} else {s});out.extend(std::iter::repeat(fill).take(extra-left));
            Value::text(&out)
        }
        Count | Find | Rfind | Index | Rindex | Startswith | Endswith => {
            let chars: Vec<usize>=s.char_indices().map(|(i,_)|i).chain(std::iter::once(s.len())).collect();
            let length=chars.len()-1;
            let adjust=|n:i64| if n<0 {(length as i64).saturating_add(n).max(0) as usize} else {n as usize};
            let start=adjust(match params.get(1) {Some(Value::Null)|None=>0,Some(v)=>integer(v,lang)?});
            let stop=adjust(match params.get(2) {Some(Value::Null)|None=>length as i64,Some(v)=>integer(v,lang)?}).min(length);
            let valid=start<=stop && start<=length;
            let window=if valid {&s[chars[start]..chars[stop]]} else {""};
            if op==Startswith || op==Endswith {
                let choices=match &params[0] {Value::Array(v)=>v.as_ref().clone(),Value::Words(v,true)=>v.iter().map(|s|Value::text(s)).collect(),v=>vec![v.clone()]};
                let mut yes=false;
                for v in choices {let wanted=text(&v,lang)?;if valid && if op==Startswith {window.starts_with(wanted)} else {window.ends_with(wanted)} {yes=true;break;}}
                Value::Flag(yes)
            } else {
                let needle=text(&params[0],lang)?;
                if op==Count {Value::Small(if !valid {0} else if needle.is_empty() {(stop-start+1) as i64} else {window.matches(needle).count() as i64})}
                else {
                    let found=if !valid {None} else if op==Rfind || op==Rindex {window.rfind(needle)} else {window.find(needle)};
                    let n=found.map(|i| (start+window[..i].chars().count()) as i64);
                    if n.is_none() && (op==Index||op==Rindex) {return Err(fault(lang,"missing"));}
                    Value::Small(n.unwrap_or(-1))
                }
            }
        }
        Replace => {
            let old=text(&params[0],lang)?;let new=text(&params[1],lang)?;let count=number(2,-1)?;
            Value::text(&s.replacen(old,new,if count<0 {usize::MAX} else {count as usize}))
        }
        Translate => {
            let Value::Map(pairs)=&params[0] else {return Err(fault(lang,"mapping"));};
            let mut out=String::new();
            for c in s.chars() {
                match pairs.iter().find(|(key,_)|key.equals(&Value::Small(c as i64))).map(|(_,v)|v) {
                    None=>out.push(c),Some(Value::Null)=>{},Some(Value::Text(t))=>out.push_str(t),
                    Some(v @ (Value::Small(_)|Value::Huge(_)|Value::Flag(_)))=>{let n=integer(v,lang)?;if !(0..0x110000).contains(&n) {return Err(fault(lang,"codepoint"));} out.push(char::from_u32(n as u32).ok_or_else(||fault(lang,"surrogate"))?);}
                    _=>return Err(fault(lang,"translation")),
                }
            }
            Value::text(&out)
        }
        Join => {
            let items=match &params[0] {
                Value::Array(items)=>items.as_ref().clone(),Value::Words(items,_)=>items.iter().map(|s|Value::text(s)).collect(),
                Value::Map(pairs)=>pairs.iter().map(|(k,_)|k.clone()).collect(),Value::Text(t)=>t.chars().map(|c|Value::text(&c.to_string())).collect(),
                _=>return Err(fault(lang,"walk")),
            };
            let mut out=String::new();for (i,v) in items.iter().enumerate() {if i>0 {out.push_str(s);}let Value::Text(t)=v else {return Err(fault(lang,"join"));};out.push_str(t);}
            Value::text(&out)
        }
        FormatMap => Value::text(&mapping_format(s,&params[0],lang,words,0)?),
        Maketrans | Repr => unreachable!(),
    };
    Ok(result)
}

fn mapping_format(s: &str, mapping: &Value, lang: &Lang, words: &Wording, depth: usize) -> Result<String,String> {
    if depth>2 {return Err(fault(lang,"format"));}
    let Value::Map(entries)=mapping else {return Err(fault(lang,"format"));};
    let mut out=String::new();let mut chars=s.chars().peekable();
    while let Some(c)=chars.next() {
        if (c=='{'||c=='}') && chars.peek()==Some(&c) {chars.next();out.push(c);continue;}
        if c=='}' {return Err(fault(lang,"format.brace"));}
        if c!='{' {out.push(c);continue;}
        let mut field=String::new();let mut nested=0;let mut closed=false;
        for c in chars.by_ref() {if c=='}' && nested==0 {closed=true;break;}if c=='{' {nested+=1;}if c=='}' {nested-=1;}field.push(c);}
        if !closed {return Err(fault(lang,"format.brace"));}
        let (head,spec)=field.split_once(':').unwrap_or((&field,""));let (key,conversion)=head.split_once('!').unwrap_or((head,""));
        if key.is_empty() || key.chars().next().unwrap().is_ascii_digit() {return Err(fault(lang,"format.positional"));}
        if key.contains(['.','[']) {return Err(fault(lang,"format"));}
        let value=entries.iter().find(|(k,_)|matches!(k,Value::Text(s) if s.as_ref()==key)).map(|(_,v)|v).ok_or_else(||fault(lang,"key")+&quoted(key))?;
        let spec=mapping_format(spec,mapping,lang,words,depth+1)?;
        let shown=if conversion=="r" || conversion=="a" {let mut shown=repr(value,words);if conversion=="a" {shown=shown.chars().map(|c|if c.is_ascii(){c.to_string()}else if c as u32<=65535 {format!("\\u{:04x}",c as u32)}else{format!("\\U{:08x}",c as u32)}).collect();}Value::text(&shown).string_field(words,&spec,"")}
            else {value.string_field(words,&spec,conversion)};
        out.push_str(&shown.ok_or_else(||fault(lang,"format"))?);
    }
    Ok(out)
}
