SCALE = 10000000000

total = SCALE
term = SCALE
n = 1

while term > 0
    term = term / n
    total = total + term
    n = n + 1

int_part = total / SCALE
frac_part = total % SCALE

print(int_part)
print(frac_part)
