# Minimal urllib.request adapter; source CPython v3.14.8 Lib/urllib/request.py.
# This build has no network opener, so the HTTP test classes that call it skip
# through test.support before reaching here. The names exist so that
# urllib.robotparser imports cleanly and so that urlopen reports the missing
# facility instead of a wrong answer.
import urllib.error
import urllib.parse
import urllib.response


def urlcleanup():
    """Release resources held by a previous urlopen call (there are none)."""


def urlopen(url, data=None, timeout=None, *, context=None):
    raise NotImplementedError(
        "urllib.request.urlopen is unavailable: no network opener")


def build_opener(*handlers):
    raise NotImplementedError(
        "urllib.request.build_opener is unavailable: no network opener")


def install_opener(opener):
    raise NotImplementedError(
        "urllib.request.install_opener is unavailable: no network opener")
