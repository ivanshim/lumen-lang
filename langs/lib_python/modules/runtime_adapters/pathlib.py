# Compare logical ancestors without constructing every intermediate path.
_parent_path_types = (PurePath, PurePosixPath, PureWindowsPath, Path, PosixPath, WindowsPath)
_parent_path_eq = PurePath.__eq__
_parent_path_parts = PurePath._from_parsed_parts
_parent_path_segments = PurePath.with_segments
_parent_membership = Sequence.__contains__
_parent_getitem = _PathParents.__getitem__
_parent_iterator = _PathParents.__iter__

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
