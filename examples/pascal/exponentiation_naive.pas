// Ported from examples/lumen/exponentiation_naive.lm by scripts/port_examples.py; edit the Lumen original, not this file.
var base: integer;
var exp: integer;
var mod_: integer;
var iterations: integer;
var result: integer;
var i: integer;
var j: integer;
begin
    base := 7;
    exp := 100;
    mod_ := 1000000007;
    iterations := 100;
    writeln('Naive exponentiation benchmark');
    write('base = ');
    writeln(base);
    write('exp  = ');
    writeln(exp);
    write('mod  = ');
    writeln(mod_);
    write('iterations = ');
    writeln(iterations);
    writeln('');
    writeln('Running naive exponentiation...');
    result := 0;
    i := 0;
    while i < iterations do begin
        result := 1;
        j := 0;
        while j < exp do begin
            result := result * base;
            j := j + 1;
        end;
        result := result mod mod_;
        i := i + 1;
    end;
    write('Result: ');
    writeln(result);
    writeln('Done!');
end.
