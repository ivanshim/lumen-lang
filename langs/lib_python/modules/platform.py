# The host supplies its own system and architecture words.
def python_implementation():
    return 'Lumen'

def python_version():
    return '3.14.8'

def system():
    word = __host_info()[1]
    names = {'linux': 'Linux', 'macos': 'Darwin', 'windows': 'Windows', 'freebsd': 'FreeBSD'}
    return names[word] if word in names else word

def machine():
    return __host_info()[2]

# The C library is not probed (no executable file is scanned for its
# version string), so the answer is the one the reference gives when it
# finds nothing: the arguments it was handed.
def libc_ver(executable=None, lib='', version='', chunksize=16384):
    return lib, version
