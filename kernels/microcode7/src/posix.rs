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
