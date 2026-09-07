# The Lumen library file lib_lumen/modular_arithmetic.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

def mod_mult(a, b, m)
    return (a * b) % m
end

def mod_pow(base, exp, m)
    result = 1
    base = base % m
    while exp > 0 do
        if exp % 2 == 1 then
            result = (result * base) % m
        end
        exp = exp / 2
        base = (base * base) % m
    end
    return result
end
