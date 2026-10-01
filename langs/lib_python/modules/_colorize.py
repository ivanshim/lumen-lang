# Selected source from CPython commit 3b564385e4c9, Lib/_colorize.py.
# Copyright (c) 2001 Python Software Foundation; All Rights Reserved.
# Used under the PSF license in tests/python/LICENSE.
import os
import sys

COLORIZE = True

class ANSIColors:
    RESET = "\x1b[0m"
    BLACK = "\x1b[30m"
    BLUE = "\x1b[34m"
    CYAN = "\x1b[36m"
    GREEN = "\x1b[32m"
    GREY = "\x1b[90m"
    MAGENTA = "\x1b[35m"
    RED = "\x1b[31m"
    WHITE = "\x1b[37m"  # more like LIGHT GRAY
    YELLOW = "\x1b[33m"

    BOLD = "\x1b[1m"
    BOLD_BLACK = "\x1b[1;30m"  # DARK GRAY
    BOLD_BLUE = "\x1b[1;34m"
    BOLD_CYAN = "\x1b[1;36m"
    BOLD_GREEN = "\x1b[1;32m"
    BOLD_MAGENTA = "\x1b[1;35m"
    BOLD_RED = "\x1b[1;31m"
    BOLD_WHITE = "\x1b[1;37m"  # actual WHITE
    BOLD_YELLOW = "\x1b[1;33m"

    # intense = like bold but without being bold
    INTENSE_BLACK = "\x1b[90m"
    INTENSE_BLUE = "\x1b[94m"
    INTENSE_CYAN = "\x1b[96m"
    INTENSE_GREEN = "\x1b[92m"
    INTENSE_MAGENTA = "\x1b[95m"
    INTENSE_RED = "\x1b[91m"
    INTENSE_WHITE = "\x1b[97m"
    INTENSE_YELLOW = "\x1b[93m"

    BACKGROUND_BLACK = "\x1b[40m"
    BACKGROUND_BLUE = "\x1b[44m"
    BACKGROUND_CYAN = "\x1b[46m"
    BACKGROUND_GREEN = "\x1b[42m"
    BACKGROUND_MAGENTA = "\x1b[45m"
    BACKGROUND_RED = "\x1b[41m"
    BACKGROUND_WHITE = "\x1b[47m"
    BACKGROUND_YELLOW = "\x1b[43m"

    INTENSE_BACKGROUND_BLACK = "\x1b[100m"
    INTENSE_BACKGROUND_BLUE = "\x1b[104m"
    INTENSE_BACKGROUND_CYAN = "\x1b[106m"
    INTENSE_BACKGROUND_GREEN = "\x1b[102m"
    INTENSE_BACKGROUND_MAGENTA = "\x1b[105m"
    INTENSE_BACKGROUND_RED = "\x1b[101m"
    INTENSE_BACKGROUND_WHITE = "\x1b[107m"
    INTENSE_BACKGROUND_YELLOW = "\x1b[103m"


ColorCodes = set()
NoColors = ANSIColors()

for attr, code in ANSIColors.__dict__.items():
    if not attr.startswith("__"):
        ColorCodes.add(code)
        setattr(NoColors, attr, "")


def decolor(text: str) -> str:
    """Remove ANSI color codes from a string."""
    for code in ColorCodes:
        text = text.replace(code, "")
    return text


def can_colorize(*, file: IO[str] | IO[bytes] | None = None) -> bool:

    def _safe_getenv(k: str, fallback: str | None = None) -> str | None:
        """Exception-safe environment retrieval. See gh-128636."""
        try:
            return os.environ.get(k, fallback)
        except Exception:
            return fallback

    if file is None:
        file = sys.stdout

    if not sys.flags.ignore_environment:
        if _safe_getenv("PYTHON_COLORS") == "0":
            return False
        if _safe_getenv("PYTHON_COLORS") == "1":
            return True
    if _safe_getenv("NO_COLOR"):
        return False
    if not COLORIZE:
        return False
    if _safe_getenv("FORCE_COLOR"):
        return True
    if _safe_getenv("TERM") == "dumb":
        return False

    if not hasattr(file, "fileno"):
        return False

    if sys.platform == "win32":
        try:
            import nt

            if not nt._supports_virtual_terminal():
                return False
        except (ImportError, AttributeError):
            return False

    try:
        return os.isatty(file.fileno())
    except OSError:
        return hasattr(file, "isatty") and file.isatty()


class _ColorlessTheme:
    # A 'fake' theme for no colors
    def __getattr__(self, name):
        # _colorize's no_color themes are just all empty strings
        # by directly using empty strings the import is avoided
        if name.startswith("_"):
            raise AttributeError(name)
        return ""

# Theme dataclasses are not supported yet. Uncolored sections use the
# same attribute policy as CPython's argparse; requests for colored
# themes fail explicitly instead of returning a theme with invented colors.
class _PlainSections:
    argparse = _ColorlessTheme()
    difflib = _ColorlessTheme()

_plain_sections = _PlainSections()

def get_theme(*, tty_file=None, force_color=False, force_no_color=False):
    if force_color or (not force_no_color and can_colorize(file=tty_file)):
        raise NotImplementedError('terminal color themes require dataclass support')
    return _plain_sections
