import unicodedata
print(unicodedata.category("A"), unicodedata.name("é"))
try:
    import _testcapi
except ImportError:
    print("no _testcapi")
