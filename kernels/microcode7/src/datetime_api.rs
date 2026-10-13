//! The lumen datetime C ABI. Handles retain live interpreter objects rather than CPython structs.
use crate::data::Value;
use crate::exec::{Escape, Machine};
use std::{ffi::{c_char, c_void, CStr, CString}, ptr::null_mut, rc::Rc};

type Handle = *mut c_void;
type Failure = *mut Handle;
pub const API_NAME: &str = "lumen.datetime_CAPI.v1";
#[repr(C)]
pub struct CalendarApi {
    pub revision: u64,
    pub bytes: usize,
    pub date_class: Handle,
    pub datetime_class: Handle,
    pub time_class: Handle,
    pub duration_class: Handle,
    pub tzinfo_class: Handle,
    pub timezone_class: Handle,
    pub utc_object: Handle,
    pub copy_object: unsafe extern "C" fn(Handle)->Handle,
    pub drop_object: unsafe extern "C" fn(Handle),
    pub drop_error: unsafe extern "C" fn(Handle),
    pub new_date: unsafe extern "C" fn(Handle,Handle,i32,i32,i32,Failure)->Handle,
    pub new_datetime: unsafe extern "C" fn(Handle,Handle,i32,i32,i32,i32,i32,i32,i32,Handle,i32,Failure)->Handle,
    pub new_time: unsafe extern "C" fn(Handle,Handle,i32,i32,i32,i32,Handle,i32,Failure)->Handle,
    pub new_duration: unsafe extern "C" fn(Handle,Handle,i32,i32,i32,Failure)->Handle,
    pub new_timezone: unsafe extern "C" fn(Handle,Handle,Handle,Handle,Failure)->Handle,
    pub timestamp: unsafe extern "C" fn(Handle,Handle,Handle,Handle,Failure)->Handle,
    pub get_attribute: unsafe extern "C" fn(Handle,Handle,*const c_char,Failure)->Handle,
}

