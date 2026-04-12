use crate::compiler::ir::{BinaryOp, CompiledNode, UnaryOp};
use crate::error::EvalError;
use crate::eval::functions;
use crate::eval::numeric::NumericResult;

/// Evaluate a compiled node tree to a single numeric result.
///
/// `args` contains argument values indexed by position (matching
/// `CompiledExpr::argument_names()` ordering).
/// `indices` contains bound index variable values from sum/product loops.
pub(crate) fn eval_node(
    node: &CompiledNode,
    args: &[NumericResult],
    indices: &mut Vec<NumericResult>,
) -> Result<NumericResult, EvalError> {
    match node {
        CompiledNode::Literal(v) => Ok(NumericResult::Real(*v)),

        CompiledNode::ComplexLiteral { re, im } => {
            Ok(NumericResult::Complex(num_complex::Complex::new(*re, *im)))
        }

        CompiledNode::Argument(idx) => Ok(args[*idx]),

        CompiledNode::Index(slot) => Ok(indices[*slot]),

        CompiledNode::Binary { op, left, right } => {
            let lv = eval_node(left, args, indices)?;
            let rv = eval_node(right, args, indices)?;
            eval_binary(*op, lv, rv)
        }

        CompiledNode::Unary { op, operand } => {
            let val = eval_node(operand, args, indices)?;
            eval_unary(*op, val)
        }

        CompiledNode::Function {
            kind,
            args: fn_args,
        } => {
            let evaluated: Vec<NumericResult> = fn_args
                .iter()
                .map(|a| eval_node(a, args, indices))
                .collect::<Result<_, _>>()?;
            Ok(functions::dispatch(*kind, &evaluated))
        }

        CompiledNode::Sum {
            index,
            lower,
            upper,
            body,
        } => {
            let mut acc = NumericResult::Real(0.0);
            // Ensure indices vec is large enough
            if indices.len() <= *index {
                indices.resize(*index + 1, NumericResult::Real(0.0));
            }
            for i in *lower..=*upper {
                indices[*index] = NumericResult::Real(i as f64);
                let val = eval_node(body, args, indices)?;
                acc = acc + val;
            }
            Ok(acc)
        }

        CompiledNode::Product {
            index,
            lower,
            upper,
            body,
        } => {
            let mut acc = NumericResult::Real(1.0);
            if indices.len() <= *index {
                indices.resize(*index + 1, NumericResult::Real(0.0));
            }
            for i in *lower..=*upper {
                indices[*index] = NumericResult::Real(i as f64);
                let val = eval_node(body, args, indices)?;
                acc = acc * val;
            }
            Ok(acc)
        }
    }
}

fn eval_binary(
    op: BinaryOp,
    left: NumericResult,
    right: NumericResult,
) -> Result<NumericResult, EvalError> {
    match op {
        BinaryOp::Add => Ok(left + right),
        BinaryOp::Sub => Ok(left - right),
        BinaryOp::Mul => Ok(left * right),
        BinaryOp::Div => {
            if matches!(right, NumericResult::Real(r) if r == 0.0) {
                return Err(EvalError::DivisionByZero);
            }
            Ok(left / right)
        }
        BinaryOp::Pow => Ok(left.pow(right)),
        BinaryOp::Mod => Ok(left.modulo(right)),
    }
}

fn eval_unary(op: UnaryOp, val: NumericResult) -> Result<NumericResult, EvalError> {
    match op {
        UnaryOp::Neg => Ok(-val),
        UnaryOp::Factorial => match val {
            NumericResult::Real(r) => {
                let n = r as u64;
                if r < 0.0 || r != (n as f64) {
                    return Err(EvalError::NumericOverflow);
                }
                Ok(NumericResult::Real(factorial(n)))
            }
            NumericResult::Complex(_) => Err(EvalError::NumericOverflow),
        },
    }
}

