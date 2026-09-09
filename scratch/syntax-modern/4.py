a = [1, 2]
b = [3, 4]
print(*a, *b)
x = 1
global x
def change():
    global x
    x = 9
change()
print(x)
type(3)
