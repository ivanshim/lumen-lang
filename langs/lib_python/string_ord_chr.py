# The Lumen library file langs/lib_lumen/string_ord_chr.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

def is_ascii(c):
    return ord(c) < 128

def is_digit(c):
    o = ord(c)
    return o >= ord("0") and o <= ord("9")

def is_alpha(c):
    o = ord(c)
    return (o >= ord("A") and o <= ord("Z")) or (o >= ord("a") and o <= ord("z"))

def is_alnum(c):
    return is_alpha(c) or is_digit(c)

def char_to_upper(c):
    o = ord(c)
    if o >= ord("a") and o <= ord("z"):
        return chr(o - 32)
    else:
        return c

def char_to_lower(c):
    o = ord(c)
    if o >= ord("A") and o <= ord("Z"):
        return chr(o + 32)
    else:
        return c

def string_to_upper(s):
    result = ""
    i = 0
    while i < len(s):
        result = result + char_to_upper(s[i])
        i = i + 1
    return result

def string_to_lower(s):
    result = ""
    i = 0
    while i < len(s):
        result = result + char_to_lower(s[i])
        i = i + 1
    return result

def reverse_characters(s):
    result = ""
    index = len(s) - 1
    while index >= 0:
        result = result + s[index]
        index = index - 1
    return result

def capitalize_first_word(s):
    result = ""
    i = 0
    done = False
    while i < len(s):
        c = s[i]
        if not done and is_alpha(c):
            result = result + char_to_upper(c)
            done = True
        else:
            result = result + c
        i = i + 1
    return result

def capitalize_words(s):
    result = ""
    i = 0
    at_word_start = True
    while i < len(s):
        c = s[i]
        if is_alpha(c):
            if at_word_start:
                result = result + char_to_upper(c)
                at_word_start = False
            else:
                result = result + c
        else:
            result = result + c
            at_word_start = True
        i = i + 1
    return result

def is_whitespace(c):
    o = ord(c)
    return o == 32 or o == 9 or o == 10 or o == 13

def trim_start(s):
    i = 0
    while i < len(s) and is_whitespace(s[i]):
        i = i + 1
    return substring_end(s, i)

def trim_end(s):
    i = len(s) - 1
    while i >= 0 and is_whitespace(s[i]):
        i = i - 1
    return substring(s, 0, i + 1)

def trim(s):
    return trim_start(trim_end(s))

def is_alpha_string(s):
    if len(s) == 0:
        return False
    i = 0
    while i < len(s):
        if not is_alpha(s[i]):
            return False
        i = i + 1
    return True
