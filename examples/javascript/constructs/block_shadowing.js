// A bare block at the top level binds names the program already has.
// The block's bindings last only inside it; after it, the outer ones are
// back. An assignment inside a block binds in the block too.
let n = 1;
let arr = [1, 2];
{
    let n = 20;
    let fresh = 5;
    console.log(n);
    console.log(fresh);
    arr.push(3);
}
console.log(n);
console.log(arr);
{
    n = 7;
    console.log(n);
}
console.log(n);
function inner() {
    let k = 1;
    {
        let k = 2;
        console.log(k);
    }
    {
        k = 3;
        console.log(k);
    }
    console.log(k);
}
inner();
