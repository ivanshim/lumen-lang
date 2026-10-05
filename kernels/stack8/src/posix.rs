// Host operations following CPython v3.14.8 Modules/posixmodule.c.
// The Python module owns argument conversion and exception construction.
use crate::value::Value;
use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::rc::Rc;
use num_traits::ToPrimitive;

struct Directory(*mut libc::DIR, bool);
impl Drop for Directory {
    fn drop(&mut self) { unsafe { if self.1 {libc::rewinddir(self.0);} libc::closedir(self.0); } }
}
thread_local! {
    static DIRECTORIES: RefCell<(i64, std::collections::HashMap<i64, Directory>)> = RefCell::new((0, std::collections::HashMap::new()));
}

fn bytes(data: Vec<u8>) -> Value {
    Value::Bytes(Rc::new(RefCell::new(data)), false, Rc::from("b"))
}
fn raw(value: &Value) -> Result<Vec<u8>, String> {
    match value.contents() {
        Value::Text(s) => Ok(s.as_bytes().to_vec()),
        Value::Bytes(b, ..) => Ok(b.borrow().clone()),
        Value::Codepoints(points) => {
            let mut encoded = Vec::new();
            for &point in points.iter() {
                if (0xdc80..=0xdcff).contains(&point) { encoded.push((point - 0xdc00) as u8); }
                else if let Some(letter) = char::from_u32(point) { let mut buffer = [0; 4]; encoded.extend_from_slice(letter.encode_utf8(&mut buffer).as_bytes()); }
                else { return Err("UnicodeEncodeError: surrogates not allowed".to_string()); }
            }
            Ok(encoded)
        },
        Value::Object(object) => {
            let payload = object.fields.borrow().iter().find(|(name, _)| name == "\0worth")
                .map(|(_, value)| value.clone());
            match payload {Some(value) => raw(&value), None => Err("TypeError: expected str or bytes".into())}
        },
        _ => Err("TypeError: expected str or bytes".into()),
    }
}
fn integer(value: &Value) -> Result<i64, String> {
    match value.contents() {
        Value::Small(n) => Ok(n),
        Value::Huge(n) => n.to_i64().ok_or_else(|| "OverflowError: Python int too large to convert to C long".into()),
        _ => Err("TypeError: an integer is required".into()),
    }
}
fn status(result: i64) -> Result<Value, i32> {
    if result < 0 { Err(std::io::Error::last_os_error().raw_os_error().unwrap_or(libc::EIO)) }
    else { Ok(Value::Small(result)) }
}
fn stat_fields(info: libc::stat) -> Value {
    let mut row = vec![Value::Small(info.st_mode.into()), Value::of_big(info.st_ino.into()),
        Value::of_big(info.st_dev.into()), Value::of_big(info.st_nlink.into())];
    row.extend([info.st_uid as i64, info.st_gid as i64, info.st_size, info.st_atime,
        info.st_mtime, info.st_ctime, info.st_atime_nsec, info.st_mtime_nsec,
        info.st_ctime_nsec, info.st_blksize as i64, info.st_blocks].into_iter().map(Value::Small));
    row.push(Value::of_big(info.st_rdev.into()));
    Value::tuple(row)
}
pub fn call(args: &[Value]) -> Result<Value, String> {
    let Some(Value::Text(op)) = args.first().map(Value::contents) else {
        return Err("TypeError: POSIX operation must be str".into());
    };
    let a = &args[1..];
    let number = |i: usize| integer(&a[i]);
    let pathname = |i: usize| CString::new(raw(&a[i])?).map_err(|_| "ValueError: embedded null byte".to_owned());
    let output: Result<Value, i32> = unsafe { match op.as_ref() {
        "getpid" => Ok(Value::Small(libc::getpid() as i64)),
        "getuid" => Ok(Value::Small(libc::getuid() as i64)),
        "geteuid" => Ok(Value::Small(libc::geteuid() as i64)),
        "getgid" => Ok(Value::Small(libc::getgid() as i64)),
        "getegid" => Ok(Value::Small(libc::getegid() as i64)),
        "cpu_count" => { let n = libc::sysconf(libc::_SC_NPROCESSORS_ONLN); Ok(if n < 1 { Value::Null } else { Value::Small(n) }) },
        "cwd" => match std::env::current_dir() {
            Ok(p) => { use std::os::unix::ffi::OsStrExt; Ok(bytes(p.as_os_str().as_bytes().to_vec())) },
            Err(e) => Err(e.raw_os_error().unwrap_or(libc::EIO)),
        },
        "environ" => {
            use std::os::unix::ffi::OsStrExt;
            let row = std::env::vars_os().map(|(k,v)| (bytes(k.as_bytes().to_vec()), bytes(v.as_bytes().to_vec()))).collect::<Vec<_>>();
            Ok(Value::Map(Rc::new(row.into())))
        },
        "stat_mode" => {
            let offered = a[0].as_big()?;
            if offered < 0.into() { return Err("OverflowError: can't convert negative value to unsigned int".into()); }
            let mode = offered.to_u64().ok_or_else(|| "OverflowError: Python int too large to convert to C unsigned long".to_string())?;
            if mode > u32::MAX as u64 {return Err("OverflowError: mode out of range".into());}
            let mode = mode as libc::mode_t;
            let operation = number(1)?;
            if operation == 0 {Ok(Value::Small(i64::from(mode & 0o7777)))}
            else if operation == 1 {Ok(Value::Small(i64::from(mode & libc::S_IFMT)))}
            else if operation < 12 {
                let kinds = [libc::S_IFDIR,libc::S_IFCHR,libc::S_IFBLK,libc::S_IFREG,libc::S_IFIFO,libc::S_IFLNK,libc::S_IFSOCK];
                Ok(Value::Flag(kinds.get((operation - 2) as usize).is_some_and(|kind| mode & libc::S_IFMT == *kind)))
            } else {
                let initial = match mode & libc::S_IFMT {libc::S_IFREG=>'-',libc::S_IFDIR=>'d',libc::S_IFLNK=>'l',libc::S_IFBLK=>'b',libc::S_IFCHR=>'c',libc::S_IFIFO=>'p',libc::S_IFSOCK=>'s',_=>'?'};
                let mut answer = initial.to_string();
                for (read,write,execute,special,lower,upper) in [(0o400,0o200,0o100,0o4000,'s','S'),(0o40,0o20,0o10,0o2000,'s','S'),(0o4,0o2,0o1,0o1000,'t','T')] {
                    answer.push(if mode & read != 0 {'r'} else {'-'});
                    answer.push(if mode & write != 0 {'w'} else {'-'});
                    answer.push(match (mode & execute != 0,mode & special != 0) {(true,true)=>lower,(false,true)=>upper,(true,false)=>'x',_=>'-'});
                }
                Ok(Value::text(&answer))
            }
        },
        "constants" => {
            let row = [libc::O_RDONLY, libc::O_WRONLY, libc::O_RDWR, libc::O_APPEND,
                libc::O_CREAT, libc::O_EXCL, libc::O_TRUNC, libc::O_NONBLOCK,
                libc::O_CLOEXEC, libc::O_DIRECTORY, libc::O_NOFOLLOW,
                libc::SEEK_SET, libc::SEEK_CUR, libc::SEEK_END, libc::F_OK,
                libc::R_OK, libc::W_OK, libc::X_OK];
            Ok(Value::tuple(row.into_iter().map(|n| Value::Small(n as i64)).collect()))
        },
        "stat" | "lstat" | "fstat" => {
            let mut record = std::mem::zeroed::<libc::stat>();
            let code = if op.as_ref() == "fstat" { libc::fstat(number(0)? as i32, &mut record) }
                else { let p = pathname(0)?; libc::fstatat(number(1)? as i32, p.as_ptr(), &mut record,
                    if op.as_ref() == "lstat" || number(2)? == 0 { libc::AT_SYMLINK_NOFOLLOW } else { 0 }) };
            status(code as i64).map(|_| stat_fields(record))
        },
        "open" => { let p = pathname(0)?; status(libc::openat(number(3)? as i32, p.as_ptr(), number(1)? as i32 | libc::O_CLOEXEC, number(2)? as libc::mode_t) as i64) },
        "close" => status(libc::close(number(0)? as i32) as i64),
        "dup" => status(libc::fcntl(number(0)? as i32, libc::F_DUPFD_CLOEXEC, 0) as i64),
        "read" => {
            let count = number(1)?;
            if count < 0 { Err(libc::EINVAL) } else {
                let mut buffer = vec![0u8; count as usize];
                let n = libc::read(number(0)? as i32, buffer.as_mut_ptr().cast(), buffer.len());
                status(n as i64).map(|_| { buffer.truncate(n as usize); bytes(buffer) })
            }
        },
        "write" => { let data = raw(&a[1])?; status(libc::write(number(0)? as i32, data.as_ptr().cast(), data.len()) as i64) },
        "lseek" => status(libc::lseek(number(0)? as i32, number(1)?, number(2)? as i32)),
        "ftruncate" => status(libc::ftruncate(number(0)? as i32, number(1)?) as i64),
        "chdir" => { let p = pathname(0)?; status(libc::chdir(p.as_ptr()) as i64) },
        "fchdir" => status(libc::fchdir(number(0)? as i32) as i64),
        "mkdir" => { let p = pathname(0)?; status(libc::mkdirat(number(2)? as i32, p.as_ptr(), number(1)? as libc::mode_t) as i64) },
        "unlink" | "rmdir" => { let p = pathname(0)?; status(libc::unlinkat(number(1)? as i32, p.as_ptr(), if op.as_ref() == "rmdir" { libc::AT_REMOVEDIR } else { 0 }) as i64) },
        "rename" => { let src = pathname(0)?; let dst = pathname(1)?; status(libc::renameat(number(2)? as i32, src.as_ptr(), number(3)? as i32, dst.as_ptr()) as i64) },
        "symlink" => { let src = pathname(0)?; let dst = pathname(1)?; status(libc::symlinkat(src.as_ptr(), number(2)? as i32, dst.as_ptr()) as i64) },
        "link" => { let src = pathname(0)?; let dst = pathname(1)?; status(libc::linkat(number(2)? as i32, src.as_ptr(), number(3)? as i32, dst.as_ptr(), if number(4)? != 0 { libc::AT_SYMLINK_FOLLOW } else {0}) as i64) },
        "readlink" => {
            let p = pathname(0)?;
            let mut data = vec![0u8; 256];
            loop {
                let size = libc::readlinkat(number(1)? as i32, p.as_ptr(), data.as_mut_ptr().cast(), data.len());
                if size < 0 { break Err(std::io::Error::last_os_error().raw_os_error().unwrap_or(libc::EIO)); }
                if (size as usize) < data.len() { data.truncate(size as usize); break Ok(bytes(data)); }
                data.resize(data.len()*2, 0);
            }
        },
        "listdir" => {
            use std::os::unix::ffi::OsStrExt;
            let p = pathname(0)?;
            match std::fs::read_dir(std::ffi::OsStr::from_bytes(p.as_bytes())) {
                Err(e) => Err(e.raw_os_error().unwrap_or(libc::EIO)),
                Ok(entries) => {
                    let mut names = Vec::new(); let mut failed = None;
                    for entry in entries { match entry { Ok(e) => names.push(bytes(e.file_name().as_bytes().to_vec())), Err(e) => { failed = Some(e.raw_os_error().unwrap_or(libc::EIO)); break; } } }
                    match failed { Some(e) => Err(e), None => Ok(Value::array(names)) }
                }
            }
        },
        "scandir" => {
            let from_fd = matches!(a[0].contents(), Value::Small(_) | Value::Huge(_));
            let directory = if from_fd {
                let fd = libc::fcntl(number(0)? as i32, libc::F_DUPFD_CLOEXEC, 0);
                if fd == -1 { std::ptr::null_mut() } else {
                    let dir = libc::fdopendir(fd);
                    if dir.is_null() {let error = *libc::__errno_location(); libc::close(fd); *libc::__errno_location() = error;}
                    dir
                }
            } else {libc::opendir(pathname(0)?.as_ptr())};
            if directory.is_null() {Err(std::io::Error::last_os_error().raw_os_error().unwrap_or(libc::EIO))}
            else {Ok(DIRECTORIES.with(|held| {let mut held = held.borrow_mut(); held.0 += 1; let key = held.0; held.1.insert(key, Directory(directory, from_fd)); Value::Small(key)}))}
        },
        "scandir_next" => {
            let handle = number(0)?;
            DIRECTORIES.with(|held| {
                let held = held.borrow();
                let Some(directory) = held.1.get(&handle) else {return Err(libc::EBADF);};
                loop {
                    *libc::__errno_location() = 0;
                    let entry = libc::readdir(directory.0);
                    if entry.is_null() {let error = *libc::__errno_location(); break if error == 0 {Ok(Value::Null)} else {Err(error)};}
                    let entry = &*entry;
                    let name = CStr::from_ptr(entry.d_name.as_ptr()).to_bytes();
                    if name == b"." || name == b".." {continue;}
                    break Ok(Value::tuple(vec![bytes(name.to_vec()), Value::of_big(entry.d_ino.into()), Value::Small(entry.d_type.into())]));
                }
            })
        },
        "scandir_close" => {let handle = number(0)?; DIRECTORIES.with(|held| {held.borrow_mut().1.remove(&handle);}); Ok(Value::Null)},
        "futime" => {
            let times = [libc::timespec {tv_sec: number(1)?, tv_nsec: number(2)?}, libc::timespec {tv_sec: number(3)?, tv_nsec: number(4)?}];
            status(libc::futimens(number(0)? as i32, if number(5)? == 0 {std::ptr::null()} else {times.as_ptr()}) as i64)
        },
        "waitstatus" => {
            let wait = number(0)? as i32;
            let (kind, code) = if libc::WIFEXITED(wait) {(0,libc::WEXITSTATUS(wait))}
                else if libc::WIFSIGNALED(wait) {(0,-libc::WTERMSIG(wait))}
                else if libc::WIFSTOPPED(wait) {(1,libc::WSTOPSIG(wait))} else {(2, wait)};
            Ok(Value::tuple(vec![Value::Small(kind), Value::Small(code as i64)]))
        },
        "chmod" => { let p = pathname(0)?; status(libc::fchmodat(number(2)? as i32, p.as_ptr(), number(1)? as libc::mode_t, if number(3)? != 0 { 0 } else { libc::AT_SYMLINK_NOFOLLOW }) as i64) },
        "fchmod" => status(libc::fchmod(number(0)? as i32, number(1)? as libc::mode_t) as i64),
        "access" => { let p = pathname(0)?; Ok(Value::Flag(libc::faccessat(number(2)? as i32, p.as_ptr(), number(1)? as i32,
            (if number(3)? != 0 {libc::AT_EACCESS} else {0}) | (if number(4)? != 0 {0} else {libc::AT_SYMLINK_NOFOLLOW})) == 0)) },
        "umask" => Ok(Value::Small(libc::umask(number(0)? as libc::mode_t) as i64)),
        "utime" => {
            let p = pathname(0)?;
            let times = [libc::timespec { tv_sec: number(1)?, tv_nsec: number(2)? }, libc::timespec { tv_sec: number(3)?, tv_nsec: number(4)? }];
            status(libc::utimensat(number(5)? as i32, p.as_ptr(), if number(6)? != 0 { times.as_ptr() } else { std::ptr::null() }, if number(7)? != 0 { 0 } else {libc::AT_SYMLINK_NOFOLLOW}) as i64)
        },
        "strerror" => Ok(Value::text(&CStr::from_ptr(libc::strerror(number(0)? as i32)).to_string_lossy())),
        "putenv" => { let key = pathname(0)?; let val = pathname(1)?; status(libc::setenv(key.as_ptr(), val.as_ptr(), 1) as i64) },
        "unsetenv" => { let key = pathname(0)?; status(libc::unsetenv(key.as_ptr()) as i64) },
        "uname" => {
            let mut u = std::mem::zeroed::<libc::utsname>();
            status(libc::uname(&mut u) as i64).map(|_| Value::tuple([&u.sysname, &u.nodename, &u.release, &u.version, &u.machine].into_iter().map(|s| Value::text(&CStr::from_ptr(s.as_ptr()).to_string_lossy())).collect()))
        },
        "terminal" => {
            let mut size = std::mem::zeroed::<libc::winsize>();
            status(libc::ioctl(number(0)? as i32, libc::TIOCGWINSZ, &mut size) as i64).map(|_| Value::tuple(vec![Value::Small(size.ws_col as i64),Value::Small(size.ws_row as i64)]))
        },
        "isatty" => Ok(Value::Flag(libc::isatty(number(0)? as i32) != 0)),
        "urandom" => {
            use std::io::Read;
            let mut data = vec![0u8; number(0)? as usize];
            match std::fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut data)) { Ok(()) => Ok(bytes(data)), Err(e) => Err(e.raw_os_error().unwrap_or(libc::EIO)) }
        },
        "socket" => status(libc::socket(number(0)? as i32, number(1)? as i32 | libc::SOCK_CLOEXEC, number(2)? as i32) as i64),
        "bind_unix" => {
            let name = raw(&a[1])?;
            let mut address = std::mem::zeroed::<libc::sockaddr_un>();
            if name.len() >= address.sun_path.len() { return Err("OSError: AF_UNIX path too long".into()); }
            address.sun_family = libc::AF_UNIX as libc::sa_family_t;
            for (to, from) in address.sun_path.iter_mut().zip(&name) { *to = *from as libc::c_char; }
            status(libc::bind(number(0)? as i32, (&address as *const libc::sockaddr_un).cast(), (std::mem::size_of::<libc::sa_family_t>() + name.len() + 1) as libc::socklen_t) as i64)
        },
        "mkfifo" => { let p = pathname(0)?; status(libc::mkfifoat(number(2)? as i32, p.as_ptr(), number(1)? as libc::mode_t) as i64) },
        _ => return Err("NotImplementedError: unknown POSIX operation".into()),
    }};
    Ok(match output { Ok(value) => Value::tuple(vec![Value::Small(0), value]), Err(e) => Value::tuple(vec![Value::Small(e as i64), Value::Null]) })
}
