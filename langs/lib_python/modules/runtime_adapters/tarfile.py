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
