use ndarray::ArrayD;
use num_complex::Complex;

/// Input value for an argument during evaluation.
///
/// Each argument can be a scalar, array, or iterator — real or complex.
/// Scalar arguments broadcast to all positions. Array arguments contribute
/// one axis to the output shape. Iterator arguments are cached incrementally.
pub enum EvalInput {
    Scalar(f64),
    Complex(Complex<f64>),
    Array(ArrayD<f64>),
    ComplexArray(ArrayD<Complex<f64>>),
    Iter(Box<dyn Iterator<Item = f64>>),
    ComplexIter(Box<dyn Iterator<Item = Complex<f64>>>),
}

impl EvalInput {
    /// Whether this input is scalar (contributes no axis to output).
    pub fn is_scalar(&self) -> bool {
        matches!(self, EvalInput::Scalar(_) | EvalInput::Complex(_))
    }

    /// Whether this input contains complex values.
    pub fn is_complex(&self) -> bool {
        matches!(
            self,
            EvalInput::Complex(_) | EvalInput::ComplexArray(_) | EvalInput::ComplexIter(_)
        )
    }

    /// Whether this input is an iterator (unknown length until exhausted).
    pub fn is_iter(&self) -> bool {
        matches!(self, EvalInput::Iter(_) | EvalInput::ComplexIter(_))
    }
}

impl From<f64> for EvalInput {
    fn from(v: f64) -> Self {
        EvalInput::Scalar(v)
    }
}

impl From<Complex<f64>> for EvalInput {
    fn from(v: Complex<f64>) -> Self {
        EvalInput::Complex(v)
    }
}

impl From<Vec<f64>> for EvalInput {
    fn from(v: Vec<f64>) -> Self {
        EvalInput::Array(ArrayD::from_shape_vec(vec![v.len()], v).unwrap())
    }
}

impl From<Vec<Complex<f64>>> for EvalInput {
    fn from(v: Vec<Complex<f64>>) -> Self {
        EvalInput::ComplexArray(ArrayD::from_shape_vec(vec![v.len()], v).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_is_scalar() {
        assert!(EvalInput::Scalar(1.0).is_scalar());
    }

    #[test]
    fn complex_is_scalar() {
        assert!(EvalInput::Complex(Complex::new(1.0, 2.0)).is_scalar());
    }

    #[test]
    fn array_not_scalar() {
        let input: EvalInput = vec![1.0, 2.0, 3.0].into();
        assert!(!input.is_scalar());
    }

    #[test]
    fn complex_input_is_complex() {
        assert!(EvalInput::Complex(Complex::new(1.0, 0.0)).is_complex());
    }

    #[test]
    fn real_scalar_not_complex() {
        assert!(!EvalInput::Scalar(1.0).is_complex());
    }

    #[test]
    fn iter_is_iter() {
        let input = EvalInput::Iter(Box::new(vec![1.0, 2.0].into_iter()));
        assert!(input.is_iter());
    }

    #[test]
    fn array_not_iter() {
        let input: EvalInput = vec![1.0].into();
        assert!(!input.is_iter());
    }

    #[test]
    fn from_vec_f64() {
        let input: EvalInput = vec![1.0, 2.0, 3.0].into();
        match input {
            EvalInput::Array(arr) => assert_eq!(arr.len(), 3),
            _ => panic!("expected Array"),
        }
    }

    #[test]
    fn from_vec_complex() {
        let input: EvalInput = vec![Complex::new(1.0, 0.0), Complex::new(0.0, 1.0)].into();
        match input {
            EvalInput::ComplexArray(arr) => assert_eq!(arr.len(), 2),
            _ => panic!("expected ComplexArray"),
        }
    }
}
