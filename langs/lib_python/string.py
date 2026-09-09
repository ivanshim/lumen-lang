# The Lumen library file langs/lib_lumen/string.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

def char_at_or_null(s, index):
    if index < 0 or index >= len(s):
        return None
    return s[index]

def substring(s, from_start, to_end):
    index = from_start
    out = ""
    while index < to_end:
        out = out + s[index]
        index = index + 1
    return out

def substring_end(s, from_here):
    return substring(s= s, from_start= from_here, to_end= len(s))

def substring_start(s, to_here):
    return substring(s= s, from_start= 0, to_end= to_here)

def starts_with(s, prefix):
    return len(prefix) <= len(s) and substring(s= s, from_start= 0, to_end= len(prefix)) == prefix

def ends_with(s, suffix):
    return len(suffix) <= len(s) and substring(s= s, from_start= len(s) - len(suffix), to_end= len(s)) == suffix

def repeat_string(s, repetitions):
    out = ""
    i = 0
    while i < repetitions:
        out = out + s
        i = i + 1
    return out

def join_strings(arr, separator):
    out = ""
    n = len(arr)
    i = 0
    while i < n:
        if i > 0:
            out = out + separator
        out = out + arr[i]
        i = i + 1
    return out

def index_of(s, needle):
    n = len(needle)
    i = 0
    while i + n <= len(s):
        if substring(s= s, from_start= i, to_end= i + n) == needle:
            return i
        i = i + 1
    return -1

def has_substring(s, needle):
    return index_of(s= s, needle= needle) >= 0
