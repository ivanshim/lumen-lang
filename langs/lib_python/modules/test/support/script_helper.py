import os

# Subprocess helpers are named but cannot run a second interpreter.
def assert_python_ok(*args, **kwargs):
    raise 'NotImplementedError: interpreter subprocesses are not supported'

def assert_python_failure(*args, **kwargs):
    raise 'NotImplementedError: interpreter subprocesses are not supported'

def make_script(script_dir, script_basename, source, omit_suffix=False):
    if not omit_suffix:
        script_basename += os.extsep + 'py'
    script_name = os.path.join(script_dir, script_basename)
    with open(script_name, 'w', encoding='utf-8') as script_file:
        script_file.write(source)
    return script_name
