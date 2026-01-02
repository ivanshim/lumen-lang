scale := 100000;
pi := 3 * scale;
k := 1;
sign := 1;
iterations := 0;
max_iter := 1000;

while (iterations < max_iter) BEGIN
    denom := (2 * k) * (2 * k + 1) * (2 * k + 2);
    term := (4 * scale) / denom;
    pi := pi + sign * term;
    sign := -sign;
    k := k + 1;
    iterations := iterations + 1;
END;

writeln(pi);
