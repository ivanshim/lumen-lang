# The Lumen library file langs/lib_lumen/array.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

def array_concat(a, b)
    out = []
    i = 0
    while i < a.length do
        out.push(a[i])
        i = i + 1
    end
    i = 0
    while i < b.length do
        out.push(b[i])
        i = i + 1
    end
    return out
end

def array_slice(a, start, stop)
    out = []
    i = start
    while i < stop do
        out.push(a[i])
        i = i + 1
    end
    return out
end

def array_index_of(a, x)
    i = 0
    while i < a.length do
        if a[i] == x then
            return i
        end
        i = i + 1
    end
    return -1
end

def array_contains(a, x)
    return array_index_of(a, x) >= 0
end

def array_reverse(a)
    out = []
    i = a.length
    while i > 0 do
        i = i - 1
        out.push(a[i])
    end
    return out
end
