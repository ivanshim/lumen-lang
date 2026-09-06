// Ported from examples/lumen/constructs/until_loop.lm by scripts/port_examples.py; edit the Lumen original, not this file.
var i: integer;
var x: integer;

begin
    write('Until loop ascending (0-9): ');
    i := 0;
    while not (i >= 10) do begin
        write(i);
        if i < 9 then begin
            write(', ');
        end;
        i := i + 1;
    end;
    writeln('');
    write('Until loop descending (15-6): ');
    x := 15;
    while not (x <= 5) do begin
        write(x);
        if x > 6 then begin
            write(', ');
        end;
        x := x - 1;
    end;
    writeln('');
end.
