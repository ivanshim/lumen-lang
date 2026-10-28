// POSIX entry points: CPython v3.14.8, Modules/posixmodule.c semantics.
use crate::data::Value;
use num_traits::ToPrimitive;
use num_bigint::BigInt;
use std::{cell::RefCell, ffi::{CString, CStr}, rc::Rc};

struct DirStream { pointer: *mut libc::DIR, reset: bool }
impl Drop for DirStream {
    fn drop(&mut self) {unsafe {if self.reset {libc::rewinddir(self.pointer);} let _ = libc::closedir(self.pointer);}}
}
thread_local! {
    static OPEN_DIRS: RefCell<std::collections::BTreeMap<i64, DirStream>> = const {RefCell::new(std::collections::BTreeMap::new())};
    static NEXT_DIR: std::cell::Cell<i64> = const {std::cell::Cell::new(1)};
}

/// Step an open directory stream to its next entry, passing over the ones
/// for the directory and its parent: the name, the inode number and the
/// kind of file, or none when the stream has run out.
pub fn step_stream(token: i64) -> Result<Option<(Vec<u8>, u64, u8)>, i32> {
    OPEN_DIRS.with(|streams| {
        let streams = streams.borrow();
        let stream = streams.get(&token).ok_or(libc::EBADF)?;
        let mut found = None;
        let mut failed = 0;
        unsafe {
            loop {
                *libc::__errno_location() = 0;
                let next = libc::readdir(stream.pointer);
                if next.is_null() { failed = errno(); break; }
                let name = CStr::from_ptr((*next).d_name.as_ptr()).to_bytes();
                if name == b"." || name == b".." { continue; }
                found = Some((name.to_vec(), (*next).d_ino as u64, (*next).d_type));
                break;
            }
        }
        if found.is_none() && failed != 0 { Err(failed) } else { Ok(found) }
    })
}
/// The row the stream's `scandir_next` operation answers with.
pub fn stream_row(name: Vec<u8>, inode: u64, kind: u8) -> Value {
    Value::tuple(vec![octets(name), Value::from_big(BigInt::from(inode)), Value::Small(kind as i64)])
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
        Value::Thing(instance) => {
            let saved = instance.holds.borrow();
            let (_, native) = saved.iter().find(|field| field.0 == "\0underlying")
                .ok_or_else(|| String::from("TypeError: expected str or bytes"))?;
            data_of(native)
        }
        _ => Err(String::from("TypeError: expected str or bytes")),
    }
}
fn record(s: &libc::stat) -> Value {
    let initial = [s.st_mode as u64, s.st_ino, s.st_dev, u64::from(s.st_nlink)];
    let mut fields: Vec<Value> = initial.into_iter().map(|unsigned| Value::from_big(BigInt::from(unsigned))).collect();
    let signed = [i64::from(s.st_uid), i64::from(s.st_gid), s.st_size, s.st_atime, s.st_mtime, s.st_ctime];
    for amount in signed {fields.push(Value::Small(amount));}
    // Each clock twice over, seconds with their fraction and then whole
    // nanoseconds, so the module has nothing left to compute per call.
    let clocks = [(s.st_atime, s.st_atime_nsec), (s.st_mtime, s.st_mtime_nsec), (s.st_ctime, s.st_ctime_nsec)];
    for (seconds, nanos) in clocks { fields.push(crate::data::worth_of_binary(seconds as f64 + nanos as f64 / 1e9, crate::math::DEFAULT_PLACES)); }
    for (seconds, nanos) in clocks { fields.push(Value::from_big(BigInt::from(seconds) * 1_000_000_000 + nanos)); }
    fields.push(Value::Small(i64::from(s.st_blksize)));
    fields.push(Value::Small(s.st_blocks));
    fields.push(Value::from_big(BigInt::from(s.st_rdev)));
    Value::tuple(fields)
}
// Copy an account record out of the host database's reusable storage.
fn account_record(user: &libc::passwd) -> Value {
    let strings = [user.pw_name, user.pw_passwd, user.pw_gecos, user.pw_dir, user.pw_shell];
    let mut row: Vec<Value> = strings.into_iter().map(|p| match unsafe {p.as_ref()} {
        Some(_) => octets(unsafe {CStr::from_ptr(p)}.to_bytes()), None => Value::Nil
    }).collect();
    row.insert(2, Value::Small(user.pw_uid.into()));
    row.insert(3, Value::Small(user.pw_gid.into()));
    Value::tuple(row)
}
// Copy each group before the next host database lookup reuses its storage.
fn group_record(group: &libc::group) -> Value {
    let text = |p: *const libc::c_char| if p.is_null() {Value::Nil}
        else {octets(unsafe {CStr::from_ptr(p)}.to_bytes())};
    let mut members = Vec::new();
    if !group.gr_mem.is_null() {
        let mut index = 0;
        loop {
            let member = unsafe { *group.gr_mem.add(index) };
            if member.is_null() { break; }
            members.push(text(member));
            index += 1;
        }
    }
    Value::tuple(vec![text(group.gr_name), text(group.gr_passwd),
        Value::Small(group.gr_gid.into()), Value::Vector(crate::tuples::Sequence::plain(members))])
}

