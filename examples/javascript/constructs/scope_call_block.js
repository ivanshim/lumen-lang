// A function called from inside a bare block sees the program's bindings,
// not the block's, even when the block binds the same name.
let n = 1;
function show() {
    console.log(n);
}
{
    let n = 20;
    console.log(n);
    show();
}
show();
