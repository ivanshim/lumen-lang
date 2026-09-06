// Digits of e by integer arithmetic; `/` is integer division.
fn main() {
    let scale = 10000000000;
    let mut sum = scale;
    let mut term = scale;
    let mut n = 1;

    while term > 0 {
        term = term / n;
        sum = sum + term;
        n = n + 1;
    }

    let int_part = sum / scale;
    let frac_part = sum % scale;

    println!("{}", int_part);
    println!("{}", frac_part);
}
