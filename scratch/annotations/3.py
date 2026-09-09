x = 7
x: NoSuchThing
print(x)
a: A | B = 8
b: Optional[Missing] = 9
c: "Forward" = 10
print(a, b, c)
d: {"kind": [NoSuchThing, (1 / 0)]} = 11
print(d)
