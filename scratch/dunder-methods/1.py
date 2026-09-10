class V:
    def __init__(self, x):
        self.x = x
    def __repr__(self):
        return f"V({self.x})"
    def __eq__(self, other):
        return self.x == other.x
    def __lt__(self, other):
        return self.x < other.x
    def __len__(self):
        return self.x
    def __getitem__(self, key):
        return [10, 20, 30][key]
    def __contains__(self, item):
        return item == self.x
    def __iter__(self):
        return iter([10, 20, 30])
print(V(1) == V(1), V(3) < V(1))
print(sorted([V(3), V(1)]))
print(len(V(3)), V(3)[1], 3 in V(3))
print(list(V(3)))
