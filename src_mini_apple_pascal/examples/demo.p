writeln(1 + 2 * 3);

x := 0;
y := 5;

if (x < y and y == 5) BEGIN
    writeln(100)
END else BEGIN
    writeln(200)
END;

i := 0;
sum := 0;

while (i < 10) BEGIN
    if (i == 5) BEGIN
        i := i + 1;
        continue
    END;

    if (i == 8) BEGIN
        break
    END;

    sum := sum + i;
    writeln(sum);
    i := i + 1
END;

writeln(sum);
writeln(true);
writeln(false);
writeln(not false);
writeln(-10 + 3)
