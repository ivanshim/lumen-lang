use std::collections::{BTreeMap, HashMap};
use std::ffi::{CString, OsStr};
use std::fs::{self, DirBuilder, File, OpenOptions};
use std::hash::{Hash, Hasher};
use std::io::{self, Read, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};

fn invalid(path: &Path, reason: &str) -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, format!("{}: {reason}", path.display()))
}

fn owned(path: &Path, uid: u32) -> io::Result<fs::Metadata> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || metadata.uid() != uid {
        return Err(invalid(path, "source cache entry is symlinked or belongs to another user"));
    }
    Ok(metadata)
}

fn private_directory(path: &Path, uid: u32) -> io::Result<()> {
    match DirBuilder::new().mode(0o700).create(path) {
        Ok(()) => {},
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {},
        Err(error) => return Err(error),
    }
    let metadata = owned(path, uid)?;
    if !metadata.is_dir() || metadata.mode() & 0o7777 != 0o700 {
        return Err(invalid(path, "source cache directory must have mode 0700"));
    }
    Ok(())
}

fn relative(path: &str) -> io::Result<&Path> {
    let path = Path::new(path);
    if path.as_os_str().is_empty() || path.components().any(|part| !matches!(part, Component::Normal(_))) {
        return Err(invalid(path, "embedded filename is not a relative source path"));
    }
    Ok(path)
}

fn directories(root: &Path, path: &Path, uid: u32) -> io::Result<()> {
    let mut current = root.to_path_buf();
    for part in path.components() {
        current.push(part);
        private_directory(&current, uid)?;
    }
    Ok(())
}

fn new_file(path: &Path) -> io::Result<File> {
    OpenOptions::new().write(true).create_new(true).mode(0o600).open(path)
}

fn temporary(parent: &Path) -> io::Result<PathBuf> {
    let template = parent.join(".building-XXXXXX");
    let mut bytes = CString::new(template.as_os_str().as_bytes())?.into_bytes_with_nul();
    // mkdtemp creates a fresh 0700 directory with an unpredictable, exclusive name.
    if unsafe { libc::mkdtemp(bytes.as_mut_ptr().cast()) }.is_null() {
        return Err(io::Error::last_os_error());
    }
    Ok(PathBuf::from(OsStr::from_bytes(&bytes[..bytes.len() - 1])))
}

struct Staging(PathBuf);
impl Drop for Staging {
    fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); }
}

fn check_tree(path: &Path, uid: u32) -> io::Result<()> {
    let metadata = owned(path, uid)?;
    if metadata.is_dir() {
        for entry in fs::read_dir(path)? { check_tree(&entry?.path(), uid)?; }
    } else if !metadata.is_file() || metadata.nlink() != 1 {
        return Err(invalid(path, "source cache entry is not a private regular file"));
    }
    Ok(())
}

fn check_cache(cache: &Path, sources: &BTreeMap<&str, &str>, uid: u32) -> io::Result<PathBuf> {
    let metadata = owned(cache, uid)?;
    if !metadata.is_dir() || metadata.mode() & 0o7777 != 0o700 {
        return Err(invalid(cache, "source cache directory must have mode 0700"));
    }
    check_tree(cache, uid)?;
    let library = cache.join("langs/lib_python/modules");
    for (filename, source) in sources {
        let path = library.join(relative(filename)?);
        let mut file = OpenOptions::new().read(true).custom_flags(libc::O_NOFOLLOW).open(&path)?;
        let metadata = file.metadata()?;
        if !metadata.is_file() || metadata.uid() != uid || metadata.nlink() != 1 {
            return Err(invalid(&path, "embedded source is not a private regular file"));
        }
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)?;
        if contents != source.as_bytes() {
            return Err(invalid(&path, "cached source differs from the embedded source"));
        }
    }
    Ok(library)
}

fn copy_resource(from: &Path, to: &Path) -> io::Result<()> {
    let mut input = OpenOptions::new().read(true).custom_flags(libc::O_NOFOLLOW).open(from)?;
    let mut output = new_file(to)?;
    io::copy(&mut input, &mut output)?;
    output.set_permissions(fs::Permissions::from_mode(input.metadata()?.mode() & 0o777))
}

fn resources(from: &Path, to: &Path, uid: u32) -> io::Result<()> {
    if !from.is_dir() { return Ok(()); }
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let path = entry.path();
        let destination = to.join(entry.file_name());
        let kind = entry.file_type()?;
        if kind.is_symlink() { return Err(invalid(&path, "source resource is symlinked")); }
        if kind.is_dir() {
            if entry.file_name() == "__pycache__" { continue; }
            private_directory(&destination, uid)?;
            resources(&path, &destination, uid)?;
        } else if kind.is_file() && path.extension().is_none_or(|extension| extension != "pyc") && !destination.try_exists()? {
            copy_resource(&path, &destination)?;
        }
    }
    Ok(())
}

