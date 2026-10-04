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
        "read" | "write" | "seek" | "truncate" => {
            extern "C" {
                fn read(fd: i32, buffer: *mut std::ffi::c_void, size: usize) -> isize;
                fn write(fd: i32, buffer: *const std::ffi::c_void, size: usize) -> isize;
                fn lseek(fd: i32, offset: i64, whence: i32) -> i64;
                fn ftruncate(fd: i32, size: i64) -> i32;
            }
            let descriptor = file.parse::<i32>().map_err(|_| "TypeError: descriptor must be an integer")?;
            match command.as_str() {
                "read" => {
                    let length = args.get(2).ok_or("TypeError: read size is required")?.as_big()?.to_string().parse::<usize>().map_err(|_| "OverflowError: read size is out of range")?;
                    let mut buffer = Vec::new();
                    buffer.try_reserve_exact(length).map_err(|_| "MemoryError: cannot allocate read buffer")?;
                    buffer.resize(length, 0);
                    let count = loop {
                        let count = unsafe { read(descriptor, buffer.as_mut_ptr().cast(), length) };
                        if count >= 0 { break Ok(count as usize); }
                        let fault = std::io::Error::last_os_error();
                        if fault.kind() != std::io::ErrorKind::Interrupted { break Err(fault); }
                    };
                    count.map(|used| {
                        buffer.truncate(used);
                        Value::Bytes(std::rc::Rc::new(std::cell::RefCell::new(buffer)), false, std::rc::Rc::from("bytes"))
                    })
                }
                "write" => {
                    let contents = args.get(2).ok_or("TypeError: write buffer is required")?.contents();
                    let Value::Bytes(bytes, ..) = contents else { return Err("TypeError: a bytes-like object is required".into()); };
                    let buffer = bytes.borrow();
                    loop {
                        let count = unsafe { write(descriptor, buffer.as_ptr().cast(), buffer.len()) };
                        if count >= 0 { break Ok(Value::Small(count as i64)); }
                        let fault = std::io::Error::last_os_error();
                        if fault.kind() != std::io::ErrorKind::Interrupted { break Err(fault); }
                    }
                }
                "seek" => {
                    let offset = args.get(2).ok_or("TypeError: seek offset is required")?.as_big()?.to_string().parse::<i64>().map_err(|_| "OverflowError: seek offset is out of range")?;
                    let whence = args.get(3).ok_or("TypeError: seek direction is required")?.as_big()?.to_string().parse::<i32>().map_err(|_| "OverflowError: seek direction is out of range")?;
                    let place = unsafe { lseek(descriptor, offset, whence) };
                    if place < 0 { Err(std::io::Error::last_os_error()) } else { Ok(Value::Small(place)) }
                }
                _ => {
                    let length = args.get(2).ok_or("TypeError: truncate size is required")?.as_big()?.to_string().parse::<i64>().map_err(|_| "OverflowError: truncate size is out of range")?;
                    if unsafe { ftruncate(descriptor, length) } < 0 { Err(std::io::Error::last_os_error()) } else { Ok(Value::Null) }
                }
            }
        }
        "fd_flags" => {
            extern "C" { fn fcntl(fd: std::ffi::c_int, operation: std::ffi::c_int, ...) -> std::ffi::c_int; }
            let fd = file.parse::<i32>().map_err(|_| "TypeError: descriptor must be an integer")?;
            let flags = unsafe { fcntl(fd, 3) };
            if flags < 0 { Err(std::io::Error::last_os_error()) } else { Ok(Value::Small(i64::from(flags))) }
        }
        "isatty" => {
            extern "C" { fn isatty(fd: std::ffi::c_int) -> std::ffi::c_int; }
            let fd = file.parse::<i32>().map_err(|_| "TypeError: descriptor must be an integer")?;
            Ok(Value::Flag(unsafe { isatty(fd) } != 0))
        }
        "close" => {
            extern "C" { fn close(fd: std::ffi::c_int) -> std::ffi::c_int; }
            let descriptor = file.parse::<i32>().map_err(|_| "TypeError: descriptor must be an integer")?;
            if unsafe { close(descriptor) } < 0 { Err(std::io::Error::last_os_error()) } else { Ok(Value::Null) }
        }
        _ => return Err("NotImplementedError: unsupported host filesystem operation".into()),
    };
    Ok(match outcome {
        Ok(value) => Value::tuple(vec![value, Value::Null]),
        Err(error) => Value::tuple(vec![Value::Null, Value::tuple(vec![Value::Small(i64::from(error.raw_os_error().unwrap_or(5))), Value::text(&error.to_string())])]),
    })
}
