import os
import warnings
from collections.abc import MutableMapping
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

def create_empty_file(filename):
    # CPython v3.14.8 / 8e6e75d9102e, Lib/test/support/os_helper.py's
    # create_empty_file; PSF License. The file is made empty whether or
    # not one stood there before.
    made = open(filename, 'w')
    made.close()

def unlink(filename):
    try:
        os.unlink(filename)
    except FileNotFoundError:
        pass

# A guard over os.environ: it remembers each name's value the first
# time a test changes or removes it, and puts every one of them back
# when the guard closes. The mapping it edits lives inside this run
# alone; nothing written here reaches the host process. It is a
# MutableMapping, so the whole mapping operations CPython's own
# guard inherits (clear, update, pop and the rest) work through the
# methods below without a second implementation.
class EnvironmentVarGuard(MutableMapping):
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

    def copy(self):
        return dict(self._environ)

    def clear(self):
        # MutableMapping.clear's own walk, kept here because this
        # adapter's guard stands on its own: drop every name in turn,
        # so the first sight of each one is remembered and put back
        # when the guard closes.
        for envvar in list(self._environ):
            del self[envvar]

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

# Derived from CPython v3.14.8 Lib/test/support; PSF License.
import contextlib
import re
import sys
import unittest
from test import support

TESTFN_ASCII = '@test_{}_tmp'.format(os.getpid())
TESTFN_UNICODE = TESTFN_ASCII + '-\xe0\xf2\u0258\u0141\u011f'
# The runtime supports UTF-8 text paths, but not surrogateescape/bytes paths.
TESTFN_UNENCODABLE = None
TESTFN_UNDECODABLE = None
FS_NONASCII = '\u00e6'
TESTFN_NONASCII = TESTFN_ASCII + FS_NONASCII
SAVEDCWD = os.getcwd()
# Retain TESTFN for existing scratch records; other names follow CPython.
_can_symlink = None
_can_hardlink = None
_can_xattr = None
_can_chmod = None
_can_dac_override = None

@contextmanager
def temp_umask(umask):
    if not hasattr(os, 'umask'):
        yield
    else:
        oldmask = os.umask(umask)
        try:
            yield
        finally:
            os.umask(oldmask)

def create_empty_file(filename):
    # File descriptors are not exposed; open has the same truncation behavior.
    with open(filename, 'wb'):
        pass

def rmdir(dirname):
    try:
        os.rmdir(dirname)
    except FileNotFoundError:
        pass

def make_bad_fd():
    """
    Create an invalid file descriptor by opening and closing a file and return
    its fd.
    """
    file = open(TESTFN, "wb")
    try:
        return file.fileno()
    finally:
        file.close()
        unlink(TESTFN)


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


def can_hardlink():
    global _can_hardlink
    if _can_hardlink is None:
        # Android blocks hard links using SELinux
        # (https://stackoverflow.com/q/32365690).
        _can_hardlink = hasattr(os, "link") and not support.is_android
    return _can_hardlink


def skip_unless_hardlink(test):
    ok = can_hardlink()
    msg = "requires hardlink support"
    return test if ok else unittest.skip(msg)(test)


def can_xattr():
    import tempfile
    global _can_xattr
    if _can_xattr is not None:
        return _can_xattr
    if not hasattr(os, "setxattr"):
        can = False
    else:
        import platform
        tmp_dir = tempfile.mkdtemp()
        tmp_fp, tmp_name = tempfile.mkstemp(dir=tmp_dir)
        try:
            with open(TESTFN, "wb") as fp:
                try:
                    # TESTFN & tempfile may use different file systems with
                    # different capabilities
                    os.setxattr(tmp_fp, b"user.test", b"")
                    os.setxattr(tmp_name, b"trusted.foo", b"42")
                    os.setxattr(fp.fileno(), b"user.test", b"")
                    # Kernels < 2.6.39 don't respect setxattr flags.
                    kernel_version = platform.release()
                    m = re.match(r"2.6.(\d{1,2})", kernel_version)
                    can = m is None or int(m.group(1)) >= 39
                except OSError:
                    can = False
        finally:
            unlink(TESTFN)
            unlink(tmp_name)
            rmdir(tmp_dir)
    _can_xattr = can
    return can


def skip_unless_xattr(test):
    """Skip decorator for tests that require functional extended attributes"""
    ok = can_xattr()
    msg = "no non-broken extended attribute support"
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
                import stat
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


@contextlib.contextmanager
def save_mode(path, *, quiet=False):
    """Context manager that restores the mode (permissions) of *path* on exit.

    Arguments:

      path: Path of the file to restore the mode of.

      quiet: if False (the default), the context manager raises an exception
        on error.  Otherwise, it issues only a warning and keeps the current
        working directory the same.

    """
    saved_mode = os.stat(path)
    try:
        yield
    finally:
        try:
            os.chmod(path, saved_mode.st_mode)
        except OSError as exc:
            if not quiet:
                raise
            warnings.warn(f'tests may fail, unable to restore the mode of '
                          f'{path!r} to {saved_mode.st_mode}: {exc}',
                          RuntimeWarning, stacklevel=3)


def can_dac_override():
    global _can_dac_override

    if not can_chmod():
        _can_dac_override = False
    if _can_dac_override is not None:
        return _can_dac_override

    try:
        with open(TESTFN, "wb") as f:
            os.chmod(TESTFN, 0o400)
            try:
                with open(TESTFN, "wb"):
                    pass
            except OSError:
                _can_dac_override = False
            else:
                _can_dac_override = True
    finally:
        try:
            os.chmod(TESTFN, 0o700)
        except OSError:
            pass
        unlink(TESTFN)

    return _can_dac_override


def skip_if_dac_override(test):
    ok = not can_dac_override()
    msg = "incompatible with CAP_DAC_OVERRIDE"
    return test if ok else unittest.skip(msg)(test)


def skip_unless_dac_override(test):
    ok = can_dac_override()
    msg = "requires CAP_DAC_OVERRIDE"
    return test if ok else unittest.skip(msg)(test)


@contextlib.contextmanager
def open_dir_fd(path):
    """Open a file descriptor to a directory."""
    assert os.path.isdir(path)
    flags = os.O_RDONLY
    if hasattr(os, "O_DIRECTORY"):
        flags |= os.O_DIRECTORY
    dir_fd = os.open(path, flags)
    try:
        yield dir_fd
    finally:
        os.close(dir_fd)


def fs_is_case_insensitive(directory):
    """Detects if the file system for the specified directory
    is case-insensitive."""
    import tempfile
    with tempfile.NamedTemporaryFile(dir=directory) as base:
        base_path = base.name
        case_path = base_path.upper()
        if case_path == base_path:
            case_path = base_path.lower()
        try:
            return os.path.samefile(base_path, case_path)
        except FileNotFoundError:
            return False


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
