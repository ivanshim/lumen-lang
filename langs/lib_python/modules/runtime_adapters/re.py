# Runtime adapter for CPython Lib/re/__init__.py at v3.14.8 /
# 8e6e75d9102e; PSF License.  The module is still growing its flag roster;
# tokenize.py and argparse.py compile patterns with re.ASCII.  Supply the
# standard value (ASCII = A = 256) only when re does not name it yet.
if not globals().get('ASCII'):
    ASCII = 256
    A = 256