// Open a terminal, fork, and make the child the session leader on the slave.
fn split_onto_terminal() -> Result<Value, i32> {
    let mut held: libc::c_int = -1;
    let born = unsafe {
        libc::forkpty(&mut held, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut())
    };
    if born < 0 {
        return Err(errno());
    }
    // Leave the disposition alone. Whatever was installed before the split,
    // including the default, is what the child still answers a hangup with.
    Ok(Value::tuple(vec![Value::Small(i64::from(born)), Value::Small(i64::from(held))]))
}

// A wait cut short by a signal runs the Python handlers, then waits again.
fn harvest(pid: i64, flags: i64, mut interrupted: impl FnMut() -> Result<(), String>) -> Result<Result<Value, i32>, String> {
    loop {
        let mut code: libc::c_int = 0;
        let got = unsafe { libc::waitpid(pid as libc::pid_t, &mut code, flags as libc::c_int) };
        if got >= 0 {
            return Ok(Ok(Value::tuple(vec![Value::Small(i64::from(got)), Value::Small(i64::from(code))])));
        }
        let cause = errno();
        if cause != libc::EINTR { return Ok(Err(cause)); }
        interrupted()?;
    }
}

// Build a descriptor set from the rows already range-checked by select.
fn fd_set_of(fds: &[i32]) -> libc::fd_set {
    let mut set = unsafe { std::mem::zeroed() };
    for &fd in fds { unsafe { libc::FD_SET(fd, &mut set); } }
    set
}
pub fn perform(given: &[Value], mut interrupted: impl FnMut() -> Result<(), String>) -> Result<Value, String> {
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
            "group" => {
                if int(0)? == 2 {
                    libc::setgrent();
                    let mut groups = Vec::new();
                    while let Some(group) = libc::getgrent().as_ref() { groups.push(group_record(group)); }
                    libc::endgrent();
                    let payload = Value::Vector(crate::tuples::Sequence::plain(groups));
                    return Ok(Value::tuple(vec![Value::Small(0), payload]));
                }
                let found = match int(0)? {0 => libc::getgrnam(path(1)?.as_ptr()), _ => libc::getgrgid(int(1)? as _)};
                match found.as_ref() { None => Ok(Value::Nil), Some(group) => Ok(group_record(group)) }
            }
            "passwd" => {
                if int(0)? == 2 {
                    let mut users = Vec::new();
                    libc::setpwent();
                    while let Some(user) = libc::getpwent().as_ref() {
                        users.push(account_record(user));
                    }
                    libc::endpwent();
                    let payload = Value::Vector(crate::tuples::Sequence::plain(users));
                    return Ok(Value::tuple(vec![Value::Small(0), payload]));
                }
                let found = match int(0)? {0 => libc::getpwnam(path(1)?.as_ptr()), _ => libc::getpwuid(int(1)? as _)};
                match found.as_ref() {
                    None => Ok(Value::Nil),
                    Some(user) => {
                        Ok(account_record(user))
                    }
                }
            }
            "fchown" => {
                let (descriptor, owner, group) = (int(0)?, int(1)?, int(2)?);
                checked(libc::fchown(descriptor as _, owner as _, group as _) as i64)
            }
            "chown" => {
                let name = path(0)?;
                let link_flag = match int(4)? {0 => libc::AT_SYMLINK_NOFOLLOW, _ => 0};
                checked(libc::fchownat(int(3)? as _, name.as_ptr(), int(1)? as _, int(2)? as _, link_flag).into())
            }
            "fstatvfs" | "statvfs" => {
                let mut capacity = std::mem::zeroed::<libc::statvfs>();
                let code = match operation.as_ref() {
                    "statvfs" => libc::statvfs(path(0)?.as_ptr(), &mut capacity),
                    _ => libc::fstatvfs(int(0)? as _, &mut capacity),
                };
                if code != 0 {Err(errno())} else {
                    let amounts = vec![capacity.f_bsize, capacity.f_frsize, capacity.f_blocks,
                        capacity.f_bfree, capacity.f_bavail, capacity.f_files, capacity.f_ffree,
                        capacity.f_favail, capacity.f_flag, capacity.f_namemax, capacity.f_fsid];
                    let values = amounts.into_iter().map(|amount| Value::from_big(BigInt::from(amount))).collect();
                    Ok(Value::tuple(values))
                }
            }
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
                let flags = vec![libc::O_RDONLY,libc::O_WRONLY,libc::O_RDWR,libc::O_APPEND,libc::O_CREAT,libc::O_EXCL,libc::O_TRUNC,libc::O_NONBLOCK,libc::O_CLOEXEC,libc::O_DIRECTORY,libc::O_NOFOLLOW,libc::SEEK_SET,libc::SEEK_CUR,libc::SEEK_END,libc::F_OK,libc::R_OK,libc::W_OK,libc::X_OK,libc::O_TMPFILE,libc::O_NOCTTY];
                Ok(Value::tuple(flags.into_iter().map(|f| Value::Small(i64::from(f))).collect()))
            }
            "environ" => {
                use std::os::unix::ffi::OsStrExt as _;
                let mut entries = Vec::new();
                for (key, val) in std::env::vars_os() { entries.push((octets(key.as_bytes()), octets(val.as_bytes()))); }
                Ok(Value::Dict(Rc::new(entries.into())))
            }
            "times" => {
                let mut ledger: libc::tms = std::mem::zeroed();
                let span = libc::times(&mut ledger);
                if span == -1 { Err(errno()) } else {
                    let hertz = libc::sysconf(libc::_SC_CLK_TCK) as f64;
                    let hertz = if hertz < 1.0 { 100.0 } else { hertz };
                    let share = |clock: libc::clock_t| crate::data::worth_of_binary(clock as f64 / hertz, 17);
                    Ok(Value::tuple(vec![share(ledger.tms_utime), share(ledger.tms_stime),
                        share(ledger.tms_cutime), share(ledger.tms_cstime), share(span)]))
                }
            }
            "cwd" => {
                use std::os::unix::ffi::OsStrExt as _;
                std::env::current_dir().map(|here| octets(here.as_os_str().as_bytes())).map_err(|err| err.raw_os_error().unwrap_or(libc::EIO))
            }
            "socket_hostbyaddr" => Ok(reverse_record(&params[0])?),
            "socket_hostname" => {
                let mut name = [0u8; 256];
                match libc::gethostname(name.as_mut_ptr().cast(),name.len()) {
                    0 => {let end=name.iter().position(|&unit| unit==0).unwrap_or(name.len());Ok(octets(&name[..end]))},
                    _ => Err(errno()),
                }
            },
        "locale_encoding" => {
            let name = libc::nl_langinfo(libc::CODESET);
            Ok(octets(CStr::from_ptr(name).to_bytes()))
        },
            "kill" => {
                let target = int(0)? as libc::pid_t;
                let requested = int(1)? as libc::c_int;
                let outcome = checked(i64::from(libc::kill(target, requested)));
                match &outcome {Ok(_) => interrupted()?, Err(_) => {}}
                outcome
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
            "sendfile" => {
                let mut start = int(3)?;
                let cursor: *mut i64 = if int(2)? == 0 { std::ptr::null_mut() } else { &mut start };
                checked(libc::sendfile(int(0)? as _, int(1)? as _, cursor, int(4)? as usize) as i64)
            }
            "copy_file_range" => {
                let (mut read_at, mut write_at) = (int(4)?, int(6)?);
                let read_cursor: *mut i64 = if int(3)? == 0 { std::ptr::null_mut() } else { &mut read_at };
                let write_cursor: *mut i64 = if int(5)? == 0 { std::ptr::null_mut() } else { &mut write_at };
                checked(libc::copy_file_range(int(0)? as _, read_cursor, int(1)? as _, write_cursor, int(2)? as usize, 0) as i64)
            }
            // Extended attributes: the first word picks get, set, list or remove; the fifth says path, link itself or descriptor.
            "xattr" => {
                let action = int(0)?;
                let how = int(4)?;
                let owner = if how == 2 { None } else { Some(path(1)?) };
                let handle = if how == 2 { int(1)? as libc::c_int } else { -1 };
                let label = if action == 2 { CString::default() } else { path(2)? };
                let payload = if action == 1 { data_of(&params[3])? } else { Vec::new() };
                let mode = int(5)? as libc::c_int;
                let first = owner.as_ref().map_or(std::ptr::null(), |c| c.as_ptr());
                let attempt = |space: *mut libc::c_void, room: usize| -> isize {
                    let names = space as *mut libc::c_char;
                    let (given, length) = (payload.as_ptr().cast::<libc::c_void>(), payload.len());
                    let outcome = match (action, how) {
                        (0, 0) => libc::getxattr(first, label.as_ptr(), space, room),
                        (0, 1) => libc::lgetxattr(first, label.as_ptr(), space, room),
                        (0, _) => libc::fgetxattr(handle, label.as_ptr(), space, room),
                        (1, 0) => libc::setxattr(first, label.as_ptr(), given, length, mode) as isize,
                        (1, 1) => libc::lsetxattr(first, label.as_ptr(), given, length, mode) as isize,
                        (1, _) => libc::fsetxattr(handle, label.as_ptr(), given, length, mode) as isize,
                        (2, 0) => libc::listxattr(first, names, room),
                        (2, 1) => libc::llistxattr(first, names, room),
                        (2, _) => libc::flistxattr(handle, names, room),
                        (_, 0) => libc::removexattr(first, label.as_ptr()) as isize,
                        (_, 1) => libc::lremovexattr(first, label.as_ptr()) as isize,
                        (_, _) => libc::fremovexattr(handle, label.as_ptr()) as isize,
                    };
                    outcome
                };
                if action == 0 || action == 2 {
                    let mut answer = Err(libc::EIO);
                    for _ in 0..64 {
                        let wanted = attempt(std::ptr::null_mut(), 0);
                        if wanted < 0 { answer = Err(errno()); break; }
                        let mut space = vec![0u8; wanted as usize + 1];
                        let used = attempt(space.as_mut_ptr().cast(), space.len());
                        if used >= 0 { space.truncate(used as usize); answer = Ok(octets(space)); break; }
                        answer = Err(errno());
                        if errno() != libc::ERANGE { break; }
                    }
                    answer
                } else {
                    checked(attempt(std::ptr::null_mut(), 0) as i64)
                }
            }
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
                let via_descriptor = matches!(params[0].settled(), Value::Small(_) | Value::Huge(_));
                if via_descriptor {
                    let copied = libc::fcntl(int(0)? as _, libc::F_DUPFD_CLOEXEC, 0);
                    stream = if copied < 0 {std::ptr::null_mut()} else {
                        let opened = libc::fdopendir(copied);
                        if opened.is_null() {let saved = errno(); libc::close(copied); *libc::__errno_location() = saved;}
                        opened
                    };
                } else {stream = libc::opendir(path(0)?.as_ptr());}
                if stream.is_null() {Err(errno())} else {
                    let token = NEXT_DIR.with(|counter| {let value = counter.get(); counter.set(value + 1); value});
                    OPEN_DIRS.with(|entries| {entries.borrow_mut().insert(token, DirStream {pointer: stream, reset: via_descriptor});});
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
            "confstr" => {
                let which = int(0)? as libc::c_int;
                let extent = libc::confstr(which, std::ptr::null_mut(), 0);
                if extent == 0 {
                    // A selector the library does not know has left an
                    // errno behind; a value that is merely empty would
                    // have asked for room for its terminating zero.
                    match std::io::Error::last_os_error().raw_os_error() {
                        Some(code) if code != 0 => Err(code),
                        _ => Err(libc::EINVAL),
                    }
                }
                else {
                    let mut landing = vec![0u8; extent];
                    libc::confstr(which, landing.as_mut_ptr().cast(), extent);
                    landing.truncate(extent - 1);
                    Ok(Value::text(&String::from_utf8_lossy(&landing)))
                }
            }
            "get_inheritable" | "set_inheritable" | "get_blocking" | "set_blocking" => {
            let (get, set, bit) = match operation.as_ref() {
                "get_inheritable" | "set_inheritable" => (libc::F_GETFD, libc::F_SETFD, libc::FD_CLOEXEC),
                _ => (libc::F_GETFL, libc::F_SETFL, libc::O_NONBLOCK),
            };
            let fd = int(0)? as i32;
            let flags = libc::fcntl(fd, get);
            if flags < 0 { Err(std::io::Error::last_os_error().raw_os_error().unwrap_or(libc::EIO)) }
            else if matches!(operation.as_ref(), "get_inheritable" | "get_blocking") { Ok(Value::Flag(flags & bit == 0)) }
            else {
                let next = if params[1].is_true() { flags & !bit } else { flags | bit };
                checked(libc::fcntl(fd, set, next) as i64)
            }
        },
        "pipe" => {
            let mut descriptors = [-1; 2];
            checked(libc::pipe2(descriptors.as_mut_ptr(), libc::O_CLOEXEC) as i64)
                .map(|_| Value::tuple(descriptors.into_iter().map(|fd| Value::Small(fd as i64)).collect()))
        },
        "tcgetattr" => {
            let mut state: libc::termios = std::mem::zeroed();
            if libc::tcgetattr(int(0)? as _, &mut state) == -1 { Err(errno()) } else {
                let mut parts: Vec<Value> = [state.c_iflag as i64, state.c_oflag as i64, state.c_cflag as i64, state.c_lflag as i64,
                    libc::cfgetispeed(&state) as i64, libc::cfgetospeed(&state) as i64].into_iter().map(Value::Small).collect();
                parts.push(octets(state.c_cc.as_slice()));
                Ok(Value::tuple(parts))
            }
        },
        "tcsetattr" => {
            let descriptor = int(0)? as libc::c_int;
            let mut state: libc::termios = std::mem::zeroed();
            if libc::tcgetattr(descriptor, &mut state) == -1 { Err(errno()) } else {
                state.c_iflag = int(2)? as _; state.c_oflag = int(3)? as _; state.c_cflag = int(4)? as _; state.c_lflag = int(5)? as _;
                let controls = data_of(&params[8])?;
                state.c_cc.iter_mut().zip(controls.iter()).for_each(|(slot, byte)| *slot = *byte);
                let (input, output) = (int(6)? as libc::speed_t, int(7)? as libc::speed_t);
                if libc::cfsetispeed(&mut state, input) == -1 || libc::cfsetospeed(&mut state, output) == -1 { Err(errno()) }
                else { checked(libc::tcsetattr(descriptor, int(1)? as _, &state) as i64) }
            }
        },
        "tcflow" => {
            let (descriptor, argument) = (int(0)? as libc::c_int, int(2)? as libc::c_int);
            checked(match int(1)? {
                0 => libc::tcsendbreak(descriptor, argument),
                1 => libc::tcdrain(descriptor),
                2 => libc::tcflush(descriptor, argument),
                _ => libc::tcflow(descriptor, argument),
            } as i64)
        },
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
            "poll" => {
                let rows = match params[0].settled() { Value::Tuple(rows) => rows, _ => return Err(String::from("TypeError: poll requests must be tuple")) };
                let mut descriptors: Vec<libc::pollfd> = Vec::with_capacity(rows.len());
                for row in rows.iter() {
                    let Value::Tuple(parts) = row.settled() else { return Err(String::from("TypeError: poll entry must be tuple")) };
                    descriptors.push(libc::pollfd { fd: parts[0].as_big()?.to_i32().ok_or("OverflowError: file descriptor out of range")?,
                        events: parts[1].as_big()?.to_i16().ok_or("OverflowError: poll event out of range")?, revents: 0 });
                }
                match libc::poll(descriptors.as_mut_ptr(), descriptors.len() as libc::nfds_t, int(1)? as i32) {
                    -1 => Err(errno()),
                    _ => Ok(Value::tuple(descriptors.into_iter().filter_map(|descriptor| (descriptor.revents != 0).then(||
                        Value::tuple(vec![Value::Small(i64::from(descriptor.fd)), Value::Small(i64::from(descriptor.revents))]))).collect())),
                }
            }
            "socket_shutdown" => checked(libc::shutdown(int(0)? as i32, int(1)? as i32) as i64),
            "socketpair" => {
                let mut pair = [0 as libc::c_int; 2];
                let result = libc::socketpair(int(0)? as i32, int(1)? as i32 | libc::SOCK_CLOEXEC, int(2)? as i32, pair.as_mut_ptr());
                if result == -1 { Err(errno()) } else { Ok(Value::tuple(vec![Value::Small(pair[0] as i64), Value::Small(pair[1] as i64)])) }
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
            // Block until some descriptor in each row is ready, mirroring
            // selectmodule.c: object conversion and range checks are the
            // Python module's; here the sets are built and the call made.
            // An address crosses this bridge as the bytes of its sockaddr: the
            // Python module alone knows each family's layout.
            "sock_bind" | "sock_connect" => {
                let target = data_of(&params[1])?;
                let (descriptor, width) = (int(0)? as i32, target.len() as libc::socklen_t);
                match operation.as_ref() {
                    "sock_bind" => checked(libc::bind(descriptor, target.as_ptr().cast(), width).into()),
                    _ => checked(libc::connect(descriptor, target.as_ptr().cast(), width).into()),
                }
            }
            "sock_listen" => checked(libc::listen(int(0)? as _, int(1)? as _).into()),
            "sock_accept" => {
                let mut room = [0u8; 128];
                let mut filled: libc::socklen_t = 128;
                match libc::accept4(int(0)? as _, room.as_mut_ptr().cast(), &mut filled, libc::SOCK_CLOEXEC) {
                    -1 => Err(errno()),
                    accepted => Ok(Value::tuple(vec![Value::Small(accepted.into()), octets(&room[..filled as usize])])),
                }
            }
            "sock_name" => {
                let mut room = [0u8; 128];
                let mut filled: libc::socklen_t = 128;
                let outcome = match int(1)? {
                    0 => libc::getsockname(int(0)? as _, room.as_mut_ptr().cast(), &mut filled),
                    _ => libc::getpeername(int(0)? as _, room.as_mut_ptr().cast(), &mut filled),
                };
                if outcome == -1 {Err(errno())} else {Ok(octets(&room[..filled as usize]))}
            }
            "sock_recv" | "sock_recvfrom" => {
                let mut incoming = vec![0u8; int(1)? as usize];
                let mut room = [0u8; 128];
                let mut filled: libc::socklen_t = 128;
                let count = libc::recvfrom(int(0)? as _, incoming.as_mut_ptr().cast(), incoming.len(), int(2)? as _, room.as_mut_ptr().cast(), &mut filled);
                if count == -1 {Err(errno())} else {
                    // Under MSG_TRUNC the count is the datagram's full length, which the
                    // buffer may not hold; the answer carries that count after the bytes.
                    incoming.truncate(count as usize);
                    let length = Value::Small(count as i64);
                    Ok(match operation.as_ref() {
                        "sock_recv" => Value::tuple(vec![octets(incoming), length]),
                        _ => Value::tuple(vec![octets(incoming), octets(&room[..filled as usize]), length]),
                    })
                }
            }
            // A fourth argument to send is how far into the buffer to begin.
            "sock_send" | "sock_sendto" => {
                let buffer = data_of(&params[1])?;
                let outgoing = match (operation.as_ref(), params.get(3)) {("sock_send", Some(_)) => &buffer[(int(3)? as usize).min(buffer.len())..], _ => &buffer[..]};
                let count = match operation.as_ref() {
                    "sock_send" => libc::send(int(0)? as _, outgoing.as_ptr().cast(), outgoing.len(), int(2)? as _),
                    _ => {
                        let target = data_of(&params[3])?;
                        libc::sendto(int(0)? as _, outgoing.as_ptr().cast(), outgoing.len(), int(2)? as _, target.as_ptr().cast(), target.len() as libc::socklen_t)
                    }
                };
                checked(count as i64)
            }
            "sock_setopt" => {
                let setting = data_of(&params[3])?;
                checked(libc::setsockopt(int(0)? as _, int(1)? as _, int(2)? as _, setting.as_ptr().cast(), setting.len() as libc::socklen_t).into())
            }
            "sock_getopt" => {
                let mut setting = vec![0u8; int(3)? as usize];
                let mut filled = setting.len() as libc::socklen_t;
                match libc::getsockopt(int(0)? as _, int(1)? as _, int(2)? as _, setting.as_mut_ptr().cast(), &mut filled) {
                    -1 => Err(errno()),
                    _ => {setting.truncate(filled as usize); Ok(octets(setting))}
                }
            }
            "getaddrinfo" => {
                let optional = |at: usize| -> Result<Option<CString>, String> {
                    match params[at].settled() {Value::Nil => Ok(None), _ => path(at).map(Some)}
                };
                let (node, service) = (optional(0)?, optional(1)?);
                let mut request: libc::addrinfo = std::mem::zeroed();
                request.ai_family = int(2)? as _;
                request.ai_socktype = int(3)? as _;
                request.ai_protocol = int(4)? as _;
                request.ai_flags = int(5)? as _;
                let mut records: *mut libc::addrinfo = std::ptr::null_mut();
                let text_or_null = |item: &Option<CString>| item.as_ref().map(|c| c.as_ptr()).unwrap_or(std::ptr::null());
                let status = libc::getaddrinfo(text_or_null(&node), text_or_null(&service), &request, &mut records);
                if status != 0 {Ok(lookup_words(status))} else {
                    let mut listing = Vec::new();
                    let mut walker = records;
                    while let Some(record) = walker.as_ref() {
                        let sockaddr = std::slice::from_raw_parts(record.ai_addr.cast::<u8>(), record.ai_addrlen as usize);
                        let official = match record.ai_canonname.as_ref() {None => octets(Vec::new()), Some(_) => octets(CStr::from_ptr(record.ai_canonname).to_bytes())};
                        listing.push(Value::tuple(vec![Value::Small(record.ai_family.into()), Value::Small(record.ai_socktype.into()),
                            Value::Small(record.ai_protocol.into()), official, octets(sockaddr)]));
                        walker = record.ai_next;
                    }
                    libc::freeaddrinfo(records);
                    Ok(Value::tuple(vec![Value::Small(0), Value::Vector(crate::tuples::Sequence::plain(listing))]))
                }
            }
            "getnameinfo" => {
                let sockaddr = data_of(&params[0])?;
                let mut node = [0 as libc::c_char; 1025];
                let mut service = [0 as libc::c_char; 32];
                let status = libc::getnameinfo(sockaddr.as_ptr().cast(), sockaddr.len() as libc::socklen_t, node.as_mut_ptr(), 1025, service.as_mut_ptr(), 32, int(1)? as _);
                match status {
                    0 => Ok(Value::tuple(vec![Value::Small(0), octets(CStr::from_ptr(node.as_ptr()).to_bytes()), octets(CStr::from_ptr(service.as_ptr()).to_bytes())])),
                    _ => Ok(lookup_words(status)),
                }
            }
            // Address text to packed bytes: family 0 takes inet_aton's loose
            // spellings, a real family inet_pton's strict ones; Nil when not an address.
            "inet_packed" => {
                let spelling = path(1)?;
                match int(0)? as i32 {
                    0 => {
                        let mut quad: libc::in_addr = std::mem::zeroed();
                        match parse_dotted(spelling.as_ptr(), &mut quad) {0 => Ok(Value::Nil), _ => Ok(octets(quad.s_addr.to_ne_bytes()))}
                    }
                    family => {
                        let mut room = [0u8; 16];
                        match pack_address(family, spelling.as_ptr(), room.as_mut_ptr().cast()) {
                            1 => Ok(octets(&room[..match family {libc::AF_INET => 4, _ => 16}])),
                            0 => Ok(Value::Nil),
                            _ => Err(errno()),
                        }
                    }
                }
            }
            "inet_text" => {
                let packed = data_of(&params[1])?;
                let mut spelling = [0 as libc::c_char; 46];
                match spell_address(int(0)? as _, packed.as_ptr().cast(), spelling.as_mut_ptr(), 46).as_ref() {
                    None => Err(errno()),
                    Some(_) => Ok(Value::text(&CStr::from_ptr(spelling.as_ptr()).to_string_lossy())),
                }
            }
            // Service and protocol databases: 0 service name to port, 1 port to
            // service name, 2 protocol name to number; Nil for no such entry.
            "sock_db" => {
                let protocol = match params[2].settled() {Value::Nil => None, _ => Some(path(2)?)};
                let protocol_text = protocol.as_ref().map(|c| c.as_ptr()).unwrap_or(std::ptr::null());
                Ok(match int(0)? {
                    0 => match libc::getservbyname(path(1)?.as_ptr(), protocol_text).as_ref() {
                        None => Value::Nil, Some(service) => Value::Small(i64::from(u16::from_be(service.s_port as u16)))
                    },
                    1 => match libc::getservbyport(i32::from((int(1)? as u16).to_be()), protocol_text).as_ref() {
                        None => Value::Nil, Some(service) => octets(CStr::from_ptr(service.s_name).to_bytes())
                    },
                    _ => match libc::getprotobyname(path(1)?.as_ptr()).as_ref() {
                        None => Value::Nil, Some(protocol) => Value::Small(protocol.p_proto.into())
                    },
                })
            }
            "socket_hostbyname" => Ok(forward_record(&params[0])?),
            "select" => {
                let descriptor_row = |at: usize| -> Result<Vec<i32>, String> {
                    match params[at].settled() {
                        Value::Vector(row) => row.iter().map(|item| match item.settled() {
                            Value::Small(n) => i32::try_from(n).map_err(|_| String::from("ValueError: filedescriptor out of range in select()")),
                            Value::Huge(_) => Err(String::from("ValueError: filedescriptor out of range in select()")),
                            _ => Err(String::from("TypeError: an integer is required")),
                        }).collect(),
                        _ => Err(String::from("TypeError: a sequence of integers is required")),
                    }
                };
                let want_read = descriptor_row(0)?;
                let want_write = descriptor_row(1)?;
                let want_error = descriptor_row(2)?;
                let mut highest = -1i32;
                for row in [&want_read, &want_write, &want_error] {
                    for &fd in row {
                        if fd < 0 || fd as usize >= libc::FD_SETSIZE { return Err(String::from("ValueError: filedescriptor out of range in select()")); }
                        if fd > highest { highest = fd; }
                    }
                }
                let mut read_set = fd_set_of(&want_read);
                let mut write_set = fd_set_of(&want_write);
                let mut error_set = fd_set_of(&want_error);
                let micros = int(3)?;
                let kept = |set: &libc::fd_set, row: &[i32]| Value::Vector(crate::tuples::Sequence::plain(
                    row.iter().copied().filter(|fd| libc::FD_ISSET(*fd, set)).map(|fd| Value::Small(fd as i64)).collect()));
                // The reference ends an interrupted call only for a signal
                // its own check takes; anything else starts the call over
                // against the original deadline with the time remaining,
                // sets built fresh. Signal delivery can raise here; a
                // returning handler leaves the deadline in force.
                let moment = std::time::Instant::now();
                let expires = if micros < 0 { None } else {
                    moment.checked_add(std::time::Duration::from_micros(micros as u64))
                };
                let mut time_left = if micros < 0 { None } else { Some(micros) };
                let nothing = || Value::tuple(vec![
                    Value::Vector(crate::tuples::Sequence::plain(Vec::new())),
                    Value::Vector(crate::tuples::Sequence::plain(Vec::new())),
                    Value::Vector(crate::tuples::Sequence::plain(Vec::new()))]);
                loop {
                    let mut limit: libc::timeval = std::mem::zeroed();
                    let when: *mut libc::timeval = match time_left {
                        None => std::ptr::null_mut(),
                        Some(whole) => { limit.tv_sec = whole / 1_000_000; limit.tv_usec = whole % 1_000_000; &mut limit },
                    };
                    let found = libc::select(highest + 1, &mut read_set, &mut write_set, &mut error_set, when);
                    if found != -1 {
                        break Ok(Value::tuple(vec![kept(&read_set, &want_read), kept(&write_set, &want_write), kept(&error_set, &want_error)]));
                    }
                    let cause = errno();
                    if cause != libc::EINTR { break Err(cause); }
                    interrupted()?;
                    if let Some(end) = expires {
                        let left = end.saturating_duration_since(std::time::Instant::now());
                        if left.is_zero() { break Ok(nothing()); }
                        let mut whole = left.as_micros() as i64;
                        if left.subsec_nanos() % 1000 != 0 { whole += 1; }
                        time_left = Some(whole);
                    }
                    read_set = fd_set_of(&want_read);
                    write_set = fd_set_of(&want_write);
                    error_set = fd_set_of(&want_error);
                }
            }
            "urandom" => {
                use std::io::Read as _;
                let mut content = vec![0; int(0)? as usize];
                let result = std::fs::File::open("/dev/urandom").and_then(|mut source| source.read_exact(&mut content));
                match result {Err(e) => Err(e.raw_os_error().unwrap_or(libc::EIO)), Ok(()) => Ok(octets(content))}
            }
            "forkpty" => split_onto_terminal(),
            "_exit" => { libc::_exit(int(0)? as libc::c_int); },
            "waitpid" => harvest(int(0)?, int(1)?, &mut interrupted)?,
            _ => return Err(String::from("NotImplementedError: unknown POSIX operation")),
        }
    };
    let (error, result) = match answer {Err(code) => (code, Value::Nil), Ok(value) => (0, value)};
    Ok(Value::tuple(vec![Value::Small(error.into()), result]))
}

