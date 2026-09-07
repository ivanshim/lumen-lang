// Ported from examples/lumen/exponentiation_exponent_then_mod.lm by scripts/port_examples.py; edit the Lumen original, not this file.
var base: integer;
var exp: integer;
var mod_: integer;
var iterations: integer;
var result: integer;
var i: integer;

begin
    base := 7;
    exp := 100;
    mod_ := 1000000007;
    iterations := 100;
    writeln('Fast modular exponentiation benchmark');
    write('base = ');
    writeln(base);
    write('exp  = ');
    writeln(exp);
    write('mod  = ');
    writeln(mod_);
    write('iterations = ');
    writeln(iterations);
    writeln('');
    writeln('Running mod_pow...');
    result := 0;
    i := 0;
    while i < iterations do begin
        result := mod_pow(base, exp, mod_);
        i := i + 1;
    end;
    write('Result: ');
    writeln(result);
    writeln('Done!');
end.