fn cache(sources: &BTreeMap<&str, &str>, original: &Path, temp: &Path) -> io::Result<PathBuf> {
    let uid = unsafe { libc::geteuid() };
    let temp = fs::canonicalize(temp)?;
    for ancestor in temp.ancestors() {
        let metadata = fs::symlink_metadata(ancestor)?;
        if !metadata.is_dir() || (metadata.uid() != uid && metadata.uid() != 0) ||
            (metadata.mode() & 0o022 != 0 && metadata.mode() & 0o1000 == 0) {
            return Err(invalid(ancestor, "temporary directory ancestry must be owner-controlled or sticky"));
        }
    }
    let parent = temp.join(format!("lumen-library-{uid}"));
    private_directory(&parent, uid)?;
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    original.hash(&mut hash);
    sources.hash(&mut hash);
    let cache = parent.join(format!("layout-{:016x}", hash.finish()));
    match fs::symlink_metadata(&cache) {
        Ok(_) => return check_cache(&cache, sources, uid),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {},
        Err(error) => return Err(error),
    }
    let staging = Staging(temporary(&parent)?);
    private_directory(&staging.0, uid)?;
    let layout = Path::new("langs/lib_python/modules");
    directories(&staging.0, layout, uid)?;
    let library = staging.0.join(layout);
    for (filename, source) in sources {
        let filename = relative(filename)?;
        if let Some(parent) = filename.parent() { directories(&library, parent, uid)?; }
        new_file(&library.join(filename))?.write_all(source.as_bytes())?;
    }
    resources(original, &library, uid)?;
    if let Some(project) = original.ancestors().nth(3).filter(|project| original.is_dir() && project.is_dir()) {
        for entry in fs::read_dir(project)? {
            let entry = entry?;
            let name = entry.file_name();
            if name == "langs" || name == "target" || name == "probe" || name.to_string_lossy().starts_with('.') { continue; }
            let destination = staging.0.join(name);
            if entry.file_type()?.is_dir() {
                private_directory(&destination, uid)?;
                resources(&entry.path(), &destination, uid)?;
            } else if entry.file_type()?.is_file() {
                copy_resource(&entry.path(), &destination)?;
            } else { return Err(invalid(&entry.path(), "project resource is not a regular file or directory")); }
        }
    }
    match fs::rename(&staging.0, &cache) {
        Ok(()) => check_cache(&cache, sources, uid),
        Err(error) => match fs::symlink_metadata(&cache) {
            Ok(_) => check_cache(&cache, sources, uid),
            Err(_) => Err(error),
        },
    }
}

