// POSIX entry points: CPython v3.14.8, Modules/posixmodule.c semantics.
use crate::data::Value;
use num_traits::ToPrimitive;
use num_bigint::BigInt;
use std::{cell::RefCell, ffi::{CString, CStr}, rc::Rc};

struct DirStream { pointer: *mut libc::DIR }
impl Drop for DirStream {
    fn drop(&mut self) {unsafe {let _ = libc::closedir(self.pointer);}}
}
thread_local! {
    static OPEN_DIRS: RefCell<std::collections::BTreeMap<i64, DirStream>> = const {RefCell::new(std::collections::BTreeMap::new())};
    static NEXT_DIR: std::cell::Cell<i64> = const {std::cell::Cell::new(1)};
}

fn octets(part: impl Into<Vec<u8>>) -> Value {
    Value::Octets { cell: Rc::new(RefCell::new(part.into())), changeable: false, lead: "b".into() }
}
fn errno() -> i32 { std::io::Error::last_os_error().raw_os_error().unwrap_or(libc::EIO) }
fn checked(code: i64) -> Result<Value, i32> {
    match code { -1 => Err(errno()), n => Ok(Value::Small(n)) }
}
fn data_of(item: &Value) -> Result<Vec<u8>, String> {
    match item.settled() {
        Value::Octets {cell, ..} => Ok(cell.borrow().to_vec()),
        Value::Text(word) => Ok(word.as_bytes().into()),
        Value::Unpaired(numbers) => {
            let mut bytes = Vec::new();
            for number in numbers.iter().copied() {
                match number {
                    0xdc80..=0xdcff => bytes.push(number as u8),
                    _ => {let character = char::from_u32(number).ok_or(String::from("UnicodeEncodeError: surrogates not allowed"))?;
                        let mut unit = [0u8; 4]; bytes.extend(character.encode_utf8(&mut unit).bytes());}
                }
            }
            Ok(bytes)
        }
        _ => Err(String::from("TypeError: expected str or bytes")),
    }
}
fn record(s: &libc::stat) -> Value {
    let initial = [s.st_mode as u64, s.st_ino, s.st_dev, u64::from(s.st_nlink)];
    let mut fields: Vec<Value> = initial.into_iter().map(|unsigned| Value::from_big(BigInt::from(unsigned))).collect();
    let signed = [i64::from(s.st_uid), i64::from(s.st_gid), s.st_size, s.st_atime,
        s.st_mtime, s.st_ctime, s.st_atime_nsec, s.st_mtime_nsec, s.st_ctime_nsec,
        i64::from(s.st_blksize), s.st_blocks];
    for amount in signed {fields.push(Value::Small(amount));}
    fields.push(Value::from_big(BigInt::from(s.st_rdev)));
    Value::tuple(fields)
}
pub fn perform(given: &[Value]) -> Result<Value, String> {
    let operation = match given.first().map(Value::settled) {
        Some(Value::Text(name)) => name,
        _ => return Err("TypeError: POSIX operation must be str".to_owned()),
    };
    let params = &given[1..];
    let int = |at: usize| -> Result<i64, String> {
        match params[at].settled() {
            Value::Small(n) => Ok(n),
            Value::Huge(n) => n.to_i64().ok_or("OverflowError: Python int too large to convert to C long".to_string()),
            _ => Err("TypeError: an integer is required".to_string()),
        }
    };
    let path = |at: usize| -> Result<CString, String> {
        CString::new(data_of(&params[at])?).map_err(|_| String::from("ValueError: embedded null byte"))
    };
    let answer = unsafe {
        match operation.as_ref() {
            "stat_mode" => {
                let whole = params[0].as_big()?;
                if whole.sign() == num_bigint::Sign::Minus {return Err(String::from("OverflowError: can't convert negative value to unsigned int"));}
                let unsigned = whole.to_u64().ok_or("OverflowError: Python int too large to convert to C unsigned long")?;
                let bits = u32::try_from(unsigned).map_err(|_| "OverflowError: mode out of range")?;
                match int(1)? {
                    0 => Ok(Value::Small((bits & 4095).into())),
                    1 => Ok(Value::Small((bits & libc::S_IFMT).into())),
                    selector @ 2..=11 => {
                        let expected = match selector {2=>Some(libc::S_IFDIR),3=>Some(libc::S_IFCHR),4=>Some(libc::S_IFBLK),5=>Some(libc::S_IFREG),6=>Some(libc::S_IFIFO),7=>Some(libc::S_IFLNK),8=>Some(libc::S_IFSOCK),_=>None};
                        Ok(Value::Flag(expected == Some(bits & libc::S_IFMT)))
                    },
                    _ => {
                        let letter = match bits & libc::S_IFMT {libc::S_IFDIR=>'d',libc::S_IFREG=>'-',libc::S_IFIFO=>'p',libc::S_IFCHR=>'c',libc::S_IFBLK=>'b',libc::S_IFLNK=>'l',libc::S_IFSOCK=>'s',_=>'?'};
                        let mut shown = vec![letter];
                        for group in 0..3 {
                            let shift = 6 - group * 3;
                            shown.push(if bits & (4 << shift) == 0 {'-'} else {'r'});
                            shown.push(if bits & (2 << shift) == 0 {'-'} else {'w'});
                            let executable = bits & (1 << shift) != 0;
                            let marked = bits & (0o4000 >> group) != 0;
                            shown.push(if marked {if group == 2 {if executable {'t'} else {'T'}} else if executable {'s'} else {'S'}} else if executable {'x'} else {'-'});
                        }
                        Ok(Value::text(&shown.into_iter().collect::<String>()))
                    }
                }
            }
            "constants" => {
                let flags = vec![libc::O_RDONLY,libc::O_WRONLY,libc::O_RDWR,libc::O_APPEND,libc::O_CREAT,libc::O_EXCL,libc::O_TRUNC,libc::O_NONBLOCK,libc::O_CLOEXEC,libc::O_DIRECTORY,libc::O_NOFOLLOW,libc::SEEK_SET,libc::SEEK_CUR,libc::SEEK_END,libc::F_OK,libc::R_OK,libc::W_OK,libc::X_OK];
                Ok(Value::tuple(flags.into_iter().map(|f| Value::Small(i64::from(f))).collect()))
            }
            "environ" => {
                use std::os::unix::ffi::OsStrExt as _;
                let mut entries = Vec::new();
                for (key, val) in std::env::vars_os() { entries.push((octets(key.as_bytes()), octets(val.as_bytes()))); }
                Ok(Value::Dict(Rc::new(entries.into())))
            }
            "cwd" => {
                use std::os::unix::ffi::OsStrExt as _;
                std::env::current_dir().map(|here| octets(here.as_os_str().as_bytes())).map_err(|err| err.raw_os_error().unwrap_or(libc::EIO))
            }
            "getpid" | "getuid" | "geteuid" | "getgid" | "getegid" => {
                let ident = match operation.as_ref() { "getpid" => libc::getpid() as i64, "getuid" => libc::getuid() as i64, "geteuid" => libc::geteuid() as i64, "getgid" => libc::getgid() as i64, _ => libc::getegid() as i64 };
                Ok(Value::Small(ident))
            }
            "cpu_count" => {
                let available = libc::sysconf(libc::_SC_NPROCESSORS_ONLN);
                Ok(match available { 1.. => Value::Small(available), _ => Value::Nil })
            }
            "stat" | "lstat" | "fstat" => {
                let mut buffer: libc::stat = std::mem::zeroed();
                let rc;
                if operation.as_ref() == "fstat" { rc = libc::fstat(int(0)? as libc::c_int, &mut buffer); }
                else {
                    let filename = path(0)?;
                    let flags = if operation.as_ref() == "lstat" || int(2)? == 0 { libc::AT_SYMLINK_NOFOLLOW } else {0};
                    rc = libc::fstatat(int(1)? as libc::c_int, filename.as_ptr(), &mut buffer, flags);
                }
                if rc == 0 { Ok(record(&buffer)) } else { Err(errno()) }
            }
            "open" => {
                let filename = path(0)?;
                checked(i64::from(libc::openat(int(3)? as _, filename.as_ptr(), (int(1)? as i32) | libc::O_CLOEXEC, int(2)? as libc::mode_t)))
            }
            "read" => {
                let capacity = int(1)?;
                if capacity < 0 { Err(libc::EINVAL) } else {
                    let mut content = vec![0; capacity as usize];
                    let used = libc::read(int(0)? as _, content.as_mut_ptr().cast(), content.len());
                    if used == -1 { Err(errno()) } else { content.truncate(used as usize); Ok(octets(content)) }
                }
            }
            "write" => {
                let content = data_of(&params[1])?;
                checked(libc::write(int(0)? as _, content.as_ptr().cast(), content.len()) as i64)
            }
            "close" => checked(libc::close(int(0)? as _) as i64),
            "lseek" => checked(libc::lseek(int(0)? as _, int(1)?, int(2)? as _)),
            "dup" => checked(libc::fcntl(int(0)? as _, libc::F_DUPFD_CLOEXEC, 0) as i64),
            "ftruncate" => checked(libc::ftruncate(int(0)? as _, int(1)?) as i64),
            "fchmod" => checked(libc::fchmod(int(0)? as _, int(1)? as libc::mode_t) as i64),
            "fchdir" => checked(libc::fchdir(int(0)? as _) as i64),
            "chdir" => checked(libc::chdir(path(0)?.as_ptr()) as i64),
            "mkdir" => checked(libc::mkdirat(int(2)? as _, path(0)?.as_ptr(), int(1)? as libc::mode_t) as i64),
            "mkfifo" => checked(libc::mkfifoat(int(2)? as _, path(0)?.as_ptr(), int(1)? as libc::mode_t) as i64),
            "rmdir" | "unlink" => {
                let flags = if operation.as_ref() == "rmdir" {libc::AT_REMOVEDIR} else {0};
                checked(libc::unlinkat(int(1)? as _, path(0)?.as_ptr(), flags) as i64)
            }
            "rename" => checked(libc::renameat(int(2)? as _, path(0)?.as_ptr(), int(3)? as _, path(1)?.as_ptr()) as i64),
            "symlink" => checked(libc::symlinkat(path(0)?.as_ptr(), int(2)? as _, path(1)?.as_ptr()) as i64),
            "link" => {
                let flags = if int(4)? == 0 {0} else {libc::AT_SYMLINK_FOLLOW};
                checked(libc::linkat(int(2)? as _, path(0)?.as_ptr(), int(3)? as _, path(1)?.as_ptr(), flags) as i64)
            }
            "chmod" => {
                let flags = match int(3)? {0 => libc::AT_SYMLINK_NOFOLLOW, _ => 0};
                checked(libc::fchmodat(int(2)? as _, path(0)?.as_ptr(), int(1)? as libc::mode_t, flags) as i64)
            }
            "access" => {
                let mut flags = 0;
                if int(3)? != 0 {flags |= libc::AT_EACCESS;}
                if int(4)? == 0 {flags |= libc::AT_SYMLINK_NOFOLLOW;}
                Ok(Value::Flag(libc::faccessat(int(2)? as _, path(0)?.as_ptr(), int(1)? as _, flags) == 0))
            }
            "scandir" => {
                let stream;
                if matches!(params[0].settled(), Value::Small(_) | Value::Huge(_)) {
                    let copied = libc::fcntl(int(0)? as _, libc::F_DUPFD_CLOEXEC, 0);
                    stream = if copied < 0 {std::ptr::null_mut()} else {
                        let opened = libc::fdopendir(copied);
                        if opened.is_null() {let saved = errno(); libc::close(copied); *libc::__errno_location() = saved;}
                        opened
                    };
                } else {stream = libc::opendir(path(0)?.as_ptr());}
                if stream.is_null() {Err(errno())} else {
                    let token = NEXT_DIR.with(|counter| {let value = counter.get(); counter.set(value + 1); value});
                    OPEN_DIRS.with(|entries| {entries.borrow_mut().insert(token, DirStream {pointer: stream});});
                    Ok(Value::Small(token))
                }
            }
            "scandir_next" => {
                let token = int(0)?;
                OPEN_DIRS.with(|entries| {
                    let entries = entries.borrow();
                    match entries.get(&token) {
                        None => Err(libc::EBADF),
                        Some(stream) => loop {
                            *libc::__errno_location() = 0;
                            let next = libc::readdir(stream.pointer);
                            if next.is_null() {break match errno() {0 => Ok(Value::Nil), error => Err(error)};}
                            let name = CStr::from_ptr((*next).d_name.as_ptr()).to_bytes();
                            match name {b"." | b".." => continue, _ => {
                                let record = vec![octets(name), Value::from_big(BigInt::from((*next).d_ino)), Value::Small((*next).d_type as i64)];
                                break Ok(Value::tuple(record));
                            }}
                        },
                    }
                })
            }
            "scandir_close" => {let token = int(0)?; OPEN_DIRS.with(|streams| {drop(streams.borrow_mut().remove(&token));}); Ok(Value::Nil)}
            "futime" => {
                let times = [libc::timespec {tv_sec: int(1)?, tv_nsec: int(2)?}, libc::timespec {tv_sec: int(3)?, tv_nsec: int(4)?}];
                let chosen = match int(5)? {0 => std::ptr::null(), _ => times.as_ptr()};
                checked(libc::futimens(int(0)? as _, chosen) as i64)
            }
            "waitstatus" => {
                let bits = int(0)? as libc::c_int;
                let mut category = 0;
                let value = if libc::WIFEXITED(bits) {libc::WEXITSTATUS(bits)} else if libc::WIFSIGNALED(bits) {-libc::WTERMSIG(bits)}
                    else if libc::WIFSTOPPED(bits) {category = 1; libc::WSTOPSIG(bits)} else {category = 2; bits};
                Ok(Value::tuple(vec![Value::Small(category), Value::Small(i64::from(value))]))
            }
            "umask" => Ok(Value::Small(i64::from(libc::umask(int(0)? as libc::mode_t)))),
            "readlink" => {
                let target = path(0)?;
                let mut capacity = 128;
                loop {
                    let mut text = vec![0; capacity];
                    let count = libc::readlinkat(int(1)? as _, target.as_ptr(), text.as_mut_ptr().cast(), capacity);
                    if count == -1 { break Err(errno()); }
                    if count as usize != capacity { text.truncate(count as usize); break Ok(octets(text)); }
                    capacity *= 2;
                }
            }
            "listdir" => {
                use std::os::unix::ffi::OsStrExt as _;
                let filename = path(0)?;
                let directory = std::ffi::OsStr::from_bytes(filename.as_bytes());
                match std::fs::read_dir(directory) {
                    Ok(listing) => {
                        let mut row = Vec::new(); let mut error = 0;
                        for result in listing {
                            match result { Err(e) => {error = e.raw_os_error().unwrap_or(libc::EIO); break;}, Ok(entry) => row.push(octets(entry.file_name().as_bytes())) }
                        }
                        if error != 0 {Err(error)} else {Ok(Value::Vector(crate::tuples::Sequence::plain(row)))}
                    }
                    Err(e) => Err(e.raw_os_error().unwrap_or(libc::EIO)),
                }
            }
            "utime" => {
                let moments = [libc::timespec {tv_sec: int(1)?, tv_nsec: int(2)?}, libc::timespec {tv_sec: int(3)?, tv_nsec: int(4)?}];
                let ptr = if int(6)? == 0 {std::ptr::null()} else {moments.as_ptr()};
                let flags = if int(7)? == 0 {libc::AT_SYMLINK_NOFOLLOW} else {0};
                checked(libc::utimensat(int(5)? as _, path(0)?.as_ptr(), ptr, flags) as i64)
            }
            "putenv" => checked(libc::setenv(path(0)?.as_ptr(), path(1)?.as_ptr(), 1) as i64),
            "unsetenv" => checked(libc::unsetenv(path(0)?.as_ptr()) as i64),
            "strerror" => {
                let message = CStr::from_ptr(libc::strerror(int(0)? as _)).to_string_lossy();
                Ok(Value::text(&message))
            }
            "isatty" => Ok(Value::Flag(libc::isatty(int(0)? as _) == 1)),
            "terminal" => {
                let mut dimensions: libc::winsize = std::mem::zeroed();
                if libc::ioctl(int(0)? as _, libc::TIOCGWINSZ, &mut dimensions) == -1 {Err(errno())}
                else {Ok(Value::tuple(vec![Value::Small(dimensions.ws_col.into()), Value::Small(dimensions.ws_row.into())]))}
            }
            "uname" => {
                let mut names: libc::utsname = std::mem::zeroed();
                if libc::uname(&mut names) < 0 {Err(errno())} else {
                    let words = [&names.sysname, &names.nodename, &names.release, &names.version, &names.machine];
                    Ok(Value::tuple(words.iter().map(|name| Value::text(&CStr::from_ptr(name.as_ptr()).to_string_lossy())).collect()))
                }
            }
            "socket" => checked(libc::socket(int(0)? as _, (int(1)? as i32) | libc::SOCK_CLOEXEC, int(2)? as _) as i64),
            "bind_unix" => {
                let filename = data_of(&params[1])?;
                let mut sockaddr: libc::sockaddr_un = std::mem::zeroed();
                if filename.len() >= sockaddr.sun_path.len() { return Err(String::from("OSError: AF_UNIX path too long")); }
                sockaddr.sun_family = libc::AF_UNIX as _;
                filename.iter().enumerate().for_each(|(i, b)| sockaddr.sun_path[i] = *b as _);
                let length = (2 + filename.len() + 1) as libc::socklen_t;
                checked(libc::bind(int(0)? as _, std::ptr::addr_of!(sockaddr).cast(), length) as i64)
            }
            "urandom" => {
                use std::io::Read as _;
                let mut content = vec![0; int(0)? as usize];
                let result = std::fs::File::open("/dev/urandom").and_then(|mut source| source.read_exact(&mut content));
                match result {Err(e) => Err(e.raw_os_error().unwrap_or(libc::EIO)), Ok(()) => Ok(octets(content))}
            }
            _ => return Err(String::from("NotImplementedError: unknown POSIX operation")),
        }
    };
    let (error, result) = match answer {Err(code) => (code, Value::Nil), Ok(value) => (0, value)};
    Ok(Value::tuple(vec![Value::Small(error.into()), result]))
}
