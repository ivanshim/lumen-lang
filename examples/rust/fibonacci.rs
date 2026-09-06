fn fib(n: i64) -> i64 {
    let mut a = 0;
    let mut b = 1;
    for i in 0..n {
        let next = a + b;
        a = b;
        b = next;
    }
    return a;
}

fn main() {
    for i in 0..10 {
        println!("{}", fib(i));
    }
}
