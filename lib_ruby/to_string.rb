# The Lumen library file lib_lumen/to_string.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

def integer_to_base_string(n, radix)
    alphabet = "0123456789abcdefghijklmnopqrstuvwxyz"
    if n == 0 then
        return "0"
    end
    negative = false
    if n < 0 then
        negative = true
        n = -n
    end
    result = ""
    while n > 0 do
        digit = n % radix
        result = alphabet[digit] + result
        n = n / radix
    end
    if negative then
        result = "-" + result
    end
    return radix.to_s + "@" + result
end

def real_to_base_string(value, radix, precision)
    i = value.to_i
    f = frac(value)
    int_part = integer_to_base_string(i, radix)
    if f == 0 then
        return int_part
    end
    frac_part = frac_to_base_string(f, radix, precision)
    return int_part + "." + frac_part
end

def frac_to_base_string(f, radix, limit)
    alphabet = "0123456789abcdefghijklmnopqrstuvwxyz"
    result = ""
    count = 0
    while count < limit do
        if f == 0.to_f then
            return result
        end
        f = f * radix.to_f
        digit = f.to_i
        result = result + alphabet[digit]
        f = frac(f)
        count = count + 1
    end
    return result
end
