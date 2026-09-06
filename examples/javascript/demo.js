console.log(1 + 2 * 3);

let x = 0;
const y = 5;

if (x < y && y === 5) {
    console.log(100);
} else if (x == y) {
    console.log(150);
} else {
    console.log(200);
}

let i = 0;
let sum = 0;

while (i < 10) {
    if (i === 5) {
        i = i + 1;
        continue;
    }

    if (i === 8) {
        break;
    }

    sum = sum + i;
    console.log(sum);
    i = i + 1;
}

console.log(sum);
console.log(true);
console.log(false);
console.log(!false);
console.log(-10 + 3);
console.log(0x1F);
console.log(2 ** 10);
/* A block comment. */
console.log("sum is " + sum);
console.log('single', "double");
