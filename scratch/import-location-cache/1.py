import importlib
import os
import sys
import tempfile
from pathlib import Path

with tempfile.TemporaryDirectory() as root:
    first = Path(root) / 'first'
    second = Path(root) / 'second'
    first.mkdir()
    second.mkdir()
    (first / 'cache_probe.py').write_text('value = 1\n')
    (second / 'cache_probe.py').write_text('value = 2\n')
    previous = sys.path[:]
    try:
        sys.path.insert(0, str(first))
        assert importlib.import_module('cache_probe').value == 1
        del sys.modules['cache_probe']
        sys.path[0] = str(second)
        assert importlib.import_module('cache_probe').value == 2
        del sys.modules['cache_probe']
        (second / 'cache_probe.py').write_text('value = 3\n')
        assert importlib.invalidate_caches() is None
        assert importlib.import_module('cache_probe').value == 3
        assert importlib.import_module('decimal') is importlib.import_module('decimal')
    finally:
        sys.path[:] = previous
        sys.modules.pop('cache_probe', None)
print('manifest caching preserves live paths, disk sources and invalidation')
