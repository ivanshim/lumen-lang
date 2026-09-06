fn main() {
    println!("{}", 1 + 2 * 3);

    let x = 0;
    let y = 5;

    if x < y && y == 5 {
        println!("{}", 100);
    } else {
        println!("{}", 200);
    }

    let mut i = 0;
    let mut sum = 0;

    while i < 10 {
        if i == 5 {
            i = i + 1;
            continue;
        }

        if i == 8 {
            break;
        }

        sum = sum + i;
        println!("{}", sum);
        i = i + 1;
    }

    println!("{}", sum);
    println!("{}", true);
    println!("{}", false);
    println!("{}", !false);
    println!("{}", -10 + 3);
    println!("{}", 0x1F);
    println!("{} and {}", "one", 2);
}
