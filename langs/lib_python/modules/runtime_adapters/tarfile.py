# Re-parsing an unchanged archive is interpreter-bound, and callers reopen
# the same bytes for every case. Reuse members when the file is unchanged.
_tar_load = TarFile._load
_tar_members = {}
_tar_loading = set()
_TAR_CACHE_LIMIT = 8 * 1024 * 1024
_TAR_FIELDS = (
    "name", "mode", "uid", "gid", "size", "mtime", "chksum", "type",
    "linkname", "uname", "gname", "devmajor", "devminor", "offset",
    "offset_data", "sparse", "pax_headers",
)


def _tar_snap(member):
    saved = {}
    for field in _TAR_FIELDS:
        value = getattr(member, field)
        if field == "pax_headers" and isinstance(value, dict):
            value = dict(value)
        elif field == "sparse" and isinstance(value, list):
            value = list(value)
        saved[field] = value
    return saved


def _tar_clone(saved, owner):
    member = TarInfo()
    for field, value in saved.items():
        setattr(member, field, value)
    headers = saved.get("pax_headers")
    if isinstance(headers, dict):
        member.pax_headers = dict(headers)
    sparse = saved.get("sparse")
    if isinstance(sparse, list):
        member.sparse = list(sparse)
    return member


def _tar_digest(path):
    with bltn_open(path, "rb", buffering=0) as handle:
        data = handle.readall()
    try:
        import zlib
        return (len(data), zlib.crc32(data), zlib.adler32(data))
    except Exception:
        import hashlib
        return hashlib.sha256(data).digest()


def _tar_meta(self):
    if self.mode != "r" or self.stream or self._extfileobj or not self.name:
        return None
    if self.tarinfo is not TarInfo:
        return None
    try:
        info = os.stat(self.name)
    except OSError:
        return None
    if info.st_size > _TAR_CACHE_LIMIT:
        return None
    start = self.members[0].offset if self.members else self.offset
    return (
        self.name, info.st_dev, info.st_ino, info.st_size, info.st_mtime,
        self.encoding, self.errors, self.ignore_zeros, start,
    )


def _tar_prefix(self, snaps):
    if len(self.members) > len(snaps):
        return False
    for index, member in enumerate(self.members):
        saved = snaps[index]
        if (
            member.name != saved["name"]
            or member.offset != saved["offset"]
            or member.size != saved["size"]
            or member.type != saved["type"]
        ):
            return False
    return True


def _tar_load_members(self):
    if id(self) in _tar_loading:
        return _tar_load(self)
    _tar_loading.add(id(self))
    try:
        key = _tar_meta(self)
        found = _tar_members.get(key) if key is not None else None
        if found is not None:
            file_digest, snaps, end_off, end_tell, headers = found
            if _tar_digest(self.name) == file_digest and _tar_prefix(self, snaps):
                try:
                    self.fileobj.seek(end_tell)
                except Exception:
                    pass
                else:
                    base = len(self.members)
                    self.members.extend([
                        _tar_clone(snaps[index], self)
                        for index in range(base, len(snaps))
                    ])
                    self.pax_headers = dict(headers)
                    self.offset = end_off
                    self.firstmember = None
                    self._loaded = True
                    return
        _tar_load(self)
        if key is not None and self._loaded:
            try:
                tell = self.fileobj.tell()
            except Exception:
                return
            _tar_members[key] = (
                _tar_digest(self.name),
                [_tar_snap(member) for member in self.members],
                self.offset,
                tell,
                dict(self.pax_headers),
            )
    finally:
        _tar_loading.discard(id(self))


TarFile._load = _tar_load_members


try:
    import posix as _tar_posix
except ImportError:
    _tar_posix = None

