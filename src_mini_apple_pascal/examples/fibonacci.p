a := 0;
b := 1;
count := 0;

while (count < 20) BEGIN
    writeln(a);
    next := a + b;
    a := b;
    b := next;
    count := count + 1
END