// Bind the platform reverse resolver and its diagnostic strings.
unsafe extern "C" {
    #[link_name = "hstrerror"]
    fn resolver_message(number: libc::c_int) -> *const libc::c_char;
    #[link_name = "gethostbyname_r"]
    fn resolve_forward(name: *const libc::c_char, record: *mut libc::hostent, work: *mut libc::c_char, available: usize,
        resolved: *mut *mut libc::hostent, diagnostic: *mut libc::c_int) -> libc::c_int;
    #[link_name = "inet_aton"]
    fn parse_dotted(spelling: *const libc::c_char, quad: *mut libc::in_addr) -> libc::c_int;
    #[link_name = "inet_pton"]
    fn pack_address(domain: libc::c_int, spelling: *const libc::c_char, room: *mut libc::c_void) -> libc::c_int;
    #[link_name = "inet_ntop"]
    fn spell_address(domain: libc::c_int, packed: *const libc::c_void, room: *mut libc::c_char, size: libc::socklen_t) -> *const libc::c_char;
    #[link_name = "gethostbyaddr_r"]
    fn resolve_reverse(ip: *const libc::c_void, count: libc::socklen_t, domain: libc::c_int,
        record: *mut libc::hostent, work: *mut libc::c_char, available: usize,
        resolved: *mut *mut libc::hostent, diagnostic: *mut libc::c_int) -> libc::c_int;
}

