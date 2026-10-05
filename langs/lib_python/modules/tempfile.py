# Where a program would put a file it means to throw away.
import os
import _pyio
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
    raise 'NotImplementedError: tempfile.mktemp would name a file this runtime cannot then create'


def NamedTemporaryFile(*args, **keywords):
    raise 'NotImplementedError: tempfile.NamedTemporaryFile needs a file to be created and opened, which this runtime does not carry'


# Reuse FileIO's descriptor ownership and mode enforcement. Only its
# reads need a bridge: this runtime has os.read but not os.readinto or
# bytearray.resize, which the unchanged canonical FileIO uses.
class _DescriptorRaw(_pyio.FileIO):
    # Read into a writable contiguous byte view with one descriptor read.
    def readinto(self, buffer):
        self._checkClosed()
        self._checkReadable()
        with memoryview(buffer) as view:
            if view.readonly:
                raise TypeError('readinto() argument must be read-write bytes-like object, not ' + type(buffer).__name__)
            if not view.c_contiguous:
                raise TypeError('readinto() argument must be read-write bytes-like object, not ' + type(buffer).__name__)
            with view.cast('B') as target:
                try:
                    piece = os.read(self._fd, min(len(target), 65536))
                except BlockingIOError:
                    return None
                target[:len(piece)] = piece
                return len(piece)

    # Read through EOF without resizing an exported bytearray.
    def readall(self):
        self._checkClosed()
        self._checkReadable()
        pieces = []
        while True:
            try:
                piece = os.read(self._fd, 65536)
            except BlockingIOError:
                return b''.join(pieces) if pieces else None
            if not piece:
                return b''.join(pieces)
            pieces.append(piece)


# The mode letters a temporary file may be opened with, read the way
# every text and binary stream reads them.
def _reading_mode(mode):
    if not isinstance(mode, str):
        raise TypeError('mode must be str, not ' + type(mode).__name__)
    if any(letter not in 'rwaxbt+' for letter in mode):
        raise ValueError('invalid mode: ' + repr(mode))
    choosing = [letter for letter in mode if letter in 'rwax']
    if len(choosing) != 1 or mode.count('+') > 1 or mode.count('b') > 1 or mode.count('t') > 1 or ('b' in mode and 't' in mode):
        raise ValueError('Must have exactly one of create/read/write/append mode and at most one plus')
    base = choosing[0]
    binary = 'b' in mode
    reading = base == 'r' or '+' in mode
    writing = base in 'wxa' or '+' in mode
    appending = base == 'a'
    return binary, reading, writing, appending


_sequence = 0


def TemporaryFile(mode='w+b', buffering=-1, encoding=None, newline=None,
                  suffix=None, prefix=None, dir=None, *, errors=None):
    binary, reading, writing, _appending = _reading_mode(mode)
    import operator
    buffering = operator.index(buffering)
    if buffering > 2147483647 or buffering < -2147483648:
        raise OverflowError('Python int too large to convert to C int')
    if encoding is not None and not isinstance(encoding, str):
        raise TypeError("open() argument 'encoding' must be str or None, not " + type(encoding).__name__)
    if errors is not None and not isinstance(errors, str):
        raise TypeError("open() argument 'errors' must be str or None, not " + type(errors).__name__)
    if binary:
        if encoding is not None:
            raise ValueError("binary mode doesn't take an encoding argument")
        if errors is not None:
            raise ValueError("binary mode doesn't take an errors argument")
        if newline is not None:
            raise ValueError("binary mode doesn't take a newline argument")
    elif buffering == 0:
        raise ValueError("can't have unbuffered text I/O")
    line_buffering = buffering == 1
    if binary and line_buffering:
        import warnings
        warnings.warn("line buffering (buffering=1) isn't supported in binary "
                      "mode, the default buffer size will be used",
                      RuntimeWarning, stacklevel=2)
    size = buffering if buffering > 1 else _pyio.DEFAULT_BUFFER_SIZE
    if suffix is None:
        suffix = ''
    if prefix is None:
        prefix = template
    if dir is None:
        dir = gettempdir()
    global _sequence
    while True:
        _sequence += 1
        name = os.path.join(dir, '%s_%d_%d%s' % (prefix, os.getpid(), _sequence, suffix))
        try:
            descriptor = os.open(name, os.O_RDWR | os.O_CREAT | os.O_EXCL, 0o600)
            break
        except FileExistsError:
            continue
    try:
        os.unlink(name)
    except BaseException:
        os.close(descriptor)
        raise
    raw = None
    try:
        raw = _DescriptorRaw(descriptor, mode.replace('t', ''))
        if binary:
            if buffering == 0:
                return raw
            if reading and writing:
                return _pyio.BufferedRandom(raw, size)
            if reading:
                return _pyio.BufferedReader(raw, size)
            return _pyio.BufferedWriter(raw, size)
        if reading and writing:
            buffered = _pyio.BufferedRandom(raw, size)
        elif reading:
            buffered = _pyio.BufferedReader(raw, size)
        else:
            buffered = _pyio.BufferedWriter(raw, size)
        text = _pyio.TextIOWrapper(buffered, encoding, errors, newline, line_buffering)
        text.mode = mode
        return text
    except BaseException:
        if raw is None:
            os.close(descriptor)
        else:
            raw.close()
        raise


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
