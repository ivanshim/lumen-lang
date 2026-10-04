use crate::data::Value;

pub(crate) fn perform(values: &[Value]) -> Result<Value, String> {
    let step = values.first().ok_or("TypeError: expected a filesystem action")?.bare();
    let path = values.get(1).ok_or("TypeError: expected a filesystem path")?.bare();
    let integer = |at: usize| -> Result<i32, String> {
        values.get(at).ok_or_else(|| "TypeError: missing numeric host argument".to_owned())?.as_big()?.to_string().parse().map_err(|_| "OverflowError: host argument outside integer range".to_owned())
    };
    let result = match step.as_str() {
        "flags" => {
            let directory = match (std::env::consts::OS, std::env::consts::ARCH) {
                ("linux", "aarch64" | "arm") => 0x4000,
                ("linux", "x86_64" | "x86") => 0x10000,
                _ => return Err(String::from("NotImplementedError: filesystem flags for this host are unsupported")),
            };
            Ok(Value::tuple(vec![Value::Small(directory), Value::Small(directory * 2)]))
        }
        "stat" | "lstat" => {
            let found = match step.as_str() { "lstat" => std::fs::symlink_metadata(&path), _ => std::fs::metadata(&path) };
            found.map(|info| {
                use std::os::unix::fs::MetadataExt as UnixMetadata;
                let fields = [i64::from(info.mode()), info.ino() as i64, info.dev() as i64, info.nlink() as i64,
                    i64::from(info.uid()), i64::from(info.gid()), info.size() as i64, info.atime(), info.mtime(), info.ctime(),
                    info.atime_nsec(), info.mtime_nsec(), info.ctime_nsec()];
                Value::tuple(fields.iter().copied().map(Value::Small).collect())
            })
        }
        "rmdir" => std::fs::remove_dir(path).map(|_| Value::Nil),
        "readlink" => std::fs::read_link(path).map(|destination| Value::text(destination.to_string_lossy().as_ref())),
        "access" => {
            extern "C" { fn access(file: *const std::ffi::c_char, bits: std::ffi::c_int) -> std::ffi::c_int; }
            let spelling = std::ffi::CString::new(path).map_err(|_| String::from("ValueError: embedded null byte"))?;
            let allowed = unsafe { access(spelling.as_ptr(), integer(2)?) };
            Ok(Value::Flag(allowed == 0))
        }
        "open" => {
            extern "C" { fn open(file: *const std::ffi::c_char, options: std::ffi::c_int, ...) -> std::ffi::c_int; }
            let spelling = std::ffi::CString::new(path).map_err(|_| String::from("ValueError: embedded null byte"))?;
            let options = integer(2)?;
            let permissions = integer(3)? as u32;
            let handle = unsafe { open(spelling.as_ptr(), options | 0x80000, permissions) };
            if handle == -1 { Err(std::io::Error::last_os_error()) } else { Ok(Value::Small(handle.into())) }
        }
        "read" | "write" | "seek" | "truncate" => {
            extern "C" {
                fn read(handle: std::ffi::c_int, into: *mut u8, count: usize) -> isize;
                fn write(handle: std::ffi::c_int, from: *const u8, count: usize) -> isize;
                fn lseek(handle: std::ffi::c_int, shift: i64, from: std::ffi::c_int) -> i64;
                fn ftruncate(handle: std::ffi::c_int, length: i64) -> std::ffi::c_int;
            }
            let fd = integer(1)?;
            if step == "read" {
                let wanted = values.get(2).ok_or("TypeError: missing read count")?.as_big()?.to_string().parse::<usize>().map_err(|_| "OverflowError: read count outside size range")?;
                let mut bytes = Vec::new();
                bytes.try_reserve_exact(wanted).map_err(|_| "MemoryError: read allocation failed")?;
                bytes.resize(wanted, 0);
                let got = loop {
                    let n = unsafe { read(fd, bytes.as_mut_ptr(), wanted) };
                    if n != -1 { break Ok(n as usize); }
                    let why = std::io::Error::last_os_error();
                    if why.raw_os_error() != Some(4) { break Err(why); }
                };
                got.map(|amount| {
                    bytes.truncate(amount);
                    Value::Octets { cell: std::rc::Rc::new(std::cell::RefCell::new(bytes)), changeable: false, lead: std::rc::Rc::from("b") }
                })
            } else if step == "write" {
                let buffer = values.get(2).ok_or("TypeError: missing write data")?.settled();
                match buffer {
                    Value::Octets { cell, .. } => {
                        let data = cell.borrow();
                        loop {
                            let used = unsafe { write(fd, data.as_ptr(), data.len()) };
                            if used != -1 { break Ok(Value::Small(used as i64)); }
                            let error = std::io::Error::last_os_error();
                            if error.raw_os_error() != Some(4) { break Err(error); }
                        }
                    },
                    _ => return Err(String::from("TypeError: a bytes-like object is required")),
                }
            } else if step == "seek" {
                let displacement = values.get(2).ok_or("TypeError: missing seek offset")?.as_big()?.to_string().parse::<i64>().map_err(|_| "OverflowError: seek offset outside file range")?;
                let origin = integer(3)?;
                match unsafe { lseek(fd, displacement, origin) } {
                    -1 => Err(std::io::Error::last_os_error()), position => Ok(Value::Small(position)),
                }
            } else {
                let amount = values.get(2).ok_or("TypeError: missing truncate size")?.as_big()?.to_string().parse::<i64>().map_err(|_| "OverflowError: truncate size outside file range")?;
                match unsafe { ftruncate(fd, amount) } {
                    0 => Ok(Value::Nil), _ => Err(std::io::Error::last_os_error()),
                }
            }
        }
        "fd_mode" => {
            use std::{mem::ManuallyDrop, os::{fd::FromRawFd, unix::fs::MetadataExt as ModeBits}};
            let handle = integer(1)?;
            let file_view = unsafe { std::fs::File::from_raw_fd(handle) };
            let retained = ManuallyDrop::new(file_view);
            match retained.metadata() {
                Ok(info) => Ok(Value::Small(info.mode().into())),
                Err(failure) => Err(failure),
            }
        }
        "fd_flags" => {
            extern "C" { fn fcntl(descriptor: std::ffi::c_int, request: std::ffi::c_int, ...) -> std::ffi::c_int; }
            let bits = unsafe { fcntl(integer(1)?, 3) };
            if bits == -1 { Err(std::io::Error::last_os_error()) } else { Ok(Value::Small(bits.into())) }
        }
        "get_inheritable" | "set_inheritable" => {
            extern "C" { fn fcntl(handle: std::ffi::c_int, action: std::ffi::c_int, ...) -> std::ffi::c_int; }
            let handle = integer(1)?;
            let current = unsafe { fcntl(handle, 1) };
            match current {
                -1 => Err(std::io::Error::last_os_error()),
                bits if step == "get_inheritable" => Ok(Value::Flag(bits & 1 != 1)),
                bits => {
                    let wanted = values.get(2).ok_or("TypeError: missing inheritable flag")?.is_true();
                    let changed = (bits & !1) | i32::from(!wanted);
                    match unsafe { fcntl(handle, 2, changed) } {
                        0 => Ok(Value::Nil), _ => Err(std::io::Error::last_os_error()),
                    }
                }
            }
        }
        "pipe" => {
            extern "C" { fn pipe2(handles: *mut std::ffi::c_int, options: std::ffi::c_int) -> std::ffi::c_int; }
            let mut handles = [0, 0];
            match unsafe { pipe2(handles.as_mut_ptr(), 0x80000) } {
                0 => Ok(Value::tuple(vec![Value::Small(handles[0].into()), Value::Small(handles[1].into())])),
                _ => Err(std::io::Error::last_os_error()),
            }
        }
        "isatty" => {
            extern "C" { fn isatty(descriptor: std::ffi::c_int) -> std::ffi::c_int; }
            Ok(Value::Flag(unsafe { isatty(integer(1)?) } == 1))
        }
        "close" => {
            extern "C" { fn close(handle: std::ffi::c_int) -> std::ffi::c_int; }
            let handle = values[1].as_big()?.to_string().parse().map_err(|_| String::from("OverflowError: descriptor outside integer range"))?;
            if unsafe { close(handle) } == -1 { Err(std::io::Error::last_os_error()) } else { Ok(Value::Nil) }
        }
        _ => return Err(String::from("NotImplementedError: unknown host filesystem action")),
    };
    let answer = match result {
        Ok(got) => vec![got, Value::Nil],
        Err(fault) => {
            let reason = vec![Value::Small(fault.raw_os_error().unwrap_or(5).into()), Value::text(&fault.to_string())];
            vec![Value::Nil, Value::tuple(reason)]
        }
    };
    Ok(Value::tuple(answer))
}
