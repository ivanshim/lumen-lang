use std::collections::HashMap;
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;

fn last_errno() -> i32 {
    std::io::Error::last_os_error().raw_os_error().unwrap_or(libc::EIO)
}

/// Resolve symbolic links matching CPython's posixpath._joinrealpath semantics.
/// mode: 0 = strict=False, 1 = strict=True, 2 = ALLOW_MISSING.
pub fn realpath_bytes(input: &[u8], mode: i64) -> Result<Result<Vec<u8>, (i32, Vec<u8>)>, String> {
    if input.contains(&0) {
        return Err("ValueError: embedded null character".into());
    }
    let strict_loop = mode != 0;
    let ignore_all = mode == 0;
    let ignore_missing = mode == 2;
    let cwd = if input.starts_with(b"/") {
        Vec::new()
    } else {
        std::env::current_dir()
            .map(|d| d.as_os_str().as_bytes().to_vec())
            .map_err(|_| "OSError: getcwd failed".to_string())?
    };
    let mut path: Vec<u8> = if input.starts_with(b"/") { vec![b'/'] } else { cwd };
    let mut rest: Vec<Option<Vec<u8>>> = input.split(|&b| b == b'/').rev().map(|p| Some(p.to_vec())).collect();
    let mut part_count = rest.len();
    let mut seen: HashMap<Vec<u8>, Option<Vec<u8>>> = HashMap::new();
    while part_count > 0 {
        let Some(name) = rest.pop().flatten() else {
            if let Some(Some(sym)) = rest.pop() {
                seen.insert(sym, Some(path.clone()));
            }
            continue;
        };
        part_count -= 1;
        if name.is_empty() || name == b"." {
            continue;
        }
        if name == b".." {
            if let Some(pos) = path.iter().rposition(|&b| b == b'/') {
                if pos == 0 { path.truncate(1); } else { path.truncate(pos); }
            }
            continue;
        }
        let mut newpath = Vec::with_capacity(path.len() + 1 + name.len());
        newpath.extend_from_slice(&path);
        if path != b"/" { newpath.push(b'/'); }
        newpath.extend_from_slice(&name);
        let Ok(c_newpath) = CString::new(newpath.as_slice()) else {
            return Err("ValueError: embedded null byte".into());
        };
        let mut st: libc::stat = unsafe { std::mem::zeroed() };
        if unsafe { libc::lstat(c_newpath.as_ptr(), &mut st) } != 0 {
            let err = last_errno();
            if ignore_all || (ignore_missing && err == libc::ENOENT) {
                path = newpath;
                continue;
            }
            return Ok(Err((err, newpath)));
        }
        let kind = st.st_mode & libc::S_IFMT;
        if kind != libc::S_IFLNK {
            if strict_loop && part_count > 0 && kind != libc::S_IFDIR {
                return Ok(Err((libc::ENOTDIR, newpath)));
            }
            path = newpath;
            continue;
        }
        if let Some(cached) = seen.get(&newpath) {
            if let Some(resolved) = cached {
                path = resolved.clone();
                continue;
            }
            if strict_loop {
                return Ok(Err((libc::ELOOP, newpath)));
            }
            path = newpath;
            continue;
        }
        let mut cap = 128usize;
        let target_res: Result<Vec<u8>, i32> = loop {
            let mut buf = vec![0u8; cap];
            let count = unsafe { libc::readlink(c_newpath.as_ptr(), buf.as_mut_ptr() as *mut libc::c_char, cap) };
            if count < 0 { break Err(last_errno()); }
            let n = count as usize;
            if n < cap { buf.truncate(n); break Ok(buf); }
            cap *= 2;
        };
        match target_res {
            Err(err) => {
                if ignore_all || (ignore_missing && err == libc::ENOENT) {
                    path = newpath;
                    continue;
                }
                return Ok(Err((err, newpath)));
            }
            Ok(target) => {
                if target.starts_with(b"/") { path = vec![b'/']; }
                seen.insert(newpath.clone(), None);
                rest.push(Some(newpath));
                rest.push(None);
                let before = rest.len();
                rest.extend(target.split(|&b| b == b'/').rev().map(|p| Some(p.to_vec())));
                part_count += rest.len() - before;
            }
        }
    }
    Ok(Ok(path))
}

