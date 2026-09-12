class V:
    def __format__(self, spec):
        return format("v", spec)
    def __pow__(self, exponent, modulus):
        return (3 ** exponent) % modulus
v = V()
print(f"{v:>4}", format(v, ""))
class D(dict):
    def __missing__(self, key):
        return "default"
print(D()["absent"])
print(pow(v, 2, 7))
