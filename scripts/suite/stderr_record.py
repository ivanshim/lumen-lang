"""Select the stderr line used by a scratch .err record."""

TRACEBACK_HEADER = "Traceback (most recent call last):"


def measured_line(stderr):
    lines = stderr.splitlines()
    if not lines or lines[0] != TRACEBACK_HEADER:
        return lines[0] if lines else ""
    return next((line for line in reversed(lines) if line.strip()), "")


if __name__ == "__main__":
    # `python3 scripts/suite/stderr_record.py < stderr-file` prints the measured line (CI's scratch job uses this).
    import sys
    text = sys.stdin.buffer.read().decode("utf-8", "surrogateescape")
    sys.stdout.buffer.write((measured_line(text) + "\n").encode("utf-8", "surrogateescape"))
