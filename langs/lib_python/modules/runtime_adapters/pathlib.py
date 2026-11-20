# Compare logical ancestors without constructing every intermediate path.
_parent_path_types = (PurePath, PurePosixPath, PureWindowsPath, Path, PosixPath, WindowsPath)
_posix_fast_types = (PosixPath, PurePosixPath)
_path_new = object.__new__
_parent_path_eq = PurePath.__eq__
_parent_path_parts = PurePath._from_parsed_parts
_parent_path_segments = PurePath.with_segments
_plain_str = PurePath.__str__
_plain_from_parsed_string = PurePath._from_parsed_string
_parent_membership = Sequence.__contains__
_parent_getitem = _PathParents.__getitem__
_parent_iterator = _PathParents.__iter__
_pathlib_orig_scandir = os.scandir

def _make_posix(kind, path_str):
    path = _path_new(kind)
    if path_str:
        path._raw_paths = [path_str]
        path._str = path_str
    else:
        path._raw_paths = ['']
        path._str = '.'
    return path

# Keep subclass and replaced-method callbacks on the original sequence road.
def _parents_contains(self, item):
    kind = type(self._path)
    other = type(item)
    if (type(self) is not _PathParents or type(self).__getitem__ is not _parent_getitem
            or type(self).__iter__ is not _parent_iterator
            or kind not in _parent_path_types or other not in _parent_path_types
            or kind.__eq__ is not _parent_path_eq or other.__eq__ is not _parent_path_eq
            or kind._from_parsed_parts is not _parent_path_parts
            or kind.with_segments is not _parent_path_segments):
        return _parent_membership(self, item)
    if self._path.parser is not item.parser or len(item._tail) >= len(self._tail):
        return False
    ancestor = self._path._from_parsed_parts(self._drv, self._root, self._tail[:len(item._tail)])
    return ancestor == item

_PathParents.__contains__ = _parents_contains

# A plain POSIX path relative to a plain ancestor is settled from the parsed
# parts alone: the ancestor's tail must open this path's tail under the same
# drive and root. Everything else keeps the original walk, which also raises
# every error in its own words.
_plain_relative_to = PurePath.relative_to
_plain_parents = PurePath.parents

def _relative_to(self, other, *, walk_up=False):
    kind = type(self)
    if (walk_up or kind not in _parent_path_types or kind.__eq__ is not _parent_path_eq
            or kind._from_parsed_parts is not _parent_path_parts
            or kind.with_segments is not _parent_path_segments
            or kind.parents is not _plain_parents or self.parser is not posixpath):
        return _plain_relative_to(self, other, walk_up=walk_up)
    if kind in _posix_fast_types and kind.__str__ is _plain_str:
        other_kind = type(other)
        if (other_kind is kind or other_kind in _posix_fast_types) and other_kind.__eq__ is _parent_path_eq and other_kind.__str__ is _plain_str:
            s = getattr(self, '_str', None)
            if s is not None and not s.startswith('//'):
                b = getattr(other, '_str', None)
                if b is None:
                    b = str(other)
                if not b.startswith('//'):
                    if b == '.':
                        if not s.startswith('/'):
                            return _make_posix(kind, '' if s == '.' else s)
                    elif b == '/':
                        if s == '/':
                            return _make_posix(kind, '')
                        if s.startswith('/'):
                            return _make_posix(kind, s[1:])
                    elif s == b:
                        return _make_posix(kind, '')
                    else:
                        n = len(b)
                        if len(s) > n + 1 and s[n] == '/' and s.startswith(b):
                            rel = s[n + 1:]
                            if rel[0] != '/':
                                return _make_posix(kind, rel)
    if not hasattr(other, 'with_segments'):
        other = self.with_segments(other)
    if type(other) not in _parent_path_types or type(other).__eq__ is not _parent_path_eq or other.parser is not posixpath:
        return _plain_relative_to(self, other)
    tail = self._tail
    head = other._tail
    if self._drv == other._drv and self._root == other._root and tail[:len(head)] == head:
        return self._from_parsed_parts('', '', tail[len(head):])
    return _plain_relative_to(self, other)

_relative_to.__name__ = _plain_relative_to.__name__
_relative_to.__qualname__ = _plain_relative_to.__qualname__
_relative_to.__doc__ = _plain_relative_to.__doc__
PurePath.relative_to = _relative_to

_plain_glob = Path.glob

def _fast_glob_recursive_all(cls, root):
    scandir = _pathlib_orig_scandir
    try:
        with scandir(root) as it:
            entries = list(it)
    except OSError:
        return
    stack = []
    use_name = (root == '.')
    for entry in entries:
        p_str = entry.name if use_name else entry.path
        yield _make_posix(cls, p_str)
        try:
            if entry.is_dir(follow_symlinks=False):
                stack.append(p_str)
        except OSError:
            pass
    while stack:
        cur = stack.pop()
        try:
            with scandir(cur) as it:
                entries = list(it)
        except OSError:
            continue
        for entry in entries:
            p_str = entry.path
            yield _make_posix(cls, p_str)
            try:
                if entry.is_dir(follow_symlinks=False):
                    stack.append(p_str)
            except OSError:
                pass

def _glob(self, pattern, *, case_sensitive=None, recurse_symlinks=False):
    kind = type(self)
    if (pattern == '**/*' and case_sensitive is None and not recurse_symlinks
            and (kind is PosixPath or kind is Path)
            and kind.parser is posixpath
            and kind._from_parsed_string is _plain_from_parsed_string
            and kind.with_segments is _parent_path_segments
            and kind.__str__ is _plain_str
            and os.scandir is _pathlib_orig_scandir):
        root = str(self)
        if not root.startswith('//'):
            sys.audit("pathlib.Path.glob", self, pattern)
            return _fast_glob_recursive_all(kind, root)
    return _plain_glob(self, pattern, case_sensitive=case_sensitive, recurse_symlinks=recurse_symlinks)

_glob.__name__ = _plain_glob.__name__
_glob.__qualname__ = _plain_glob.__qualname__
_glob.__doc__ = _plain_glob.__doc__
Path.glob = _glob
