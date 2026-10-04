"""Probe immutable release bindings, legacy migration, and comparison guards."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]
SCRIPTS = ROOT / 'scripts/suite'
(ROOT / 'probe').mkdir(exist_ok=True)
with tempfile.TemporaryDirectory(prefix='count-integrity-', dir=ROOT / 'probe') as temporary:
    base = Path(temporary)
    root = base / 'root'
    (root / 'langs/python').mkdir(parents=True)
    tests = root / 'tests/python-3.14.8'
    tests.mkdir(parents=True)
    (tests / 'test_fixture.py').write_text('pass\n')
    table = root / 'langs/python/versions.json'
    version = {'release': '3.14.8', 'tests': 'tests/python-3.14.8'}
    def table_write(versions):
        table.write_text(json.dumps({'window': 2, 'versions': versions}))
    table_write({'3.14': version})
    marker = base / 'invocations'
    binary = base / 'fake-binary'
    binary.write_text('#!/bin/sh\necho called >> "$PROBE_MARKER"\nprintf ".\\nRan 1 test\\n" >&2\n')
    binary.chmod(0o755)
    env = dict(os.environ, LUMEN_ROOT=str(root), PROBE_MARKER=str(marker), TMPDIR=str(base), SUITE_JOBS='1')
    env.pop('LUMEN_PYTHON', None)
    def run(script, *args, success=True):
        p = subprocess.run([sys.executable, str(SCRIPTS / script), *map(str, args)],
                           capture_output=True, text=True, env=env, timeout=20)
        assert (p.returncode == 0) == success, (script, p.returncode, p.stdout, p.stderr)
        return p.stdout + p.stderr
    def snapshot(directory):
        return {p.name: p.read_bytes() for p in directory.iterdir() if p.is_file()}
    raw = base / 'raw'
    run('count_run.py', raw, 5, binary)
    binding = (raw / 'suite.json').read_bytes()
    run('rerun_one.py', raw, 'test_fixture', 'stack8', 5, binary)
    assert (raw / 'suite.json').read_bytes() == binding
    calls = marker.read_bytes()
    run('count_run.py', raw, 5, binary)
    assert marker.read_bytes() == calls
    print('PASS same-release rerun/resume preserve metadata; existing results reused')
    saved = snapshot(raw)
    table_write({'3.14': version, '3.15': {'release': '3.15.0', 'tests': 'tests/python-3.15.0'}})
    for script, args in [('count_run.py', [raw, 5, binary]),
                         ('rerun_one.py', [raw, 'test_fixture', 'stack8', 5, binary])]:
        out = run(script, *args, success=False)
        assert 'measured CPython 3.14.8; requested CPython 3.15.0' in out
        assert snapshot(raw) == saved and marker.read_bytes() == calls
        print(f'PASS {script} rejects newly registered default before any write or execution')
    table_write({'3.14': {'release': '3.14.9', 'tests': 'tests/python-3.14.9'}})
    for script, args in [('count_run.py', [raw, 5, binary]),
                         ('rerun_one.py', [raw, 'test_fixture', 'stack8', 5, binary])]:
        out = run(script, *args, '--python', '3.14.8', success=False)
        assert 'requested CPython 3.14.9' in out
        assert snapshot(raw) == saved and marker.read_bytes() == calls
        print(f'PASS {script} rejects micro refresh before any write or execution')
    output = run('count.py', raw)
    assert 'CPython 3.14.8 suite' in output and '3.14.9' not in output
    assert snapshot(raw) == saved
    print('PASS reporting retains measured 3.14.8 after table refresh to 3.14.9')
    legacy = base / 'legacy'
    legacy.mkdir()
    (legacy / 'test_fixture.stack8.txt').write_bytes(saved['test_fixture.stack8.txt'])
    legacy_saved = snapshot(legacy)
    for script, args in [('count.py', [legacy]), ('count.py', [raw, legacy]),
                         ('count_run.py', [legacy, 5, binary]),
                         ('rerun_one.py', [legacy, 'test_fixture', 'stack8', 5, binary])]:
        out = run(script, *args, success=False)
        assert 'migrat' in out
        assert snapshot(legacy) == legacy_saved and marker.read_bytes() == calls
        print(f'PASS {script} refuses unbound legacy data without writes')
    evidence = base / 'original-inventory.json'
    inventory = {'release': '3.14.8', 'sha256': {name: hashlib.sha256(data).hexdigest() for name, data in legacy_saved.items()}}
    evidence.write_text(json.dumps(inventory))
    run('migrate_results.py', legacy, '--release', '3.14', '--provenance', evidence, success=False)
    run('migrate_results.py', legacy, '--release', '3.14.9', '--provenance', evidence, success=False)
    inventory['sha256']['test_fixture.stack8.txt'] = 'bad'
    evidence.write_text(json.dumps(inventory))
    run('migrate_results.py', legacy, '--release', '3.14.8', '--provenance', evidence, success=False)
    assert snapshot(legacy) == legacy_saved
    print('PASS migration refuses series-only, provenance release mismatch, and byte mismatch')
    inventory['sha256']['test_fixture.stack8.txt'] = hashlib.sha256(legacy_saved['test_fixture.stack8.txt']).hexdigest()
    evidence.write_text(json.dumps(inventory))
    run('migrate_results.py', legacy, '--release', '3.14.8', '--provenance', evidence)
    assert (legacy / 'test_fixture.stack8.txt').read_bytes() == legacy_saved['test_fixture.stack8.txt']
    migrated = snapshot(legacy)
    run('migrate_results.py', legacy, '--release', '3.14.8', '--provenance', evidence, success=False)
    assert snapshot(legacy) == migrated
    run('count.py', raw, legacy)
    print('PASS verified explicit legacy migration preserves results; repeated migration refused; same-release comparison allowed')
    other = base / 'other'
    other.mkdir()
    (other / 'suite.json').write_text(json.dumps({'release': '3.15.0'}))
    for a, b in [(raw, other), (other, raw)]:
        out = run('count.py', a, b, success=False)
        assert 'Cannot compare CPython' in out
    print('PASS cross-release comparisons refused in both directions')
    (other / 'suite.json').write_text(json.dumps({'release': '3.14'}))
    run('count.py', other, success=False)
    print('PASS malformed binding refused instead of guessed')
print('count integrity: all probes passed')
