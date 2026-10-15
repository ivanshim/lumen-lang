#!/usr/bin/env python3
"""Stage stock reference files beside the runtime library on Lambda."""
import os
import shutil
import tempfile

# Import a reference source under its package name without changing script startup.
def reference_arguments(root, path):
    return [os.path.join(root, 'scripts', 'suite', 'reference_entry.py'), str(path)]

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
    context = os.path.join(stage, 'reference')
    os.mkdir(context)
    os.symlink(staged_suite, os.path.join(context, 'test'))
    path = os.path.join(context, 'test', relative)
    return reference_arguments(root, path), stage
