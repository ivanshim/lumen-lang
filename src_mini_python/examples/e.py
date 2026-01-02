e = 1
i = 1

while i < 10
    factorial = 1
    j = 1
    while j <= i
        factorial = factorial * j
        j = j + 1

    e = e + 1 / factorial
    i = i + 1

print(e)
