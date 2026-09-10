from fractions import Fraction as F
print(F(1, 3))
print(F(1, 3) + F(1, 6))
print(F('3/4'))
print(F(0.5))
print(F(7, 2).limit_denominator(1))
print(F(True, True))
print(F(0.1))
print(F(1, 2) + 0.5)
