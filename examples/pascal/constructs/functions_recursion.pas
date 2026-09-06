// Ported from examples/lumen/constructs/functions_recursion.lm by scripts/port_examples.py; edit the Lumen original, not this file.
function factorial(n: integer): integer;
begin
    if n <= 1 then begin
        exit(1);
    end else begin
        exit(n * factorial(n - 1));
    end;
end;

function countdown(n: integer): integer;
begin
    if n <= 0 then begin
        writeln('Done');
    end else begin
        writeln(n);
        exit(countdown(n - 1));
    end;
end;

begin
    writeln('Test: Recursion');
    writeln('Factorial of 5:');
    writeln(factorial(5));
    writeln('Countdown from 3:');
    countdown(3);
end.
