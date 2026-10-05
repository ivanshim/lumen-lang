# Native stat helpers following CPython v3.14.8 Modules/_stat.c.
# Copyright (c) Python Software Foundation; PSF License in tests/python/LICENSE.
import operator as _operator
_host = __posix

ST_MODE = 0
ST_INO = 1
ST_DEV = 2
ST_NLINK = 3
ST_UID = 4
ST_GID = 5
ST_SIZE = 6
ST_ATIME = 7
ST_MTIME = 8
ST_CTIME = 9
S_IFDIR = 16384
S_IFCHR = 8192
S_IFBLK = 24576
S_IFREG = 32768
S_IFIFO = 4096
S_IFLNK = 40960
S_IFSOCK = 49152
S_IFDOOR = 0
S_IFPORT = 0
S_IFWHT = 0
S_ISUID = 2048
S_ISGID = 1024
S_ISVTX = 512
S_IREAD = 256
S_IWRITE = 128
S_IEXEC = 64
S_IRWXU = 448
S_IRUSR = 256
S_IWUSR = 128
S_IXUSR = 64
S_IRWXG = 56
S_IRGRP = 32
S_IWGRP = 16
S_IXGRP = 8
S_IRWXO = 7
S_IROTH = 4
S_IWOTH = 2
S_IXOTH = 1
UF_SETTABLE = 65535
UF_NODUMP = 1
UF_IMMUTABLE = 2
UF_APPEND = 4
UF_OPAQUE = 8
UF_NOUNLINK = 16
UF_COMPRESSED = 32
UF_TRACKED = 64
UF_DATAVAULT = 128
UF_HIDDEN = 32768
SF_SETTABLE = 4294901760
SF_ARCHIVED = 65536
SF_IMMUTABLE = 131072
SF_APPEND = 262144
SF_RESTRICTED = 524288
SF_NOUNLINK = 1048576
SF_SNAPSHOT = 2097152
SF_FIRMLINK = 8388608
SF_DATALESS = 1073741824

S_ENFMT = 1024

def S_IMODE(mode, /):
    return _host('stat_mode', _operator.index(mode), 0)[1]

def S_IFMT(mode, /):
    return _host('stat_mode', _operator.index(mode), 1)[1]

def S_ISDIR(mode, /):
    return _host('stat_mode', _operator.index(mode), 2)[1]

def S_ISCHR(mode, /):
    return _host('stat_mode', _operator.index(mode), 3)[1]

def S_ISBLK(mode, /):
    return _host('stat_mode', _operator.index(mode), 4)[1]

def S_ISREG(mode, /):
    return _host('stat_mode', _operator.index(mode), 5)[1]

def S_ISFIFO(mode, /):
    return _host('stat_mode', _operator.index(mode), 6)[1]

def S_ISLNK(mode, /):
    return _host('stat_mode', _operator.index(mode), 7)[1]

def S_ISSOCK(mode, /):
    return _host('stat_mode', _operator.index(mode), 8)[1]

def S_ISDOOR(mode, /):
    return _host('stat_mode', _operator.index(mode), 9)[1]

def S_ISPORT(mode, /):
    return _host('stat_mode', _operator.index(mode), 10)[1]

def S_ISWHT(mode, /):
    return _host('stat_mode', _operator.index(mode), 11)[1]

def filemode(mode, /):
    return _host('stat_mode', _operator.index(mode), 12)[1]
