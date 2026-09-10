def f():
    for lhs_stmt, rhs_stmt in [("pass", "pass")]:
        print(lhs_stmt, rhs_stmt)
    return ()
print("tuple body read")
