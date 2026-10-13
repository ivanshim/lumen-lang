#!/usr/bin/env python3
"""Stage stock reference files beside the runtime library on Lambda."""
import ast
import os
import shutil
import tempfile

def needs_module_collection(path):
    """Honor script entry points unless collection requires an imported caller."""
    with open(path, encoding='utf-8') as source:
        try:
            tree = ast.parse(source.read(), filename=path)
        except SyntaxError:
            # A newer reference syntax may exceed the staging host's parser.
            # The interpreter, not this host, will run that source's entry point.
            return False
    modules = {}
    functions = {}
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            for name in node.names:
                local = name.asname or name.name.split('.', 1)[0]
                modules[local] = name.name if name.asname else local
        elif isinstance(node, ast.ImportFrom) and node.module:
            for name in node.names:
                functions[name.asname or name.name] = (node.module, name.name)
    script_entry = False
    for node in ast.walk(tree):
        if not isinstance(node, ast.Call):
            continue
        target = node.func
        if isinstance(target, ast.Name):
            binding = functions.get(target.id)
        elif isinstance(target, ast.Attribute) and isinstance(target.value, ast.Name):
            binding = (modules.get(target.value.id), target.attr)
        else:
            continue
        if binding == ('unittest', 'main'):
            script_entry = True
        elif binding == ('doctest', 'DocTestSuite'):
            module = node.args[0] if node.args else next(
                (arg.value for arg in node.keywords if arg.arg == 'module'), None)
            if module is None or isinstance(module, ast.Constant) and module.value is None:
                return True
    return not script_entry


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
    if relative.endswith('/__main__.py'):
        return ['-m', 'test.' + relative.split('/')[0]], stage
    if needs_module_collection(path):
        return [os.path.join(root, 'scripts', 'suite', 'reference_entry.py'), path], stage
    return [path], stage
