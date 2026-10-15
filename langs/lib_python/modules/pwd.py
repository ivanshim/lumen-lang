# Native account lookup adapter for CPython v3.14.8 Modules/pwdmodule.c.
# Copyright (c) Python Software Foundation; PSF License in tests/python-3.14.8/LICENSE.
from posix import _call, _owner_id, _path, _decode, _stat_dict_missing


class struct_passwd(tuple):
    __slots__ = ()
    n_sequence_fields = 7
    n_fields = 7
    n_unnamed_fields = 0

    # Construct the native account record with its fixed sequence size.
    def __new__(cls, sequence, dict=_stat_dict_missing):
        if dict is not _stat_dict_missing and not isinstance(dict, type({})):
            raise TypeError('pwd.struct_passwd() takes a dict as second arg, if any')
        values = tuple(sequence)
        if len(values) != 7:
            raise TypeError('pwd.struct_passwd() takes a 7-sequence (' + str(len(values)) + '-sequence given)')
        return tuple.__new__(cls, values)

    pw_name = property(lambda self: self[0])
    pw_passwd = property(lambda self: self[1])
    pw_uid = property(lambda self: self[2])
    pw_gid = property(lambda self: self[3])
    pw_gecos = property(lambda self: self[4])
    pw_dir = property(lambda self: self[5])
    pw_shell = property(lambda self: self[6])

    # Render the native account fields.
    def __repr__(self):
        names = ('pw_name', 'pw_passwd', 'pw_uid', 'pw_gid', 'pw_gecos', 'pw_dir', 'pw_shell')
        return 'pwd.struct_passwd(' + ', '.join(name + '=' + repr(value) for name, value in zip(names, self)) + ')'

    # Refuse subclassing of the native result type.
    def __init_subclass__(cls, **kwargs):
        raise TypeError("type 'pwd.struct_passwd' is not an acceptable base type")


# Decode the host's account record using filesystem encoding.
def _record(values):
    return struct_passwd(tuple(_decode(value) if isinstance(value, bytes) else value for value in values))


# Resolve an account by its numeric owner id.
def getpwuid(uid, /):
    try:
        uid = _owner_id(uid, 'uid')
    except OverflowError:
        raise KeyError('getpwuid(): uid not found') from None
    values = _call('passwd', 1, uid)
    if values is None:
        raise KeyError('getpwuid(): uid not found: ' + str(uid))
    return _record(values)


# Resolve an account by its name through the host account database.
def getpwnam(name, /):
    if not isinstance(name, str):
        raise TypeError('getpwnam() argument must be str, not ' + type(name).__name__)
    values = _call('passwd', 0, _path(name))
    if values is None:
        raise KeyError('getpwnam(): name not found: ' + repr(name))
    return _record(values)


# Enumerate the host's account database, preserving its native records.
def getpwall():
    return [_record(values) for values in _call('passwd', 2, 0)]
