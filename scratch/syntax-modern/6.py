def given():
    print("the body must not start")
    yield *[1, 2],
given()
