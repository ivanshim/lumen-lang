function fib(n) {
    let a = 0;
    let b = 1;
    let i = 0;
    while (i < n) {
        const c = a + b;
        a = b;
        b = c;
        i = i + 1;
    }
    return a;
}

let i = 0;
while (i < 10) {
    console.log(fib(i));
    i = i + 1;
}
