//! Versioned C interface to the live Python datetime classes, using owned lumen handles.
use std::ffi::{c_void, CStr, CString};
use std::ptr;
use std::rc::Rc;
use crate::{engine::{Engine, Fault}, value::Value};

type Object = *mut c_void;
type ErrorOut = *mut Object;
pub const CAPSULE_NAME: &str = "lumen.datetime_CAPI.v1";

/// See langs/python_datetime_capi.h. This is not the CPython PyObject ABI.
#[repr(C)]
pub struct DateTimeApi {
    pub version: u64, pub size: usize,
    pub date_type: Object, pub datetime_type: Object, pub time_type: Object,
    pub delta_type: Object, pub tzinfo_type: Object, pub timezone_type: Object, pub utc: Object,
    pub retain: unsafe extern "C" fn(Object) -> Object,
    pub release: unsafe extern "C" fn(Object),
    pub release_error: unsafe extern "C" fn(Object),
    pub date: unsafe extern "C" fn(Object, Object, i32, i32, i32, ErrorOut) -> Object,
    pub datetime: unsafe extern "C" fn(Object, Object, i32, i32, i32, i32, i32, i32, i32, Object, i32, ErrorOut) -> Object,
    pub time: unsafe extern "C" fn(Object, Object, i32, i32, i32, i32, Object, i32, ErrorOut) -> Object,
    pub delta: unsafe extern "C" fn(Object, Object, i32, i32, i32, ErrorOut) -> Object,
    pub timezone: unsafe extern "C" fn(Object, Object, Object, Object, ErrorOut) -> Object,
    pub from_timestamp: unsafe extern "C" fn(Object, Object, Object, Object, ErrorOut) -> Object,
    pub attribute: unsafe extern "C" fn(Object, Object, *const std::ffi::c_char, ErrorOut) -> Object,
}

