def bound():
    print("bound")
    return 1

def given():
    print("value")
    return [9, 8]

a = [0, 1, 2, 3]
a[bound():3] = given()
print(a)
