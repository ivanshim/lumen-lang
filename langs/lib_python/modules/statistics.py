import math

class StatisticsError(ValueError):
    pass

def _values(data):
    values = list(data)
    if len(values) == 0:
        raise 'StatisticsError: data must not be empty'
    return values

def mean(data):
    values = _values(data)
    total = sum(values)
    if type(total) == type(1) and total % len(values) == 0:
        return total // len(values)
    return total / len(values)

def fmean(data, weights=None):
    values = _values(data)
    if weights is None:
        return __math('fdiv', math.fsum(values), len(values))
    weights = list(weights)
    if len(weights) != len(values):
        raise 'StatisticsError: data and weights must be the same length'
    total = math.fsum(weights)
    if total == 0:
        raise 'StatisticsError: sum of weights must be non-zero'
    return __math('fdiv', math.fsum([values[i] * weights[i] for i in range(len(values))]), total)

def median(data):
    values = sorted(_values(data))
    middle = len(values) // 2
    if len(values) % 2:
        return values[middle]
    return (values[middle - 1] + values[middle]) / 2

def median_low(data):
    values = sorted(_values(data))
    return values[(len(values) - 1) // 2]

def median_high(data):
    values = sorted(_values(data))
    return values[len(values) // 2]

def mode(data):
    values = _values(data)
    best = values[0]
    most = 0
    for value in values:
        count = sum([1 for item in values if item == value])
        if count > most:
            best, most = value, count
    return best

def variance(data, xbar=None):
    values = _values(data)
    if len(values) < 2:
        raise 'StatisticsError: variance requires at least two data points'
    if xbar is None:
        xbar = mean(values)
    return sum([(x - xbar) ** 2 for x in values]) / (len(values) - 1)

def pvariance(data, mu=None):
    values = _values(data)
    if mu is None:
        mu = mean(values)
    return sum([(x - mu) ** 2 for x in values]) / len(values)

def stdev(data, xbar=None):
    return math.sqrt(variance(data, xbar))

def pstdev(data, mu=None):
    return math.sqrt(pvariance(data, mu))
