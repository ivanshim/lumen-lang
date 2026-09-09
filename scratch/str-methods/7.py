d = {"a": 1}
e = d
print(d.setdefault("b", 2), d.setdefault("a", 8), d.get("c"))
d.update(c=3)
print(sorted(e.keys()), list(e.values()), list(e.items()))
f = d.copy()
d.clear()
print(len(d), f.pop("c"), f.get("a"), f.pop("none", 9))
print((0).bit_length(), (-255).bit_length(), (3.0).is_integer(), (2.5).as_integer_ratio())
