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

def win32_edition():
    # Only a Windows host names an edition; the registry is absent on the
    # others, so the missing import answers the question by itself.
    try:
        import winreg
    except ImportError:
        return None
    try:
        key = r'SOFTWARE\Microsoft\Windows NT\CurrentVersion'
        with winreg.OpenKeyEx(winreg.HKEY_LOCAL_MACHINE, key) as handle:
            return winreg.QueryValueEx(handle, 'EditionId')[0]
    except OSError:
        return None
