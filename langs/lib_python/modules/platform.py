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

import os
import re
import sys

# http://php.net/manual/en/function.version-compare.php

_ver_stages = {
    # any string not found in this dict, will get 0 assigned
    'dev': 10,
    'alpha': 20, 'a': 20,
    'beta': 30, 'b': 30,
    'c': 40,
    'RC': 50, 'rc': 50,
    # number, will get 100 assigned
    'pl': 200, 'p': 200,
}


def _comparable_version(version):
    component_re = re.compile(r'([0-9]+|[._+-])')
    result = []
    for v in component_re.split(version):
        if v not in '._+-':
            try:
                v = int(v, 10)
                t = 100
            except ValueError:
                t = _ver_stages.get(v, 0)
            result.extend((t, v))
    return result

### Platform specific APIs


def libc_ver(executable=None, lib='', version='', chunksize=16384):

    """ Tries to determine the libc version that the file executable
        (which defaults to the Python interpreter) is linked against.

        Returns a tuple of strings (lib,version) which default to the
        given parameters in case the lookup fails.

        Note that the function has intimate knowledge of how different
        libc versions add symbols to the executable and thus is probably
        only usable for executables compiled using gcc.

        The file is read and scanned in chunks of chunksize bytes.

    """
    if not executable:
        if sys.platform == "emscripten":
            # Emscripten's os.confstr reports that it is glibc, so special case
            # it.
            ver = ".".join(str(x) for x in sys._emscripten_info.emscripten_version)
            return ("emscripten", ver)
        try:
            ver = os.confstr('CS_GNU_LIBC_VERSION')
            # parse 'glibc 2.28' as ('glibc', '2.28')
            parts = ver.split(maxsplit=1)
            if len(parts) == 2:
                return tuple(parts)
        except (AttributeError, ValueError, OSError):
            # os.confstr() or CS_GNU_LIBC_VERSION value not available
            pass

        executable = sys.executable

        if not executable:
            # sys.executable is not set.
            return lib, version

    libc_search = re.compile(br"""
          (__libc_init)
        | (GLIBC_([0-9.]+))
        | (libc(_\w+)?\.so(?:\.(\d[0-9.]*))?)
        | (musl-([0-9.]+))
        | ((?:libc\.|ld-)musl(?:-\w+)?.so(?:\.(\d[0-9.]*))?)
        """,
        re.ASCII | re.VERBOSE)

    V = _comparable_version
    # We use os.path.realpath()
    # here to work around problems with Cygwin not being
    # able to open symlinks for reading
    executable = os.path.realpath(executable)
    ver = None
    with open(executable, 'rb') as f:
        binary = f.read(chunksize)
        pos = 0
        while pos < len(binary):
            if b'libc' in binary or b'GLIBC' in binary or b'musl' in binary:
                m = libc_search.search(binary, pos)
            else:
                m = None
            if not m or m.end() == len(binary):
                chunk = f.read(chunksize)
                if chunk:
                    binary = binary[max(pos, len(binary) - 1000):] + chunk
                    pos = 0
                    continue
                if not m:
                    break
            decoded_groups = [s.decode('latin1') if s is not None else s
                              for s in m.groups()]
            (libcinit, glibc, glibcversion, so, threads, soversion,
             musl, muslversion, musl_so, musl_sover) = decoded_groups
            if libcinit and not lib:
                lib = 'libc'
            elif glibc:
                if lib != 'glibc':
                    lib = 'glibc'
                    ver = glibcversion
                elif V(glibcversion) > V(ver):
                    ver = glibcversion
            elif so:
                if lib not in ('glibc', 'musl'):
                    lib = 'libc'
                    if soversion and (not ver or V(soversion) > V(ver)):
                        ver = soversion
                    if threads and ver[-len(threads):] != threads:
                        ver = ver + threads
            elif musl:
                lib = 'musl'
                if not ver or V(muslversion) > V(ver):
                    ver = muslversion
            elif musl_so:
                lib = 'musl'
                if musl_sover and (not ver or V(musl_sover) > V(ver)):
                    ver = musl_sover
            pos = m.end()
    return lib, version if ver is None else ver
