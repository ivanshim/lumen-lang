class P:
    def __init__(self):
        self.x = 7
        self.x: Missing
        self.absent: Missing[Unknown]
        self.append: Missing
p = P()
(p.x): Missing = 8
p.x: Missing
print(p.x)
def subject():
    print("subject")
    return p
subject().absent: Missing
print("done")
