# Supplemental archive helper from CPython v3.14.8 Lib/test/support/script_helper.py; PSF License.
from importlib.util import source_from_cache
from test.support.import_helper import make_legacy_pyc

def make_zip_script(zip_dir, zip_basename, script_name, name_in_zip=None):
    import zipfile
    zip_filename = zip_basename+os.extsep+'zip'
    zip_name = os.path.join(zip_dir, zip_filename)
    with zipfile.ZipFile(zip_name, 'w') as zip_file:
        if name_in_zip is None:
            parts = script_name.split(os.sep)
            if len(parts) >= 2 and parts[-2] == '__pycache__':
                legacy_pyc = make_legacy_pyc(source_from_cache(script_name))
                name_in_zip = os.path.basename(legacy_pyc)
                script_name = legacy_pyc
            else:
                name_in_zip = os.path.basename(script_name)
        zip_file.write(script_name, name_in_zip)
    #if test.support.verbose:
    #    with zipfile.ZipFile(zip_name, 'r') as zip_file:
    #        print 'Contents of %r:' % zip_name
    #        zip_file.printdir()
    return zip_name, os.path.join(zip_name, name_in_zip)


# From CPython v3.14.8 Lib/test/support/script_helper.py; PSF License.
__cached_interp_requires_environment = None

def interpreter_requires_environment():
    """
    Returns True if our sys.executable interpreter requires environment
    variables in order to be able to run at all.

    This is designed to be used with @unittest.skipIf() to annotate tests
    that need to use an assert_python*() function to launch an isolated
    mode (-I) or no environment mode (-E) sub-interpreter process.

    A normal build & test does not run into this situation but it can happen
    when trying to run the standard library test suite from an interpreter that
    doesn't have an obvious home with Python's current home finding logic.

    Setting PYTHONHOME is one way to get most of the testsuite to run in that
    situation.  PYTHONPATH or PYTHONUSERSITE are other common environment
    variables that might impact whether or not the interpreter can start.
    """
    global __cached_interp_requires_environment
    if __cached_interp_requires_environment is None:
        # If PYTHONHOME is set, assume that we need it
        if 'PYTHONHOME' in os.environ:
            __cached_interp_requires_environment = True
            return True
        # cannot run subprocess, assume we don't need it
        if not support.has_subprocess_support:
            __cached_interp_requires_environment = False
            return False

        # Try running an interpreter with -E to see if it works or not.
        try:
            subprocess.check_call([sys.executable, '-E',
                                   '-c', 'import sys; sys.exit(0)'])
        except subprocess.CalledProcessError:
            __cached_interp_requires_environment = True
        else:
            __cached_interp_requires_environment = False

    return __cached_interp_requires_environment


# From CPython v3.14.8 Lib/test/support/script_helper.py; PSF License.
def spawn_python(*args, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, **kw):
    """Run a Python subprocess with the given arguments.

    kw is extra keyword args to pass to subprocess.Popen. Returns a Popen
    object.
    """
    cmd_line = [sys.executable]
    if not interpreter_requires_environment():
        cmd_line.append('-E')
    cmd_line.extend(args)
    # Under Fedora (?), GNU readline can output junk on stderr when initialized,
    # depending on the TERM setting.  Setting TERM=vt100 is supposed to disable
    # that.  References:
    # - http://reinout.vanrees.org/weblog/2009/08/14/readline-invisible-character-hack.html
    # - http://stackoverflow.com/questions/15760712/python-readline-module-prints-escape-character-during-import
    # - http://lists.gnu.org/archive/html/bug-readline/2007-08/msg00004.html
    env = kw.setdefault('env', dict(os.environ))
    env['TERM'] = 'vt100'
    return subprocess.Popen(cmd_line, stdin=subprocess.PIPE,
                            stdout=stdout, stderr=stderr,
                            **kw)
