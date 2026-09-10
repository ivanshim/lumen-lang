class C:
    def __complex__(self):
        return 2j
print(complex(C()))
