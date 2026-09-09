i = 0
while i < 3:
    i = i + 1
    try:
        continue
    finally:
        print(i)
print("continued")
while True:
    try:
        raise "discarded"
    finally:
        break
print("broken")
try:
    while True:
        break
except:
    print("wrong")
else:
    print("loop-ended")
def f():
    try:
        return 4
    except:
        print("wrong")
    else:
        print("wrong")
print(f())
