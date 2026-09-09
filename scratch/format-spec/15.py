print(format(0xdeadbeef, "_x"), format(0xdeadbeef, "#_X"), format(2**80, "#x"))
print(format(123456.123456, "021_._f"), format(123456.123456, ".10_f"))
print("{[[]}|{[}]}|{[{}]}".format({"[": 1}, {"}": 2}, {"{}": 3}))
print("%u|%e|%E|%f|%F|%g|%G|%a" % (7, 1.25, 1.25, 1.25, 1.25, 1.25, 1.25, "é"))
