use num_complex::Complex;

use crate::compiler::ir::BuiltinFn;
use crate::eval::numeric::NumericResult;

/// Dispatch a built-in function call on the given arguments.
pub(crate) fn dispatch(kind: BuiltinFn, args: &[NumericResult]) -> NumericResult {
    match kind {
        // Single-argument trig
        BuiltinFn::Sin => unary(args, f64::sin, Complex::sin),
        BuiltinFn::Cos => unary(args, f64::cos, Complex::cos),
        BuiltinFn::Tan => unary(args, f64::tan, Complex::tan),
        BuiltinFn::Asin => unary_promote(args, f64::asin, Complex::asin),
        BuiltinFn::Acos => unary_promote(args, f64::acos, Complex::acos),
        BuiltinFn::Atan => unary(args, f64::atan, Complex::atan),

        // Hyperbolic
        BuiltinFn::Sinh => unary(args, f64::sinh, Complex::sinh),
        BuiltinFn::Cosh => unary(args, f64::cosh, Complex::cosh),
        BuiltinFn::Tanh => unary(args, f64::tanh, Complex::tanh),

        // Exponential / logarithmic
        BuiltinFn::Exp => unary(args, f64::exp, Complex::exp),
        BuiltinFn::Ln => unary_promote(args, f64::ln, Complex::ln),
        BuiltinFn::Log2 => unary_promote(args, f64::log2, |c| c.ln() / 2.0_f64.ln()),
        BuiltinFn::Log10 => unary_promote(args, f64::log10, |c| c.ln() / 10.0_f64.ln()),
        BuiltinFn::Log => binary(
            args,
            |base, val| val.log(base),
            |base, val| val.ln() / base.ln(),
        ),

        // Power / root
        BuiltinFn::Sqrt => {
            let a = args[0];
            a.sqrt()
        }
        BuiltinFn::Cbrt => unary(args, f64::cbrt, |c| c.powf(1.0 / 3.0)),
        BuiltinFn::Abs => {
            let a = args[0];
            match a {
                NumericResult::Real(r) => NumericResult::Real(r.abs()),
                NumericResult::Complex(c) => NumericResult::Real(c.norm()),
            }
        }

        // Rounding
        BuiltinFn::Floor => unary_real_only(args, f64::floor),
        BuiltinFn::Ceil => unary_real_only(args, f64::ceil),
        BuiltinFn::Round => unary_real_only(args, f64::round),

        // Binary
        BuiltinFn::Atan2 => binary_real_only(args, f64::atan2),
        BuiltinFn::Min => binary_real_only(args, f64::min),
        BuiltinFn::Max => binary_real_only(args, f64::max),
    }
}

/// Expected argument count for each built-in function.
pub(crate) fn arity(kind: BuiltinFn) -> usize {
    match kind {
        BuiltinFn::Sin
        | BuiltinFn::Cos
        | BuiltinFn::Tan
        | BuiltinFn::Asin
        | BuiltinFn::Acos
        | BuiltinFn::Atan
        | BuiltinFn::Sinh
        | BuiltinFn::Cosh
        | BuiltinFn::Tanh
        | BuiltinFn::Exp
        | BuiltinFn::Ln
        | BuiltinFn::Log2
        | BuiltinFn::Log10
        | BuiltinFn::Sqrt
        | BuiltinFn::Cbrt
        | BuiltinFn::Abs
        | BuiltinFn::Floor
        | BuiltinFn::Ceil
        | BuiltinFn::Round => 1,

        BuiltinFn::Atan2 | BuiltinFn::Log | BuiltinFn::Min | BuiltinFn::Max => 2,
    }
}

