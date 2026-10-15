# Native group lookup adapter for CPython v3.14.8 Modules/grpmodule.c.
# Copyright (c) Python Software Foundation; PSF License in tests/python-3.14.8/LICENSE.
from posix import _call, _owner_id, _path, _decode, _stat_dict_missing


class struct_group(tuple):
    __slots__ = ()
    n_sequence_fields = 4
    n_fields = 4
    n_unnamed_fields = 0

    # Construct the native group record with its fixed sequence size.
    def __new__(cls, sequence, dict=_stat_dict_missing):
        if dict is not _stat_dict_missing and not isinstance(dict, type({})):
            raise TypeError('grp.struct_group() takes a dict as second arg, if any')
        values = tuple(sequence)
        if len(values) != 4:
            raise TypeError('grp.struct_group() takes a 4-sequence (' + str(len(values)) + '-sequence given)')
        return tuple.__new__(cls, values)

    gr_name = property(lambda self: self[0])
    gr_passwd = property(lambda self: self[1])
    gr_gid = property(lambda self: self[2])
    gr_mem = property(lambda self: self[3])

    # Render the native group fields.
    def __repr__(self):
        names = ('gr_name', 'gr_passwd', 'gr_gid', 'gr_mem')
        return 'grp.struct_group(' + ', '.join(name + '=' + repr(value) for name, value in zip(names, self)) + ')'

    # Refuse subclassing of the native result type.
    def __init_subclass__(cls, **kwargs):
        raise TypeError("type 'grp.struct_group' is not an acceptable base type")


# Decode the host's group record and its member names.
def _record(values):
    return struct_group((None if values[0] is None else _decode(values[0]), None if values[1] is None else _decode(values[1]), values[2], [_decode(name) for name in values[3]]))


# Resolve a group by its numeric group id.
def getgrgid(gid, /):
    gid = _owner_id(gid, 'gid')
    values = _call('group', 1, gid)
    if values is None:
        raise KeyError('getgrgid(): gid not found: ' + str(gid))
    return _record(values)


# Resolve a group by its name through the host group database.
def getgrnam(name, /):
    if not isinstance(name, str):
        raise TypeError("getgrnam() argument 'name' must be str, not " + type(name).__name__)
    values = _call('group', 0, _path(name))
    if values is None:
        raise KeyError('getgrnam(): name not found: ' + repr(name))
    return _record(values)
