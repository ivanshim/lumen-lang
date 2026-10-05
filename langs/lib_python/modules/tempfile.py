# Where a program would put a file it means to throw away.
import os
import shutil as _shutil
import weakref as _weakref

__all__ = ['NamedTemporaryFile', 'TemporaryFile', 'SpooledTemporaryFile',
           'TemporaryDirectory', 'mkstemp', 'mkdtemp', 'mktemp', 'gettempdir',
           'gettempprefix', 'tempdir', 'template']

template = 'tmp'
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
                    except OSError:
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

    def write(self, data):
        return self.file.write(data)

    def close(self):
        self._closer.close()

    def __getattr__(self, name):
        return getattr(self.file, name)

    def __enter__(self):
        return self

    def __exit__(self, kind, value, traceback):
        # The context leaving always cleans up, whether the file was
        # already closed or not, as CPython's wrapper guarantees.
        self._closer.cleanup()


_name_counter = 0


def NamedTemporaryFile(mode='w+b', buffering=-1, encoding=None, newline=None, suffix=None, prefix=None, dir=None, delete=True, delete_on_close=True, *, errors=None):
    global _name_counter
    if dir is None:
        dir = gettempdir()
    if prefix is None:
        prefix = template
    if suffix is None:
        suffix = ''
    import time
    # The file is made exclusively through the host's open, the way
    # CPython's _mkstemp_inner asks for O_CREAT|O_EXCL; if wrapping it
    # then fails, it is removed again rather than left behind.
    flags = os.O_RDWR | os.O_CREAT | os.O_EXCL
    for _ in range(100):
        _name_counter += 1
        seed = int(time.time() * 1000000)
        name = dir + '/' + '%s%06x%04x%s' % (prefix, seed % 16777216, _name_counter % 65536, suffix)
        try:
            fd = os.open(name, flags, 0o600)
        except FileExistsError:
            continue
        except FileNotFoundError:
            raise FileNotFoundError(2, 'No such file or directory', dir)
        break
    else:
        raise FileExistsError(17, 'File exists', name)
    try:
        os.close(fd)
        file = open(name, mode, buffering=buffering, encoding=encoding, errors=errors, newline=newline)
    except BaseException:
        os.unlink(name)
        raise
    if delete:
        return _TemporaryFileWrapper(file, name, delete, delete_on_close)
    return file


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
