// A value answers the names its definition gives it. Mutable collections
// retain their holding place when a method is kept for a later call.

use crate::data::{Names, Value};
use num_bigint::BigInt;
use num_traits::{Signed, ToPrimitive, Zero};
use std::rc::Rc;

type ResultValue = Result<Value, String>;

pub struct Request<'a> {
    pub target: &'a Value,
    pub operation: &'a str,
    pub given: Vec<Value>,
    pub named: &'a [(String, Value)],
    pub names: Names<'a>,
    pub complaint: &'a dyn Fn(&str) -> String,
}

pub fn gather(source: &Value, bad: &dyn Fn(&str)->String) -> Result<Vec<Value>,String> {
    let source=source.settled();
    let mut result=Vec::new();
    match source {
        Value::Text(t) => result.extend(t.chars().map(|ch|Value::text(&String::from(ch)))),
        Value::Vector(v) | Value::Row(v) => result.extend(v.iter().cloned()),
        Value::Dict(d) => result.extend(d.iter().map(|entry|entry.0.clone())),
        Value::Progression(p) => {let mut at=BigInt::from(0);while at<p.count(){result.push(Value::from_big(&p.first+&p.stride*&at));at+=1;}},
        _ => return Err(bad("arguments")),
    }
    Ok(result)
}

fn whole(v:&Value, bad:&dyn Fn(&str)->String)->Result<i64,String> {
    let n=match v.settled(){Value::Small(i)=>return Ok(i),Value::Flag(b)=>return Ok(b as i64),Value::Huge(i)=>i,_=>return Err(bad("arguments"))};
    Ok(n.to_i64().unwrap_or_else(||if n.is_negative(){i64::MIN}else{i64::MAX}))
}
fn letters(v:&Value,bad:&dyn Fn(&str)->String)->Result<String,String>{
    match v.settled(){Value::Text(chars)=>Ok(chars.to_string()),_=>Err(bad("arguments"))}
}
fn place(number:i64,size:usize)->usize {
    let positive=if number<0{number.saturating_add(size as i64).max(0)}else{number};
    (positive as usize).min(size)
}

