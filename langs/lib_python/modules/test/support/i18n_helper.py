# Translation snapshots compare the message ids a module's source contains
# with a checked-in list. Making that comparison runs the gettext tools
# through a second interpreter over the Tools tree; the reference guards it
# with the subprocess check and a Tools-tree check, and the same guards are
# kept here, so a test deriving from the shared base skips only when one of
# those prerequisites is genuinely absent.
import os
import re
import subprocess
import sys
import unittest
from test.support import REPO_ROOT, TEST_HOME_DIR, requires_subprocess


pygettext = os.path.join(REPO_ROOT, 'Tools', 'i18n', 'pygettext.py')

msgid_pattern = re.compile(r'msgid(.*?)(?:msgid_plural|msgctxt|msgstr)',
                           re.DOTALL)
msgid_string_pattern = re.compile(r'"((?:\\"|[^"])*)"')


def skip_if_missing(tool=None):
    if tool:
        tooldir = os.path.join(REPO_ROOT, 'Tools', tool)
    else:
        tooldir = os.path.join(REPO_ROOT, 'Tools', 'scripts')
    if not os.path.isdir(tooldir):
        raise unittest.SkipTest('the Tools tree is not carried here')


def _generate_po_file(path, *, stdout_only=True):
    res = subprocess.run([sys.executable, pygettext,
                          '--no-location', '-o', '-', path],
                         stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                         text=True)
    if stdout_only:
        return res.stdout
    return res


def _extract_msgids(po):
    msgids = []
    for msgid in msgid_pattern.findall(po):
        msgid_string = ''.join(msgid_string_pattern.findall(msgid))
        msgid_string = msgid_string.replace('\\"', '"')
        if msgid_string:
            msgids.append(msgid_string)
    return sorted(msgids)


def _get_snapshot_path(module_name):
    return os.path.join(TEST_HOME_DIR, 'translationdata', module_name,
                        'msgids.txt')


@requires_subprocess()
class TestTranslationsBase(unittest.TestCase):

    def assertMsgidsEqual(self, module):
        skip_if_missing('i18n')
        res = _generate_po_file(module.__file__, stdout_only=False)
        self.assertEqual(res.returncode, 0)
        self.assertEqual(res.stderr, '')
        msgids = _extract_msgids(res.stdout)
        snapshot_path = _get_snapshot_path(module.__name__)
        with open(snapshot_path, encoding='utf-8') as snapshot_file:
            snapshot = snapshot_file.read().splitlines()
        self.assertListEqual(msgids, snapshot)


def update_translation_snapshots(module):
    skip_if_missing('i18n')
    contents = _generate_po_file(module.__file__)
    msgids = _extract_msgids(contents)
    snapshot_path = _get_snapshot_path(module.__name__)
    with open(snapshot_path, 'w', encoding='utf-8') as snapshot_file:
        snapshot_file.write('\n'.join(msgids))
