# From CPython 3.14, Lib/posixpath.py, trimmed: the reference spells every
# POSIX path rule in this module and makes os.path an alias for it, while
# this runtime keeps those rules in os.path; the name is re-exported here so
# that `import posixpath` works and fnmatch's `os.path is posixpath` check
# falls back to normcasing each name in the loop, which is the same answer.
# Copyright (c) 2001 Python Software Foundation; All Rights Reserved.
# The PSF license is kept in tests/python/LICENSE.
from os import path

normcase = path.normcase