fn factorial(n: u64) -> f64 {
    (1..=n).fold(1.0, |acc, i| acc * i as f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::ir::BuiltinFn;
    use approx::assert_abs_diff_eq;

    fn lit(v: f64) -> CompiledNode {
        CompiledNode::Literal(v)
    }

    fn arg(idx: usize) -> CompiledNode {
        CompiledNode::Argument(idx)
    }

    fn binary(op: BinaryOp, left: CompiledNode, right: CompiledNode) -> CompiledNode {
        CompiledNode::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    #[test]
    fn eval_literal() {
        let node = lit(42.0);
        let result = eval_node(&node, &[], &mut vec![]).unwrap();
        assert_eq!(result, NumericResult::Real(42.0));
    }

    #[test]
    fn eval_complex_literal() {
        let node = CompiledNode::ComplexLiteral { re: 1.0, im: 2.0 };
        let result = eval_node(&node, &[], &mut vec![]).unwrap();
        assert!(result.is_complex());
    }

    #[test]
    fn eval_argument() {
        let node = arg(0);
        let args = [NumericResult::Real(7.0)];
        let result = eval_node(&node, &args, &mut vec![]).unwrap();
        assert_eq!(result, NumericResult::Real(7.0));
    }

    #[test]
    fn eval_addition() {
        let node = binary(BinaryOp::Add, lit(3.0), lit(4.0));
        let result = eval_node(&node, &[], &mut vec![]).unwrap();
        assert_eq!(result, NumericResult::Real(7.0));
    }

    #[test]
    fn eval_subtraction() {
        let node = binary(BinaryOp::Sub, lit(10.0), lit(3.0));
        let result = eval_node(&node, &[], &mut vec![]).unwrap();
        assert_eq!(result, NumericResult::Real(7.0));
    }

    #[test]
    fn eval_multiplication() {
        let node = binary(BinaryOp::Mul, lit(3.0), lit(4.0));
        let result = eval_node(&node, &[], &mut vec![]).unwrap();
        assert_eq!(result, NumericResult::Real(12.0));
    }

    #[test]
    fn eval_division() {
        let node = binary(BinaryOp::Div, lit(10.0), lit(4.0));
        let result = eval_node(&node, &[], &mut vec![]).unwrap();
        assert_eq!(result, NumericResult::Real(2.5));
    }

    #[test]
    fn eval_division_by_zero() {
        let node = binary(BinaryOp::Div, arg(0), lit(0.0));
        let args = [NumericResult::Real(5.0)];
        let err = eval_node(&node, &args, &mut vec![]).unwrap_err();
        assert!(matches!(err, EvalError::DivisionByZero));
    }

    #[test]
    fn eval_power() {
        let node = binary(BinaryOp::Pow, lit(2.0), lit(10.0));
        let result = eval_node(&node, &[], &mut vec![]).unwrap();
        assert_abs_diff_eq!(result.to_f64().unwrap(), 1024.0, epsilon = 1e-10);
    }

    #[test]
    fn eval_modulo() {
        let node = binary(BinaryOp::Mod, lit(7.0), lit(3.0));
        let result = eval_node(&node, &[], &mut vec![]).unwrap();
        assert_abs_diff_eq!(result.to_f64().unwrap(), 1.0, epsilon = 1e-10);
    }

    #[test]
    fn eval_negation() {
        let node = CompiledNode::Unary {
            op: UnaryOp::Neg,
            operand: Box::new(lit(5.0)),
        };
        let result = eval_node(&node, &[], &mut vec![]).unwrap();
        assert_eq!(result, NumericResult::Real(-5.0));
    }

    #[test]
    fn eval_factorial() {
        let node = CompiledNode::Unary {
            op: UnaryOp::Factorial,
            operand: Box::new(lit(5.0)),
        };
        let result = eval_node(&node, &[], &mut vec![]).unwrap();
        assert_abs_diff_eq!(result.to_f64().unwrap(), 120.0, epsilon = 1e-10);
    }

    #[test]
    fn eval_function_sin() {
        let node = CompiledNode::Function {
            kind: BuiltinFn::Sin,
            args: vec![lit(std::f64::consts::FRAC_PI_2)],
        };
        let result = eval_node(&node, &[], &mut vec![]).unwrap();
        assert_abs_diff_eq!(result.to_f64().unwrap(), 1.0, epsilon = 1e-15);
    }

    #[test]
    fn eval_nested_expression() {
        // sin(x^2 + 1) with x = 0 → sin(1)
        let x_sq = binary(BinaryOp::Pow, arg(0), lit(2.0));
        let plus_one = binary(BinaryOp::Add, x_sq, lit(1.0));
        let node = CompiledNode::Function {
            kind: BuiltinFn::Sin,
            args: vec![plus_one],
        };
        let args = [NumericResult::Real(0.0)];
        let result = eval_node(&node, &args, &mut vec![]).unwrap();
        assert_abs_diff_eq!(result.to_f64().unwrap(), 1.0_f64.sin(), epsilon = 1e-15);
    }

    #[test]
    fn eval_sum() {
        // Σ_{k=1}^{5} k = 15
        let node = CompiledNode::Sum {
            index: 0,
            lower: 1,
            upper: 5,
            body: Box::new(CompiledNode::Index(0)),
        };
        let result = eval_node(&node, &[], &mut vec![]).unwrap();
        assert_abs_diff_eq!(result.to_f64().unwrap(), 15.0, epsilon = 1e-10);
    }

    #[test]
    fn eval_product() {
        // Π_{k=1}^{4} k = 24
        let node = CompiledNode::Product {
            index: 0,
            lower: 1,
            upper: 4,
            body: Box::new(CompiledNode::Index(0)),
        };
        let result = eval_node(&node, &[], &mut vec![]).unwrap();
        assert_abs_diff_eq!(result.to_f64().unwrap(), 24.0, epsilon = 1e-10);
    }

    #[test]
    fn eval_sum_with_argument() {
        // Σ_{k=1}^{3} (x * k) with x = 2 → 2*1 + 2*2 + 2*3 = 12
        let body = binary(BinaryOp::Mul, arg(0), CompiledNode::Index(0));
        let node = CompiledNode::Sum {
            index: 0,
            lower: 1,
            upper: 3,
            body: Box::new(body),
        };
        let args = [NumericResult::Real(2.0)];
        let result = eval_node(&node, &args, &mut vec![]).unwrap();
        assert_abs_diff_eq!(result.to_f64().unwrap(), 12.0, epsilon = 1e-10);
    }

    #[test]
    fn eval_expression_with_two_args() {
        // x^2 + y with x=3, y=10 → 19
        let x_sq = binary(BinaryOp::Pow, arg(0), lit(2.0));
        let node = binary(BinaryOp::Add, x_sq, arg(1));
        let args = [NumericResult::Real(3.0), NumericResult::Real(10.0)];
        let result = eval_node(&node, &args, &mut vec![]).unwrap();
        assert_abs_diff_eq!(result.to_f64().unwrap(), 19.0, epsilon = 1e-10);
    }
}
