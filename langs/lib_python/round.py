# The Lumen library file langs/lib_lumen/round.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

def round(x, decimals):
    scale = 1
    i = 0
    while i < decimals:
        scale = scale * 10
        i = i + 1
    y = x * scale
    if y >= 0:
        r = (y * 2 + 1) // 2
    else:
        r = (y * 2 - 1) // 2
    return r / scale
