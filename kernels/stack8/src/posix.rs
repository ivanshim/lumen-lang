use crate::value::Value;

pub(crate) fn operate(args: &[Value]) -> Result<Value, String> {
    let command = args.first().ok_or("TypeError: host operation is required")?.plain();
    let file = args.get(1).ok_or("TypeError: host path is required")?.plain();
    let outcome: std::io::Result<Value> = match command.as_str() {
        "flags" => {
            let bits = if cfg!(all(target_os = "linux", any(target_arch = "aarch64", target_arch = "arm"))) { [16384, 32768] }
                else if cfg!(all(target_os = "linux", any(target_arch = "x86_64", target_arch = "x86"))) { [65536, 131072] }
                else { return Err("NotImplementedError: host open flags are unavailable".into()); };
            Ok(Value::tuple(bits.into_iter().map(Value::Small).collect()))
        }
        "stat" | "lstat" => {
            let metadata = if command == "stat" { std::fs::metadata(&file) } else { std::fs::symlink_metadata(&file) };
            metadata.map(|data| {
                use std::os::unix::fs::MetadataExt;
                let mut row = vec![data.mode() as i64, data.ino() as i64, data.dev() as i64, data.nlink() as i64, data.uid() as i64, data.gid() as i64, data.size() as i64];
                row.extend([data.atime(), data.mtime(), data.ctime(), data.atime_nsec(), data.mtime_nsec(), data.ctime_nsec()]);
                Value::tuple(row.into_iter().map(Value::Small).collect())
            })
        }
        "rmdir" => std::fs::remove_dir(&file).map(|()| Value::Null),
        "readlink" => std::fs::read_link(&file).map(|path| Value::text(&path.to_string_lossy())),
        "access" => {
            extern "C" { fn access(path: *const std::ffi::c_char, mode: std::ffi::c_int) -> std::ffi::c_int; }
            let name = std::ffi::CString::new(file).map_err(|_| "ValueError: embedded null byte")?;
            let mode = args.get(2).ok_or("TypeError: access mode is required")?.as_big()?.to_string().parse::<i32>().map_err(|_| "OverflowError: mode does not fit an int")?;
            Ok(Value::Flag(unsafe { access(name.as_ptr(), mode) } == 0))
        }
        "open" => {
            extern "C" { fn open(path: *const std::ffi::c_char, flags: std::ffi::c_int, ...) -> std::ffi::c_int; }
            let name = std::ffi::CString::new(file).map_err(|_| "ValueError: embedded null byte")?;
            let flags = args.get(2).ok_or("TypeError: open flags are required")?.as_big()?.to_string().parse::<i32>().map_err(|_| "OverflowError: flags do not fit an int")?;
            let mode = args.get(3).ok_or("TypeError: creation mode is required")?.as_big()?.to_string().parse::<u32>().map_err(|_| "OverflowError: mode does not fit an unsigned int")?;
            let fd = unsafe { open(name.as_ptr(), flags | 524288, mode) };
            if fd < 0 { Err(std::io::Error::last_os_error()) } else { Ok(Value::Small(i64::from(fd))) }
        }
        "close" => {
            extern "C" { fn close(fd: std::ffi::c_int) -> std::ffi::c_int; }
            let descriptor = file.parse::<i32>().map_err(|_| "TypeError: descriptor must be an integer")?;
            if unsafe { close(descriptor) } < 0 { Err(std::io::Error::last_os_error()) } else { Ok(Value::Null) }
        }
        "isatty" => {
            // Whether a descriptor answers a terminal; a number that
            // names none reads false, as the C isatty answers 0.
            extern "C" { fn isatty(fd: std::ffi::c_int) -> std::ffi::c_int; }
            let descriptor = file.parse::<i32>().map_err(|_| "TypeError: descriptor must be an integer")?;
            Ok(Value::Flag(unsafe { isatty(descriptor) } == 1))
        }
        _ => return Err("NotImplementedError: unsupported host filesystem operation".into()),
    };
    Ok(match outcome {
        Ok(value) => Value::tuple(vec![value, Value::Null]),
        Err(error) => Value::tuple(vec![Value::Null, Value::tuple(vec![Value::Small(i64::from(error.raw_os_error().unwrap_or(5))), Value::text(&error.to_string())])]),
    })
}
