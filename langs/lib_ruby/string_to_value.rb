# The Lumen library file langs/lib_lumen/string_to_value.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

def character_to_value(c)
    if is_digit(c) then
        return c.ord - "0".ord
    end
    code = c.ord
    if code >= "A".ord && code <= "Z".ord then
        return code - "A".ord + 10
    end
    if code >= "a".ord && code <= "z".ord then
        return code - "a".ord + 10
    end
    return -1
end

def digits_to_base_value(s, i, base)
    start = i
    value = 0
    scale = 1
    while i < s.length do
        c = s[i]
        d = character_to_value(c)
        if d < 0 || d >= base then
            break
        end
        value = value * base + d
        scale = scale * base
        i = i + 1
    end
    if i == start then
        raise("expected digit")
    end
    return [value, scale, i]
end
