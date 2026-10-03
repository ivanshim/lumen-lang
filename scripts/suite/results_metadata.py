"""Immutable release bindings for saved Python suite measurements."""
import hashlib
import json
from pathlib import Path
import re


def full_release(value):
    if not isinstance(value, str) or not re.fullmatch(r'(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)', value):
        raise ValueError('Measurement metadata requires an exact full release (x.y.z)')
    return value


def read_binding(directory):
    directory = Path(directory)
    metadata = directory / 'suite.json'
    if not metadata.is_file():
        raise ValueError(f'{directory}: missing suite.json; explicitly migrate legacy measurements with migrate_results.py before use')
    data = json.loads(metadata.read_text())
    full_release(data['release'])
    return data


def write_binding(directory, data):
    # Exclusive creation: an existing binding is never overwritten or relabeled.
    with (directory / 'suite.json').open('x') as stream:
        stream.write(json.dumps(data, sort_keys=True) + '\n')


def bind_for_run(directory, version):
    directory = Path(directory)
    release = full_release(version['release'])
    if (directory / 'suite.json').exists():
        measured = read_binding(directory)['release']
        if measured != release:
            raise ValueError(f'{directory}: measured CPython {measured}; requested CPython {release}; use a separate results directory')
        return
    if directory.exists() and any(directory.iterdir()):
        raise ValueError(f'{directory}: nonempty legacy directory has no suite.json; explicitly migrate its actually measured release with migrate_results.py before use')
    directory.mkdir(parents=True, exist_ok=True)
    write_binding(directory, version)


def compare_bindings(new_directory, old_directory=None):
    release = read_binding(new_directory)['release']
    if old_directory is not None:
        previous = read_binding(old_directory)['release']
        if previous != release:
            raise ValueError(f'Cannot compare CPython {release} suite with CPython {previous} suite')
    return release


def migrate(directory, release, provenance):
    """Verify an explicit original-measurement inventory before binding legacy data."""
    directory = Path(directory)
    release = full_release(release)
    if (directory / 'suite.json').exists():
        raise ValueError('Directory is already bound; migration must never replace suite.json')
    evidence_path = Path(provenance)
    evidence_bytes = evidence_path.read_bytes()
    evidence = json.loads(evidence_bytes)
    if full_release(evidence['release']) != release:
        raise ValueError('Requested migration release differs from original measurement provenance')
    files = {path.name: hashlib.sha256(path.read_bytes()).hexdigest()
             for path in sorted(directory.glob('*.txt')) if path.is_file()}
    if not files or evidence['sha256'] != files:
        raise ValueError('Original measurement provenance must match every saved .txt result byte for byte')
    write_binding(directory, {'release': release, 'migration': {
        'provenance': str(evidence_path.resolve()),
        'provenance_sha256': hashlib.sha256(evidence_bytes).hexdigest()}})
