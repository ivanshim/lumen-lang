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
    # Stub: no real process identity is promised.
    return 1

def urandom(size):
    # Stub: byte storage is not yet available; no text masquerades as bytes.
    raise 'NotImplementedError: os.urandom needs byte values'

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

class terminal_size(tuple):
    __slots__ = ()
    def __new__(cls, fields):
        return tuple.__new__(cls, fields)
    @property
    def columns(self):
        return self[0]
    @property
    def lines(self):
        return self[1]

class stat_result(tuple):
    __slots__ = ()
    def __new__(cls, fields):
        return tuple.__new__(cls, fields)
    @property
    def st_mode(self):
        return self[0]
    @property
    def st_ino(self):
        return self[1]
    @property
    def st_dev(self):
        return self[2]
    @property
    def st_nlink(self):
        return self[3]
    @property
    def st_uid(self):
        return self[4]
    @property
    def st_gid(self):
        return self[5]
    @property
    def st_size(self):
        return self[6]
    @property
    def st_atime(self):
        return self[7]
    @property
    def st_mtime(self):
        return self[8]
    @property
    def st_ctime(self):
        return self[9]

def _stat_error(number, path):
    if number == 2:
        return FileNotFoundError(2, 'No such file or directory', path)
    if number == 20:
        return OSError(20, 'Not a directory', path)
    if number == 36:
        return OSError(36, 'File name too long', path)
    if number == 13:
        return OSError(13, 'Permission denied', path)
    return OSError(number, None, path)

def stat(path, *, dir_fd=None, follow_symlinks=True):
    if dir_fd is not None:
        raise 'NotImplementedError: os.stat directory descriptors are not supported'
    if not follow_symlinks:
        raise 'NotImplementedError: os.stat without following symbolic links is not supported'
    if '\x00' in path:
        raise ValueError('embedded null byte')
    if path.isascii():
        at = len(path)
    else:
        at = 0
    while at < len(path):
        code = ord(path[at])
        if 0xD800 <= code <= 0xDFFF:
            end = at + 2 if 0xDB00 <= code <= 0xDBFF and at + 1 < len(path) and 0xDC00 <= ord(path[at + 1]) <= 0xDFFF else at + 1
            raise UnicodeEncodeError('utf-8', path, at, end, 'surrogates not allowed')
        at += 1
    answer = __file_stat(path)
    if isinstance(answer, int):
        raise _stat_error(answer, path)
    return stat_result(answer)

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

    def isabs(self, path):
        return path[:1] == "/"

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

    def normcase(self, path):
        # The path rules below are POSIX, where normcase is the identity;
        # fnmatch measures it against the reference.
        return path

path = _Path()
