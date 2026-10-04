"""Native accelerators for the pinned statistics module."""
def _normal_dist_inv_cdf(p, mu, sigma, /):
    return __math('normal_dist_inv_cdf', p, mu, sigma)
