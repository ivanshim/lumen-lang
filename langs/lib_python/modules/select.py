# A partial select module: it carries the fault the module raises, which
# is the operating system's own, and nothing that waits on descriptors.
# What a program asks of a real select is absent, so hasattr says no
# rather than a wrong answer.
error = OSError
