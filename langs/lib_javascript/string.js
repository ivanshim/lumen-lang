// The Lumen library file langs/lib_lumen/string.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

function char_at_or_null(s, index) {
    if (index < 0 || index >= s.length) {
        return null;
    }
    return s.charAt(index);
}

function substring(s, from_start, to_end) {
    let index = from_start;
    let out = "";
    while (index < to_end) {
        out = out + s.charAt(index);
        index = index + 1;
    }
    return out;
}

function substring_end(s, from_here) {
    return substring(s, from_here, s.length);
}

function substring_start(s, to_here) {
    return substring(s, 0, to_here);
}

function starts_with(s, prefix) {
    return prefix.length <= s.length && substring(s, 0, prefix.length) === prefix;
}

function ends_with(s, suffix) {
    return suffix.length <= s.length && substring(s, s.length - suffix.length, s.length) === suffix;
}

function repeat_string(s, repetitions) {
    let out = "";
    let i = 0;
    while (i < repetitions) {
        out = out + s;
        i = i + 1;
    }
    return out;
}

function join_strings(arr, separator) {
    let out = "";
    const n = arr.length;
    let i = 0;
    while (i < n) {
        if (i > 0) {
            out = out + separator;
        }
        out = out + arr[i];
        i = i + 1;
    }
    return out;
}

function index_of(s, needle) {
    const n = needle.length;
    let i = 0;
    while (i + n <= s.length) {
        if (substring(s, i, i + n) === needle) {
            return i;
        }
        i = i + 1;
    }
    return -1;
}

function has_substring(s, needle) {
    return index_of(s, needle) >= 0;
}