// Gather canonical names, aliases and addresses from the host database.
fn reverse_record(value: &Value) -> Result<Value, String> {
    let query = CString::new(data_of(value)?).map_err(|_| String::from("ValueError: embedded null byte"))?;
    unsafe {
        let mut request = std::mem::zeroed::<libc::addrinfo>();
        request.ai_socktype = libc::SOCK_STREAM;
        request.ai_family = libc::AF_UNSPEC;
        let mut records: *mut libc::addrinfo = std::ptr::null_mut();
        let status = libc::getaddrinfo(query.as_ptr(),std::ptr::null(),&request,&mut records);
        if status != 0 { return Ok(lookup_failure(1,status,libc::gai_strerror(status))); }
        if records.is_null() { return Ok(lookup_failure(1,libc::EAI_NONAME,libc::gai_strerror(libc::EAI_NONAME))); }
        let domain = (*records).ai_family;
        let payload: Vec<u8> = match domain {
            libc::AF_INET6 => (*(*records).ai_addr.cast::<libc::sockaddr_in6>()).sin6_addr.s6_addr.into(),
            libc::AF_INET => {
                let address = (*(*records).ai_addr.cast::<libc::sockaddr_in>()).sin_addr;
                address.s_addr.to_ne_bytes().into()
            }
            _ => {libc::freeaddrinfo(records);return Ok(lookup_failure(1,libc::EAI_FAMILY,libc::gai_strerror(libc::EAI_FAMILY)));}
        };
        libc::freeaddrinfo(records);
        let mut scratch = vec![0u8; 8192];
        let mut host = std::mem::zeroed::<libc::hostent>();
        let mut returned: *mut libc::hostent = std::ptr::null_mut();
        let mut problem = 0;
        loop {
            match resolve_reverse(payload.as_ptr().cast(),payload.len() as libc::socklen_t,domain,
                &mut host,scratch.as_mut_ptr().cast(),scratch.len(),&mut returned,&mut problem) {
                libc::ERANGE => {let next=scratch.len().checked_mul(2).ok_or("MemoryError: resolver buffer is too large")?;scratch.resize(next,0);}
                _ => break,
            }
        }
        if returned.is_null() { return Ok(lookup_failure(2,problem,resolver_message(problem))); }
        host_record(&host)
    }
}

