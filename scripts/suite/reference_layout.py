#!/usr/bin/env python3
"""Stage stock reference files beside the runtime library on Lambda."""
import ast
import os
import shutil
import tempfile

# Import a reference source under its package name without changing script startup.
def reference_arguments(root, path):
    # An import-only script with a main guard delegates its entry point;
    # importing it as an empty unittest module would never call that entry.
    try:
        with open(path, encoding='utf-8') as source:
            statements = ast.parse(source.read()).body
    except SyntaxError:
        statements = []
    if statements and isinstance(statements[0], ast.Expr) and isinstance(statements[0].value, ast.Constant) and isinstance(statements[0].value.value, str):
        statements = statements[1:]
    if statements and all(isinstance(node, (ast.Import, ast.ImportFrom)) for node in statements[:-1]):
        guard = statements[-1]
        if isinstance(guard, ast.If) and not guard.orelse:
            test = guard.test
            if (isinstance(test, ast.Compare) and len(test.ops) == 1 and isinstance(test.ops[0], ast.Eq)
                    and isinstance(test.left, ast.Name) and test.left.id == '__name__'
                    and len(test.comparators) == 1 and isinstance(test.comparators[0], ast.Constant)
                    and test.comparators[0].value == '__main__'):
                return [str(path)]
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
