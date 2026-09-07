// The Lumen library file langs/lib_lumen/primes.lm, ported by scripts/port_examples.py; edit the Lumen original, not this file.

procedure next_prime(n: integer);
var k: integer;
begin
    k := n + 1;
    while true do begin
        if is_prime(k) then begin
            k;
        end;
        k := k + 1;
    end;
end;
