// Text primitives take a subject and a row of arguments. The table lends
// them their names and their complaints; no word chooses a language here.

use crate::data::{Names, Value};
use crate::table::Table;
use crate::unicode::{altered, property};
use std::rc::Rc;
use num_traits::ToPrimitive;

#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
pub enum Work {
    SPLITLINES, PARTITION, RPARTITION, EXPANDTABS, SWAPCASE, CASEFOLD,
    CAPITALIZE, TITLE, ISTITLE, ISIDENTIFIER, ISPRINTABLE, ISDECIMAL,
    ISNUMERIC, ISASCII, REMOVEPREFIX, REMOVESUFFIX, FORMATMAP, MAKETRANS,
    TRANSLATE, ENCODE, JOIN, SPLIT, RSPLIT, STRIP, LSTRIP, RSTRIP,
    CENTER, LJUST, RJUST, ZFILL, COUNT, FIND, RFIND, INDEX, RINDEX,
    STARTSWITH, ENDSWITH, REPLACE, UPPER, LOWER, LENGTH, REPR,
}

pub fn complaint(table: &Table, reason: &str) -> String {
    table.single(&format!("ext.builtin.text.fault.{}", reason)).unwrap_or_default().to_owned()
}

pub fn bears_kind(table: &Table, message: &str) -> bool {
    table.strings("ext.builtin.text.complaint").iter().any(|prefix| message.starts_with(prefix))
}

fn count(value: &Value, table: &Table) -> Result<i64, String> {
    if let Value::Flag(flag) = value { return Ok(if *flag { 1 } else { 0 }); }
    if let Value::Small(n) = value { return Ok(*n); }
    if let Value::Huge(n) = value {
        return Ok(n.to_i64().unwrap_or_else(|| if **n < num_bigint::BigInt::from(0) { i64::MIN } else { i64::MAX }));
    }
    Err(complaint(table, "integer"))
}

fn letters<'s>(value: &'s Value, table: &Table) -> Result<&'s str, String> {
    if let Value::Text(word) = value { Ok(word) } else { Err(complaint(table, "string")) }
}

pub fn fit_names(table: &Table, work: Work, values: &mut Vec<Value>, named: Vec<(String, Value)>) -> Result<(), String> {
    let filled = values.len();
    let mut assigned = std::collections::HashSet::new();
    for (name, value) in named {
        let candidates: &[(usize, &str)] = match work {
            Work::ENCODE => &[(1,"encoding"),(2,"errors")],
            Work::SPLIT | Work::RSPLIT => &[(1,"sep"),(2,"maxsplit")],
            Work::EXPANDTABS => &[(1,"tabsize")],
            Work::SPLITLINES => &[(1,"keepends")],
            _ => &[],
        };
        let chosen = candidates.iter().find(|(_, tail)| table.spells(&format!("ext.builtin.text.keyword.{}",tail), &name));
        let Some(&(position,_)) = chosen else { return Err(complaint(table,"arguments")); };
        if position < filled || !assigned.insert(position) { return Err(complaint(table,"arguments")); }
        values.resize(values.len().max(position+1),Value::Nil);
        values[position]=value;
    }
    Ok(())
}

pub fn quotation(word: &str) -> String {
    let mark=if word.contains('\'') && !word.contains('"') {'"'} else {'\''};
    let mut pieces=vec![mark.to_string()];
    for letter in word.chars() {
        let number=letter as u32;
        pieces.push(match letter {
            '\\'=>String::from("\\\\"), '\r'=>String::from("\\r"),
            '\n'=>String::from("\\n"), '\t'=>String::from("\\t"),
            _ if letter==mark=>format!("\\{}",letter),
            _ if !property(letter,1)=> match number {
                0..=255=>format!("\\x{:02x}",number),
                256..=65535=>format!("\\u{:04x}",number),
                _=>format!("\\U{:08x}",number),
            },
            _=>letter.to_string(),
        });
    }
    pieces.push(mark.to_string());
    pieces.concat()
}

