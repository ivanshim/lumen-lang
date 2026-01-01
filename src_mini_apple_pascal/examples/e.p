scale := 1000000;
sum := scale;
term := scale;
n := 1;
iterations := 0;
max_iter := 100;

while (iterations < max_iter) BEGIN
    term := term / n;
    sum := sum + term;
    n := n + 1;
    iterations := iterations + 1;
END;

writeln(sum);
