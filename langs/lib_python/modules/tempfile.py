# Runtime tempfile adapter for CPython v3.14.8 Lib/tempfile.py.
# Copyright (c) Python Software Foundation; PSF License in tests/python-3.14.8/LICENSE.
import os
from random import Random as _Random
import shutil as _shutil
import weakref as _weakref

__all__ = ['NamedTemporaryFile', 'TemporaryFile', 'SpooledTemporaryFile',
           'TemporaryDirectory', 'mkstemp', 'mkdtemp', 'mktemp', 'gettempdir',
           'gettempprefix', 'tempdir', 'template']

template = 'tmp'
TMP_MAX = 20
tempdir = None


def gettempprefix():
    return template


def gettempdir():
    # The host names the place in the environment, and the first of the
    # three usual names that is set wins, as CPython's own search does.
    # Nothing is made or written to check it.
    if tempdir is not None:
        return tempdir
    places = os.environ
    for name in ['TMPDIR', 'TEMP', 'TMP']:
        if name in places:
            return places[name]
    return '/tmp'


def mkdtemp(suffix=None, prefix=None, dir=None):
    if suffix is None:
        suffix = ''
    if prefix is None:
        prefix = template
    if dir is None:
        dir = gettempdir()
    made = __make_dir(dir, prefix, suffix)
    if made is False:
        raise FileNotFoundError(2, 'No such file or directory', dir)
    return made


def mkstemp(suffix=None, prefix=None, dir=None, text=False):
    raise 'NotImplementedError: tempfile.mkstemp needs a file to be created and opened, which this runtime does not carry'


def mktemp(suffix='', prefix=template, dir=None):
    global _name_counter
    if dir is None:
        dir = gettempdir()
    import time
    for _ in range(100):
        _name_counter += 1
        seed = int(time.time() * 1000000)
        name = dir + '/' + '%s%06x%04x%s' % (prefix, seed % 16777216, _name_counter % 65536, suffix)
        if not os.path.exists(name):
            return name
    raise FileExistsError(17, 'File exists', name)


def _infer_return_type(*args):
    """Look at the type of all args and divine their implied return type."""
    return_type = None
    for arg in args:
        if arg is None:
            continue

        if isinstance(arg, os.PathLike):
            arg = os.fspath(arg)

        if isinstance(arg, bytes):
            if return_type is str:
                raise TypeError("Can't mix bytes and non-bytes in "
                                "path components.")
            return_type = bytes
        else:
            if return_type is bytes:
                raise TypeError("Can't mix bytes and non-bytes in "
                                "path components.")
            return_type = str
    if return_type is None:
        if tempdir is None or isinstance(tempdir, str):
            return str  # tempfile APIs return a str by default.
        else:
            # we could check for bytes but it'll fail later on anyway
            return bytes
    return return_type


def _sanitize_params(prefix, suffix, dir):
    """Common parameter processing for most APIs in this module."""
    output_type = _infer_return_type(prefix, suffix, dir)
    if suffix is None:
        suffix = output_type()
    if prefix is None:
        if output_type is str:
            prefix = template
        else:
            prefix = os.fsencode(template)
    if dir is None:
        if output_type is str:
            dir = gettempdir()
        else:
            dir = os.fsencode(gettempdir())
    return prefix, suffix, dir, output_type


class _RandomNameSequence:
    """An instance of _RandomNameSequence generates an endless
    sequence of unpredictable strings which can safely be incorporated
    into file names.  Each string is eight characters long.  Multiple
    threads can safely use the same instance at the same time.

    _RandomNameSequence is an iterator."""

    characters = "abcdefghijklmnopqrstuvwxyz0123456789_"

    @property
    def rng(self):
        cur_pid = os.getpid()
        if cur_pid != getattr(self, '_rng_pid', None):
            self._rng = _Random()
            self._rng_pid = cur_pid
        return self._rng

    def __iter__(self):
        return self

    def __next__(self):
        return ''.join(self.rng.choices(self.characters, k=8))

_name_sequence = None


def _get_candidate_names():
    global _name_sequence
    if _name_sequence is None:
        _name_sequence = _RandomNameSequence()
    return _name_sequence


class _TemporaryFileCloser:
    # delete_on_close asks for removal when the file is closed; delete
    # asks for removal when the wrapper is let go of at all, context
    # exit included. They are kept apart, as CPython keeps them.
    def __init__(self, file, name, delete=True, delete_on_close=True):
        self.file = file
        self.name = name
        self.delete = delete
        self.delete_on_close = delete_on_close
        self.cleanup_called = False
        self.close_called = False

    def cleanup(self):
        if not self.cleanup_called:
            self.cleanup_called = True
            try:
                if not self.close_called:
                    self.close_called = True
                    self.file.close()
            finally:
                if self.delete:
                    try:
                        os.unlink(self.name)
                    except FileNotFoundError:
                        pass

    def close(self):
        if not self.close_called:
            self.close_called = True
            try:
                self.file.close()
            finally:
                if self.delete and self.delete_on_close:
                    self.cleanup()

    def __del__(self):
        # Let go of without a close or a context exit: clean up all the
        # same, as CPython's closer does from its own __del__.
        self.cleanup()