/// Resolve a function name string to a BuiltinFn enum, if known.
pub(crate) fn resolve(name: &str) -> Option<BuiltinFn> {
    match name {
        "sin" => Some(BuiltinFn::Sin),
        "cos" => Some(BuiltinFn::Cos),
        "tan" => Some(BuiltinFn::Tan),
        "asin" | "arcsin" => Some(BuiltinFn::Asin),
        "acos" | "arccos" => Some(BuiltinFn::Acos),
        "atan" | "arctan" => Some(BuiltinFn::Atan),
        "atan2" => Some(BuiltinFn::Atan2),
        "sinh" => Some(BuiltinFn::Sinh),
        "cosh" => Some(BuiltinFn::Cosh),
        "tanh" => Some(BuiltinFn::Tanh),
        "exp" => Some(BuiltinFn::Exp),
        "ln" => Some(BuiltinFn::Ln),
        "log2" => Some(BuiltinFn::Log2),
        "log10" => Some(BuiltinFn::Log10),
        "log" => Some(BuiltinFn::Log),
        "sqrt" => Some(BuiltinFn::Sqrt),
        "cbrt" => Some(BuiltinFn::Cbrt),
        "abs" => Some(BuiltinFn::Abs),
        "floor" => Some(BuiltinFn::Floor),
        "ceil" => Some(BuiltinFn::Ceil),
        "round" => Some(BuiltinFn::Round),
        "min" => Some(BuiltinFn::Min),
        "max" => Some(BuiltinFn::Max),
        _ => None,
    }
}

fn unary(
    args: &[NumericResult],
    real_fn: fn(f64) -> f64,
    complex_fn: fn(Complex<f64>) -> Complex<f64>,
) -> NumericResult {
    match args[0] {
        NumericResult::Real(r) => NumericResult::Real(real_fn(r)),
        NumericResult::Complex(c) => NumericResult::Complex(complex_fn(c)).simplify(),
    }
}

/// For functions where real input might produce complex output (e.g., asin(2), ln(-1)).
fn unary_promote(
    args: &[NumericResult],
    real_fn: fn(f64) -> f64,
    complex_fn: fn(Complex<f64>) -> Complex<f64>,
) -> NumericResult {
    match args[0] {
        NumericResult::Real(r) => {
            let result = real_fn(r);
            if result.is_nan() {
                // Try complex path
                let c = complex_fn(Complex::new(r, 0.0));
                NumericResult::Complex(c).simplify()
            } else {
                NumericResult::Real(result)
            }
        }
        NumericResult::Complex(c) => NumericResult::Complex(complex_fn(c)).simplify(),
    }
}

fn unary_real_only(args: &[NumericResult], f: fn(f64) -> f64) -> NumericResult {
    match args[0] {
        NumericResult::Real(r) => NumericResult::Real(f(r)),
        NumericResult::Complex(c) => {
            // Apply to real part only for rounding functions on complex
            NumericResult::Real(f(c.re))
        }
    }
}

fn binary(
    args: &[NumericResult],
    real_fn: fn(f64, f64) -> f64,
    complex_fn: fn(Complex<f64>, Complex<f64>) -> Complex<f64>,
) -> NumericResult {
    match (args[0], args[1]) {
        (NumericResult::Real(a), NumericResult::Real(b)) => NumericResult::Real(real_fn(a, b)),
        (a, b) => NumericResult::Complex(complex_fn(a.to_complex(), b.to_complex())).simplify(),
    }
}

fn binary_real_only(args: &[NumericResult], f: fn(f64, f64) -> f64) -> NumericResult {
    match (args[0], args[1]) {
        (NumericResult::Real(a), NumericResult::Real(b)) => NumericResult::Real(f(a, b)),
        (a, b) => NumericResult::Real(f(a.to_complex().re, b.to_complex().re)),
    }
}

trait Simplify {
    fn simplify(self) -> NumericResult;
}

