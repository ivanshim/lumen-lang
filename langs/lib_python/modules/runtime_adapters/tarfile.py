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
    import hashlib
    hasher = hashlib.sha256()
    with bltn_open(path, "rb") as handle:
        hasher.update(handle.read())
    return hasher.digest()


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


_tar_open_func = TarFile.__dict__["open"].__func__
# A transparent read tries every compressor before the plain header.
# Building those decompressors costs more than the archive itself.
_TAR_MAGIC = (
    (b"\x1f\x8b", "gz"),
    (b"BZh", "bz2"),
    (b"\xfd7zXZ\x00", "xz"),
    (b"\x28\xb5\x2f\xfd", "zst"),
)

def _tar_kind(name, fileobj):
    if fileobj is not None:
        try:
            pos = fileobj.tell()
            head = fileobj.read(6)
            fileobj.seek(pos)
        except Exception:
            return None
    else:
        try:
            handle = bltn_open(name, "rb")
        except OSError:
            return None
        try:
            head = handle.read(6)
        finally:
            handle.close()
    for magic, kind in _TAR_MAGIC:
        if head.startswith(magic):
            return kind
    return "tar"

def _tar_open_rest(cls, name, fileobj, failed_kind, failed, **kwargs):
    # The matching opener already failed. A second call would consume a
    # one-shot mock and drop that failure from the ReadError text.
    def not_compressed(comptype):
        return cls.OPEN_METH[comptype] == "taropen"
    error_msgs = []
    for comptype in sorted(cls.OPEN_METH, key=not_compressed):
        if comptype == failed_kind:
            error_msgs.append("- method %s: %r" % (comptype, failed))
            continue
        func = getattr(cls, cls.OPEN_METH[comptype])
        if fileobj is not None:
            saved_pos = fileobj.tell()
        try:
            return func(name, "r", fileobj, **kwargs)
        except (ReadError, CompressionError) as err:
            error_msgs.append("- method %s: %r" % (comptype, err))
            if fileobj is not None:
                fileobj.seek(saved_pos)
            continue
    summary = "\n".join(error_msgs)
    raise ReadError("file could not be opened successfully:\n%s" % summary)


def _tar_open_matched(cls, name=None, mode="r", fileobj=None, bufsize=RECORDSIZE, **kwargs):
    if mode in ("r", "r:*"):
        pos = None
        if fileobj is not None:
            try:
                pos = fileobj.tell()
            except Exception:
                pos = None
        kind = _tar_kind(name, fileobj)
        method = None
        if kind is not None:
            try:
                method = cls.OPEN_METH[kind]
            except KeyError:
                method = None
        if method is not None:
            try:
                return getattr(cls, method)(name, "r", fileobj, **kwargs)
            except (ReadError, CompressionError) as err:
                if fileobj is not None and pos is not None:
                    try:
                        fileobj.seek(pos)
                    except Exception:
                        raise
                return _tar_open_rest(cls, name, fileobj, kind, err, **kwargs)
    return _tar_open_func(cls, name, mode, fileobj, bufsize, **kwargs)

TarFile.open = classmethod(_tar_open_matched)
open = TarFile.open

