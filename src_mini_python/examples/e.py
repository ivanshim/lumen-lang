SCALE = 10000000000

total = SCALE
term = SCALE
n = 1

while term > 0
    term = term / n
    total = total + term
    n = n + 1

print(total)
