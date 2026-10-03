import os
import warnings
from contextlib import contextmanager
from shutil import rmtree as _rmtree

# The name CPython's tests give the one scratch file they write.
TESTFN = '@test'

def rmtree(path):
    try:
        _rmtree(path)
    except FileNotFoundError:
        pass

@contextmanager
def temp_dir(path=None, quiet=False):
    """Return a context manager that creates a temporary directory.

    Arguments:

      path: the directory to create temporarily.  If omitted or None,
        defaults to creating a temporary directory using tempfile.mkdtemp.

      quiet: if False (the default), the context manager raises an exception
        on error.  Otherwise, if the path is specified and cannot be
        created, only a warning is issued.

    """
    import tempfile
    dir_created = False
    if path is None:
        path = tempfile.mkdtemp()
        dir_created = True
        path = os.path.realpath(path)
    else:
        try:
            os.mkdir(path)
            dir_created = True
        except OSError as exc:
            if not quiet:
                raise
            warnings.warn('tests may fail, unable to create '
                          'temporary directory %r: %s' % (path, exc),
                          RuntimeWarning, stacklevel=3)
    if dir_created:
        pid = os.getpid()
    try:
        yield path
    finally:
        # In case the process forks, let only the parent remove the
        # directory. The child has a different process id. (bpo-30028)
        if dir_created and pid == os.getpid():
            rmtree(path)

@contextmanager
def change_cwd(path, quiet=False):
    """Return a context manager that changes the current working directory.

    Arguments:

      path: the directory to use as the temporary current working directory.

      quiet: if False (the default), the context manager raises an exception
        on error.  Otherwise, it issues only a warning and keeps the current
        working directory the same.

    """
    saved_dir = os.getcwd()
    try:
        os.chdir(os.path.realpath(path))
    except OSError as exc:
        if not quiet:
            raise
        warnings.warn('tests may fail, unable to change the current working '
                      'directory to %r: %s' % (path, exc),
                      RuntimeWarning, stacklevel=3)
    try:
        yield os.getcwd()
    finally:
        os.chdir(saved_dir)

@contextmanager
def temp_cwd(name='tempcwd', quiet=False):
    """
    Context manager that temporarily creates and changes the CWD.

    The function temporarily changes the current working directory
    after creating a temporary directory in the current directory with
    name *name*.  If *name* is None, the temporary directory is
    created using tempfile.mkdtemp.

    If *quiet* is False (default) and it is not possible to
    create or change the CWD, an error is raised.  If *quiet* is True,
    only a warning is raised and the original CWD is used.

    """
    with temp_dir(path=name, quiet=quiet) as temp_path:
        with change_cwd(temp_path, quiet=quiet) as cwd_dir:
            yield cwd_dir

def unlink(filename):
    try:
        os.unlink(filename)
    except FileNotFoundError:
        pass

# A guard over os.environ: it remembers each name's value the first
# time a test changes or removes it, and puts every one of them back
# when the guard closes. POSIX environment changes belong to this process.
class EnvironmentVarGuard:
    def __init__(self):
        self._environ = os.environ
        self._changed = {}

    def __getitem__(self, envvar):
        return self._environ[envvar]

    def __setitem__(self, envvar, value):
        if envvar not in self._changed:
            self._changed[envvar] = self._environ.get(envvar)
        self._environ[envvar] = value

    def __delitem__(self, envvar):
        if envvar not in self._changed:
            self._changed[envvar] = self._environ.get(envvar)
        if envvar in self._environ:
            del self._environ[envvar]

    def keys(self):
        return self._environ.keys()

    def __contains__(self, envvar):
        return envvar in self._environ

    def __iter__(self):
        return iter(self._environ)

    def __len__(self):
        return len(self._environ)

    def set(self, envvar, value):
        self[envvar] = value

    def unset(self, envvar):
        del self[envvar]

    def clear(self):
        for key in list(self._environ):
            self.unset(key)

    def copy(self):
        return dict(self._environ)

    def __enter__(self):
        return self

    def __exit__(self, *ignore_exc):
        for key, value in self._changed.items():
            if value is None:
                if key in self._environ:
                    del self._environ[key]
            else:
                self._environ[key] = value
        return False

import unittest
import stat

def rmdir(dirname):
    try:
        os.rmdir(dirname)
    except FileNotFoundError:
        pass

class FakePath:
    """Simple implementation of the path protocol.
    """
    def __init__(self, path):
        self.path = path

    def __repr__(self):
        return f'<FakePath {self.path!r}>'

    def __fspath__(self):
        if (isinstance(self.path, BaseException) or
            isinstance(self.path, type) and
                issubclass(self.path, BaseException)):
            raise self.path
        else:
            return self.path

_can_symlink = None


def can_symlink():
    global _can_symlink
    if _can_symlink is not None:
        return _can_symlink
    # WASI / wasmtime prevents symlinks with absolute paths, see man
    # openat2(2) RESOLVE_BENEATH. Almost all symlink tests use absolute
    # paths. Skip symlink tests on WASI for now.
    src = os.path.abspath(TESTFN)
    symlink_path = src + "can_symlink"
    try:
        os.symlink(src, symlink_path)
        can = True
    except (OSError, NotImplementedError, AttributeError):
        can = False
    else:
        os.remove(symlink_path)
    _can_symlink = can
    return can


