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
