# The package spelling shares the same path operations as os.path.
from os import path as _path
join = _path.join
split = _path.split
splitext = _path.splitext
basename = _path.basename
dirname = _path.dirname
exists = _path.exists
isfile = _path.isfile
isdir = _path.isdir
isabs = _path.isabs
abspath = _path.abspath
realpath = _path.realpath
normcase = _path.normcase
