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
