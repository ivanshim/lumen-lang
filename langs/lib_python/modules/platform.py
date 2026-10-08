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

def libc_ver():
    # Which C library the run stands on, read from the libraries the
    # host has mapped into it; their names carry no version here.
    try:
        loaded = open('/proc/self/maps').read()
    except OSError:
        return ('', '')
    if 'libc.musl' in loaded or 'ld-musl' in loaded:
        return ('musl', '')
    if 'libc.so.6' in loaded or 'libc-' in loaded:
        return ('glibc', '')
    return ('', '')
