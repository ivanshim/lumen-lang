from fractions import Fraction as F
print(F(1, 3) + F(1, 6), F("3/4"), F(0.5), F(7, 2).limit_denominator(1))