pub fn lend_machine<R>(machine: &mut Machine<'_>, visit: impl FnOnce(Handle)->R)->R {
    visit(std::ptr::from_mut(machine).cast())
}
pub fn hold_object(object: Value)->Handle { Box::into_raw(Box::new(object)).cast::<c_void>() }
/// # Safety
/// The handle must be an owned live object from this kernel, and is consumed.
pub unsafe fn own_object(pointer: Handle)->Value {
    let allocation = unsafe { Box::from_raw(pointer as *mut Value) };
    *allocation
}
/// # Safety
/// Transfer ownership of an error returned by this kernel's C ABI.
pub unsafe fn own_error(pointer: Handle)->Escape { *unsafe { Box::from_raw(pointer as *mut Escape) } }
unsafe fn borrowed(pointer: Handle)->Value {
    match pointer.is_null() { true=>Value::Nil, false=>unsafe { (*pointer.cast::<Value>()).clone() } }
}
unsafe extern "C" fn duplicate(pointer: Handle)->Handle { hold_object(unsafe { borrowed(pointer) }) }
unsafe extern "C" fn discard(pointer: Handle) {
    if pointer.is_null() { return; }
    let object = unsafe { own_object(pointer) }; drop(object);
}
unsafe extern "C" fn discard_error(pointer: Handle) {
    if !pointer.is_null() { drop(unsafe { own_error(pointer) }); }
}
fn respond(context: Handle, slot: Failure, task: impl FnOnce(&mut Machine<'_>)->Result<Value,Escape>)->Handle {
    if slot.is_null() { return null_mut(); }
    unsafe { slot.write(null_mut()); }
    let outcome = match context.is_null() {
        true=>Err(Escape::Error("RuntimeError: datetime C API requires an active context".to_owned())),
        false=>match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| task(unsafe { &mut *context.cast::<Machine<'_>>() }))) {
            Ok(answer)=>answer,
            Err(_)=>Err(Escape::Error("SystemError: panic in datetime C API".to_owned())),
        },
    };
    if let Ok(object)=outcome { return hold_object(object); }
    let Err(error)=outcome else { unreachable!() };
    unsafe { slot.write(Box::into_raw(Box::new(error)).cast()); }
    null_mut()
}
fn integers(input: impl IntoIterator<Item=i32>)->Vec<Value> { input.into_iter().map(|n|Value::Small(n.into())).collect() }
fn keyword(key: &str, item: Value)->Value { Value::Couple(Rc::new((Value::text(key),item))) }
unsafe extern "C" fn make_date(ctx: Handle, of: Handle, y: i32, m: i32, d: i32, fault: Failure)->Handle {
    respond(ctx,fault,|vm| vm.apply_class_member(unsafe { borrowed(of) },integers([y,m,d])))
}
unsafe extern "C" fn make_datetime(ctx: Handle, of: Handle, y: i32, m: i32, d: i32, h: i32, minute: i32, sec: i32, usec: i32, tz: Handle, fold: i32, fault: Failure)->Handle {
    respond(ctx,fault,|vm| {
        let tail=[unsafe { borrowed(tz) },keyword("fold",Value::Small(fold.into()))];
        let args=integers([y,m,d,h,minute,sec,usec]).into_iter().chain(tail).collect();
        vm.apply_class_member(unsafe { borrowed(of) },args)
    })
}
unsafe extern "C" fn make_time(ctx: Handle, of: Handle, h: i32, m: i32, s: i32, us: i32, tz: Handle, fold: i32, fault: Failure)->Handle {
    respond(ctx,fault,|vm| {
        let mut argv=integers([h,m,s,us]);
        argv.extend([unsafe { borrowed(tz) },keyword("fold",Value::Small(fold.into()))]);
        vm.apply_class_member(unsafe { borrowed(of) },argv)
    })
}
unsafe extern "C" fn make_duration(ctx: Handle, of: Handle, d: i32, s: i32, us: i32, fault: Failure)->Handle {
    respond(ctx,fault,|vm| vm.apply_class_member(unsafe { borrowed(of) },integers([d,s,us])))
}
unsafe extern "C" fn make_zone(ctx: Handle, of: Handle, offset: Handle, name: Handle, fault: Failure)->Handle {
    respond(ctx,fault,|vm| {
        let mut argv=Vec::with_capacity(2);
        argv.push(unsafe { borrowed(offset) });
        if !name.is_null() { argv.push(unsafe { borrowed(name) }); }
        vm.apply_class_member(unsafe { borrowed(of) },argv)
    })
}
unsafe extern "C" fn make_timestamp(ctx: Handle, of: Handle, args: Handle, kwargs: Handle, fault: Failure)->Handle {
    respond(ctx,fault,|vm| {
        let mut argv=match unsafe { borrowed(args) }.settled() {
            Value::Tuple(items)=>items.to_vec(),
            _=>return Err("TypeError: timestamp arguments must be a tuple".to_owned().into()),
        };
        if !kwargs.is_null() {
            match unsafe { borrowed(kwargs) }.settled() {
                Value::Dict(pairs)=>for (k,v) in pairs.iter() {
                    match k { Value::Text(k)=>argv.push(keyword(k,v.clone())), _=>return Err("TypeError: keywords must be strings".to_owned().into()) }
                },
                _=>return Err("TypeError: timestamp keywords must be a dictionary".to_owned().into()),
            }
        }
        let constructor=vm.read_class_member(unsafe { borrowed(of) },"fromtimestamp",false)?;
        vm.apply_class_member(constructor,argv)
    })
}
unsafe extern "C" fn read_attribute(ctx: Handle, object: Handle, word: *const c_char, fault: Failure)->Handle {
    respond(ctx,fault,|vm| {
        if word.is_null() { return Err("TypeError: attribute name cannot be null".to_owned().into()); }
        match unsafe { CStr::from_ptr(word) }.to_str() {
            Ok(word)=>vm.read_class_member(unsafe { borrowed(object) },word,false),
            Err(_)=>Err("UnicodeError: attribute name must be UTF-8".to_owned().into()),
        }
    })
}
impl Drop for CalendarApi {
    fn drop(&mut self) {
        let handles=[self.utc_object,self.timezone_class,self.tzinfo_class,self.duration_class,self.time_class,self.datetime_class,self.date_class];
        handles.into_iter().for_each(|pointer|unsafe { discard(pointer) });
    }
}
unsafe extern "C" fn dispose(pointer: Handle) { let table=unsafe { Box::from_raw(pointer as *mut CalendarApi) }; drop(table); }
pub(crate) fn make_capsule(objects: Vec<Value>)->Result<Value,Escape> {
    let [date,datetime,time,delta,tzinfo,timezone,utc]: [Value;7]=objects.into_iter().map(|object| object.settled()).collect::<Vec<_>>().try_into().map_err(|_|Escape::Error("TypeError: datetime C API requires six classes and UTC".to_owned()))?;
    let table=CalendarApi {
        revision:1, bytes:std::mem::size_of::<CalendarApi>(),
        date_class:hold_object(date), datetime_class:hold_object(datetime), time_class:hold_object(time), duration_class:hold_object(delta),
        tzinfo_class:hold_object(tzinfo), timezone_class:hold_object(timezone), utc_object:hold_object(utc),
        copy_object:duplicate, drop_object:discard, drop_error:discard_error,
        new_date:make_date, new_datetime:make_datetime, new_time:make_time, new_duration:make_duration,
        new_timezone:make_zone, timestamp:make_timestamp, get_attribute:read_attribute,
    };
    let memory=Box::into_raw(Box::new(table)).cast();
    let label=Some(CString::new(API_NAME).unwrap());
    let capsule=unsafe { crate::capsule::PointerCapsule::take(memory,label,Some(dispose)) }.expect("allocated C table has an address");
    Ok(Value::Opaque(Rc::new(capsule)))
}

#[cfg(test)]
mod checks {
    use super::*;
    mod library { include!("../../../langs/lib_python/modules/manifest.rs"); }
    fn get(vm:&mut Machine<'_>, object:&Value, word:&str)->Value {
        match vm.read_class_member(object.clone(),word,false) { Ok(value)=>value.settled(), Err(_)=>panic!("attribute {word}") }
    }
    fn receive(object:Handle, error:Handle)->Value {
        assert!(!object.is_null() && error.is_null(), "C constructor failed"); unsafe { own_object(object) }
    }
    #[test]
    fn calendar_abi_owns_and_constructs() {
        let table=crate::table::Table::parse(include_str!("../../../langs/python.json")).unwrap();
        let mut names=table.strings("ext.system.module.name").to_vec();
        for key in ["ext.system.args.list","ext.system.args.count"] { if let Some(name)=table.single(key) { names.push(name.to_string()); } }
        let mut vm=Machine::new(&table,names);
        table.strings("ext.system.module.name").iter().for_each(|name| vm.define(name,Value::text("__main__")));
        if let Some(name)=table.single("ext.system.args.list") { vm.define(name,Value::Vector(crate::tuples::Sequence::plain(vec![Value::text("native-capi")]))); }
        if let Some(name)=table.single("ext.system.args.count") { vm.define(name,Value::Small(1)); }
        vm.register_main_module();
        for (name,contents,file) in library::MODULES {
            vm.library_sources.insert(name.to_string(),contents.to_string());
            vm.library_files.insert(name.to_string(),format!("langs/lib_python/modules/{file}"));
        }
        for (module,alias) in library::MODULE_ALIASES { vm.library_aliases.insert((*module).into(),(*alias).into()); }
        let namespace=vm.load_namespace("_pydatetime").unwrap_or_else(|message|panic!("{message}"));
        let originals=["date","datetime","time","timedelta","tzinfo","timezone","UTC"].map(|word|get(&mut vm,&namespace,word));
        let capsule=make_capsule(originals.to_vec()).unwrap_or_else(|_|panic!("C table initialization"));
        let pointer=match &capsule {
            Value::Opaque(resource)=>resource.find_pointer(Some(&CString::new(API_NAME).unwrap())).unwrap(),
            _=>panic!("expected native resource"),
        };
        let api=unsafe { &*pointer.cast::<CalendarApi>() };
        assert_eq!((api.revision,api.bytes),(1,std::mem::size_of::<CalendarApi>()));
        let mut error=null_mut();
        let object=lend_machine(&mut vm,|context|unsafe { (api.new_date)(context,api.date_class,2024,2,29,&mut error) });
        let day=receive(object,error);
        let kind=get(&mut vm,&day,"__class__");
        match (&kind,&originals[0]) { (Value::Blueprint(x),Value::Blueprint(y))=>assert!(Rc::ptr_eq(x,y)), _=>panic!("class identity") }
        assert!(matches!(get(&mut vm,&day,"day"),Value::Small(29)));
        let object=lend_machine(&mut vm,|context|unsafe { (api.new_time)(context,api.time_class,3,4,5,6,null_mut(),1,&mut error) });
        let clock=receive(object,error);
        assert!(matches!(get(&mut vm,&clock,"microsecond"),Value::Small(6)));
        let object=lend_machine(&mut vm,|context|unsafe { (api.new_datetime)(context,api.datetime_class,2024,2,29,23,45,6,123456,api.utc_object,1,&mut error) });
        let dateclock=receive(object,error);
        assert!(matches!(get(&mut vm,&dateclock,"fold"),Value::Small(1)));
        match (get(&mut vm,&dateclock,"tzinfo"),&originals[6]) { (Value::Thing(x),Value::Thing(y))=>assert!(Rc::ptr_eq(&x,y)), _=>panic!("UTC identity") }
        let object=lend_machine(&mut vm,|context|unsafe { (api.new_duration)(context,api.duration_class,0,86401,-1,&mut error) });
        let duration=receive(object,error);
        for (name,expected) in [("days",1),("seconds",0),("microseconds",999999)] {
            assert!(matches!(get(&mut vm,&duration,name),Value::Small(n) if n==expected));
        }
        let offset=hold_object(duration);
        let invalid=lend_machine(&mut vm,|context|unsafe { (api.new_timezone)(context,api.timezone_class,offset,null_mut(),&mut error) });
        assert!(invalid.is_null() && !error.is_null());
        assert!(matches!(unsafe { own_error(error) },Escape::Thrown(_)));
        unsafe { (api.drop_object)(offset); }
        let offset=lend_machine(&mut vm,|context|unsafe { (api.new_duration)(context,api.duration_class,0,0,0,&mut error) });
        assert!(error.is_null());
        let object=lend_machine(&mut vm,|context|unsafe { (api.new_timezone)(context,api.timezone_class,offset,null_mut(),&mut error) });
        let zone=receive(object,error); unsafe { (api.drop_object)(offset); }
        match (&zone,&originals[6]) { (Value::Thing(x),Value::Thing(y))=>assert!(Rc::ptr_eq(x,y)), _=>panic!("zero-offset singleton") }
        let argv=hold_object(Value::tuple(vec![Value::Small(0),originals[6].clone()]));
        let object=lend_machine(&mut vm,|context|unsafe { (api.timestamp)(context,api.datetime_class,argv,null_mut(),&mut error) });
        let epoch=receive(object,error); unsafe { (api.drop_object)(argv); }
        assert!(matches!(get(&mut vm,&epoch,"year"),Value::Small(1970)));
        let missing=lend_machine(&mut vm,|context|unsafe { (api.new_date)(context,api.date_class,2023,2,29,&mut error) });
        assert!(missing.is_null());
        let exception=match unsafe { own_error(error) } { Escape::Thrown(value)=>value, _=>panic!("exception payload lost") };
        let kind=get(&mut vm,&exception,"__class__"); assert_eq!(get(&mut vm,&kind,"__name__").bare(),"ValueError");
        let handle=hold_object(day.clone());
        let name=CString::new("year").unwrap();
        let object=lend_machine(&mut vm,|context|unsafe { (api.get_attribute)(context,handle,name.as_ptr(),&mut error) });
        assert!(matches!(receive(object,error),Value::Small(2024)));
        let second=unsafe { (api.copy_object)(handle) }; unsafe { (api.drop_object)(handle); }
        let absent=unsafe { (api.new_date)(null_mut(),api.date_class,2000,1,1,&mut error) };
        assert!(absent.is_null()); assert!(matches!(unsafe { own_error(error) },Escape::Error(_)));
        drop(capsule);
        match (unsafe { own_object(second) },day) { (Value::Thing(x),Value::Thing(y))=>assert!(Rc::ptr_eq(&x,&y)), _=>panic!("retained object") }
    }
}
