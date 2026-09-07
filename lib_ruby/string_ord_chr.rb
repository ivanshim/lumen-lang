# The Lumen library file lib_lumen/string_ord_chr.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

def is_ascii(c)
    return c.ord < 128
end

def is_digit(c)
    o = c.ord
    return o >= "0".ord && o <= "9".ord
end

def is_alpha(c)
    o = c.ord
    return (o >= "A".ord && o <= "Z".ord) || (o >= "a".ord && o <= "z".ord)
end

def is_alnum(c)
    return is_alpha(c) || is_digit(c)
end

def char_to_upper(c)
    o = c.ord
    if o >= "a".ord && o <= "z".ord then
        return (o - 32).chr
    else
        return c
    end
end

def char_to_lower(c)
    o = c.ord
    if o >= "A".ord && o <= "Z".ord then
        return (o + 32).chr
    else
        return c
    end
end

def string_to_upper(s)
    result = ""
    i = 0
    while i < s.length do
        result = result + char_to_upper(s[i])
        i = i + 1
    end
    return result
end

def string_to_lower(s)
    result = ""
    i = 0
    while i < s.length do
        result = result + char_to_lower(s[i])
        i = i + 1
    end
    return result
end

def reverse_characters(s)
    result = ""
    index = s.length - 1
    while index >= 0 do
        result = result + s[index]
        index = index - 1
    end
    return result
end

def capitalize_first_word(s)
    result = ""
    i = 0
    done = false
    while i < s.length do
        c = s[i]
        if !done && is_alpha(c) then
            result = result + char_to_upper(c)
            done = true
        else
            result = result + c
        end
        i = i + 1
    end
    return result
end

def capitalize_words(s)
    result = ""
    i = 0
    at_word_start = true
    while i < s.length do
        c = s[i]
        if is_alpha(c) then
            if at_word_start then
                result = result + char_to_upper(c)
                at_word_start = false
            else
                result = result + c
            end
        else
            result = result + c
            at_word_start = true
        end
        i = i + 1
    end
    return result
end

def is_whitespace(c)
    o = c.ord
    return o == 32 || o == 9 || o == 10 || o == 13
end

def trim_start(s)
    i = 0
    while i < s.length && is_whitespace(s[i]) do
        i = i + 1
    end
    return substring_end(s, i)
end

def trim_end(s)
    i = s.length - 1
    while i >= 0 && is_whitespace(s[i]) do
        i = i - 1
    end
    return substring(s, 0, i + 1)
end

def trim(s)
    return trim_start(trim_end(s))
end

def is_alpha_string(s)
    if s.length == 0 then
        return false
    end
    i = 0
    while i < s.length do
        if !is_alpha(s[i]) then
            return false
        end
        i = i + 1
    end
    return true
end
