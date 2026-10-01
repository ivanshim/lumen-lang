# The colouring part of CPython 3.14's Lib/_colorize.py, written out in
# Python for the difflib colouring difflib.py reaches for. The full module
# builds every theme section out of dataclasses with frozen and keyword-only
# fields, which the dataclasses here do not yet provide; only the difflib
# section and the two entry points difflib imports are carried.
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
    WHITE = "\x1b[37m"
    YELLOW = "\x1b[33m"
    BOLD = "\x1b[1m"

class Difflib:
    def __init__(self, color):
        if color:
            self.added = ANSIColors.GREEN
            self.context = ANSIColors.RESET
            self.header = ANSIColors.BOLD
            self.hunk = ANSIColors.CYAN
            self.removed = ANSIColors.RED
            self.reset = ANSIColors.RESET
        else:
            self.added = ""
            self.context = ""
            self.header = ""
            self.hunk = ""
            self.removed = ""
            self.reset = ""

class Theme:
    def __init__(self, color=True):
        self.difflib = Difflib(color)

def can_colorize(*, file=None):
    if os.getenv('PYTHON_COLORS') == '0':
        return False
    if os.getenv('PYTHON_COLORS') == '1':
        return True
    if os.getenv('NO_COLOR'):
        return False
    if not COLORIZE:
        return False
    if os.getenv('FORCE_COLOR'):
        return True
    if os.getenv('TERM') == 'dumb':
        return False
    return False

def get_theme(*, tty_file=None, force_color=False, force_no_color=False):
    if force_color or (not force_no_color and can_colorize(file=tty_file)):
        return _theme
    return theme_no_color

default_theme = Theme(color=True)
theme_no_color = Theme(color=False)
_theme = default_theme

def set_theme(theme):
    global _theme
    _theme = theme