impl Simplify for NumericResult {
    fn simplify(self) -> NumericResult {
        if let NumericResult::Complex(c) = self {
            if c.im.abs() < 1e-15 {
                return NumericResult::Real(c.re);
            }
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;
    use std::f64::consts::{E, FRAC_PI_2, PI};

    fn real(v: f64) -> NumericResult {
        NumericResult::Real(v)
    }

    fn complex(re: f64, im: f64) -> NumericResult {
        NumericResult::Complex(Complex::new(re, im))
    }

    #[test]
    fn sin_pi_half() {
        let r = dispatch(BuiltinFn::Sin, &[real(FRAC_PI_2)]);
        assert_abs_diff_eq!(r.to_f64().unwrap(), 1.0, epsilon = 1e-15);
    }

    #[test]
    fn cos_zero() {
        let r = dispatch(BuiltinFn::Cos, &[real(0.0)]);
        assert_abs_diff_eq!(r.to_f64().unwrap(), 1.0, epsilon = 1e-15);
    }

    #[test]
    fn tan_zero() {
        let r = dispatch(BuiltinFn::Tan, &[real(0.0)]);
        assert_abs_diff_eq!(r.to_f64().unwrap(), 0.0, epsilon = 1e-15);
    }

    #[test]
    fn exp_zero() {
        let r = dispatch(BuiltinFn::Exp, &[real(0.0)]);
        assert_abs_diff_eq!(r.to_f64().unwrap(), 1.0, epsilon = 1e-15);
    }

    #[test]
    fn exp_one() {
        let r = dispatch(BuiltinFn::Exp, &[real(1.0)]);
        assert_abs_diff_eq!(r.to_f64().unwrap(), E, epsilon = 1e-15);
    }

    #[test]
    fn ln_e() {
        let r = dispatch(BuiltinFn::Ln, &[real(E)]);
        assert_abs_diff_eq!(r.to_f64().unwrap(), 1.0, epsilon = 1e-15);
    }

    #[test]
    fn ln_negative_promotes_complex() {
        let r = dispatch(BuiltinFn::Ln, &[real(-1.0)]);
        assert!(r.is_complex());
        let c = r.to_complex();
        assert_abs_diff_eq!(c.re, 0.0, epsilon = 1e-15);
        assert_abs_diff_eq!(c.im, PI, epsilon = 1e-15);
    }

    #[test]
    fn sqrt_four() {
        let r = dispatch(BuiltinFn::Sqrt, &[real(4.0)]);
        assert_abs_diff_eq!(r.to_f64().unwrap(), 2.0, epsilon = 1e-15);
    }

    #[test]
    fn sqrt_negative_promotes() {
        let r = dispatch(BuiltinFn::Sqrt, &[real(-4.0)]);
        assert!(r.is_complex());
    }

    #[test]
    fn abs_negative() {
        let r = dispatch(BuiltinFn::Abs, &[real(-5.0)]);
        assert_abs_diff_eq!(r.to_f64().unwrap(), 5.0, epsilon = 1e-15);
    }

    #[test]
    fn abs_complex() {
        let r = dispatch(BuiltinFn::Abs, &[complex(3.0, 4.0)]);
        assert_abs_diff_eq!(r.to_f64().unwrap(), 5.0, epsilon = 1e-15);
    }

    #[test]
    fn floor_positive() {
        let r = dispatch(BuiltinFn::Floor, &[real(3.7)]);
        assert_abs_diff_eq!(r.to_f64().unwrap(), 3.0, epsilon = 1e-15);
    }

    #[test]
    fn ceil_positive() {
        let r = dispatch(BuiltinFn::Ceil, &[real(3.2)]);
        assert_abs_diff_eq!(r.to_f64().unwrap(), 4.0, epsilon = 1e-15);
    }

    #[test]
    fn round_half() {
        let r = dispatch(BuiltinFn::Round, &[real(2.5)]);
        // Rust rounds half to even: 2.5 -> 2.0 (banker's rounding? no, Rust uses round-half-away-from-zero actually)
        // f64::round(2.5) = 3.0
        assert_abs_diff_eq!(r.to_f64().unwrap(), 3.0, epsilon = 1e-15);
    }

    #[test]
    fn atan2_basic() {
        let r = dispatch(BuiltinFn::Atan2, &[real(1.0), real(1.0)]);
        assert_abs_diff_eq!(r.to_f64().unwrap(), PI / 4.0, epsilon = 1e-15);
    }

    #[test]
    fn min_two() {
        let r = dispatch(BuiltinFn::Min, &[real(3.0), real(5.0)]);
        assert_abs_diff_eq!(r.to_f64().unwrap(), 3.0, epsilon = 1e-15);
    }

    #[test]
    fn max_two() {
        let r = dispatch(BuiltinFn::Max, &[real(3.0), real(5.0)]);
        assert_abs_diff_eq!(r.to_f64().unwrap(), 5.0, epsilon = 1e-15);
    }

    #[test]
    fn log_base_10() {
        let r = dispatch(BuiltinFn::Log, &[real(10.0), real(100.0)]);
        assert_abs_diff_eq!(r.to_f64().unwrap(), 2.0, epsilon = 1e-12);
    }

    #[test]
    fn log2_eight() {
        let r = dispatch(BuiltinFn::Log2, &[real(8.0)]);
        assert_abs_diff_eq!(r.to_f64().unwrap(), 3.0, epsilon = 1e-15);
    }

    #[test]
    fn log10_thousand() {
        let r = dispatch(BuiltinFn::Log10, &[real(1000.0)]);
        assert_abs_diff_eq!(r.to_f64().unwrap(), 3.0, epsilon = 1e-15);
    }

    #[test]
    fn cbrt_eight() {
        let r = dispatch(BuiltinFn::Cbrt, &[real(8.0)]);
        assert_abs_diff_eq!(r.to_f64().unwrap(), 2.0, epsilon = 1e-15);
    }

    #[test]
    fn sinh_zero() {
        let r = dispatch(BuiltinFn::Sinh, &[real(0.0)]);
        assert_abs_diff_eq!(r.to_f64().unwrap(), 0.0, epsilon = 1e-15);
    }

    #[test]
    fn cosh_zero() {
        let r = dispatch(BuiltinFn::Cosh, &[real(0.0)]);
        assert_abs_diff_eq!(r.to_f64().unwrap(), 1.0, epsilon = 1e-15);
    }

    #[test]
    fn tanh_zero() {
        let r = dispatch(BuiltinFn::Tanh, &[real(0.0)]);
        assert_abs_diff_eq!(r.to_f64().unwrap(), 0.0, epsilon = 1e-15);
    }

    #[test]
    fn asin_one() {
        let r = dispatch(BuiltinFn::Asin, &[real(1.0)]);
        assert_abs_diff_eq!(r.to_f64().unwrap(), FRAC_PI_2, epsilon = 1e-15);
    }

    #[test]
    fn asin_two_promotes_complex() {
        let r = dispatch(BuiltinFn::Asin, &[real(2.0)]);
        assert!(r.is_complex());
    }

    #[test]
    fn acos_one() {
        let r = dispatch(BuiltinFn::Acos, &[real(1.0)]);
        assert_abs_diff_eq!(r.to_f64().unwrap(), 0.0, epsilon = 1e-15);
    }

    #[test]
    fn atan_one() {
        let r = dispatch(BuiltinFn::Atan, &[real(1.0)]);
        assert_abs_diff_eq!(r.to_f64().unwrap(), PI / 4.0, epsilon = 1e-15);
    }

    #[test]
    fn sin_complex() {
        let r = dispatch(BuiltinFn::Sin, &[complex(0.0, 1.0)]);
        // sin(i) = i*sinh(1)
        assert!(r.is_complex());
    }

    #[test]
    fn resolve_known_functions() {
        assert_eq!(resolve("sin"), Some(BuiltinFn::Sin));
        assert_eq!(resolve("arcsin"), Some(BuiltinFn::Asin));
        assert_eq!(resolve("arccos"), Some(BuiltinFn::Acos));
        assert_eq!(resolve("arctan"), Some(BuiltinFn::Atan));
        assert_eq!(resolve("log"), Some(BuiltinFn::Log));
    }

    #[test]
    fn resolve_unknown() {
        assert_eq!(resolve("foobar"), None);
    }

    #[test]
    fn arity_unary_functions() {
        assert_eq!(arity(BuiltinFn::Sin), 1);
        assert_eq!(arity(BuiltinFn::Sqrt), 1);
        assert_eq!(arity(BuiltinFn::Abs), 1);
    }

    #[test]
    fn arity_binary_functions() {
        assert_eq!(arity(BuiltinFn::Atan2), 2);
        assert_eq!(arity(BuiltinFn::Log), 2);
        assert_eq!(arity(BuiltinFn::Min), 2);
        assert_eq!(arity(BuiltinFn::Max), 2);
    }
}