pub fn written_row(values: &[String], fixed: bool) -> String {
    let mut out=String::from(if fixed {"("} else {"["});
    for (i,value) in values.iter().enumerate() {
        if i>0 {out.push_str(", ");}
        out.push_str(&quotation(value));
    }
    if fixed && values.len()==1 {out.push(',');}
    out.push(if fixed {')'} else {']'});
    out
}

fn expression(value: &Value, words: Names) -> String {
    match value {
        Value::Text(word)=>quotation(word),
        Value::TextRow(row,closed)=>written_row(row,*closed),
        Value::Vector(row)=>{
            let contents: Vec<_>=row.iter().map(|v|expression(v,words)).collect();
            format!("[{}]",contents.join(", "))
        }
        Value::Dict(pairs)=>{
            let contents: Vec<_>=pairs.iter().map(|(key,item)|format!("{}: {}",expression(key,words),expression(item,words))).collect();
            format!("{{{}}}",contents.join(", "))
        }
        _=>value.render(words),
    }
}

fn blank(letter: char) -> bool { matches!(letter,'\u{1c}'..='\u{1f}') || letter.is_whitespace() }

fn case_changed(word: &str, work: Work) -> String {
    let characters: Vec<_>=word.chars().collect();
    characters.iter().enumerate().map(|(i,&letter)| {
        let lower=match work {
            Work::LOWER=>true,
            Work::CAPITALIZE=>i!=0,
            Work::TITLE=>i>0 && property(characters[i-1],16),
            Work::SWAPCASE=>property(letter,64),
            _=>false,
        };
        if lower {
            if letter=='Σ' {
                let before=characters[..i].iter().rev().copied().find(|c|!property(*c,32));
                let after=characters[i+1..].iter().copied().find(|c|!property(*c,32));
                if before.map_or(false,|c|property(c,16)) && !after.map_or(false,|c|property(c,16)) {return String::from('ς');}
            }
            return altered(letter,0);
        }
        let shape=match work {
            Work::CAPITALIZE|Work::TITLE=>2,
            Work::CASEFOLD=>3,
            Work::SWAPCASE if !property(letter,128)=>return letter.to_string(),
            _=>1,
        };
        altered(letter,shape)
    }).collect()
}

struct Given<'a> { tail: &'a [Value], table: &'a Table }
impl Given<'_> {
    fn whole(&self, place: usize, otherwise: i64) -> Result<i64,String> {
        self.tail.get(place).map(|v|count(v,self.table)).unwrap_or(Ok(otherwise))
    }
    fn word(&self, place: usize) -> Result<&str,String> { letters(&self.tail[place],self.table) }
    fn bad(&self, reason: &str) -> String { complaint(self.table,reason) }
}

fn make_table(input: &[Value], table: &Table) -> Result<Value,String> {
    let mut entries=Vec::<(Value,Value)>::new();
    let mut insert=|key: Value,value: Value| {
        entries.retain(|(k,_)|!k.equals(&key)); entries.push((key,value));
    };
    match input {
        [Value::Dict(mapping)]=>{
            for (key,replacement) in mapping.iter() {
                let number=match key {
                    Value::Text(word)=>{
                        let mut it=word.chars(); let a=it.next();
                        if a.is_none() || it.next().is_some() {return Err(complaint(table,"maketrans.key"));}
                        a.unwrap() as i64
                    }
                    Value::Small(_)|Value::Huge(_)|Value::Flag(_)=>count(key,table)?,
                    _=>return Err(complaint(table,"maketrans.type")),
                };
                insert(Value::Small(number),replacement.clone());
            }
        }
        [_,_] | [_,_,_]=>{
            let before=letters(&input[0],table)?;
            let after=letters(&input[1],table)?;
            if before.chars().count()!=after.chars().count() {return Err(complaint(table,"maketrans.length"));}
            for (a,b) in before.chars().zip(after.chars()) {insert(Value::Small(a as i64),Value::Small(b as i64));}
            if input.len()==3 {for c in letters(&input[2],table)?.chars() {insert(Value::Small(c as i64),Value::Nil);}}
        }
        [_]=>return Err(complaint(table,"mapping")),
        _=>return Err(complaint(table,"arguments")),
    }
    Ok(Value::Dict(Rc::new(entries)))
}

