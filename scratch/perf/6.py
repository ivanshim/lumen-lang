class Box:
    def __init__(self, value):
        self.value = value
box = Box(7)
result = 0
for i in range(10**5):
    result = result + box.value
print(result)
