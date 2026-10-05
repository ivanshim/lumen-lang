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
    raise 'NotImplementedError: tempfile.mktemp would name a file this runtime cannot then create'


def NamedTemporaryFile(*args, **keywords):
    raise 'NotImplementedError: tempfile.NamedTemporaryFile needs a file to be created and opened, which this runtime does not carry'


# A file the runtime opens by descriptor alone. It stands on an
# operating-system handle, reads and writes through the posix
# primitives, and keeps no name of its own; the directory entry is
# taken away as soon as the handle is made, the way an unnamed
# temporary file works.
class _DescriptorFile:
    def __init__(self, descriptor, mode):
        self._descriptor = descriptor
        self.mode = mode
        self._binary = 'b' in mode
        self.encoding = None if self._binary else 'utf-8'
        self.errors = 'strict'
        self.closed = False
        self.name = descriptor

    def _open(self):
        if self.closed:
            raise ValueError('I/O operation on closed file.')

    def readable(self):
        return 'r' in self.mode or '+' in self.mode

    def writable(self):
        return 'w' in self.mode or 'a' in self.mode or 'x' in self.mode or '+' in self.mode

    def seekable(self):
        return True

    def fileno(self):
        self._open()
        return self._descriptor

    def flush(self):
        self._open()

    def write(self, data):
        self._open()
        if self._binary:
            if not isinstance(data, (bytes, bytearray)):
                raise TypeError('a bytes-like object is required, not ' + type(data).__name__)
        else:
            if not isinstance(data, str):
                raise TypeError('write() argument must be str, not ' + type(data).__name__)
            data = data.encode(self.encoding, self.errors)
        whole = len(data)
        total = 0
        while total < whole:
            piece = data[total:total + 65536]
            count = os.write(self._descriptor, piece)
            if not count:
                break
            total += count
        return total if self._binary else len(data.decode(self.encoding, self.errors))

    def read(self, size=-1):
        self._open()
        if size is None or size < 0:
            pieces = []
            while True:
                piece = os.read(self._descriptor, 65536)
                if not piece:
                    break
                pieces.append(piece)
            data = b''.join(pieces)
        else:
            data = os.read(self._descriptor, size)
        return data if self._binary else data.decode(self.encoding, self.errors)

    def readinto(self, buffer):
        self._open()
        piece = os.read(self._descriptor, min(len(buffer), 65536))
        count = len(piece)
        buffer[:count] = piece
        return count

    def readline(self, size=-1):
        self._open()
        line = bytearray()
        while size is None or size < 0 or len(line) < size:
            piece = os.read(self._descriptor, 1)
            if not piece:
                break
            line += piece
            if piece == b'\n':
                break
        data = bytes(line)
        return data if self._binary else data.decode(self.encoding, self.errors)

    def seek(self, offset, whence=0):
        self._open()
        return os.lseek(self._descriptor, offset, whence)

    def tell(self):
        self._open()
        return os.lseek(self._descriptor, 0, 1)

    def close(self):
        if self.closed:
            return
        self.closed = True
        descriptor = self._descriptor
        self._descriptor = -1
        try:
            os.close(descriptor)
        except OSError:
            pass

    def __enter__(self):
        self._open()
        return self

    def __exit__(self, *ignored):
        self.close()

    def __iter__(self):
        return self

    def __next__(self):
        line = self.readline()
        if not line:
            raise StopIteration
        return line

    def __repr__(self):
        return "<tempfile._DescriptorFile descriptor=%d mode=%r>" % (self._descriptor, self.mode)


_sequence = 0


def TemporaryFile(mode='w+b', buffering=-1, encoding=None, newline=None,
                  suffix=None, prefix=None, dir=None, *, errors=None):
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
    os.unlink(name)
    return _DescriptorFile(descriptor, mode)


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
