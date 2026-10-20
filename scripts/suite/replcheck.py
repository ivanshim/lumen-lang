#!/usr/bin/env python3
"""Check interactive statement boundaries with stdin kept open."""
import os
from pathlib import Path
import select
import subprocess
import sys
import tempfile
import time

# Read through the next prompt without closing the session's input.
def read_prompt(child, prompt):
    output = b''
    deadline = time.monotonic() + 60
    while not output.endswith(prompt):
        left = deadline - time.monotonic()
        assert left > 0, ('prompt timeout', output)
        ready, _, _ = select.select([child.stdout], [], [], left)
        assert ready, ('prompt timeout', output)
        part = os.read(child.stdout.fileno(), 65536)
        assert part, ('session ended early', output)
        output += part
    return output

# Compare each completed input chunk before any later input can conceal it.
def check(command):
    child = subprocess.Popen(command + ['-i', '-q'], stdin=subprocess.PIPE,
                             stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    steps = [
        ('hook', "import sys; sys.displayhook = lambda value: print(type(value).__name__ if callable(value) else repr(value), flush=True) if value is not None else None\n", b'>>> ', b''),
        ('decorator', '@staticmethod\n', b'... ', b''),
        ('definition', 'def f():\n', b'... ', b''),
        ('body', '    return 7\n', b'... ', b''),
        ('blank', '\n', b'>>> ', b''),
        ('call', 'print(f(), flush=True)\n', b'>>> ', b'7\n'),
        ('class-decorator', '@staticmethod\n', b'... ', b''),
        ('class', 'class C:\n', b'... ', b''),
        ('class-body', '    value = 8\n', b'... ', b''),
        ('class-blank', '\n', b'>>> ', b''),
        ('class-call', 'print(C().value, flush=True)\n', b'>>> ', b'8\n'),
        ('keywords', 'dict(a=1)\n', b'>>> ', b"{'a': 1}\n"),
        ('semicolons', '1; 2\n', b'>>> ', b'1\n2\n'),
        ('assignment-expression', 'x = 3; x\n', b'>>> ', b'3\n'),
        ('statement-expression', 'pass; 4\n', b'>>> ', b'4\n'),
        ('lambda-default', 'lambda a=1: a\n', b'>>> ', b'function\n'),
        ('prefix-assignment', 'class1 = 9\n', b'>>> ', b''),
        ('prefix-expression', 'class1\n', b'>>> ', b'9\n'),
        ('prefix-assignment-2', 'return1 = 10\n', b'>>> ', b''),
        ('prefix-expression-2', 'return1\n', b'>>> ', b'10\n'),
        ('joined-assignment', 'joined = 1 + \\\n', b'... ', b''),
        ('joined-tail', '    2\n', b'>>> ', b''),
        ('joined-print', 'print(joined, flush=True)\n', b'>>> ', b'3\n'),
        ('joined-expression', '1 + \\\n', b'... ', b''),
        ('joined-expression-tail', '    2\n', b'>>> ', b'3\n'),
        ('joined-string', 'text = "[#' + chr(92) + '\n', b'... ', b''),
        ('joined-string-tail', ')]"\n', b'>>> ', b''),
        ('joined-string-print', 'print(text, flush=True)\n', b'>>> ', b'[#)]\n'),
        ('joined-crlf-string', 'text = "a' + chr(92) + '\r\n', b'... ', b''),
        ('joined-crlf-tail', '([#"\r\n', b'>>> ', b''),
        ('joined-crlf-print', 'print(text, flush=True)\n', b'>>> ', b'a([#\n'),
        ('later-statement', 'print("after", flush=True)\n', b'>>> ', b'after\n'),
        ('long-header', 'if True:\n', b'... ', b''),
        ('long-body', '    large = "' + 'x' * 4100 + '"\n', b'... ', b''),
        ('long-tail', '    print(len(large), flush=True)\n', b'... ', b''),
        ('long-blank', '\n', b'>>> ', b'4100\n'),
        ('long-expression', 'len("' + 'x' * 4100 + '")\n', b'>>> ', b'4100\n'),
        ('rebind', 'sys = 0; compile = 0\n', b'>>> ', b''),
        ('rebind-after', 'print("isolated", flush=True)\n', b'>>> ', b'isolated\n'),
    ]
    try:
        assert read_prompt(child, b'>>> ') == b'>>> '
        for name, source, prompt, expected in steps:
            child.stdin.write(source.encode())
            child.stdin.flush()
            observed = read_prompt(child, prompt)
            assert observed == expected + prompt, (name, observed, expected + prompt)
            print(name + ': ' + repr(observed))
        child.stdin.close()
        tail = child.stdout.read()
        assert tail == b'\n', tail
        assert child.wait(timeout=60) == 0
        print('EOF: exit 0')
    finally:
        if child.poll() is None:
            child.kill()
            child.wait()

def check_module(command):
    command = [str(Path(command[0]).resolve())] + command[1:]
    with tempfile.TemporaryDirectory(dir=os.environ.get('REPL_PROBE_DIR', 'probe')) as directory:
        directory = str(Path(directory).resolve())
        env = os.environ.copy()
        env['LUMEN_ROOT'] = directory
        child = subprocess.Popen(command + ['-i', '-q', '-m', 'colorsys'],
                                 cwd=directory, env=env, stdin=subprocess.PIPE,
                                 stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        try:
            assert read_prompt(child, b'>>> ') == b'>>> '
            for source, expected in [
                ('print(__name__, __package__, __spec__.name, __file__.split("/")[-1], flush=True)\n', b'__main__  colorsys colorsys.py\n'),
                ('print(rgb_to_hsv(1.0, 0.0, 0.0), flush=True)\n', b'(0.0, 1.0, 1.0)\n'),
                ('print(__spec__.origin == __file__, flush=True)\n', b'True\n'),
            ]:
                child.stdin.write(source.encode())
                child.stdin.flush()
                observed = read_prompt(child, b'>>> ')
                assert observed == expected + b'>>> ', (observed, expected)
                print('module: ' + repr(observed))
            child.stdin.close()
            assert child.stdout.read() == b'\n'
            assert child.wait(timeout=60) == 0
            print('module EOF: exit 0')
        finally:
            if child.poll() is None:
                child.kill()
                child.wait()


if __name__ == '__main__':
    command = sys.argv[1:] or [str(Path(__file__).resolve().parents[2] / 'target/debug/lumen-lang')]
    check(command)
    check_module(command)
