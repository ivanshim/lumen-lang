pi = 3
k = 1

while k < 50
    num = 4
    den = (2 * k) * (2 * k + 1) * (2 * k + 2)

    if k % 2 == 1
        pi = pi + (num / den)
    else
        pi = pi - (num / den)

    k = k + 1

print(pi)
