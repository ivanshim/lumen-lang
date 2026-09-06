func fib(n: Int) -> Int {
    var a = 0
    var b = 1
    for i in 0..<n {
        let c = a + b
        a = b
        b = c
    }
    return a
}

for i in 0..<10 {
    print(fib(n: i))
}
