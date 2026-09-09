# The Lumen library file langs/lib_lumen/to_string.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

def integer_to_base_string(n, radix):
    alphabet = "0123456789abcdefghijklmnopqrstuvwxyz"
    if n == 0:
        return "0"
    negative = False
    if n < 0:
        negative = True
        n = -n
    result = ""
    while n > 0:
        digit = n % radix
        result = alphabet[digit] + result
        n = n // radix
    if negative:
        result = "-" + result
    return str(radix) + "@" + result

def real_to_base_string(value, radix, precision):
    i = int(value)
    f = frac(value)
    int_part = integer_to_base_string(i, radix)
    if f == 0:
        return int_part
    frac_part = frac_to_base_string(f, radix, precision)
    return int_part + "." + frac_part

def frac_to_base_string(f, radix, limit):
    alphabet = "0123456789abcdefghijklmnopqrstuvwxyz"
    result = ""
    count = 0
    while count < limit:
        if f == float(0):
            return result
        f = f * float(radix)
        digit = int(f)
        result = result + alphabet[digit]
        f = frac(f)
        count = count + 1
    return result