/// Export one owned handle. Retaining a handle preserves the original Python identity.
pub fn export(value: Value) -> Object { Box::into_raw(Box::new(value)).cast() }
/// # Safety
/// Consume an owned, unreleased object handle made by this kernel.
pub unsafe fn take(value: Object) -> Value { *unsafe { Box::from_raw(value.cast::<Value>()) } }
/// # Safety
/// Consume an owned, unreleased error handle made by this kernel.
pub unsafe fn take_error(error: Object) -> Fault { *unsafe { Box::from_raw(error.cast::<Fault>()) } }
/// The context is borrowed for this call only, on this thread. C code must not retain it.
pub fn with_context<T>(engine: &mut Engine<'_>, body: impl FnOnce(Object) -> T) -> T {
    body((engine as *mut Engine<'_>).cast())
}
unsafe fn read(handle: Object) -> Value {
    if handle.is_null() { Value::Null } else { unsafe { &*handle.cast::<Value>() }.clone() }
}
unsafe extern "C" fn retain(handle: Object) -> Object { export(unsafe { read(handle) }) }
unsafe extern "C" fn release(handle: Object) { if !handle.is_null() { drop(unsafe { take(handle) }); } }
unsafe extern "C" fn release_error(handle: Object) { if !handle.is_null() { drop(unsafe { take_error(handle) }); } }

fn answer(context: Object, error: ErrorOut, work: impl FnOnce(&mut Engine<'_>) -> Result<Value, Fault>) -> Object {
    if error.is_null() { return ptr::null_mut(); }
    unsafe { *error = ptr::null_mut(); }
    let result = if context.is_null() { Err(Fault::Note("RuntimeError: datetime C API requires an active context".into())) }
        else { std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| work(unsafe { &mut *context.cast::<Engine<'_>>() })))
            .unwrap_or_else(|_| Err(Fault::Note("SystemError: panic in datetime C API".into()))) };
    match result {
        Ok(value) => export(value),
        Err(fault) => { unsafe { *error = Box::into_raw(Box::new(fault)).cast(); } ptr::null_mut() }
    }
}
fn numbers(values: &[i32]) -> Vec<Value> { values.iter().map(|value| Value::Small(i64::from(*value))).collect() }
fn named(name: &str, value: Value) -> Value { Value::Tie(Rc::new((Value::text(name), value))) }
unsafe extern "C" fn date(context: Object, class: Object, year: i32, month: i32, day: i32, error: ErrorOut) -> Object {
    answer(context, error, |engine| engine.class_apply(unsafe { read(class) }, numbers(&[year, month, day])))
}
unsafe extern "C" fn datetime(context: Object, class: Object, year: i32, month: i32, day: i32, hour: i32, minute: i32, second: i32, micros: i32, zone: Object, fold: i32, error: ErrorOut) -> Object {
    answer(context, error, |engine| {
        let mut args = numbers(&[year, month, day, hour, minute, second, micros]);
        args.push(unsafe { read(zone) }); args.push(named("fold", Value::Small(i64::from(fold))));
        engine.class_apply(unsafe { read(class) }, args)
    })
}
unsafe extern "C" fn time(context: Object, class: Object, hour: i32, minute: i32, second: i32, micros: i32, zone: Object, fold: i32, error: ErrorOut) -> Object {
    answer(context, error, |engine| {
        let mut args = numbers(&[hour, minute, second, micros]);
        args.push(unsafe { read(zone) }); args.push(named("fold", Value::Small(i64::from(fold))));
        engine.class_apply(unsafe { read(class) }, args)
    })
}
unsafe extern "C" fn delta(context: Object, class: Object, days: i32, seconds: i32, micros: i32, error: ErrorOut) -> Object {
    answer(context, error, |engine| engine.class_apply(unsafe { read(class) }, numbers(&[days, seconds, micros])))
}
unsafe extern "C" fn timezone(context: Object, class: Object, offset: Object, name: Object, error: ErrorOut) -> Object {
    answer(context, error, |engine| {
        let mut args = vec![unsafe { read(offset) }];
        if !name.is_null() { args.push(unsafe { read(name) }); }
        engine.class_apply(unsafe { read(class) }, args)
    })
}
unsafe extern "C" fn from_timestamp(context: Object, class: Object, positional: Object, keywords: Object, error: ErrorOut) -> Object {
    answer(context, error, |engine| {
        let Value::Tuple(items) = unsafe { read(positional) }.contents() else { return Err("TypeError: timestamp arguments must be a tuple".into()); };
        let mut args = items.to_vec();
        if !keywords.is_null() {
            let Value::Map(entries) = unsafe { read(keywords) }.contents() else { return Err("TypeError: timestamp keywords must be a dictionary".into()); };
            for (key, value) in entries.iter() {
                let Value::Text(key) = key else { return Err("TypeError: keywords must be strings".into()); };
                args.push(named(key, value.clone()));
            }
        }
        let method = engine.class_get(unsafe { read(class) }, "fromtimestamp", false)?;
        engine.class_apply(method, args)
    })
}
unsafe extern "C" fn attribute(context: Object, value: Object, name: *const std::ffi::c_char, error: ErrorOut) -> Object {
    answer(context, error, |engine| {
        if name.is_null() { return Err("TypeError: attribute name cannot be null".into()); }
        let name = unsafe { CStr::from_ptr(name) }.to_str().map_err(|_| Fault::Note("UnicodeError: attribute name must be UTF-8".into()))?;
        engine.class_get(unsafe { read(value) }, name, false)
    })
}
impl Drop for DateTimeApi {
    fn drop(&mut self) {
        for handle in [self.date_type, self.datetime_type, self.time_type, self.delta_type, self.tzinfo_type, self.timezone_type, self.utc] { unsafe { release(handle); } }
    }
}
unsafe extern "C" fn dispose_api(pointer: Object) { drop(unsafe { Box::from_raw(pointer.cast::<DateTimeApi>()) }); }
pub(crate) fn capsule(classes: Vec<Value>) -> Result<Value, Fault> {
    if classes.len() != 7 { return Err("TypeError: datetime C API requires six classes and UTC".into()); }
    let mut values = classes.into_iter().map(|value| export(value.contents()));
    let api = Box::new(DateTimeApi {
        version: 1, size: std::mem::size_of::<DateTimeApi>(),
        date_type: values.next().unwrap(), datetime_type: values.next().unwrap(), time_type: values.next().unwrap(),
        delta_type: values.next().unwrap(), tzinfo_type: values.next().unwrap(), timezone_type: values.next().unwrap(), utc: values.next().unwrap(),
        retain, release, release_error, date, datetime, time, delta, timezone, from_timestamp, attribute,
    });
    let pointer = Box::into_raw(api).cast();
    let name = CString::new(CAPSULE_NAME).unwrap();
    let resource = unsafe { crate::capsule::NativeCapsule::new(pointer, Some(&name), Some(dispose_api)) }.expect("owned API pointer is non-null");
    Ok(Value::Capsule(Rc::new(resource)))
}

#[cfg(test)]
mod tests {
    use super::*;
    mod sources { include!("../../../langs/lib_python/modules/manifest.rs"); }
    fn field(engine: &mut Engine<'_>, object: &Value, key: &str) -> Value {
        engine.class_get(object.clone(), key, false).unwrap_or_else(|_| panic!("missing {key}")).contents()
    }
    fn returned(pointer: Object, error: Object) -> Value {
        assert!(error.is_null(), "constructor raised"); assert!(!pointer.is_null()); unsafe { take(pointer) }
    }
    #[test]
    fn datetime_c_interface_constructs_live_objects() {
        let lang = crate::lang::Lang::parse(include_str!("../../../langs/python.json")).unwrap();
        let mut registry = crate::compile::Registry::default();
        for name in lang.module_names.iter().chain(lang.args_list.iter()).chain(lang.args_count.iter()) { registry.slot(name); }
        let mut engine = Engine::new(&lang, registry);
        for name in &lang.module_names { engine.define(name, Value::text("__main__")); }
        if let Some(name) = &lang.args_list { engine.define(name, Value::array(vec![Value::text("native-capi")])); }
        if let Some(name) = &lang.args_count { engine.define(name, Value::Small(1)); }
        engine.register_main_module();
        for &(name, source, path) in sources::MODULES {
            engine.module_sources.insert(name.into(), source.into());
            engine.module_files.insert(name.into(), format!("langs/lib_python/modules/{path}"));
        }
        engine.module_aliases.extend(sources::MODULE_ALIASES.iter().map(|(name, member)| (name.to_string(), member.to_string())));
        let module = engine.import_module("_pydatetime").unwrap_or_else(|fault| match fault { Fault::Note(message) | Fault::Stopped(message) => panic!("{message}"), Fault::Thrown(Value::Object(object)) => panic!("import threw {} {:?}", object.class_now().name, object.fields.borrow().iter().filter(|(name,_)| name == "\0arguments" || name == "\0diagnostic").map(|(name,value)|(name.clone(),value.plain())).collect::<Vec<_>>()), Fault::Thrown(value) => panic!("import threw {}", value.plain()), Fault::Finished => panic!("import exited") });
        let mut classes = Vec::new();
        for name in ["date", "datetime", "time", "timedelta", "tzinfo", "timezone", "UTC"] { classes.push(field(&mut engine, &module, name)); }
        let held = capsule(classes.clone()).unwrap_or_else(|_| panic!("capsule construction"));
        let Value::Capsule(resource) = &held else { panic!("not native capsule"); };
        let label = CString::new(CAPSULE_NAME).unwrap();
        let api = unsafe { &*resource.pointer(Some(&label)).unwrap().as_ptr().cast::<DateTimeApi>() };
        assert_eq!(api.version, 1); assert_eq!(api.size, std::mem::size_of::<DateTimeApi>());
        let mut error = ptr::null_mut();
        let date = returned(with_context(&mut engine, |ctx| unsafe { (api.date)(ctx, api.date_type, 2024, 2, 29, &mut error) }), error);
        let actual = field(&mut engine, &date, "__class__");
        let (Value::Class(actual), Value::Class(expected)) = (&actual, &classes[0]) else { panic!("type bridge"); };
        assert!(Rc::ptr_eq(actual, expected));
        assert!(matches!(field(&mut engine, &date, "day"), Value::Small(29)));
        let datetime = returned(with_context(&mut engine, |ctx| unsafe { (api.datetime)(ctx, api.datetime_type, 2024, 2, 29, 23, 45, 6, 123456, api.utc, 1, &mut error) }), error);
        assert!(matches!(field(&mut engine, &datetime, "fold"), Value::Small(1)));
        let zone = field(&mut engine, &datetime, "tzinfo");
        let (Value::Object(a), Value::Object(b)) = (&zone, &classes[6]) else { panic!("UTC bridge"); }; assert!(Rc::ptr_eq(a, b));
        let time = returned(with_context(&mut engine, |ctx| unsafe { (api.time)(ctx, api.time_type, 3, 4, 5, 6, ptr::null_mut(), 1, &mut error) }), error);
        assert!(matches!(field(&mut engine, &time, "microsecond"), Value::Small(6)));
        let duration = returned(with_context(&mut engine, |ctx| unsafe { (api.delta)(ctx, api.delta_type, 0, 86401, -1, &mut error) }), error);
        assert!(matches!(field(&mut engine, &duration, "days"), Value::Small(1)));
        assert!(matches!(field(&mut engine, &duration, "microseconds"), Value::Small(999999)));
        let offset = export(duration);
        let badzone = with_context(&mut engine, |ctx| unsafe { (api.timezone)(ctx, api.timezone_type, offset, ptr::null_mut(), &mut error) });
        assert!(badzone.is_null()); assert!(!error.is_null());
        assert!(matches!(unsafe { take_error(error) }, Fault::Thrown(_))); unsafe { (api.release)(offset); }
        let zero = with_context(&mut engine, |ctx| unsafe { (api.delta)(ctx, api.delta_type, 0, 0, 0, &mut error) }); assert!(error.is_null());
        let utc = returned(with_context(&mut engine, |ctx| unsafe { (api.timezone)(ctx, api.timezone_type, zero, ptr::null_mut(), &mut error) }), error);
        unsafe { (api.release)(zero); }
        let (Value::Object(a), Value::Object(b)) = (&utc, &classes[6]) else { panic!("UTC singleton"); }; assert!(Rc::ptr_eq(a, b));
        let arguments = export(Value::tuple(vec![Value::Small(0), classes[6].clone()]));
        let epoch = returned(with_context(&mut engine, |ctx| unsafe { (api.from_timestamp)(ctx, api.datetime_type, arguments, ptr::null_mut(), &mut error) }), error);
        assert!(matches!(field(&mut engine, &epoch, "year"), Value::Small(1970)));
        unsafe { (api.release)(arguments); }
        let invalid = with_context(&mut engine, |ctx| unsafe { (api.date)(ctx, api.date_type, 2023, 2, 29, &mut error) });
        assert!(invalid.is_null());
        let Fault::Thrown(exception) = (unsafe { take_error(error) }) else { panic!("lost Python exception"); };
        let kind = field(&mut engine, &exception, "__class__");
        assert_eq!(field(&mut engine, &kind, "__name__").plain(), "ValueError");
        let handle = export(date.clone()); let key = CString::new("year").unwrap();
        let year = returned(with_context(&mut engine, |ctx| unsafe { (api.attribute)(ctx, handle, key.as_ptr(), &mut error) }), error);
        assert!(matches!(year, Value::Small(2024)));
        let retained = unsafe { (api.retain)(handle) }; unsafe { (api.release)(handle); }
        let _ = unsafe { (api.date)(ptr::null_mut(), api.date_type, 2000, 1, 1, &mut error) };
        assert!(matches!(unsafe { take_error(error) }, Fault::Note(_)));
        drop(held);
        let kept = unsafe { take(retained) };
        let (Value::Object(one), Value::Object(two)) = (&date, &kept) else { panic!("owned handle"); }; assert!(Rc::ptr_eq(one, two));
    }
}
