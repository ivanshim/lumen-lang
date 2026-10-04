# Finite binary64 density calculations use the same operation order natively.
# Protocols, domain errors, and exceptional values retain the pinned method.
NormalDist.pdf = __math('method', 'normal_pdf', NormalDist.pdf)

_integer_sqrt_of_frac_rto = __math('method', 'sqrt_frac_rto', _integer_sqrt_of_frac_rto)