pub fn apply(table: &Table, work: Work, name: &str, input: &[Value], names: Names) -> Result<Value,String> {
    use Work::*;
    if work==MAKETRANS {
        let input=if !name.contains('.') && input.len()>2 && matches!(input[0],Value::Text(_)) {&input[1..]} else {input};
        return make_table(input,table);
    }
    if work==REPR {
        return if input.len()==1 {Ok(Value::text(&expression(&input[0],names)))} else {Err(complaint(table,"arguments"))};
    }
    let Some(Value::Text(subject))=input.first() else {return Err(complaint(table,"receiver"));};
    let g=Given{tail:&input[1..],table};
    let allowed=match work {
        SPLIT|RSPLIT|ENCODE=>0..=2,
        SPLITLINES|EXPANDTABS|STRIP|LSTRIP|RSTRIP=>0..=1,
        CENTER|LJUST|RJUST=>1..=2,
        REPLACE=>2..=3,
        COUNT|FIND|RFIND|INDEX|RINDEX|STARTSWITH|ENDSWITH=>1..=3,
        PARTITION|RPARTITION|REMOVEPREFIX|REMOVESUFFIX|FORMATMAP|TRANSLATE|JOIN|ZFILL=>1..=1,
        _=>0..=0,
    };
    if !allowed.contains(&g.tail.len()) {return Err(g.bad("arguments"));}
    let source=subject.as_ref();
    let many=source.chars().count();
    let answer=match work {
        ENCODE=>return Err(g.bad("encode")),
        REPR|MAKETRANS=>unreachable!(),
        LENGTH=>Value::Small(many as i64),
        LOWER|UPPER|CAPITALIZE|TITLE|CASEFOLD|SWAPCASE=>Value::text(&case_changed(source,work)),
        ISASCII=>Value::Flag(source.is_ascii()),
        ISPRINTABLE=>Value::Flag(source.chars().all(|c|property(c,1))),
        ISDECIMAL=>Value::Flag(many>0 && source.chars().all(|c|property(c,2))),
        ISNUMERIC=>Value::Flag(many>0 && source.chars().all(|c|property(c,512))),
        ISIDENTIFIER=>Value::Flag(many>0 && source.chars().enumerate().all(|(i,c)|if i==0 {c=='_'||property(c,4)} else {property(c,8)})),
        ISTITLE=>{
            let mut previous=false;
            let mut has_case=false;
            let right=source.chars().all(|c|{
                let upper=property(c,64|256); let lower=property(c,128);
                let fits=if upper {!previous} else if lower {previous} else {true};
                previous=upper||lower;has_case|=previous;fits
            });
            Value::Flag(right && has_case)
        }
        SPLITLINES=>{
            let ends=g.whole(0,0)?!=0;
            let chars: Vec<_>=source.char_indices().collect();
            let mut cursor=0;let mut begin=0;let mut lines=Vec::new();
            while cursor<chars.len() {
                let (position,letter)=chars[cursor]; cursor+=1;
                if !matches!(letter,'\r'|'\n'|'\u{85}'|'\u{2028}'|'\u{2029}'|'\u{b}'|'\u{c}'|'\u{1c}'|'\u{1d}'|'\u{1e}') {continue;}
                if letter=='\r' && chars.get(cursor).map_or(false,|(_,c)|*c=='\n') {cursor+=1;}
                let following=chars.get(cursor).map_or(source.len(),|(i,_)|*i);
                lines.push(source[begin..if ends {following} else {position}].to_owned());begin=following;
            }
            if begin<source.len() {lines.push(source[begin..].to_owned());}
            Value::TextRow(Rc::new(lines),false)
        }
        PARTITION|RPARTITION=>{
            let separator=g.word(0)?;
            if separator.is_empty() {return Err(g.bad("separator"));}
            let position=match work {PARTITION=>source.find(separator),_=>source.rfind(separator)};
            let row=if let Some(i)=position {vec![source[..i].to_owned(),separator.to_owned(),source[i+separator.len()..].to_owned()]}
                else if work==RPARTITION {vec![String::new(),String::new(),source.to_owned()]}
                else {vec![source.to_owned(),String::new(),String::new()]};
            Value::TextRow(Rc::new(row),true)
        }
        EXPANDTABS=>{
            let stop=g.whole(0,8)?.max(0) as usize;
            let mut column=0;let mut expanded=String::new();
            for letter in source.chars() {
                match letter {
                    '\t'=>{
                        let spaces=if stop>0 {stop-column%stop} else {0};
                        expanded.try_reserve(spaces).map_err(|_|g.bad("room"))?;
                        expanded.extend((0..spaces).map(|_|' ')); column+=spaces;
                    }
                    '\n'|'\r'=>{column=0;expanded.push(letter);}
                    _=>{column+=1;expanded.push(letter);}
                }
            }
            Value::text(&expanded)
        }
        REMOVEPREFIX|REMOVESUFFIX=>{
            let part=g.word(0)?;
            let remove=match work {REMOVEPREFIX=>source.starts_with(part),_=>source.ends_with(part)};
            let kept=if !remove {source} else if work==REMOVEPREFIX {&source[part.len()..]} else {&source[..source.len()-part.len()]};
            Value::text(kept)
        }
        STRIP|LSTRIP|RSTRIP=>{
            let chosen=match g.tail.first() {Some(Value::Nil)|None=>None,Some(v)=>Some(letters(v,table)?)};
            let ignored=|c|if let Some(chars)=chosen {chars.contains(c)} else {blank(c)};
            let mut kept=source;
            if work!=RSTRIP {kept=kept.trim_start_matches(ignored);}
            if work!=LSTRIP {kept=kept.trim_end_matches(ignored);}
            Value::text(kept)
        }
        SPLIT|RSPLIT=>{
            let maximum=g.whole(1,-1)?;
            let quota=if maximum<0 {usize::MAX} else {maximum as usize};
            let on=match g.tail.first() {None|Some(Value::Nil)=>None,Some(v)=>Some(letters(v,table)?)};
            let mut divided=Vec::new();let mut remaining=source;
            if on==Some("") {return Err(g.bad("separator"));}
            loop {
                if on.is_none() {
                    remaining=if work==SPLIT {remaining.trim_start_matches(blank)} else {remaining.trim_end_matches(blank)};
                    if remaining.is_empty() {break;}
                }
                if divided.len()==quota {divided.push(remaining.to_owned());break;}
                let at=match (work,on) {
                    (SPLIT,Some(sep))=>remaining.find(sep),(RSPLIT,Some(sep))=>remaining.rfind(sep),
                    (SPLIT,None)=>remaining.find(blank),_=>remaining.rfind(blank),
                };
                let Some(at)=at else {divided.push(remaining.to_owned());break;};
                let size=on.map_or_else(||remaining[at..].chars().next().unwrap().len_utf8(),str::len);
                if work==SPLIT {divided.push(remaining[..at].to_owned());remaining=&remaining[at+size..];}
                else {divided.push(remaining[at+size..].to_owned());remaining=&remaining[..at];}
            }
            if work==RSPLIT {divided.reverse();}
            Value::TextRow(Rc::new(divided),false)
        }
        CENTER|LJUST|RJUST|ZFILL=>{
            let width=g.whole(0,0)?.max(0) as usize;
            let padding=if work==ZFILL {'0'} else if g.tail.len()==1 {' '} else {
                let mut letters=g.word(1)?.chars();let first=letters.next();
                if first.is_none() || letters.next().is_some() {return Err(g.bad("fill"));}first.unwrap()
            };
            let spare=width.saturating_sub(many);
            let before=if work==LJUST {0} else if work==CENTER {spare/2+usize::from(spare%2==1 && width%2==1)} else {spare};
            let mut result=String::new();
            let required=spare.checked_mul(padding.len_utf8()).and_then(|n|n.checked_add(source.len())).ok_or_else(||g.bad("room"))?;
            result.try_reserve(required).map_err(|_|g.bad("room"))?;
            let mut body=source;
            if work==ZFILL && source.starts_with(['-','+']) {result.push(source.chars().next().unwrap());body=&source[1..];}
            result.extend((0..before).map(|_|padding));result.push_str(body);result.extend((before..spare).map(|_|padding));
            Value::text(&result)
        }
        FIND|RFIND|INDEX|RINDEX|COUNT|STARTSWITH|ENDSWITH=>seek(work,source,&g)?,
        REPLACE=>{
            let quota=g.whole(2,-1)?;
            let maximum=if quota<0 {usize::MAX} else {quota as usize};
            Value::text(&source.replacen(g.word(0)?,g.word(1)?,maximum))
        }
        JOIN=>{
            let row=match &g.tail[0] {
                Value::Vector(items)=>items.to_vec(),
                Value::TextRow(items,_)=>items.iter().map(|s|Value::text(s)).collect(),
                Value::Text(word)=>word.chars().map(|c|Value::text(&String::from(c))).collect(),
                Value::Dict(entries)=>entries.iter().map(|(key,_)|key.clone()).collect(),
                _=>return Err(g.bad("walk")),
            };
            let mut portions=Vec::new();
            for item in &row {match item {Value::Text(t)=>portions.push(t.as_ref()),_=>return Err(g.bad("join"))}}
            Value::text(&portions.join(source))
        }
        TRANSLATE=>{
            let Value::Dict(entries)=&g.tail[0] else {return Err(g.bad("mapping"));};
            let mut result=String::new();
            for letter in source.chars() {
                let numbered=Value::Small(letter as i64);
                let replace=entries.iter().find(|(k,_)|k.equals(&numbered)).map(|(_,v)|v);
                if let Some(value)=replace {
                    match value {
                        Value::Nil=>(),Value::Text(word)=>result.push_str(word),
                        Value::Small(_)|Value::Huge(_)|Value::Flag(_)=>{
                            let code=count(value,table)?;
                            if code<0 || code>=0x110000 {return Err(g.bad("codepoint"));}
                            let character=char::from_u32(code as u32).ok_or_else(||g.bad("surrogate"))?;
                            result.push(character);
                        }
                        _=>return Err(g.bad("translation")),
                    }
                } else {result.push(letter);}
            }
            Value::text(&result)
        }
        FORMATMAP=>Value::text(&fill_mapping(table,source,&g.tail[0],names,0)?),
    };
    Ok(answer)
}

