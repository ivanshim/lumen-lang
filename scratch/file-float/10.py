def check(locale):
    if not locale.localeconv()["decimal_point"] == ",":
        return
print("read")