class _TemporaryFileWrapper:
    def __init__(self, file, name, delete=True, delete_on_close=True):
        self.file = file
        self.name = name
        self._closer = _TemporaryFileCloser(file, name, delete, delete_on_close)

    def close(self):
        self._closer.close()

    def __getattr__(self, name):
        file = self.__dict__['file']
        value = getattr(file, name)
        if callable(value):
            method = value
            def forwarded(*args, **kwargs):
                return method(*args, **kwargs)
            forwarded._closer = self._closer
            value = forwarded
        if not isinstance(value, int):
            setattr(self, name, value)
        return value

    def __iter__(self):
        for line in self.file:
            yield line

    def __enter__(self):
        self.file.__enter__()
        return self

    def __exit__(self, kind, value, traceback):
        # The context leaving always cleans up, whether the file was
        # already closed or not, as CPython's wrapper guarantees.
        result = self.file.__exit__(kind, value, traceback)
        self._closer.cleanup()
        return result


_name_counter = 0


def NamedTemporaryFile(mode='w+b', buffering=-1, encoding=None, newline=None, suffix=None, prefix=None, dir=None, delete=True, *, errors=None, delete_on_close=True):
    prefix, suffix, dir, output_type = _sanitize_params(prefix, suffix, dir)
    dir = os.fspath(dir)
    name = None

    def opener(path, flags):
        nonlocal name
        exclusive = os.O_RDWR | os.O_CREAT | os.O_EXCL
        exclusive |= getattr(os, 'O_NOFOLLOW', 0)
        directory = os.path.abspath(dir)
        names = _get_candidate_names()
        for _ in range(TMP_MAX):
            middle = next(names)
            if output_type is bytes:
                middle = os.fsencode(middle)
            candidate = os.path.join(directory, prefix + middle + suffix)
            import sys
            sys.audit('tempfile.mkstemp', candidate)
            try:
                descriptor = os.open(candidate, exclusive, 0o600)
                name = candidate
                return descriptor
            except FileExistsError:
                continue
        raise FileExistsError(17, 'No usable temporary file name found')

    try:
        file = open(dir, mode, buffering=buffering, encoding=encoding,
                    errors=errors, newline=newline, opener=opener)
        try:
            raw = getattr(file, 'buffer', file)
            raw = getattr(raw, 'raw', raw)
            raw.name = name
            return _TemporaryFileWrapper(file, name, delete, delete_on_close)
        except BaseException:
            file.close()
            raise
    except BaseException:
        if name is not None:
            os.unlink(name)
        raise


def TemporaryFile(*args, **keywords):
    raise 'NotImplementedError: tempfile.TemporaryFile needs a file to be created and opened, which this runtime does not carry'


def SpooledTemporaryFile(*args, **keywords):
    # CPython's keeps the writing in memory until it grows past a limit
    # and then moves it into a real file. The first half could be an
    # in-memory stream here, but a program that writes past the limit
    # would go on writing into memory and never learn that the move did
    # not happen, so the whole of it refuses.
    raise 'NotImplementedError: tempfile.SpooledTemporaryFile cannot roll over into a file, which this runtime does not carry'


class TemporaryDirectory:
    """Create and return a temporary directory.  This has the same
    behavior as mkdtemp but can be used as a context manager.  For
    example:

        with TemporaryDirectory() as tmpdir:
            ...

    Upon exiting the context, the directory and everything contained
    in it are removed (unless delete=False is passed).

    Optional Arguments:
        suffix - A str suffix for the directory name.  (see mkdtemp)
        prefix - A str prefix for the directory name.  (see mkdtemp)
        dir - A directory to create this temp dir in.  (see mkdtemp)
        ignore_cleanup_errors - False; ignore exceptions during cleanup?
        delete - True; whether the directory is automatically deleted.
    """

    def __init__(self, suffix=None, prefix=None, dir=None,
                 ignore_cleanup_errors=False, *, delete=True):
        self.name = mkdtemp(suffix, prefix, dir)
        self._ignore_cleanup_errors = ignore_cleanup_errors
        self._delete = delete
        self._finalizer = _weakref.finalize(self, self._remove, self.name,
                                            ignore_errors=ignore_cleanup_errors,
                                            delete=delete)

    @classmethod
    def _remove(cls, name, ignore_errors=False, delete=True):
        if delete:
            _shutil.rmtree(name, ignore_errors=ignore_errors)

    def __repr__(self):
        return '<{} {!r}>'.format(self.__class__.__name__, self.name)

    def __enter__(self):
        return self.name

    def __exit__(self, exc, value, tb):
        if self._delete:
            self.cleanup()

    def cleanup(self):
        if self._finalizer.detach() or os.path.exists(self.name):
            self._remove(self.name, ignore_errors=self._ignore_cleanup_errors)