fn seek(work: Work, source: &str, g: &Given) -> Result<Value,String> {
    let letters:Vec<char>=source.chars().collect();let size=letters.len();
    let bound=|place:usize,default:usize|->Result<usize,String>{
        let number=match g.tail.get(place) {None|Some(Value::Nil)=>return Ok(default),Some(v)=>count(v,g.table)?};
        Ok(if number>=0 {number as usize} else {(size as i64).saturating_add(number).max(0) as usize})
    };
    let begin=bound(1,0)?;let end=bound(2,size)?.min(size);
    let fitting=begin<=end && begin<=size;
    let piece:String=if fitting {letters[begin..end].iter().collect()} else {String::new()};
    if matches!(work,Work::STARTSWITH|Work::ENDSWITH) {
        let candidates=match &g.tail[0] {
            Value::Vector(values)=>values.to_vec(),
            Value::TextRow(words,true)=>words.iter().map(|s|Value::text(s)).collect(),
            v=>vec![v.clone()],
        };
        for candidate in candidates {
            let affix=letters_of_candidate(&candidate,g)?;
            let matches=match work {Work::STARTSWITH=>piece.starts_with(affix),_=>piece.ends_with(affix)};
            if fitting && matches {return Ok(Value::Flag(true));}
        }
        return Ok(Value::Flag(false));
    }
    let sought=g.word(0)?;
    if work==Work::COUNT {
        return Ok(Value::Small(if !fitting {0} else if sought.is_empty() {end as i64-begin as i64+1} else {piece.matches(sought).count() as i64}));
    }
    let position=if !fitting {None} else if matches!(work,Work::RFIND|Work::RINDEX) {piece.rfind(sought)} else {piece.find(sought)};
    if let Some(byte)=position {return Ok(Value::Small((begin+piece[..byte].chars().count()) as i64));}
    if work==Work::INDEX||work==Work::RINDEX {return Err(g.bad("missing"));}
    Ok(Value::Small(-1))
}

