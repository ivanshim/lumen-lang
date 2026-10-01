# The terminal colouring the reference keeps in its _colorize module.
# Nothing here holds a theme: no terminal stands behind this run's
# streams, and every question about colouring is answered with no, the
# decolouring of a text being the text itself. The reference's argparse
# and difflib lend their colouring through this name.
# Copyright (c) 2001 Python Software Foundation; All Rights Reserved.
# The PSF license is kept in tests/python/LICENSE.

import os


def can_colorize(*, file=None):
    # No theme machinery stands here, so nothing is colourized.
    return False


def decolor(text):
    # A text without colour in it is itself.
    return text


class _NoTheme:
    # A theme with nothing in it, asked for by name as the reference's
    # formatters ask for their own.
    argparse = {}
    difflib = {}


def get_theme(*, name=None, force_color=None):
    return _NoTheme()
