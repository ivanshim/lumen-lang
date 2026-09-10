def members(self):
    self.x = 1
    self.x[0] = 2
    return self.method().x
values = []
values.append(3)
print(values[0])
