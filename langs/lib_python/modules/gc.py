# Collection is a stub: values are kept by reference counts. Enabling
# and disabling records the requested state but cannot change reclamation.
_enabled = True

def collect(generation=2):
    return 0

def disable():
    global _enabled
    _enabled = False

def enable():
    global _enabled
    _enabled = True

def isenabled():
    return _enabled
