use num_complex::Complex;
use std::ops::{Add, Div, Mul, Neg, Sub};

/// Result of evaluating a compiled expression — either a real or complex number.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum NumericResult {
    Real(f64),
    Complex(Complex<f64>),
}

impl NumericResult {
    pub fn is_complex(&self) -> bool {
        matches!(self, NumericResult::Complex(_))
    }

    pub fn to_complex(self) -> Complex<f64> {
        match self {
            NumericResult::Real(r) => Complex::new(r, 0.0),
            NumericResult::Complex(c) => c,
        }
    }

    pub fn to_f64(self) -> Option<f64> {
        match self {
            NumericResult::Real(r) => Some(r),
            NumericResult::Complex(_) => None,
        }
    }

    pub fn pow(self, exp: NumericResult) -> NumericResult {
        match (self, exp) {
            (NumericResult::Real(base), NumericResult::Real(e)) => {
                let result = base.powf(e);
                if result.is_nan() && base < 0.0 {
                    // Negative base with fractional exponent → complex
                    let c = Complex::new(base, 0.0).powc(Complex::new(e, 0.0));
                    NumericResult::Complex(c).simplify()
                } else {
                    NumericResult::Real(result)
                }
            }
            (base, exp) => {
                let c = base.to_complex().powc(exp.to_complex());
                NumericResult::Complex(c).simplify()
            }
        }
    }

    pub fn modulo(self, rhs: NumericResult) -> NumericResult {
        match (self, rhs) {
            (NumericResult::Real(a), NumericResult::Real(b)) => NumericResult::Real(a % b),
            _ => {
                // Complex modulo not standard; return NaN-like behavior
                NumericResult::Real(f64::NAN)
            }
        }
    }

    pub fn sqrt(self) -> NumericResult {
        match self {
            NumericResult::Real(r) if r >= 0.0 => NumericResult::Real(r.sqrt()),
            NumericResult::Real(r) => {
                NumericResult::Complex(Complex::new(0.0, (-r).sqrt())).simplify()
            }
            NumericResult::Complex(c) => NumericResult::Complex(c.sqrt()).simplify(),
        }
    }

    fn simplify(self) -> NumericResult {
        if let NumericResult::Complex(c) = self {
            if c.im.abs() < 1e-15 {
                return NumericResult::Real(c.re);
            }
        }
        self
    }
}

impl From<f64> for NumericResult {
    fn from(v: f64) -> Self {
        NumericResult::Real(v)
    }
}

impl From<Complex<f64>> for NumericResult {
    fn from(v: Complex<f64>) -> Self {
        NumericResult::Complex(v)
    }
}

impl From<i64> for NumericResult {
    fn from(v: i64) -> Self {
        NumericResult::Real(v as f64)
    }
}

impl Add for NumericResult {
    type Output = NumericResult;

    fn add(self, rhs: NumericResult) -> NumericResult {
        match (self, rhs) {
            (NumericResult::Real(a), NumericResult::Real(b)) => NumericResult::Real(a + b),
            (a, b) => NumericResult::Complex(a.to_complex() + b.to_complex()).simplify(),
        }
    }
}

impl Sub for NumericResult {
    type Output = NumericResult;

    fn sub(self, rhs: NumericResult) -> NumericResult {
        match (self, rhs) {
            (NumericResult::Real(a), NumericResult::Real(b)) => NumericResult::Real(a - b),
            (a, b) => NumericResult::Complex(a.to_complex() - b.to_complex()).simplify(),
        }
    }
}

impl Mul for NumericResult {
    type Output = NumericResult;

    fn mul(self, rhs: NumericResult) -> NumericResult {
        match (self, rhs) {
            (NumericResult::Real(a), NumericResult::Real(b)) => NumericResult::Real(a * b),
            (a, b) => NumericResult::Complex(a.to_complex() * b.to_complex()).simplify(),
        }
    }
}

impl Div for NumericResult {
    type Output = NumericResult;

    fn div(self, rhs: NumericResult) -> NumericResult {
        match (self, rhs) {
            (NumericResult::Real(a), NumericResult::Real(b)) => NumericResult::Real(a / b),
            (a, b) => NumericResult::Complex(a.to_complex() / b.to_complex()).simplify(),
        }
    }
}

impl Neg for NumericResult {
    type Output = NumericResult;

