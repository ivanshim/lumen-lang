def stop():
    print("bound")
    return 3
for item in range(stop()):
    print(item)
print("last", item)
item = 9
for item in range(0):
    print("must not run")
print("kept", item)
for item in range(-2,):
    print("must not run")
print("kept", item)
for item in range(3,):
    if item == 1:
        continue
    print("item", item)
print("last", item)