/// Iteratively create directory and missing parents (mode 0o777, exist_ok=True).
pub fn makedirs_bytes(input: &[u8]) -> Result<Result<(), (i32, Vec<u8>)>, String> {
    if input.contains(&0) {
        return Err("ValueError: embedded null character".into());
    }
    if input.is_empty() {
        return Ok(Ok(()));
    }
    let mut cur = input.to_vec();
    while cur.len() > 1 && cur.last() == Some(&b'/') {
        cur.pop();
    }
    let mut missing: Vec<Vec<u8>> = Vec::new();
    loop {
        if cur.is_empty() { break; }
        let Ok(c_cur) = CString::new(cur.as_slice()) else {
            return Err("ValueError: embedded null character".into());
        };
        let mut st: libc::stat = unsafe { std::mem::zeroed() };
        if unsafe { libc::stat(c_cur.as_ptr(), &mut st) } == 0 {
            if (st.st_mode & libc::S_IFMT) == libc::S_IFDIR {
                break;
            }
            if missing.is_empty() {
                return Ok(Err((libc::EEXIST, cur)));
            }
            break;
        }
        missing.push(cur.clone());
        match cur.iter().rposition(|&b| b == b'/') {
            Some(0) | None => break,
            Some(pos) => {
                cur.truncate(pos);
                while cur.len() > 1 && cur.last() == Some(&b'/') { cur.pop(); }
            }
        }
    }
    for dir in missing.into_iter().rev() {
        if dir.ends_with(b"/.") || dir == b"." { continue; }
        let Ok(c_dir) = CString::new(dir.as_slice()) else {
            return Err("ValueError: embedded null character".into());
        };
        if unsafe { libc::mkdir(c_dir.as_ptr(), 0o777) } != 0 {
            let err = last_errno();
            let mut st: libc::stat = unsafe { std::mem::zeroed() };
            if !(unsafe { libc::stat(c_dir.as_ptr(), &mut st) } == 0 && (st.st_mode & libc::S_IFMT) == libc::S_IFDIR) {
                return Ok(Err((err, dir)));
            }
        }
    }
    Ok(Ok(()))
}


struct DirFrame {
    fd: libc::c_int,
    name_in_parent: CString,
    path: Vec<u8>,
    subdirs: Vec<(CString, Vec<u8>)>,
}

struct FdStack(Vec<DirFrame>);

impl Drop for FdStack {
    fn drop(&mut self) {
        for frame in self.0.drain(..) {
            unsafe { libc::close(frame.fd); }
        }
    }
}

fn join_child(parent: &[u8], name: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(parent.len() + 1 + name.len());
    out.extend_from_slice(parent);
    if !parent.is_empty() && !parent.ends_with(b"/") {
        out.push(b'/');
    }
    out.extend_from_slice(name);
    out
}

fn scan_and_unlink(
    dir_fd: libc::c_int,
    dir_path: &[u8],
    ignore_errors: bool,
) -> Result<Vec<(CString, Vec<u8>)>, (i32, Vec<u8>)> {
    let dup_fd = unsafe { libc::dup(dir_fd) };
    if dup_fd < 0 {
        let err = last_errno();
        return if ignore_errors { Ok(Vec::new()) } else { Err((err, dir_path.to_vec())) };
    }
    let dir_ptr = unsafe { libc::fdopendir(dup_fd) };
    if dir_ptr.is_null() {
        let err = last_errno();
        unsafe { libc::close(dup_fd); }
        return if ignore_errors { Ok(Vec::new()) } else { Err((err, dir_path.to_vec())) };
    }
    let mut subdirs = Vec::new();
    let mut first_err: Option<(i32, Vec<u8>)> = None;
    loop {
        unsafe { *libc::__errno_location() = 0; }
        let entry = unsafe { libc::readdir(dir_ptr) };
        if entry.is_null() {
            let err = unsafe { *libc::__errno_location() };
            if err != 0 && !ignore_errors && first_err.is_none() {
                first_err = Some((err, dir_path.to_vec()));
            }
            break;
        }
        let name_bytes = unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
        if name_bytes == b"." || name_bytes == b".." {
            continue;
        }
        let d_type = unsafe { (*entry).d_type };
        let is_dir = match d_type {
            libc::DT_DIR => true,
            libc::DT_UNKNOWN => {
                let mut cst: libc::stat = unsafe { std::mem::zeroed() };
                unsafe {
                    libc::fstatat(dir_fd, (*entry).d_name.as_ptr(), &mut cst, libc::AT_SYMLINK_NOFOLLOW) == 0
                        && (cst.st_mode & libc::S_IFMT) == libc::S_IFDIR
                }
            }
            _ => false,
        };
        if is_dir {
            if let Ok(c_name) = CString::new(name_bytes) {
                subdirs.push((c_name, join_child(dir_path, name_bytes)));
            }
            continue;
        }
        if unsafe { libc::unlinkat(dir_fd, (*entry).d_name.as_ptr(), 0) } != 0 {
            let err = last_errno();
            if err != libc::ENOENT && !ignore_errors {
                first_err = Some((err, join_child(dir_path, name_bytes)));
                break;
            }
        }
    }
    unsafe { libc::closedir(dir_ptr); }
    match first_err {
        Some(err) => Err(err),
        None => Ok(subdirs),
    }
}

