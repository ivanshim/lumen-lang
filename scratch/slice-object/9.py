import operator
print(operator.itemgetter(slice(1, 3))([0, 1, 2, 3]))
print(operator.getitem("abcd", slice(1, 3)))
class G:
    def __getitem__(self, key):
        return key
print(operator.itemgetter(slice(1, 3))(G()))
