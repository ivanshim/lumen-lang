# The Lumen library file langs/lib_lumen/string.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

def char_at_or_null(s, index)
    if index < 0 || index >= s.length then
        return nil
    end
    return s[index]
end

def substring(s, from_start, to_end)
    index = from_start
    out = ""
    while index < to_end do
        out = out + s[index]
        index = index + 1
    end
    return out
end

def substring_end(s, from_here)
    return substring(s, from_here, s.length)
end

def substring_start(s, to_here)
    return substring(s, 0, to_here)
end

def starts_with(s, prefix)
    return prefix.length <= s.length && substring(s, 0, prefix.length) == prefix
end

def ends_with(s, suffix)
    return suffix.length <= s.length && substring(s, s.length - suffix.length, s.length) == suffix
end

def repeat_string(s, repetitions)
    out = ""
    i = 0
    while i < repetitions do
        out = out + s
        i = i + 1
    end
    return out
end

def join_strings(arr, separator)
    out = ""
    n = arr.length
    i = 0
    while i < n do
        if i > 0 then
            out = out + separator
        end
        out = out + arr[i]
        i = i + 1
    end
    return out
end

def index_of(s, needle)
    n = needle.length
    i = 0
    while i + n <= s.length do
        if substring(s, i, i + n) == needle then
            return i
        end
        i = i + 1
    end
    return -1
end

def has_substring(s, needle)
    return index_of(s, needle) >= 0
end