    fn neg(self) -> NumericResult {
        match self {
            NumericResult::Real(r) => NumericResult::Real(-r),
            NumericResult::Complex(c) => NumericResult::Complex(-c),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    #[test]
    fn real_add_real_stays_real() {
        let r = NumericResult::Real(2.0) + NumericResult::Real(3.0);
        assert_eq!(r, NumericResult::Real(5.0));
    }

    #[test]
    fn real_add_complex_promotes() {
        let r = NumericResult::Real(1.0) + NumericResult::Complex(Complex::new(2.0, 3.0));
        assert_eq!(r, NumericResult::Complex(Complex::new(3.0, 3.0)));
    }

    #[test]
    fn real_sub_real() {
        let r = NumericResult::Real(5.0) - NumericResult::Real(3.0);
        assert_eq!(r, NumericResult::Real(2.0));
    }

    #[test]
    fn real_mul_real() {
        let r = NumericResult::Real(3.0) * NumericResult::Real(4.0);
        assert_eq!(r, NumericResult::Real(12.0));
    }

    #[test]
    fn real_div_real() {
        let r = NumericResult::Real(10.0) / NumericResult::Real(4.0);
        assert_eq!(r, NumericResult::Real(2.5));
    }

    #[test]
    fn neg_real() {
        let r = -NumericResult::Real(5.0);
        assert_eq!(r, NumericResult::Real(-5.0));
    }

    #[test]
    fn neg_complex() {
        let r = -NumericResult::Complex(Complex::new(1.0, 2.0));
        assert_eq!(r, NumericResult::Complex(Complex::new(-1.0, -2.0)));
    }

    #[test]
    fn complex_mul_complex() {
        // (1+2i) * (3+4i) = 3+4i+6i+8i² = 3+10i-8 = -5+10i
        let a = NumericResult::Complex(Complex::new(1.0, 2.0));
        let b = NumericResult::Complex(Complex::new(3.0, 4.0));
        let r = a * b;
        assert_eq!(r, NumericResult::Complex(Complex::new(-5.0, 10.0)));
    }

    #[test]
    fn sqrt_negative_returns_complex() {
        let r = NumericResult::Real(-1.0).sqrt();
        match r {
            NumericResult::Complex(c) => {
                assert_abs_diff_eq!(c.re, 0.0, epsilon = 1e-15);
                assert_abs_diff_eq!(c.im, 1.0, epsilon = 1e-15);
            }
            _ => panic!("expected complex"),
        }
    }

    #[test]
    fn sqrt_positive_stays_real() {
        let r = NumericResult::Real(4.0).sqrt();
        assert_eq!(r, NumericResult::Real(2.0));
    }

    #[test]
    fn complex_with_zero_im_simplifies_to_real() {
        let c = NumericResult::Complex(Complex::new(5.0, 0.0));
        let simplified = c.simplify();
        assert_eq!(simplified, NumericResult::Real(5.0));
    }

    #[test]
    fn pow_real_real() {
        let r = NumericResult::Real(2.0).pow(NumericResult::Real(3.0));
        assert_eq!(r, NumericResult::Real(8.0));
    }

    #[test]
    fn pow_negative_base_fractional_exp_promotes() {
        let r = NumericResult::Real(-8.0).pow(NumericResult::Real(1.0 / 3.0));
        assert!(r.is_complex());
    }

    #[test]
    fn from_f64() {
        let r: NumericResult = 3.14.into();
        assert_eq!(r, NumericResult::Real(3.14));
    }

    #[test]
    fn from_complex() {
        let c = Complex::new(1.0, 2.0);
        let r: NumericResult = c.into();
        assert_eq!(r, NumericResult::Complex(c));
    }

    #[test]
    fn from_i64() {
        let r: NumericResult = 42i64.into();
        assert_eq!(r, NumericResult::Real(42.0));
    }

    #[test]
    fn to_f64_real() {
        assert_eq!(NumericResult::Real(3.0).to_f64(), Some(3.0));
    }

    #[test]
    fn to_f64_complex_returns_none() {
        assert_eq!(
            NumericResult::Complex(Complex::new(1.0, 2.0)).to_f64(),
            None
        );
    }

    #[test]
    fn modulo_real() {
        let r = NumericResult::Real(7.0).modulo(NumericResult::Real(3.0));
        assert_eq!(r, NumericResult::Real(1.0));
    }
}
