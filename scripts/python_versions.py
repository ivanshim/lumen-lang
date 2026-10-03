"""The version table shared by suite tools; no tool guesses a suite path."""
import json
import os
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

def supported(root=ROOT):
    table = json.loads((Path(root) / 'langs/python/versions.json').read_text())
    versions = table['versions']
    if table['window'] != 2 or not 1 <= len(versions) <= 2:
        raise ValueError('Python requires one pin per series in a two-series window')
    return sorted(versions.values(), key=lambda v: tuple(map(int, v['release'].split('.'))))

def select(requested=None, root=ROOT):
    versions = supported(root)
    if requested is None:
        requested = os.environ.get('LUMEN_PYTHON')
    if requested is None:
        return versions[-1]
    parts = requested.split('.')
    if len(parts) in (2, 3) and all(p.isascii() and p.isdigit() and (len(p) == 1 or not p.startswith('0')) for p in parts):
        series = '.'.join(parts[:2])
        for version in versions:
            if version['release'].rsplit('.', 1)[0] == series:
                if len(parts) == 3 and requested != version['release']:
                    import sys
                    print(f"Warning: requested {requested}, running {version['release']} compatibility, tested against the CPython {version['release']} suite", file=sys.stderr)
                return version
    raise ValueError(f"Unsupported Python version {requested!r}; supported releases: " + ', '.join(v['release'] for v in versions))

def option(argv):
    """Remove the suite tools' optional --python value before positional parsing."""
    if '--python' not in argv:
        return None
    at = argv.index('--python')
    if at + 1 == len(argv):
        raise ValueError('--python requires a version')
    value = argv[at + 1]
    del argv[at:at + 2]
    return value

if __name__ == '__main__':
    for version in supported():
        print(version['release'], version['tests'], sep='\t')
