#!/usr/bin/env python3
"""Probe selection and precedence on both full kernels using an isolated sidecar."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from python_versions import supported
ROOT = Path(__file__).resolve().parents[2]
BINARY = ROOT / 'target/debug/lumen-lang'
versions = supported(ROOT)
newest = versions[-1]['release']
pin = versions[0]['release']
series = pin.rsplit('.', 1)[0]
wrong = series + '.' + str(max(0, int(pin.rsplit('.', 1)[1]) - 2) if int(pin.rsplit('.', 1)[1]) else 1)
unsupported = '4.0'
for candidate in ('3.13', '3.16', '4.0'):
    if candidate not in (v['release'].rsplit('.', 1)[0] for v in versions):
        unsupported = candidate
        break
with tempfile.TemporaryDirectory(prefix='python-version-probe-') as directory:
    file = Path(directory) / 'identity.py'
    file.write_text('import sys\nimport platform\nprint(sys.version)\nprint(sys.version_info)\nprint(sys.hexversion)\nprint(platform.python_version())\n')
    sidecar = Path(str(file) + '.lumen.json')
    cases = [
        ('default', [], None, None, newest, False),
        ('series', ['--python', series], None, None, pin, False),
        ('exact', ['--python', pin], None, None, pin, False),
        ('micro-warning', ['--python', wrong], None, None, pin, True),
        ('large-micro-warning', ['--python', series + '.' + '9' * 100], None, None, pin, True),
        ('unsupported', ['--python', unsupported], None, None, None, False),
        ('malformed', ['--python', '3.14.x'], None, None, None, False),
        ('empty', ['--python', ''], None, None, None, False),
        ('environment', [], series, None, pin, False),
        ('configuration', [], None, series, pin, False),
        ('environment>configuration', [], series, unsupported, pin, False),
        ('command>environment>configuration', ['--python', pin], unsupported, unsupported, pin, False),
        ('environment-error', [], unsupported, series, None, False),
        ('configuration-error', [], None, unsupported, None, False),
    ]
    cases += [(f'unsupported-{value}', ['--python', value], None, None, None, False)
              for value in ('3.13', '3.16', '4.0')
              if value not in (v['release'].rsplit('.', 1)[0] for v in versions)]
    cases += [(f'malformed-{value}', ['--python', value], None, None, None, False)
              for value in ('3', '3..14', '3.14.8.1', 'v3.14.8', '3.014', ' 3.14')]
    cases += [('environment-warning', [], wrong, None, pin, True),
              ('configuration-warning', [], None, wrong, pin, True)]
    for kernel in ('stack8', 'microcode7'):
        for label, args, environment, configuration, expected, warning in cases:
            sidecar.unlink(missing_ok=True)
            if configuration is not None:
                sidecar.write_text(json.dumps({'python': {'version': configuration}}))
            env = {k: v for k, v in os.environ.items() if k not in ('LUMEN_PYTHON', 'LUMEN_LANG', 'LUMEN_BARE')}
            if environment is not None:
                env['LUMEN_PYTHON'] = environment
            result = subprocess.run([str(BINARY), '--kernel', kernel, *args, str(file)], env=env, text=True, capture_output=True, stdin=subprocess.DEVNULL, timeout=30)
            if expected is None:
                assert result.returncode != 0 and not result.stdout, (kernel, label, result)
                assert 'Supported releases:' in result.stderr and all(v['release'] in result.stderr for v in versions), result.stderr
            else:
                major, minor, micro = map(int, expected.split('.'))
                assert result.returncode == 0, (kernel, label, result.stderr)
                assert result.stdout.splitlines() == [expected + ' (Lumen)', f"({major}, {minor}, {micro}, 'final', 0)", str((major << 24) | (minor << 16) | (micro << 8) | 0xf0), expected], result.stdout
                if warning:
                    requested = args[args.index('--python') + 1] if '--python' in args else environment or configuration
                    assert f'requested {requested}, running {expected} compatibility, tested against the CPython {expected} suite' in result.stderr, result.stderr
                else:
                    assert not result.stderr, result.stderr
            print(f'{kernel} {label}: exit {result.returncode}; {result.stdout.strip() or result.stderr.strip()}' + (f'\nstderr: {result.stderr.strip()}' if expected and warning else ''))
print('all selection probes passed')
