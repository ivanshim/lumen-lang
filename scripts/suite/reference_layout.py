#!/usr/bin/env python3
"""Stage stock reference files beside the runtime library on Lambda."""
import os
import shutil
import tempfile

# Return interpreter arguments and the disposable stage for a reference source.
def staged_reference(root, prog, versions):
    tests = next((v['tests'] for v in versions.values()
                  if prog == v['tests'] or prog.startswith(v['tests'] + '/')), None)
    if tests is None:
        return None, None
    suite = os.path.join(root, tests)
    relative = os.path.relpath(os.path.join(root, prog), suite)
    stage = tempfile.mkdtemp()
    libroot = os.path.join(stage, 'tests')
    os.mkdir(libroot)
    library = os.path.join(root, 'langs', 'lib_python', 'modules')
    for name in os.listdir(library):
        os.symlink(os.path.join(library, name), os.path.join(libroot, name))
    staged_suite = os.path.join(libroot, os.path.basename(tests))
    shutil.copytree(suite, staged_suite)
    path = os.path.join(staged_suite, relative)
    if relative in ('test_patma.py', 'test_tabnanny.py', 'test_type_aliases.py'):
        return [os.path.join(root, 'scripts', 'suite', 'reference_entry.py'), path], stage
    if relative.endswith('/__main__.py'):
        return [os.path.join(root, 'scripts', 'suite', 'reference_entry.py'), path], stage
    return [path], stage
