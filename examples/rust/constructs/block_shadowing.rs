// A bare block binds a name that an outer binding already has. The
// block's binding lives only inside the block; after it, the outer
// binding is back, and can still be read, written and grown.
fn main() {
    let mut v = [1, 2, 3];
    let mut n = 1;
    {
        let v = 10;
        let n = 20;
        println!("{} {}", v, n);
    }
    println!("{} {}", v, n);
    v.push(4);
    v[0] = 9;
    n = n + 1;
    println!("{} {} {}", v, v[0], n);
    {
        let n = 30;
        {
            let n = 40;
            println!("{}", n);
        }
        println!("{}", n);
    }
    println!("{}", n);
}

fn outer_seen_from_function() {
    let total = 0;
    {
        let total = 5;
        println!("{}", total);
    }
    println!("{}", total);
}