if _tar_posix is not None and hasattr(_tar_posix, "_realpath") and hasattr(_tar_posix, "_makedirs"):
    _tar_orig_get_filtered_attrs = _get_filtered_attrs
    _tar_orig_extract_member = TarFile._extract_member
    _TAR_ORIG_REALPATH = os.path.realpath
    _TAR_ORIG_ISABS = os.path.isabs
    _TAR_ORIG_NORMPATH = os.path.normpath
    _TAR_ORIG_JOIN = os.path.join
    _TAR_ORIG_COMMONPATH = os.path.commonpath
    _TAR_ORIG_DIRNAME = os.path.dirname
    _TAR_ORIG_LSTAT = os.lstat
    _TAR_ORIG_READLINK = os.readlink
    _TAR_ORIG_MAKEDIRS = os.makedirs
    _TAR_ORIG_MKDIR = os.mkdir
    _TAR_ORIG_EXISTS = os.path.exists
    _TAR_ORIG_ISDIR = os.path.isdir
    _TAR_ORIG_FSPATH = os.fspath
    import builtins as _tar_builtins
    _TAR_ORIG_OPEN = bltn_open
    _TAR_ORIG_BLTN_OPEN = _tar_builtins.open
    _TAR_ORIG_COPYFILEOBJ = copyfileobj
    _TAR_ORIG_OS_OPEN = os.open
    _TAR_ORIG_OS_WRITE = os.write
    _TAR_ORIG_OS_CLOSE = os.close
    _TAR_ORIG_MAKEFILE = TarFile.makefile
    _TAR_WB_FLAGS = os.O_WRONLY | os.O_CREAT | os.O_TRUNC | getattr(os, "O_BINARY", 0)

    def _tar_outside(target_path, dest_path):
        return (target_path != dest_path
                and not (dest_path == '/' and target_path.startswith('/'))
                and not target_path.startswith(dest_path + '/'))

    def _tar_get_filtered_attrs(member, dest_path, for_data=True):
        if (type(member.name) is not str
                or os.path.realpath is not _TAR_ORIG_REALPATH
                or os.lstat is not _TAR_ORIG_LSTAT
                or os.readlink is not _TAR_ORIG_READLINK
                or os.path.isabs is not _TAR_ORIG_ISABS
                or os.path.normpath is not _TAR_ORIG_NORMPATH
                or os.path.join is not _TAR_ORIG_JOIN
                or os.path.commonpath is not _TAR_ORIG_COMMONPATH
                or os.path.dirname is not _TAR_ORIG_DIRNAME
                or os.fspath is not _TAR_ORIG_FSPATH):
            return _tar_orig_get_filtered_attrs(member, dest_path, for_data)
        if type(dest_path) is not str:
            if isinstance(dest_path, os.PathLike):
                dest_path_str = _TAR_ORIG_FSPATH(dest_path)
                if type(dest_path_str) is not str:
                    return _tar_orig_get_filtered_attrs(member, dest_path, for_data)
                dest_path = dest_path_str
            else:
                return _tar_orig_get_filtered_attrs(member, dest_path, for_data)
        new_attrs = {}
        name = member.name
        dest_path = _tar_posix._realpath(dest_path, 2)
        if name.startswith(('/', os.sep)):
            name = new_attrs['name'] = member.path.lstrip('/' + os.sep)
        if _TAR_ORIG_ISABS(name):
            raise AbsolutePathError(member)
        if '..' in name and '..' in name.replace(os.sep, '/').split('/'):
            normalized = _TAR_ORIG_NORMPATH(name)
            if normalized != name:
                name = new_attrs['name'] = normalized
        target_path = _tar_posix._realpath(_TAR_ORIG_JOIN(dest_path, name), 2)
        if _tar_outside(target_path, dest_path):
            raise OutsideDestinationError(member, target_path)
        mode = member.mode
        if mode is not None:
            mode = mode & 0o755
            if for_data:
                if member.isreg() or member.islnk():
                    if not mode & 0o100:
                        mode &= ~0o111
                    mode |= 0o600
                elif member.isdir() or member.issym():
                    mode = None
                else:
                    raise SpecialFileError(member)
            if mode != member.mode:
                new_attrs['mode'] = mode
        if for_data:
            if member.uid is not None:
                new_attrs['uid'] = None
            if member.gid is not None:
                new_attrs['gid'] = None
            if member.uname is not None:
                new_attrs['uname'] = None
            if member.gname is not None:
                new_attrs['gname'] = None
            if member.islnk() or member.issym():
                if _TAR_ORIG_ISABS(member.linkname):
                    raise AbsoluteLinkError(member)
                if target_path == dest_path:
                    raise OutsideDestinationError(member, target_path)
                normalized = _TAR_ORIG_NORMPATH(member.linkname)
                if normalized != member.linkname:
                    new_attrs['linkname'] = normalized
                if member.issym():
                    link_dir = _TAR_ORIG_DIRNAME(name.rstrip('/' + os.sep))
                    target_path = _TAR_ORIG_JOIN(dest_path, link_dir, normalized)
                else:
                    target_path = _TAR_ORIG_JOIN(dest_path, normalized)
                target_path = _tar_posix._realpath(target_path, 2)
                if _tar_outside(target_path, dest_path):
                    raise LinkOutsideDestinationError(member, target_path)
        return new_attrs

    def _tar_extract_member(self, tarinfo, targetpath, set_attrs=True,
                            numeric_owner=False, *, filter_function=None,
                            extraction_root=None):
        if (type(targetpath) is not str
                or os.makedirs is not _TAR_ORIG_MAKEDIRS
                or os.mkdir is not _TAR_ORIG_MKDIR
                or os.path.exists is not _TAR_ORIG_EXISTS
                or os.path.isdir is not _TAR_ORIG_ISDIR
                or os.path.dirname is not _TAR_ORIG_DIRNAME):
            return _tar_orig_extract_member(
                self, tarinfo, targetpath, set_attrs=set_attrs,
                numeric_owner=numeric_owner, filter_function=filter_function,
                extraction_root=extraction_root)
        targetpath = targetpath.rstrip("/")
        targetpath = targetpath.replace("/", os.sep)
        upperdirs = _TAR_ORIG_DIRNAME(targetpath)
        if upperdirs and not _TAR_ORIG_EXISTS(upperdirs):
            _tar_posix._makedirs(upperdirs)
        if tarinfo.islnk() or tarinfo.issym():
            self._dbg(1, "%s -> %s" % (tarinfo.name, tarinfo.linkname))
        else:
            self._dbg(1, tarinfo.name)
        if tarinfo.isreg():
            self.makefile(tarinfo, targetpath)
        elif tarinfo.isdir():
            self.makedir(tarinfo, targetpath)
        elif tarinfo.isfifo():
            self.makefifo(tarinfo, targetpath)
        elif tarinfo.ischr() or tarinfo.isblk():
            self.makedev(tarinfo, targetpath)
        elif tarinfo.islnk() or tarinfo.issym():
            self.makelink_with_filter(
                tarinfo, targetpath,
                filter_function=filter_function,
                extraction_root=extraction_root)
        elif tarinfo.type not in SUPPORTED_TYPES:
            self.makeunknown(tarinfo, targetpath)
        else:
            self.makefile(tarinfo, targetpath)
        if set_attrs:
            self.chown(tarinfo, targetpath, numeric_owner)
            if not tarinfo.issym():
                self.chmod(tarinfo, targetpath)
                self.utime(tarinfo, targetpath)

    def _tar_write_all(fd, buf):
        written = _TAR_ORIG_OS_WRITE(fd, buf)
        if written == len(buf):
            return
        mv = memoryview(buf)
        offset = written
        total = len(buf)
        while offset < total:
            step = _TAR_ORIG_OS_WRITE(fd, mv[offset:])
            if step <= 0:
                raise OSError("short write")
            offset += step

    def _tar_makefile(self, tarinfo, targetpath):
        length = tarinfo.size
        if (tarinfo.sparse is not None
                or type(targetpath) is not str
                or type(length) is not int
                or length < 0
                or bltn_open is not _TAR_ORIG_OPEN
                or _tar_builtins.open is not _TAR_ORIG_BLTN_OPEN
                or copyfileobj is not _TAR_ORIG_COPYFILEOBJ
                or os.open is not _TAR_ORIG_OS_OPEN
                or os.write is not _TAR_ORIG_OS_WRITE
                or os.close is not _TAR_ORIG_OS_CLOSE):
            return _TAR_ORIG_MAKEFILE(self, tarinfo, targetpath)
        source = self.fileobj
        source.seek(tarinfo.offset_data)
        fd = _TAR_ORIG_OS_OPEN(targetpath, _TAR_WB_FLAGS, 0o666)
        try:
            if length > 0:
                bufsize = self.copybufsize or 16384
                if length <= bufsize:
                    buf = source.read(length)
                    if len(buf) < length:
                        raise ReadError("unexpected end of data")
                    _tar_write_all(fd, buf)
                else:
                    blocks, remainder = divmod(length, bufsize)
                    for _ in range(blocks):
                        buf = source.read(bufsize)
                        if len(buf) < bufsize:
                            raise ReadError("unexpected end of data")
                        _tar_write_all(fd, buf)
                    if remainder:
                        buf = source.read(remainder)
                        if len(buf) < remainder:
                            raise ReadError("unexpected end of data")
                        _tar_write_all(fd, buf)
        finally:
            _TAR_ORIG_OS_CLOSE(fd)

    _get_filtered_attrs = _tar_get_filtered_attrs
    TarFile._extract_member = _tar_extract_member
    TarFile.makefile = _tar_makefile
