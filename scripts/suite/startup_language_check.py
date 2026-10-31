#!/usr/bin/env python3
"""Check language selection before Python stdin and terminal startup."""
import os
from pathlib import Path
import pty
import subprocess
import sys

from replcheck import read_prompt


# Check terminal and piped startup without allowing environment leakage.
def check(command):
    clean = {key: value for key, value in os.environ.items()
             if not key.startswith(('LUMEN_', 'PYTHON'))}
    cases = [
        ('default', [], None, True),
        ('empty-environment', [], '', True),
        ('python-flag', ['--lang', 'python'], None, True),
        ('python-environment', [], 'python', True),
        ('python-overrides-php', ['--lang', 'python'], 'langs/php.json', True),
        ('php-flag', ['--lang', 'langs/php.json'], None, False),
        ('lumen-flag', ['--lang', 'lumen'], None, False),
        ('php-environment', [], 'langs/php.json', False),
        ('lumen-environment', [], 'lumen', False),
        ('php-overrides-python', ['--lang', 'langs/php.json'], 'python', False),
        ('lumen-overrides-python', ['--lang', 'lumen'], 'python', False),
        ('interactive-then-php', ['-i', '--lang', 'langs/php.json'], None, False),
        ('interactive-then-lumen', ['-i', '--lang', 'lumen'], None, False),
    ]
    for name, args, language, python in cases:
        env = dict(clean)
        if language is not None:
            env['LUMEN_LANG'] = language
        result = subprocess.run(command + args, env=env, input=b'print(42)\n',
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=60)
        if python:
            assert (result.returncode, result.stdout, result.stderr) == (0, b'42\n', b''), (name, result)
        else:
            assert result.returncode != 0 and result.stderr.startswith(b'Usage:'), (name, result)
            assert not result.stdout and b'>>> ' not in result.stderr, (name, result)
        master, slave = pty.openpty()
        child = subprocess.Popen(command + args, env=env, stdin=slave,
                                 stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        os.close(slave)
        try:
            if python:
                greeting = read_prompt(child, b'>>> ')
                assert b'Python ' in greeting, (name, greeting)
                os.write(master, b'print(42, flush=True)\n')
                assert read_prompt(child, b'>>> ') == b'42\n>>> ', name
                os.write(master, b'\x04')
                tail, _ = child.communicate(timeout=60)
                assert child.returncode == 0 and tail == b'\n', (name, tail)
            else:
                output, _ = child.communicate(timeout=60)
                assert child.returncode != 0 and output.startswith(b'Usage:'), (name, output)
                assert b'>>> ' not in output, (name, output)
        finally:
            os.close(master)
            if child.poll() is None:
                child.kill()
                child.wait()
        print(name + ': pipe and terminal PASS', flush=True)
    print(f'startup language: {len(cases)} pipe + {len(cases)} terminal cases PASS', flush=True)


if __name__ == '__main__':
    check(sys.argv[1:] or [str(Path(__file__).resolve().parents[2] / 'target/debug/lumen-lang')])
