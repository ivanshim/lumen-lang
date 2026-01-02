SCALE = 10000000000

x = SCALE / 5
x2 = (x * x) / SCALE

term = x
sum1 = term
k = 1

while term > 0
    term = (term * x2) / SCALE
    k = k + 2

    if (k / 2) * 2 == k
        sum1 = sum1 - (term / k)
    else
        sum1 = sum1 + (term / k)

x = SCALE / 239
x2 = (x * x) / SCALE

term = x
sum2 = term
k = 1

while term > 0
    term = (term * x2) / SCALE
    k = k + 2

    if (k / 2) * 2 == k
        sum2 = sum2 - (term / k)
    else
        sum2 = sum2 + (term / k)

pi_scaled = (16 * sum1) - (4 * sum2)

print(pi_scaled)
