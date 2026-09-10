# The host supplies its own system and architecture words.
def python_implementation():
    return 'Lumen'

def python_version():
    return '3.14.0'

def system():
    word = __host_info()[1]
    names = {'linux': 'Linux', 'macos': 'Darwin', 'windows': 'Windows', 'freebsd': 'FreeBSD'}
    return names[word] if word in names else word

def machine():
    return __host_info()[2]
