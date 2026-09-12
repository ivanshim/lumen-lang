print({(1, 2): "t", True: "b", 1.5: "f", None: "n"}[1], sorted({"b": 2, "a": 1}, key=lambda k: -{"b": 2, "a": 1}[k]), list(reversed({"a": 1, "b": 2})))