fn letters_of_candidate<'a>(candidate: &'a Value,g:&Given) -> Result<&'a str,String> {letters(candidate,g.table)}

fn fill_mapping(table:&Table, pattern:&str, mapping:&Value, names:Names, level:usize) -> Result<String,String> {
    let refuse=|why|complaint(table,why);
    if level>2 {return Err(refuse("format"));}
    let Value::Dict(entries)=mapping else {return Err(refuse("format"));};
    let chars:Vec<char>=pattern.chars().collect();let mut cursor=0;let mut output=String::new();
    while cursor<chars.len() {
        let ch=chars[cursor];cursor+=1;
        if ch!='{' && ch!='}' {output.push(ch);continue;}
        if chars.get(cursor)==Some(&ch) {output.push(ch);cursor+=1;continue;}
        if ch=='}' {return Err(refuse("format.brace"));}
        let begin=cursor;let mut nesting=0;
        while cursor<chars.len() {
            match chars[cursor] {'}' if nesting==0=>break,'}'=>nesting-=1,'{'=>nesting+=1,_=>()}
            cursor+=1;
        }
        if cursor==chars.len() {return Err(refuse("format.brace"));}
        let field:String=chars[begin..cursor].iter().collect();cursor+=1;
        let colon=field.find(':').unwrap_or(field.len());let head=&field[..colon];
        let bang=head.find('!').unwrap_or(head.len());let key=&head[..bang];
        if key.is_empty() || key.starts_with(|c:char|c.is_ascii_digit()) {return Err(refuse("format.positional"));}
        if key.contains('.')||key.contains('[') {return Err(refuse("format"));}
        let value=entries.iter().find_map(|(k,v)|if matches!(k,Value::Text(t) if t.as_ref()==key) {Some(v)} else {None})
            .ok_or_else(||refuse("key")+&quotation(key))?;
        let spec=if colon<field.len() {fill_mapping(table,&field[colon+1..],mapping,names,level+1)?} else {String::new()};
        let conversion=if bang<head.len() {&head[bang+1..]} else {""};
        let rendered=match conversion {
            "r"|"a"=>{
                let mut literal=expression(value,names);
                if conversion=="a" {
                    literal=literal.chars().map(|c|match c as u32 {0..=127=>c.to_string(),128..=65535=>format!("\\u{:04x}",c as u32),_=>format!("\\U{:08x}",c as u32)}).collect();
                }
                Value::text(&literal).in_field(names,&spec,"")
            }
            manner=>value.in_field(names,&spec,manner),
        }.ok_or_else(||refuse("format"))?;
        output.push_str(&rendered);
    }
    Ok(output)
}
