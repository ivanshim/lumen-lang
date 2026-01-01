print(1 + 2 * 3);

let x = 0;
let y = 5;

if x < y && y == 5 {
    print(100);
} else {
    print(200);
}

let i = 0;
let sum = 0;

while i < 10 {
    if i == 5 {
        i = i + 1;
        continue;
    }

    if i == 8 {
        break;
    }

    sum = sum + i;
    print(sum);
    i = i + 1;
}

print(sum);
print(true);
print(false);
print(!false);
print(-10 + 3);