/// Recursively remove a plain directory tree rooted at `input`.
/// Returns `Ok(Ok(false))` without modifying disk if top-level is missing, non-dir, or a symlink.
pub fn rmtree_bytes(input: &[u8], ignore_errors: bool) -> Result<Result<bool, (i32, Vec<u8>)>, String> {
    if input.contains(&0) {
        return Err("ValueError: embedded null character".into());
    }
    if input.is_empty() {
        return Ok(Ok(false));
    }
    let Ok(c_top) = CString::new(input) else {
        return Err("ValueError: embedded null character".into());
    };
    let mut top_st: libc::stat = unsafe { std::mem::zeroed() };
    if unsafe { libc::lstat(c_top.as_ptr(), &mut top_st) } != 0 {
        return Ok(Ok(false));
    }
    if (top_st.st_mode & libc::S_IFMT) != libc::S_IFDIR {
        return Ok(Ok(false));
    }
    let top_fd = unsafe { libc::open(c_top.as_ptr(), libc::O_RDONLY | libc::O_NONBLOCK | libc::O_NOFOLLOW) };
    if top_fd < 0 {
        return Ok(Ok(false));
    }
    let mut top_fst: libc::stat = unsafe { std::mem::zeroed() };
    if unsafe { libc::fstat(top_fd, &mut top_fst) } != 0
        || top_fst.st_dev != top_st.st_dev
        || top_fst.st_ino != top_st.st_ino
    {
        unsafe { libc::close(top_fd); }
        return Ok(Ok(false));
    }
    let root_subdirs = match scan_and_unlink(top_fd, input, ignore_errors) {
        Ok(subs) => subs,
        Err(err) => {
            unsafe { libc::close(top_fd); }
            return Ok(Err(err));
        }
    };
    let mut stack = FdStack(vec![DirFrame {
        fd: top_fd,
        name_in_parent: CString::default(),
        path: input.to_vec(),
        subdirs: root_subdirs,
    }]);
    while let Some(top) = stack.0.last_mut() {
        if let Some((c_name, child_path)) = top.subdirs.pop() {
            let parent_fd = top.fd;
            let mut cst: libc::stat = unsafe { std::mem::zeroed() };
            let have_lst = unsafe {
                libc::fstatat(parent_fd, c_name.as_ptr(), &mut cst, libc::AT_SYMLINK_NOFOLLOW)
            } == 0;
            let child_fd = unsafe {
                libc::openat(parent_fd, c_name.as_ptr(), libc::O_RDONLY | libc::O_NONBLOCK | libc::O_NOFOLLOW)
            };
            if child_fd < 0 {
                let err = last_errno();
                if err == libc::ENOENT || ignore_errors {
                    continue;
                }
                return Ok(Err((err, child_path)));
            }
            let mut cfst: libc::stat = unsafe { std::mem::zeroed() };
            if unsafe { libc::fstat(child_fd, &mut cfst) } != 0
                || (have_lst && (cfst.st_dev != cst.st_dev || cfst.st_ino != cst.st_ino))
                || (cfst.st_mode & libc::S_IFMT) != libc::S_IFDIR
            {
                unsafe { libc::close(child_fd); }
                if ignore_errors {
                    continue;
                }
                return Ok(Err((libc::ELOOP, child_path)));
            }
            let subdirs = match scan_and_unlink(child_fd, &child_path, ignore_errors) {
                Ok(subs) => subs,
                Err(err) => {
                    unsafe { libc::close(child_fd); }
                    return Ok(Err(err));
                }
            };
            stack.0.push(DirFrame { fd: child_fd, name_in_parent: c_name, path: child_path, subdirs });
        } else {
            let done = stack.0.pop().unwrap();
            unsafe { libc::close(done.fd); }
            let rm_res = if let Some(parent) = stack.0.last() {
                unsafe { libc::unlinkat(parent.fd, done.name_in_parent.as_ptr(), libc::AT_REMOVEDIR) }
            } else {
                unsafe { libc::rmdir(c_top.as_ptr()) }
            };
            if rm_res != 0 {
                let err = last_errno();
                if err != libc::ENOENT && !ignore_errors {
                    return Ok(Err((err, done.path)));
                }
            }
        }
    }
    Ok(Ok(true))
}
