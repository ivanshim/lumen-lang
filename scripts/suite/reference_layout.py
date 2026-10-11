"""Stage the registered reference suite with its real library layout."""
from pathlib import Path
import shutil
import tempfile

from python_tests import python_test_module


def staged_reference(root, program, versions):
    root = Path(root).resolve()
    source = (root / program).resolve()
    for version in versions.values():
        suite = (root / version['tests']).resolve()
        try:
            relative = source.relative_to(suite)
        except ValueError:
            continue
        break
    else:
        return None, None

    stage = Path(tempfile.mkdtemp(prefix='lumen-reference-'))
    library = stage / 'tests'
    library.mkdir()
    for member in (root / 'langs/lib_python/modules').iterdir():
        (library / member.name).symlink_to(member)
    copied = library / suite.name
    shutil.copytree(suite, copied)
    module = python_test_module(source)
    entry = copied / '_lumen_reference_entry.py'
    shutil.copyfile(root / 'scripts/suite/reference_entry.py', entry)
    return [str(entry), str(copied), module, str(copied / relative)], str(stage)
