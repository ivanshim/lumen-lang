# Ported from examples/lumen/constructs/array_library.lm by scripts/port_examples.py; edit the Lumen original, not this file.
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

a = [1, 2, 3]
b = [4, 5]
both = array_concat(a, b)
puts(both)
puts(array_slice(both, 1, 4))
puts(array_index_of(b, 5))
puts(array_index_of(b, 9))
puts(array_contains(a, 2))
puts(array_contains(a, 7))
puts(array_reverse(both))
puts(array_reverse([]).length)
