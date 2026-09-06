// Two while loops; the second writes on one line.
let x = 0;
while (x < 5) {
    console.log(x);
    x = x + 1;
}

let i = 5;
while (i < 10) {
    process.stdout.write(i + " ");
    i = i + 1;
}
process.stdout.write("\n");