impl Request<'_> {
    fn fail(&self,key:&str)->String{(self.complaint)(key)}
    fn takes(&self,minimum:usize,maximum:usize)->Result<(),String>{
        if (minimum..=maximum).contains(&self.given.len()){Ok(())}else{Err(self.fail("arguments"))}
    }
    fn number(&self,at:usize,default:i64)->Result<i64,String>{
        match self.given.get(at){None|Some(Value::Nil)=>Ok(default),Some(v)=>whole(v,self.complaint)}
    }
    fn string(&self,at:usize)->Result<String,String>{
        letters(self.given.get(at).ok_or_else(||self.fail("arguments"))?,self.complaint)
    }
    fn replace(&self,new_value:Value)->ResultValue {
        let Value::Mutable(cell,_) = self.target else{return Err(self.fail("unready"));};
        if circular(&new_value,cell,0) {return Err(self.fail("unready"));}
        cell.replace(new_value);
        Ok(Value::Nil)
    }
    pub fn answer(mut self)->ResultValue {
        let positional=self.given.len();
        if !["format","update","encode"].contains(&self.operation){
            for (name,value) in self.named {
                let at=match (self.operation,name.as_str()){("split"|"rsplit","sep")=>0,("split"|"rsplit","maxsplit")=>1,_=>return Err(self.fail("arguments"))};
                if at<positional{return Err(self.fail("arguments"));}
                if self.given.len()<=at{self.given.resize(at+1,Value::Nil);}self.given[at]=value.clone();
            }
        }
        match self.target.settled(){
            Value::Text(chars)=>self.on_text(&chars),
            Value::Vector(items)=>self.on_list(items.to_vec()),
            Value::Dict(entries)=>self.on_map(entries.to_vec()),
            number @ (Value::Small(_)|Value::Huge(_)|Value::Frac(_))=>self.on_number(number),
            _=>Err(self.fail("attribute")),
        }
    }
    fn on_number(&self,value:Value)->ResultValue{
        self.takes(0,0)?;
        let real=matches!(value,Value::Frac(_));
        match self.operation {
            "conjugate" | "real" => return Ok(value),
            "imag" => return Ok(if real { Value::Frac(Rc::new(crate::data::Ratio { above: BigInt::from(0), beneath: BigInt::from(1), places: Some(16), under: false, pointed: true })) } else { Value::Small(0) }),
            "__index__" | "numerator" if !real => return Ok(Value::from_big(value.as_big()?)),
            "denominator" if !real => return Ok(Value::Small(1)),
            _ => (),
        }
        if !real && self.operation == "bit_count" {
            let magnitude = value.as_big()?.magnitude().clone();
            let mut total = BigInt::from(0);
            for limb in magnitude.iter_u32_digits() { total += limb.count_ones(); }
            return Ok(Value::from_big(total));
        }
        if self.operation=="bit_length" && !real{return Ok(Value::Small(value.as_big()?.bits() as i64));}
        if self.operation=="is_integer"{return Ok(Value::Flag(match &value{Value::Frac(r)=>!r.past_numbers()&&(&r.above%&r.beneath).is_zero(),_=>true}));}
        if self.operation=="as_integer_ratio"{
            let pair=match value{Value::Frac(r) if !r.past_numbers()=>crate::data::binary_worth(crate::data::nearest_binary(&r.above,&r.beneath)).ok_or_else(||self.fail("unready"))?,Value::Frac(_)=>return Err(self.fail("unready")),other=>(other.as_big()?,BigInt::from(1))};
            let mut divisor=pair.0.abs();let mut remainder=pair.1.clone();
            while !remainder.is_zero(){let next=&divisor%&remainder;divisor=remainder;remainder=next;}
            return Ok(Value::Row(Rc::new(vec![Value::from_big(pair.0/&divisor),Value::from_big(pair.1/divisor)])));
        }
        if self.operation=="hex" && real {
            let Value::Frac(ratio)=value else {unreachable!()};
            let binary=crate::data::nearest_binary(&ratio.above,&ratio.beneath);
            let negative=binary.is_sign_negative()||ratio.under;
            let sign=if negative {"-"}else{""};
            let output=if binary.is_nan(){String::from("nan")}else if binary.is_infinite(){format!("{}inf",sign)}else if binary==0.0{format!("{}0x0.0p+0",sign)}else{
                let encoding=binary.to_bits();let exponent=((encoding>>52)&2047) as i32;
                let integer=if exponent==0{0}else{1};let power=if exponent==0{-1022}else{exponent-1023};
                format!("{}0x{}.{:013x}p{:+}",sign,integer,encoding&((1u64<<52)-1),power)
            };return Ok(Value::text(&output));
        }
        Err(self.fail("attribute"))
    }
    fn on_text(&self,s:&str)->ResultValue{
        let op=self.operation;
        if op=="encode"{return Err(self.fail("bytes"));}
        if op=="format"{return self.fill_fields(s).map(|t|Value::text(&t));}
        if ["upper","lower","title","capitalize"].contains(&op){
            self.takes(0,0)?;
            let converted=match op{
                "upper"=>s.to_uppercase(),"lower"=>s.to_lowercase(),_=>{
                    if !s.is_ascii(){return Err(self.fail("unicode"));}
                    let mut out=String::new();let mut prior=false;
                    for (at,ch) in s.chars().enumerate(){let capital=if op=="title"{!prior}else{at==0};out.push(if capital{ch.to_ascii_uppercase()}else{ch.to_ascii_lowercase()});prior=ch.is_ascii_alphabetic();}out
                }
            };return Ok(Value::text(&converted));
        }
        if ["strip","lstrip","rstrip"].contains(&op){
            self.takes(0,1)?;
            let set=match self.given.first(){None|Some(Value::Nil)=>None,Some(v)=>Some(letters(v,self.complaint)?)};
            let removes=|ch:char|match &set{Some(chars)=>chars.contains(ch),None=>ch.is_whitespace()||matches!(ch,'\u{1c}'..='\u{1f}')};
            let trimmed=if op=="lstrip"{s.trim_start_matches(removes)}else if op=="rstrip"{s.trim_end_matches(removes)}else{s.trim_matches(removes)};
            return Ok(Value::text(trimmed));
        }
        if op=="split"||op=="rsplit"{return self.split_text(s);}
        if op=="join"{
            self.takes(1,1)?;let mut strings=Vec::new();for item in gather(&self.given[0],self.complaint)?{strings.push(letters(&item,self.complaint)?);}return Ok(Value::text(&strings.join(s)));
        }
        if op=="replace"{
            self.takes(2,3)?;let limit=self.number(2,-1)?;let old=self.string(0)?;let new=self.string(1)?;
            return Ok(Value::text(&s.replacen(&old,&new,if limit<0{usize::MAX}else{limit as usize})));
        }
        if ["find","rfind","index","count","startswith","endswith"].contains(&op){return self.search_text(s);}
        if ["isdigit","isalpha","isalnum","isspace","islower","isupper"].contains(&op){
            self.takes(0,0)?;if !s.is_ascii(){return Err(self.fail("unicode"));}
            let mut yes=!s.is_empty();let mut cased=false;
            for ch in s.chars(){
                cased|=ch.is_ascii_alphabetic();
                yes&=match op{"isdigit"=>ch.is_ascii_digit(),"isalpha"=>ch.is_ascii_alphabetic(),"isalnum"=>ch.is_ascii_alphanumeric(),"isspace"=>ch.is_ascii_whitespace()||matches!(ch,'\u{1c}'..='\u{1f}'),"islower"=>!ch.is_ascii_uppercase(),_=>!ch.is_ascii_lowercase()};
            }
            if op=="islower"||op=="isupper"{yes&=cased;}
            return Ok(Value::Flag(yes));
        }
        if ["center","ljust","rjust","zfill"].contains(&op){
            self.takes(1,if op=="zfill"{1}else{2})?;
            let size=self.number(0,0)?.max(0) as usize;if size>1_000_000{return Err(self.fail("unready"));}
            let padding=if self.given.len()>1{self.string(1)?}else{String::from(" ")};
            if padding.chars().count()!=1{return Err(self.fail("fill"));}
            let count=size.saturating_sub(s.chars().count());
            let mut answer=String::new();
            if op=="zfill"{
                let sign=s.chars().next().filter(|ch|*ch=='+'||*ch=='-');if let Some(sign)=sign{answer.push(sign);}answer.push_str(&"0".repeat(count));answer.push_str(if sign.is_some(){&s[1..]}else{s});
            }else{
                let left=if op=="rjust"{count}else if op=="center"{(count/2)+((count&1)&(size&1))}else{0};
                answer.push_str(&padding.repeat(left));answer.push_str(s);answer.push_str(&padding.repeat(count-left));
            }
            return Ok(Value::text(&answer));
        }
        Err(self.fail("attribute"))
    }
    fn split_text(&self,s:&str)->ResultValue{
        self.takes(0,2)?;let reverse=self.operation=="rsplit";let bound=self.number(1,-1)?;
        let maximum=if bound<0{usize::MAX}else{bound as usize};let mut chunks=Vec::new();
        let delimiter=match self.given.first(){Some(v) if !matches!(v,Value::Nil)=>Some(letters(v,self.complaint)?),_=>None};
        match delimiter{
            Some(delimiter)=>{
                if delimiter.is_empty(){return Err(self.fail("separator"));}
                if reverse{chunks.extend(s.rsplitn(maximum.saturating_add(1),&delimiter).map(str::to_owned));}else{chunks.extend(s.splitn(maximum.saturating_add(1),&delimiter).map(str::to_owned));}
            }
            None=>{
                let blank=|ch:char|ch.is_whitespace()||matches!(ch,'\u{1c}'..='\u{1f}');let mut tail=s;
                loop{
                    tail=if reverse{tail.trim_end_matches(blank)}else{tail.trim_start_matches(blank)};
                    if tail.is_empty(){break;}
                    if chunks.len()==maximum{chunks.push(tail.to_owned());break;}
                    let cut=if reverse{tail.char_indices().rev().find(|(_,ch)|blank(*ch))}else{tail.char_indices().find(|(_,ch)|blank(*ch))};
                    if let Some((offset,ch))=cut{
                        if reverse{chunks.push(tail[offset+ch.len_utf8()..].to_owned());tail=&tail[..offset];}
                        else{chunks.push(tail[..offset].to_owned());tail=&tail[offset+ch.len_utf8()..];}
                    }else{chunks.push(tail.to_owned());break;}
                }
            }
        }
        if reverse{chunks.reverse();}
        Ok(Value::Vector(Rc::new(chunks.iter().map(|part|Value::text(part)).collect())).keep(true))
    }
    fn search_text(&self,s:&str)->ResultValue{
        self.takes(1,3)?;let length=s.chars().count();let raw=self.number(1,0)?;
        let lo=place(raw,length);let hi=place(self.number(2,length as i64)?,length);
        let valid=raw<=length as i64&&lo<=hi;
        let part:String=s.chars().skip(lo).take(hi.saturating_sub(lo)).collect();
        if self.operation=="startswith"||self.operation=="endswith"{
            let patterns=if let Value::Row(row)=&self.given[0]{row.to_vec()}else{vec![self.given[0].clone()]};
            for pattern in patterns{let text=letters(&pattern,self.complaint)?;let matches=if self.operation=="startswith"{part.starts_with(&text)}else{part.ends_with(&text)};if valid&&matches{return Ok(Value::Flag(true));}}
            return Ok(Value::Flag(false));
        }
        let needle=self.string(0)?;
        if self.operation=="count"{let count=if !valid{0}else if needle.is_empty(){part.chars().count()+1}else{part.matches(&needle).count()};return Ok(Value::Small(count as i64));}
        let at=if !valid{None}else if self.operation=="rfind"{part.rfind(&needle)}else{part.find(&needle)};
        if self.operation=="index"&&at.is_none(){return Err(self.fail("substring"));}
        Ok(Value::Small(match at{None=>-1,Some(byte)=>(lo+part[..byte].chars().count()) as i64}))
    }
    fn on_list(&self,mut values:Vec<Value>)->ResultValue{
        match self.operation{
            "append"=>{self.takes(1,1)?;values.push(self.given[0].clone());}
            "extend"=>{self.takes(1,1)?;values.extend(gather(&self.given[0],self.complaint)?);}
            "insert"=>{self.takes(2,2)?;values.insert(place(self.number(0,0)?,values.len()),self.given[1].clone());}
            "clear"=>{self.takes(0,0)?;values=Vec::new();}
            "reverse"=>{self.takes(0,0)?;values.reverse();}
            "copy"=>{self.takes(0,0)?;return Ok(Value::Vector(Rc::new(values)).keep(true));}
            "pop"=>{
                self.takes(0,1)?;if values.is_empty(){return Err(self.fail("pop"));}let mut offset=self.number(0,-1)?;
                if offset<0{offset=offset.saturating_add(values.len() as i64);}if offset<0||offset as usize>=values.len(){return Err(self.fail("index"));}
                let result=values.remove(offset as usize);self.replace(Value::Vector(Rc::new(values)))?;return Ok(result);
            }
            "remove"|"index"|"count"=>{
                self.takes(1,if self.operation=="index"{3}else{1})?;
                let start=place(self.number(1,0)?,values.len());let stop=place(self.number(2,values.len() as i64)?,values.len());
                let mut first=None;let mut count=0;
                for (at,value) in values.iter().enumerate().take(stop).skip(start){if same_item(value,&self.given[0]){count+=1;if first.is_none(){first=Some(at);}}}
                if self.operation=="count"{return Ok(Value::Small(count));}
                let at=first.ok_or_else(||self.fail(if self.operation=="remove"{"remove"}else{"list_index"}))?;
                if self.operation=="index"{return Ok(Value::Small(at as i64));}values.remove(at);
            }
            _=>return Err(self.fail("attribute")),
        }
        self.replace(Value::Vector(Rc::new(values)))
    }
    fn on_map(&self,mut entries:Vec<(Value,Value)>)->ResultValue{
        match self.operation{
            "keys"|"values"|"items"=>{
                self.takes(0,0)?;
                let portion=if self.operation=="keys"{'k'}else if self.operation=="values"{'v'}else{'i'};
                return Ok(Value::Window(Rc::new(self.target.clone()),portion));
            }
            "copy"=>{self.takes(0,0)?;return Ok(Value::Dict(Rc::new(entries)).keep(true));}
            "clear"=>{self.takes(0,0)?;entries.clear();}
            "get"|"setdefault"|"pop"=>{
                self.takes(1,2)?;let key=&self.given[0];
                if matches!(key.settled(),Value::Vector(_)|Value::Dict(_)){return Err(self.fail("arguments"));}
                if let Some(index)=entries.iter().position(|e|same_item(&e.0,key)){
                    let answer=entries[index].1.clone();if self.operation=="pop"{entries.remove(index);self.replace(Value::Dict(Rc::new(entries)))?;}return Ok(answer);
                }
                if self.operation=="pop"&&self.given.len()==1{return Err(self.fail("key")+&key.repr(&self.names));}
                let answer=self.given.get(1).cloned().unwrap_or(Value::Nil);
                if self.operation=="setdefault"{entries.push((key.clone(),answer.clone()));self.replace(Value::Dict(Rc::new(entries)))?;}return Ok(answer);
            }
            "update"=>{
                self.takes(0,1)?;let mut incoming=Vec::new();
                if let Some(source)=self.given.first(){
                    match source.settled(){Value::Dict(d)=>incoming=d.to_vec(),other=>{
                        for item in gather(&other,self.complaint)?{let values=gather(&item,self.complaint)?;if values.len()!=2{return Err(self.fail("arguments"));}incoming.push((values[0].clone(),values[1].clone()));}
                    }}
                }
                for (key,value) in self.named{incoming.push((Value::text(key),value.clone()));}
                for (key,value) in incoming{match entries.iter_mut().find(|entry|entry.0.equals(&key)){Some(entry)=>entry.1=value,None=>entries.push((key,value))}}
            }
            _=>return Err(self.fail("attribute")),
        }
        self.replace(Value::Dict(Rc::new(entries)))
    }
    fn fill_fields(&self,template:&str)->Result<String,String>{
        let input:Vec<char>=template.chars().collect();let mut pos=0;let mut ordinal=0;let mut mode=0u8;let mut output=String::new();
        while pos<input.len(){
            let ch=input[pos];pos+=1;
            if ch!='{'&&ch!='}'{output.push(ch);continue;}
            if input.get(pos)==Some(&ch){pos+=1;output.push(ch);continue;}
            if ch=='}'{return Err(self.fail("format"));}
            let start=pos;while input.get(pos).map_or(false,|ch|*ch!='}') {if input[pos]=='{'{return Err(self.fail("spec"));}pos+=1;}
            if pos==input.len(){return Err(self.fail("format"));}
            let field:String=input[start..pos].iter().collect();pos+=1;
            let mut halves=field.splitn(2,':');let head=halves.next().unwrap();let pattern=halves.next().unwrap_or("");
            let mut head=head.splitn(2,'!');let key=head.next().unwrap();let conversion=head.next().unwrap_or("");
            if !["","s","r","a"].contains(&conversion){return Err(self.fail("format"));}
            let value=if key.is_empty(){if mode==2{return Err(self.fail("mixed"));}mode=1;let index=ordinal;ordinal+=1;self.given.get(index).ok_or_else(||self.fail("missing"))?}
                else if let Ok(index)=key.parse::<usize>(){if mode==1{return Err(self.fail("mixed"));}mode=2;self.given.get(index).ok_or_else(||self.fail("missing"))?}
                else{if key.contains('.')||key.contains('['){return Err(self.fail("unready"));}match self.named.iter().find(|entry|entry.0==key){Some(entry)=>&entry.1,None=>return Err(self.fail("key")+&Value::text(key).repr(&self.names))}};
            output.push_str(&self.field(value,pattern,conversion)?);
        }
        Ok(output)
    }
    fn field(&self,value:&Value,pattern:&str,conversion:&str)->Result<String,String>{
        let value=value.settled();
        if pattern=="x"{return match value{Value::Small(_)|Value::Huge(_)=>Ok(value.as_big()?.to_str_radix(16)),_=>Err(self.fail("arguments"))};}
        if pattern==","{
            if !matches!(value,Value::Small(_)|Value::Huge(_)){return Err(self.fail("spec"));}
            let whole=value.as_big()?;let unsigned=whole.abs().to_string();let mut reversed=Vec::new();
            for (i,c) in unsigned.chars().rev().enumerate(){if i!=0&&i%3==0{reversed.push(',');}reversed.push(c);}
            if whole.is_negative(){reversed.push('-');}return Ok(reversed.into_iter().rev().collect());
        }
        let mut valid=pattern.is_empty();let marks:Vec<char>=pattern.chars().collect();
        for offset in [1usize,2]{if marks.len()>=offset&&['<','>','^'].contains(&marks[offset-1]){valid|=marks[offset..].iter().collect::<String>().parse::<usize>().map_or(false,|size|size<=100000);}}
        if pattern.starts_with('.')&&pattern.ends_with('f')&&matches!(value,Value::Small(_)|Value::Huge(_)|Value::Frac(_)){
            valid|=pattern[1..pattern.len()-1].parse::<usize>().map_or(false,|precision|precision<=1000);
        }
        if !valid{return Err(self.fail("spec"));}
        if pattern.ends_with('f') && conversion.is_empty(){return value.in_field(self.names,pattern,conversion).ok_or_else(|| self.fail("spec"));}
        let text=match &value{
            Value::Text(_)=>value.in_field(self.names,"",conversion).ok_or_else(|| self.fail("spec"))?,
            Value::Frac(ratio)=>{
                let mut binary=crate::data::nearest_binary(&ratio.above,&ratio.beneath);
                if binary==0.0&&ratio.under{binary = -0.0;}
                if binary.is_nan(){String::from("nan")}else if binary.is_infinite(){String::from(if binary<0.0{"-inf"}else{"inf"})}else{
                    let expanded=format!("{:e}",binary);let cut=expanded.find('e').unwrap();let exponent=expanded[cut+1..].parse::<i32>().unwrap();
                    if exponent>=16||exponent< -4{format!("{}e{}{:02}",&expanded[..cut],if exponent<0{"-"}else{"+"},exponent.abs())}
                    else{let short=binary.to_string();if short.contains('.') {short}else{short+".0"}}
                }
            }
            _=>value.repr(&self.names),
        };
        Value::text(&text).in_field(self.names,pattern,"").ok_or_else(|| self.fail("spec"))
    }
}