def skip_unless_symlink(test):
    """Skip decorator for tests that require functional symlink"""
    ok = can_symlink()
    msg = "Requires functional symlink implementation"
    return test if ok else unittest.skip(msg)(test)


def can_chmod():
    global _can_chmod
    if _can_chmod is not None:
        return _can_chmod
    if not hasattr(os, "chmod"):
        _can_chmod = False
        return _can_chmod
    try:
        with open(TESTFN, "wb") as f:
            try:
                os.chmod(TESTFN, 0o555)
                mode1 = os.stat(TESTFN).st_mode
                os.chmod(TESTFN, 0o777)
                mode2 = os.stat(TESTFN).st_mode
            except OSError as e:
                can = False
            else:
                can = stat.S_IMODE(mode1) != stat.S_IMODE(mode2)
    finally:
        unlink(TESTFN)
    _can_chmod = can
    return can



def skip_unless_working_chmod(test):
    """Skip tests that require working os.chmod()

    WASI SDK 15.0 cannot change file mode bits.
    """
    ok = can_chmod()
    msg = "requires working os.chmod()"
    return test if ok else unittest.skip(msg)(test)


import sys
TESTFN_ASCII = TESTFN
# FS_NONASCII: non-ASCII character encodable by os.fsencode(),
# or an empty string if there is no such character.
FS_NONASCII = ''
for character in (
    # First try printable and common characters to have a readable filename.
    # For each character, the encoding list are just example of encodings able
    # to encode the character (the list is not exhaustive).

    # U+00E6 (Latin Small Letter Ae): cp1252, iso-8859-1
    '\u00E6',
    # U+0130 (Latin Capital Letter I With Dot Above): cp1254, iso8859_3
    '\u0130',
    # U+0141 (Latin Capital Letter L With Stroke): cp1250, cp1257
    '\u0141',
    # U+03C6 (Greek Small Letter Phi): cp1253
    '\u03C6',
    # U+041A (Cyrillic Capital Letter Ka): cp1251
    '\u041A',
    # U+05D0 (Hebrew Letter Alef): Encodable to cp424
    '\u05D0',
    # U+060C (Arabic Comma): cp864, cp1006, iso8859_6, mac_arabic
    '\u060C',
    # U+062A (Arabic Letter Teh): cp720
    '\u062A',
    # U+0E01 (Thai Character Ko Kai): cp874
    '\u0E01',

    # Then try more "special" characters. "special" because they may be
    # interpreted or displayed differently depending on the exact locale
    # encoding and the font.

    # U+00A0 (No-Break Space)
    '\u00A0',
    # U+20AC (Euro Sign)
    '\u20AC',
):
    try:
        # If Python is set up to use the legacy 'mbcs' in Windows,
        # 'replace' error mode is used, and encode() returns b'?'
        # for characters missing in the ANSI codepage
        if os.fsdecode(os.fsencode(character)) != character:
            raise UnicodeError
    except UnicodeError:
        pass
    else:
        FS_NONASCII = character
        break

# Save the initial cwd
SAVEDCWD = os.getcwd()

# TESTFN_UNDECODABLE is a filename (bytes type) that should *not* be able to be
# decoded from the filesystem encoding (in strict mode). It can be None if we
# cannot generate such filename (ex: the latin1 encoding can decode any byte
# sequence). On UNIX, TESTFN_UNDECODABLE can be decoded by os.fsdecode() thanks
# to the surrogateescape error handler (PEP 383), but not from the filesystem
# encoding in strict mode.
TESTFN_UNDECODABLE = None
for name in (
    # b'\xff' is not decodable by os.fsdecode() with code page 932. Windows
    # accepts it to create a file or a directory, or don't accept to enter to
    # such directory (when the bytes name is used). So test b'\xe7' first:
    # it is not decodable from cp932.
    b'\xe7w\xf0',
    # undecodable from ASCII, UTF-8
    b'\xff',
    # undecodable from iso8859-3, iso8859-6, iso8859-7, cp424, iso8859-8, cp856
    # and cp857
    b'\xae\xd5'
    # undecodable from UTF-8 (UNIX and Mac OS X)
    b'\xed\xb2\x80', b'\xed\xb4\x80',
    # undecodable from shift_jis, cp869, cp874, cp932, cp1250, cp1251, cp1252,
    # cp1253, cp1254, cp1255, cp1257, cp1258
    b'\x81\x98',
):
    try:
        name.decode(sys.getfilesystemencoding())
    except UnicodeDecodeError:
        try:
            name.decode(sys.getfilesystemencoding(),
                        sys.getfilesystemencodeerrors())
        except UnicodeDecodeError:
            continue
        TESTFN_UNDECODABLE = os.fsencode(TESTFN_ASCII) + name
        break

if FS_NONASCII:
    TESTFN_NONASCII = TESTFN_ASCII + FS_NONASCII
else:
    TESTFN_NONASCII = None
_can_chmod = None
