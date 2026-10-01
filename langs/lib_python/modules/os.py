# Host primitives used by class bodies need non-private module bindings.
_host_file_exists = __file_exists
_host_file_kind = __file_kind

# The path rules below are POSIX rules. Host facts are read afresh.
name = 'posix'
sep = '/'
extsep = '.'
altsep = None
linesep = '\n'
curdir = '.'
pardir = '..'
pathsep = ':'
environ = __host_info()[3]

def getenv(key, default=None):
    return environ.get(key, default)

def getcwd():
    directory = __host_info()[0]
    if directory is None:
        raise 'OSError: working directory is unavailable'
    return directory

def chdir(path):
    if not __change_dir(path):
        if not _host_file_exists(path):
            raise FileNotFoundError(2, 'No such file or directory', path)
        raise OSError(20, 'Not a directory', path)

def getpid():
    return __random('pid')

def urandom(size):
    # The system's own disorder, lent by the kernel (ext.builtin._random)
    # as so many bytes. The size is asked for as an index, as the
    # reference asks for it; one outside what a place's count can hold is
    # refused before the kernel is asked, as the reference refuses it.
    if type(size) is not int:
        if getattr(type(size), '__index__', None) is None:
            raise TypeError("'{}' object cannot be interpreted as an integer".format(type(size).__name__))
        from operator import index
        size = index(size)
    if size > 9223372036854775807 or size < -9223372036854775808:
        raise OverflowError('Python int too large to convert to C ssize_t')
    if size < 0:
        raise ValueError('negative argument not allowed')
    return __random('bytes', size)

def listdir(path='.'):
    if _host_file_kind(path) == 1:
        raise OSError(20, 'Not a directory', path)
    entries = __list_dir(path)
    if entries is False:
        raise FileNotFoundError(2, 'No such file or directory', path)
    return entries

def mkdir(path, mode=511, *, dir_fd=None):
    if dir_fd is not None:
        raise 'NotImplementedError: os.mkdir directory descriptors are not supported'
    if not __make_dir_one(path):
        if _host_file_exists(path):
            raise OSError(17, 'File exists', path)
        raise FileNotFoundError(2, 'No such file or directory', path)

def remove(path, *, dir_fd=None):
    if dir_fd is not None:
        raise 'NotImplementedError: os.remove directory descriptors are not supported'
    if not __remove_file(path):
        if not __file_exists(path):
            raise FileNotFoundError(2, 'No such file or directory', path)
        raise OSError(1, 'Operation not permitted', path)

unlink = remove

class _Path:
    def join(self, path, *parts):
        for part in parts:
            if part[:1] == '/':
                path = part
            elif path == '' or path[-1:] == '/':
                path += part
            else:
                path += '/' + part
        return path

    def split(self, path):
        at = len(path)
        while at > 0 and path[at - 1] != '/':
            at -= 1
        head = path[:at]
        named = False
        for letter in list(head):
            if letter != '/':
                named = True
        if named:
            while head[-1:] == '/':
                head = head[:-1]
        return (head, path[at:])

    def basename(self, path):
        return self.split(path)[1]

    def dirname(self, path):
        return self.split(path)[0]

    def splitext(self, path):
        at = len(path) - 1
        while at >= 0 and path[at] != '/':
            if path[at] == '.':
                start = at - 1
                while start >= 0 and path[start] != '/':
                    if path[start] != '.':
                        return (path[:at], path[at:])
                    start -= 1
                break
            at -= 1
        return (path, '')

    def exists(self, path):
        return _host_file_exists(path)

    def isfile(self, path):
        return _host_file_kind(path) == 1

    def isdir(self, path):
        return _host_file_kind(path) == 2

    def abspath(self, path):
        if path[:1] != '/':
            path = self.join(getcwd(), path)
        parts = []
        words = []
        word = ''
        for letter in list(path + '/'):
            if letter == '/':
                words.append(word)
                word = ''
            else:
                word += letter
        for part in words:
            if part == '..':
                if len(parts) > 0:
                    parts = parts[:-1]
            elif part != '' and part != '.':
                parts.append(part)
        answer = ''
        for part in parts:
            answer += '/' + part
        return answer if answer else '/'

    def realpath(self, path):
        # The host gives no way to follow a link, so the answer is
        # the path made absolute and tidy, which is the whole of it
        # for a file a run has made itself.
        return self.abspath(path)

path = _Path()

class terminal_size(tuple):
    # A width and a height, as a tuple of the two.
    def __new__(cls, size):
        return tuple.__new__(cls, size)

    @property
    def columns(self):
        return self[0]

    @property
    def lines(self):
        return self[1]