fn circular(value:&Value, receiver:&Rc<std::cell::RefCell<Value>>, level:usize)->bool {
    if level>=101{return true;}
    if let Value::Mutable(place,_) = value {
        return Rc::ptr_eq(place,receiver)||circular(&place.borrow(),receiver,level+1);
    }
    let parts=match value {
        Value::Vector(items)|Value::Row(items)=>items.to_vec(),
        Value::Window(owner,_)=>vec![owner.as_ref().clone()],
        Value::Dict(entries)=>entries.iter().flat_map(|(key,value)|[key.clone(),value.clone()]).collect(),
        _=>return false,
    };
    for part in parts {if circular(&part,receiver,level+1){return true;}}
    false
}

fn same_item(left:&Value,right:&Value)->bool{
    if let (Value::Frac(a),Value::Frac(b))=(left,right){if Rc::ptr_eq(a,b){return true;}}
    left.equals(right)
}

/// Read the significand whole, then ask the binary width for its worth.
pub fn hexadecimal(given: &[Value], bad: &dyn Fn(&str) -> String) -> ResultValue {
    if given.len() != 1 { return Err(bad("arguments")); }
    let Value::Text(chars) = &given[0] else { return Err(bad("arguments")); };
    let lower = chars.trim_matches(|ch: char| ch.is_ascii_whitespace()).to_ascii_lowercase();
    let minus = lower.as_bytes().first() == Some(&b'-');
    let mut body = lower.as_str();
    if body.starts_with(['-', '+']) { body = &body[1..]; }
    let unsigned = match body {
        "nan" => f64::NAN,
        "inf" | "infinity" => f64::INFINITY,
        _ => {
            if body.starts_with("0x") { body = &body[2..]; }
            let mut pieces = body.split('p');
            let significand = pieces.next().unwrap_or("");
            let exponent = pieces.next().unwrap_or("0");
            if pieces.next().is_some() { return Err(bad("hex")); }
            let mut magnitude = BigInt::from(0);
            let mut fractional = None;
            let mut count = 0;
            for byte in significand.bytes() {
                if byte == b'.' && fractional.is_none() { fractional = Some(0i64); continue; }
                let digit = (byte as char).to_digit(16).ok_or_else(|| bad("hex"))?;
                magnitude = magnitude * 16 + digit;
                count += 1;
                if let Some(places) = fractional.as_mut() { *places += 4; }
            }
            if count == 0 { return Err(bad("hex")); }
            let exponent_body = exponent.trim_start_matches(['-', '+']);
            if exponent_body.is_empty() || !exponent_body.bytes().all(|b| b.is_ascii_digit()) || exponent.len() - exponent_body.len() > 1 { return Err(bad("hex")); }
            let shift = exponent.parse::<i64>().unwrap_or_else(|_| if exponent.starts_with('-') { i64::MIN } else { i64::MAX }).saturating_sub(fractional.unwrap_or(0));
            let extent = (magnitude.bits() as i64).saturating_add(shift);
            if magnitude.is_zero() || extent < -1075 { 0.0 }
            else {
                if extent > 1025 { return Err(bad("hex_overflow")); }
                let denominator;
                if shift < 0 { denominator = BigInt::from(1) << shift.unsigned_abs() as usize; }
                else { magnitude <<= shift as usize; denominator = BigInt::from(1); }
                let binary = crate::data::nearest_binary(&magnitude, &denominator);
                if !binary.is_finite() { return Err(bad("hex_overflow")); }
                binary
            }
        }
    };
    let signed = if minus { -unsigned } else { unsigned };
    let pair = match crate::data::binary_worth(signed) {
        Some(ratio) => ratio,
        None => (BigInt::from(if signed.is_nan() { 0 } else { if minus { -1 } else { 1 } }), BigInt::from(0)),
    };
    Ok(Value::Frac(Rc::new(crate::data::Ratio { above: pair.0, beneath: pair.1, places: Some(16), under: minus, pointed: true })))
}