pub fn library(request: &[(String, String, String, bool)], original: &Path) -> io::Result<PathBuf> {
    let filenames: HashMap<&str, &str> = request.iter().filter(|row| row.0 == "MODULE_FILE")
        .map(|row| (row.1.as_str(), row.2.as_str())).collect();
    let modules: HashMap<&str, &str> = request.iter().filter(|row| row.0 == "MODULE")
        .map(|row| (row.1.as_str(), row.2.as_str())).collect();
    let mut sources = BTreeMap::new();
    for (name, source) in modules {
        let filename = filenames.get(name).ok_or_else(|| io::Error::other("embedded module has no filename"))?;
        relative(filename)?;
        if let Some(previous) = sources.insert(*filename, source) {
            if previous != source { return Err(io::Error::other(format!("embedded modules disagree on source filename {filename} ({name})"))); }
        }
    }
    if let Some(first) = sources.keys().next() {
        if OpenOptions::new().write(true).open(original.join(first)).is_ok() { return Ok(original.to_path_buf()); }
    }
    cache(&sources, original, &std::env::temp_dir())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{symlink, PermissionsExt};

    fn sandbox() -> Staging { Staging(temporary(&std::env::temp_dir()).unwrap()) }
    fn sources() -> BTreeMap<&'static str, &'static str> { BTreeMap::from([("pkg/__init__.py", "answer = 42\n")]) }

    #[test]
    fn private_creation_reuse_and_content_validation() {
        let space = sandbox();
        let source = space.0.join("missing");
        let first = cache(&sources(), &source, &space.0).unwrap();
        assert_eq!(fs::read(first.join("pkg/__init__.py")).unwrap(), b"answer = 42\n");
        assert_eq!(cache(&sources(), &source, &space.0).unwrap(), first);
        let parent = first.ancestors().nth(4).unwrap();
        assert_eq!(fs::metadata(parent).unwrap().mode() & 0o7777, 0o700);
        fs::write(first.join("pkg/__init__.py"), b"answer = 0\n").unwrap();
        assert!(cache(&sources(), &source, &space.0).is_err());
    }

    #[test]
    fn rejects_symlinks_permissions_and_foreign_ownership() {
        let space = sandbox();
        let uid = unsafe { libc::geteuid() };
        let parent = space.0.join(format!("lumen-library-{uid}"));
        let victim = space.0.join("victim");
        private_directory(&victim, uid).unwrap();
        symlink(&victim, &parent).unwrap();
        assert!(cache(&sources(), &space.0.join("missing"), &space.0).is_err());
        fs::remove_file(&parent).unwrap();
        private_directory(&parent, uid).unwrap();
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(cache(&sources(), &space.0.join("missing"), &space.0).is_err());
        assert!(owned(&victim, uid.wrapping_add(1)).is_err());
        assert!(fs::read_dir(&victim).unwrap().next().is_none());
    }

    #[test]
    fn marker_and_symlink_cannot_substitute_sources_or_redirect_writes() {
        let space = sandbox();
        let source = space.0.join("missing");
        let library = cache(&sources(), &source, &space.0).unwrap();
        let directory = library.ancestors().nth(3).unwrap();
        let victim = space.0.join("victim.py");
        fs::write(&victim, b"unchanged\n").unwrap();
        fs::remove_file(library.join("pkg/__init__.py")).unwrap();
        symlink(&victim, library.join("pkg/__init__.py")).unwrap();
        fs::write(directory.join(".complete"), b"complete\n").unwrap();
        assert!(cache(&sources(), &source, &space.0).is_err());
        assert!(new_file(&library.join("pkg/__init__.py")).is_err());
        assert_eq!(fs::read(&victim).unwrap(), b"unchanged\n");
    }

    #[test]
    fn cache_directory_symlinks_and_writable_ancestors_are_rejected() {
        let space = sandbox();
        let source = space.0.join("missing");
        let library = cache(&sources(), &source, &space.0).unwrap();
        let directory = library.ancestors().nth(3).unwrap();
        let moved = directory.with_extension("moved");
        fs::rename(directory, &moved).unwrap();
        symlink(&moved, directory).unwrap();
        assert!(cache(&sources(), &source, &space.0).is_err());
        let unsafe_parent = space.0.join("unsafe");
        fs::create_dir(&unsafe_parent).unwrap();
        fs::set_permissions(&unsafe_parent, fs::Permissions::from_mode(0o777)).unwrap();
        assert!(cache(&sources(), &source, &unsafe_parent).is_err());
    }

    #[test]
    fn resources_are_copied_without_cache_symlinks() {
        let space = sandbox();
        let project = space.0.join("installation");
        let original = project.join("langs/lib_python/modules");
        fs::create_dir_all(original.join("pkg")).unwrap();
        fs::write(original.join("pkg/__init__.py"), b"disk source\n").unwrap();
        fs::write(original.join("pkg/data.bin"), [0, 255, 42]).unwrap();
        fs::write(project.join("fixture.bin"), [7, 0, 9]).unwrap();
        fs::set_permissions(project.join("fixture.bin"), fs::Permissions::from_mode(0o750)).unwrap();
        let library = cache(&sources(), &original, &space.0).unwrap();
        assert_eq!(fs::read(library.join("pkg/__init__.py")).unwrap(), b"answer = 42\n");
        assert_eq!(fs::read(library.join("pkg/data.bin")).unwrap(), [0, 255, 42]);
        assert_eq!(fs::read(library.ancestors().nth(3).unwrap().join("fixture.bin")).unwrap(), [7, 0, 9]);
        assert_eq!(fs::metadata(library.ancestors().nth(3).unwrap().join("fixture.bin")).unwrap().mode() & 0o777, 0o750);
        assert_eq!(cache(&sources(), &original, &space.0).unwrap(), library);
    }

    #[test]
    fn later_module_bindings_replace_earlier_registrations() {
        let space = sandbox();
        fs::write(space.0.join("pkg.py"), b"answer = 42\n").unwrap();
        let request = vec![
            ("MODULE".into(), "pkg".into(), "answer = 0\n".into(), false),
            ("MODULE_FILE".into(), "pkg".into(), "pkg.py".into(), false),
            ("MODULE".into(), "pkg".into(), "answer = 42\n".into(), false),
        ];
        assert_eq!(library(&request, &space.0).unwrap(), space.0);
    }

    #[test]
    fn exclusive_staging_and_competing_publication() {
        let space = sandbox();
        let first = Staging(temporary(&space.0).unwrap());
        let second = Staging(temporary(&space.0).unwrap());
        assert_ne!(first.0, second.0);
        let handles: Vec<_> = (0..4).map(|_| {
            let temp = space.0.clone();
            std::thread::spawn(move || cache(&sources(), &temp.join("missing"), &temp).unwrap())
        }).collect();
        let results: Vec<_> = handles.into_iter().map(|handle| handle.join().unwrap()).collect();
        assert!(results.iter().all(|path| path == &results[0]));
        assert!(!fs::read_dir(results[0].ancestors().nth(4).unwrap()).unwrap()
            .any(|entry| entry.unwrap().file_name().to_string_lossy().starts_with(".building-")));
    }
}
