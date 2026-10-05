# Where a program would put a file it means to throw away.
import os
import shutil as _shutil
import weakref as _weakref

__all__ = ['NamedTemporaryFile', 'TemporaryFile', 'SpooledTemporaryFile',
           'TemporaryDirectory', 'mkstemp', 'mkdtemp', 'mktemp', 'gettempdir',
           'gettempprefix', 'tempdir', 'template']

template = 'tmp'
tempdir = None
_count = 0


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
    raise 'NotImplementedError: tempfile.mktemp would name a file this runtime cannot then create'


def _forget(path):
    try:
        os.remove(path)
    except OSError:
        pass


def _named_path(suffix, prefix, dir):
    # The host's process number and a count kept here together name a
    # path, and one already standing is passed over, so no call in this
    # run is handed a name another call has used.
    if suffix is None:
        suffix = ''
    if prefix is None:
        prefix = template
    if dir is None:
        dir = gettempdir()
    global _count
    while True:
        _count += 1
        candidate = os.path.join(dir, '%s%d_%d%s' % (prefix, os.getpid(), _count, suffix))
        if not os.path.exists(candidate):
            return candidate


class _NamedTemporaryFile:
    def __init__(self, handle, name, delete, delete_on_close):
        self._handle = handle
        self.name = name
        self.delete = delete
        self.delete_on_close = delete_on_close

    def __getattr__(self, word):
        return getattr(self._handle, word)

    def close(self):
        if self._handle.closed:
            return
        self._handle.close()
        if self.delete and self.delete_on_close:
            _forget(self.name)

    def __enter__(self):
        return self

    def __exit__(self, kind, value, traceback):
        self.close()
        if self.delete:
            _forget(self.name)
        return False


def NamedTemporaryFile(mode='w+b', buffering=-1, encoding=None, newline=None,
                       suffix=None, prefix=None, dir=None, delete=True, *,
                       errors=None, delete_on_close=True):
    path = _named_path(suffix, prefix, dir)
    made = __file_create(path)
    if made is not None:
        _forget(path)
        raise OSError('cannot create a temporary file at ' + path)
    handle = open(path, mode)
    return _NamedTemporaryFile(handle, path, delete, delete_on_close)


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
