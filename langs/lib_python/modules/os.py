# The path rules below are POSIX rules. Host facts are read afresh.
name = 'posix'
sep = '/'
altsep = None
linesep = '\n'
curdir = '.'
pardir = '..'
pathsep = ':'
environ = __host_info()[3]

def getcwd():
    directory = __host_info()[0]
    if directory is None:
        raise 'OSError: working directory is unavailable'
    return directory

def getpid():
    # Stub: no real process identity is promised.
    return 1

def urandom(size):
    # Stub: byte storage is not yet available; no text masquerades as bytes.
    raise 'NotImplementedError: os.urandom needs byte values'

def listdir(path='.'):
    raise 'NotImplementedError: os.listdir is not supported'

def mkdir(path, mode=511, *, dir_fd=None):
    raise 'NotImplementedError: os.mkdir is not supported'

def remove(path, *, dir_fd=None):
    if dir_fd is not None:
        raise 'NotImplementedError: os.remove directory descriptors are not supported'
    if not __remove_file(path):
        raise 'OSError: os.remove failed'

unlink = remove

class _Path:
    def join(self, path, *parts):
        for part in parts:
            if part.startswith('/'):
                path = part
            elif path == '' or path.endswith('/'):
                path += part
            else:
                path += '/' + part
        return path

    def split(self, path):
        at = len(path)
        while at > 0 and path[at - 1] != '/':
            at -= 1
        head = path[:at]
        if head != '/' * len(head):
            while head.endswith('/'):
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
        return __file_exists(path)

    def isfile(self, path):
        return __file_kind(path) == 1

    def isdir(self, path):
        return __file_kind(path) == 2

    def abspath(self, path):
        if not path.startswith('/'):
            path = self.join(getcwd(), path)
        parts = []
        for part in path.split('/'):
            if part == '..':
                if len(parts) > 0:
                    parts.pop()
            elif part != '' and part != '.':
                parts.append(part)
        return '/' + '/'.join(parts)

path = _Path()
