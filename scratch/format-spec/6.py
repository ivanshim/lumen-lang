class Box:
    def __init__(self):
        self.value = 9
print("{1}|{name}|{0[value]}|{2.value}|{0[value]:{width}}".format({"value": 12}, "first", Box(), name="word", width=5))
print("{}|{}|{{ok}}|{!a}".format("a", "b", "é"))
print("{:{}x}|{:{}}".format(20, "#", "x", "^5"))
print("%(name)s|%(n)04d" % {"name": "word", "n": 7})
