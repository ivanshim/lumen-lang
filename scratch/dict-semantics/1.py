d = dict(zip("xy", [1, 2])); d.setdefault("z", 0); print(d.popitem(), d.pop("x"), d.pop("q", "none"), list(d.items()), dict.fromkeys("ab", 0))
