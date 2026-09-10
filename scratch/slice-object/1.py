class G:
    def __getitem__(self, key):
        print(key)
g = G()
g[1:2]
g[1:2, 3]
g[...]
g[1, 2]
g[()]
g["x":"y"]
