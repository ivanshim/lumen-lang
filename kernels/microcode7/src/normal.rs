// AS241 coefficients from CPython v3.14.8 Modules/_statisticsmodule.c; PSF License.
const TOP: [[f64; 8]; 3] = [
    [2.5090809287301226727e+3, 3.3430575583588128105e+4, 6.7265770927008700853e+4, 4.5921953931549871457e+4, 1.3731693765509461125e+4, 1.9715909503065514427e+3, 1.3314166789178437745e+2, 3.3871328727963666080e+0],
    [7.74545014278341407640e-4, 2.27238449892691845833e-2, 2.41780725177450611770e-1, 1.27045825245236838258e+0, 3.64784832476320460504e+0, 5.76949722146069140550e+0, 4.63033784615654529590e+0, 1.42343711074968357734e+0],
    [2.01033439929228813265e-7, 2.71155556874348757815e-5, 1.24266094738807843860e-3, 2.65321895265761230930e-2, 2.96560571828504891230e-1, 1.78482653991729133580e+0, 5.46378491116411436990e+0, 6.65790464350110377720e+0],
];
const BOTTOM: [[f64; 8]; 3] = [
    [5.2264952788528545610e+3, 2.8729085735721942674e+4, 3.9307895800092710610e+4, 2.1213794301586595867e+4, 5.3941960214247511077e+3, 6.8718700749205790830e+2, 4.2313330701600911252e+1, 1.0],
    [1.05075007164441684324e-9, 5.47593808499534494600e-4, 1.51986665636164571966e-2, 1.48103976427480074590e-1, 6.89767334985100004550e-1, 1.67638483018380384940e+0, 2.05319162663775882187e+0, 1.0],
    [2.04426310338993978564e-15, 1.42151175831644588870e-7, 1.84631831751005468180e-5, 7.86869131145613259100e-4, 1.48753612908506148525e-2, 1.36929880922735805310e-1, 5.99832206555887937690e-1, 1.0],
];
fn polynomial(coefficients: &[f64; 8], argument: f64) -> f64 {
    coefficients[1..].iter().fold(coefficients[0], |value, coefficient| value * argument + coefficient)
}
pub(crate) fn quantile(probability: f64, location: f64, spread: f64) -> Result<f64, String> {
    if probability <= 0.0 || probability >= 1.0 {
        return Err(String::from("ValueError: inv_cdf undefined for these parameters"));
    }
    let difference = probability - 0.5;
    let (region, point, multiplier) = if difference.abs() <= 0.425 {
        (0, 0.180625 - difference * difference, difference)
    } else {
        let tail = if difference <= 0.0 { probability } else { 1.0 - probability };
        if tail <= 0.0 || tail >= 1.0 { return Err(String::from("ValueError: inv_cdf undefined for these parameters")); }
        let distance = (-tail.ln()).sqrt();
        let sign = if difference < 0.0 { -1.0 } else { 1.0 };
        if distance <= 5.0 { (1, distance - 1.6, sign) } else { (2, distance - 5.0, sign) }
    };
    let divisor = polynomial(&BOTTOM[region], point);
    if divisor == 0.0 { return Err(String::from("ValueError: inv_cdf undefined for these parameters")); }
    let numerator = polynomial(&TOP[region], point);
    let standard = if region == 0 { numerator * multiplier / divisor } else { numerator / divisor * multiplier };
    Ok(location + standard * spread)
}
