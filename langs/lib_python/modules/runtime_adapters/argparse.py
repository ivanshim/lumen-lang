# HelpFormatter always imports the color themes, and that import pulls in
# inspect. With color disabled the theme is empty strings, so the import
# only changes how long startup takes.
import os as _lumen_os
import sys as _lumen_sys

class _LumenNoColor:
    def __getattr__(self, name):
        return ""

def _lumen_getenv(key):
    try:
        return _lumen_os.environ.get(key)
    except Exception:
        return None

def _lumen_can_colorize(file=None):
    if file is None:
        file = _lumen_sys.stdout
    # Honor a replaced can_colorize once _colorize is imported. That import
    # is the startup cost the plain path skips.
    loaded = _lumen_sys.modules.get("_colorize")
    if loaded is not None:
        return loaded.can_colorize(file=file)
    if not _lumen_sys.flags.ignore_environment:
        colors = _lumen_getenv("PYTHON_COLORS")
        if colors == "0":
            return False
        if colors == "1":
            return True
    if _lumen_getenv("NO_COLOR"):
        return False
    if _lumen_getenv("FORCE_COLOR"):
        return True
    if _lumen_getenv("TERM") == "dumb":
        return False
    if not hasattr(file, "fileno"):
        return False
    if _lumen_sys.platform == "win32":
        try:
            import nt
            if not nt._supports_virtual_terminal():
                return False
        except (ImportError, AttributeError):
            return False
    try:
        return _lumen_os.isatty(file.fileno())
    except OSError:
        return hasattr(file, "isatty") and file.isatty()

_lumen_set_color = HelpFormatter._set_color

def _lumen_set_color_plain(self, color):
    if color and _lumen_can_colorize():
        return _lumen_set_color(self, color)
    self._theme = _LumenNoColor()
    self._decolor = _identity

HelpFormatter._set_color = _lumen_set_color_plain

_lumen_help_init = HelpFormatter.__init__

def _lumen_help_columns():
    try:
        return int(_lumen_os.environ["COLUMNS"])
    except (KeyError, ValueError):
        return 0

def _lumen_help_init_width(self, prog, indent_increment=2, max_help_position=24, width=None, color=True):
    # The width is known without asking the terminal when stdout is not a
    # tty. Asking imports shutil, which this command line does not need.
    if width is None:
        columns = _lumen_help_columns()
        if columns <= 0:
            try:
                tty = _lumen_os.isatty(_lumen_sys.__stdout__.fileno())
            except Exception:
                tty = False
            if not tty:
                columns = 80
        if columns > 0:
            width = columns - 2
    return _lumen_help_init(self, prog, indent_increment, max_help_position, width, color)

HelpFormatter.__init__ = _lumen_help_init_width
