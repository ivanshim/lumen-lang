"""Select the stderr line used by a scratch .err record."""

TRACEBACK_HEADER = "Traceback (most recent call last):"


def measured_line(stderr):
    lines = stderr.splitlines()
    if not lines or lines[0] != TRACEBACK_HEADER:
        return lines[0] if lines else ""
    return next((line for line in reversed(lines) if line.strip()), "")