// A failed lookup; and a found host's official name, nicknames and numbers.
fn lookup_failure(category: i64, number: i32, message: *const libc::c_char) -> Value {
    let diagnostic = unsafe {if message.is_null() { Vec::new() } else { CStr::from_ptr(message).to_bytes().into() }};
    Value::tuple(vec![Value::Small(category),Value::Small(i64::from(number)),octets(diagnostic),Value::Nil])
}
fn host_record(host: &libc::hostent) -> Result<Value, String> {
    unsafe {
        let mut nicknames = Vec::new();
        if !host.h_aliases.is_null() {
            let mut offset = 0;
            while !(*host.h_aliases.add(offset)).is_null() {
                let alias = CStr::from_ptr(*host.h_aliases.add(offset)).to_bytes();
                nicknames.push(octets(alias)); offset += 1;
            }
        }
        let mut numbers = Vec::new();
        if !host.h_addr_list.is_null() {
            let mut offset = 0;
            while !(*host.h_addr_list.add(offset)).is_null() {
                let address = std::slice::from_raw_parts((*host.h_addr_list.add(offset)).cast::<u8>(),host.h_length as usize);
                let representation = match (host.h_addrtype,address.len()) {
                    (libc::AF_INET,4) => std::net::Ipv4Addr::from(<[u8;4]>::try_from(address).unwrap()).to_string(),
                    (libc::AF_INET6,16) => std::net::Ipv6Addr::from(<[u8;16]>::try_from(address).unwrap()).to_string(),
                    _ => return Err(String::from("OSError: resolver returned an unsupported address family")),
                };
                numbers.push(octets(representation.into_bytes()));offset += 1;
            }
        }
        let official = octets(CStr::from_ptr(host.h_name).to_bytes());
        let resolved = Value::tuple(vec![official,Value::Vector(crate::tuples::Sequence::plain(nicknames)),Value::Vector(crate::tuples::Sequence::plain(numbers))]);
        Ok(Value::tuple(vec![Value::Small(0),Value::Small(0),octets(Vec::new()),resolved]))
    }
}

// A resolver failure as (status, words), for the Python side to raise.
fn lookup_words(status: i32) -> Value {
    let words = unsafe {CStr::from_ptr(libc::gai_strerror(status))}.to_bytes();
    Value::tuple(vec![Value::Small(status.into()), octets(words)])
}

// Look a host name up in the host database, answering like reverse_record.
fn forward_record(value: &Value) -> Result<Value, String> {
    let query = CString::new(data_of(value)?).map_err(|_| String::from("ValueError: embedded null byte"))?;
    unsafe {
        let mut scratch = vec![0u8; 8192];
        let mut host = std::mem::zeroed::<libc::hostent>();
        let mut returned: *mut libc::hostent = std::ptr::null_mut();
        let mut problem = 0;
        loop {
            match resolve_forward(query.as_ptr(), &mut host, scratch.as_mut_ptr().cast(), scratch.len(), &mut returned, &mut problem) {
                libc::ERANGE => {let next=scratch.len().checked_mul(2).ok_or("MemoryError: resolver buffer is too large")?;scratch.resize(next,0);}
                _ => break,
            }
        }
        if returned.is_null() { return Ok(lookup_failure(2,problem,resolver_message(problem))); }
        host_record(&host)
    }
}
