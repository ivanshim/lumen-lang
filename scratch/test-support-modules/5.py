from fractions import Fraction as F
print(F(-2, -4), F('1.25e-1'), F.from_float(0.5))
print(F(3, 4) - F(1, 2), 2 * F(1, 3), 1 / F(2, 3), F(7, 3) // F(1, 2), F(7, 3) % F(1, 2))
print(-F(1, 2), int(F(-7, 3)), float(F(1, 2)), F(1, 2) + 0.5)
print(F(3, 4) < 1, F(1, 2) == 0.5, F(1, 3).__repr__())
pair = F(1, 3).as_integer_ratio()
print(pair[0], pair[1])
