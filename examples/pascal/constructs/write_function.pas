// Ported from examples/lumen/constructs/write_function.lm by scripts/port_examples.py; edit the Lumen original, not this file.
var i: integer;

begin
    write('Hello');
    write(' ');
    write('World');
    write('!');
    writeln('');
    write('Numbers: ');
    i := 1;
    while i <= 5 do begin
        write(i);
        if i < 5 then begin
            write(', ');
        end;
        i := i + 1;
    end;
    writeln('');
end.
