pi = 0
i = 0

while i < 1000
    term = 1
    if i > 0
        j = 0
        sign = -1
        while j < i
            term = term * (-1) / (2 * j + 1)
            j = j + 1

    pi = pi + term
    i = i + 1

print(pi * 4)
