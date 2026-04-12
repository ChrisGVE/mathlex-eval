use std::collections::HashMap;

use ndarray::ArrayD;

use crate::broadcast::{
    ResolvedArg, build_args_for_index, compute_shape, eval_broadcast, flat_to_multi, resolve_input,
    results_to_array, total_elements,
};
use crate::compiler::ir::CompiledExpr;
use crate::error::EvalError;
use crate::eval::input::EvalInput;
use crate::eval::numeric::NumericResult;
use crate::eval::scalar;

/// Lazy evaluation handle returned by [`eval()`].
///
/// Computes output shape from input shapes but defers actual evaluation
/// until the caller chooses a consumption mode.
pub struct EvalHandle {
    expr: CompiledExpr,
    resolved: Vec<ResolvedArg>,
    shape: Vec<usize>,
    axis_args: Vec<usize>,
}

impl std::fmt::Debug for EvalHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EvalHandle")
            .field("shape", &self.shape)
            .field("num_args", &self.resolved.len())
            .finish()
    }
}

impl EvalHandle {
    /// Output shape. Empty for scalar output, `[n]` for 1-D, `[n, m]` for 2-D, etc.
    pub fn shape(&self) -> &[usize] {
        &self.shape
    }

    /// Total number of output elements.
    pub fn len(&self) -> usize {
        total_elements(&self.shape)
    }

    /// Whether the output is empty (zero elements due to empty input array).
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Consume as scalar. Errors if output is not 0-d.
    pub fn scalar(self) -> Result<NumericResult, EvalError> {
        if !self.shape.is_empty() {
            return Err(EvalError::ShapeMismatch {
                details: format!("expected scalar output but shape is {:?}", self.shape),
            });
        }
        let args: Vec<NumericResult> = self.resolved.iter().map(|r| r.get(0)).collect();
        scalar::eval_node(&self.expr.root, &args, &mut vec![])
    }

    /// Consume eagerly into a full N-dimensional array.
    pub fn to_array(self) -> Result<ArrayD<NumericResult>, EvalError> {
        let (results, shape) = eval_broadcast(&self.expr, &self.resolved)?;
        results_to_array(results, &shape)
    }

    /// Consume lazily — yields results as they become computable.
    pub fn iter(self) -> EvalIter {
        let total = total_elements(&self.shape);
        EvalIter {
            expr: self.expr,
            resolved: self.resolved,
            shape: self.shape,
            axis_args: self.axis_args,
            current: 0,
            total,
        }
    }
}

/// Streaming result iterator over broadcast evaluation.
pub struct EvalIter {
    expr: CompiledExpr,
    resolved: Vec<ResolvedArg>,
    shape: Vec<usize>,
    axis_args: Vec<usize>,
    current: usize,
    total: usize,
}

impl Iterator for EvalIter {
    type Item = Result<NumericResult, EvalError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.total {
            return None;
        }
        let multi = flat_to_multi(self.current, &self.shape);
        let args = build_args_for_index(&self.resolved, &self.axis_args, &multi);
        self.current += 1;
        Some(scalar::eval_node(&self.expr.root, &args, &mut vec![]))
    }
}

impl EvalIter {
    /// Number of remaining elements.
    pub fn remaining(&self) -> usize {
        self.total - self.current
    }
}

/// Create a lazy eval handle from a compiled expression and arguments.
///
/// Validates that all expected arguments are provided and no unknown
/// arguments are present. Resolves inputs and computes output shape,
/// but defers evaluation until consumed via `scalar()`, `to_array()`,
/// or `iter()`.
pub fn eval(
    expr: &CompiledExpr,
    mut args: HashMap<&str, EvalInput>,
) -> Result<EvalHandle, EvalError> {
    let expected = expr.argument_names();

    // Check for unknown arguments
    for name in args.keys() {
        if !expected.iter().any(|e| e == name) {
            return Err(EvalError::UnknownArgument {
                name: name.to_string(),
            });
        }
    }

    // Resolve arguments in declaration order
    let mut resolved = Vec::with_capacity(expected.len());
    for name in expected {
        match args.remove_entry(name.as_str()) {
            Some((_, input)) => resolved.push(resolve_input(input)),
            None => {
                return Err(EvalError::MissingArgument { name: name.clone() });
            }
        }
    }

    let (shape, axis_args) = compute_shape(&resolved);

    Ok(EvalHandle {
        expr: expr.clone(),
        resolved,
        shape,
        axis_args,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::ir::{BinaryOp, CompiledNode};
    use approx::assert_abs_diff_eq;

    fn make_expr(root: CompiledNode, arg_names: Vec<&str>) -> CompiledExpr {
        CompiledExpr {
            root,
            argument_names: arg_names.into_iter().map(String::from).collect(),
            is_complex: false,
        }
    }

    // x^2 + y
    fn x_sq_plus_y() -> CompiledExpr {
        let x_sq = CompiledNode::Binary {
            op: BinaryOp::Pow,
            left: Box::new(CompiledNode::Argument(0)),
            right: Box::new(CompiledNode::Literal(2.0)),
        };
        make_expr(
            CompiledNode::Binary {
                op: BinaryOp::Add,
                left: Box::new(x_sq),
                right: Box::new(CompiledNode::Argument(1)),
            },
            vec!["x", "y"],
        )
    }

    #[test]
    fn eval_scalar_result() {
        let expr = x_sq_plus_y();
        let mut args = HashMap::new();
        args.insert("x", EvalInput::Scalar(3.0));
        args.insert("y", EvalInput::Scalar(10.0));
        let handle = eval(&expr, args).unwrap();
        assert!(handle.shape().is_empty());
        assert_eq!(handle.len(), 1);
        let result = handle.scalar().unwrap();
        assert_abs_diff_eq!(result.to_f64().unwrap(), 19.0, epsilon = 1e-10);
    }

    #[test]
    fn eval_1d_array() {
        let expr = make_expr(
            CompiledNode::Binary {
                op: BinaryOp::Pow,
                left: Box::new(CompiledNode::Argument(0)),
                right: Box::new(CompiledNode::Literal(2.0)),
            },
            vec!["x"],
        );
        let mut args = HashMap::new();
        args.insert("x", EvalInput::from(vec![1.0, 2.0, 3.0]));
        let handle = eval(&expr, args).unwrap();
        assert_eq!(handle.shape(), &[3]);
        let arr = handle.to_array().unwrap();
        assert_eq!(arr.shape(), &[3]);
        assert_eq!(*arr.get([0]).unwrap(), NumericResult::Real(1.0));
        assert_eq!(*arr.get([1]).unwrap(), NumericResult::Real(4.0));
        assert_eq!(*arr.get([2]).unwrap(), NumericResult::Real(9.0));
    }

    #[test]
    fn eval_2d_cartesian() {
        let expr = x_sq_plus_y();
        let mut args = HashMap::new();
        args.insert("x", EvalInput::from(vec![1.0, 2.0, 3.0]));
        args.insert("y", EvalInput::from(vec![10.0, 20.0]));
        let handle = eval(&expr, args).unwrap();
        assert_eq!(handle.shape(), &[3, 2]);
        assert_eq!(handle.len(), 6);
        let arr = handle.to_array().unwrap();
        assert_eq!(*arr.get([0, 0]).unwrap(), NumericResult::Real(11.0));
        assert_eq!(*arr.get([0, 1]).unwrap(), NumericResult::Real(21.0));
        assert_eq!(*arr.get([2, 1]).unwrap(), NumericResult::Real(29.0));
    }

    #[test]
    fn eval_iter_matches_to_array() {
        let expr = x_sq_plus_y();
        let mut args1 = HashMap::new();
        args1.insert("x", EvalInput::from(vec![1.0, 2.0]));
        args1.insert("y", EvalInput::from(vec![10.0, 20.0]));

        let mut args2 = HashMap::new();
        args2.insert("x", EvalInput::from(vec![1.0, 2.0]));
        args2.insert("y", EvalInput::from(vec![10.0, 20.0]));

        let handle1 = eval(&expr, args1).unwrap();
        let handle2 = eval(&expr, args2).unwrap();

        let arr_results: Vec<NumericResult> = handle1.to_array().unwrap().iter().copied().collect();
        let iter_results: Vec<NumericResult> = handle2.iter().map(|r| r.unwrap()).collect();

        assert_eq!(arr_results, iter_results);
    }

    #[test]
    fn eval_unknown_argument_error() {
        let expr = make_expr(CompiledNode::Argument(0), vec!["x"]);
        let mut args = HashMap::new();
        args.insert("x", EvalInput::Scalar(1.0));
        args.insert("z", EvalInput::Scalar(2.0));
        let err = eval(&expr, args).unwrap_err();
        assert!(matches!(err, EvalError::UnknownArgument { .. }));
    }

    #[test]
    fn eval_missing_argument_error() {
        let expr = x_sq_plus_y();
        let mut args = HashMap::new();
        args.insert("x", EvalInput::Scalar(1.0));
        let err = eval(&expr, args).unwrap_err();
        assert!(matches!(err, EvalError::MissingArgument { .. }));
    }

    #[test]
    fn eval_scalar_on_nonscalar_errors() {
        let expr = make_expr(CompiledNode::Argument(0), vec!["x"]);
        let mut args = HashMap::new();
        args.insert("x", EvalInput::from(vec![1.0, 2.0]));
        let handle = eval(&expr, args).unwrap();
        let err = handle.scalar().unwrap_err();
        assert!(matches!(err, EvalError::ShapeMismatch { .. }));
    }

    #[test]
    fn eval_no_args_expression() {
        // Constant expression: 42
        let expr = make_expr(CompiledNode::Literal(42.0), vec![]);
        let args = HashMap::new();
        let handle = eval(&expr, args).unwrap();
        let result = handle.scalar().unwrap();
        assert_eq!(result, NumericResult::Real(42.0));
    }

    #[test]
    fn eval_iter_remaining() {
        let expr = make_expr(CompiledNode::Argument(0), vec!["x"]);
        let mut args = HashMap::new();
        args.insert("x", EvalInput::from(vec![1.0, 2.0, 3.0]));
        let handle = eval(&expr, args).unwrap();
        let mut iter = handle.iter();
        assert_eq!(iter.remaining(), 3);
        iter.next();
        assert_eq!(iter.remaining(), 2);
    }

    #[test]
    fn eval_with_iterator_input() {
        let expr = make_expr(
            CompiledNode::Binary {
                op: BinaryOp::Mul,
                left: Box::new(CompiledNode::Argument(0)),
                right: Box::new(CompiledNode::Literal(2.0)),
            },
            vec!["x"],
        );
        let mut args = HashMap::new();
        args.insert(
            "x",
            EvalInput::Iter(Box::new(vec![1.0, 2.0, 3.0].into_iter())),
        );
        let handle = eval(&expr, args).unwrap();
        let arr = handle.to_array().unwrap();
        assert_eq!(arr.shape(), &[3]);
    }

    #[test]
    fn eval_empty_array() {
        let expr = make_expr(CompiledNode::Argument(0), vec!["x"]);
        let mut args = HashMap::new();
        args.insert("x", EvalInput::from(vec![] as Vec<f64>));
        let handle = eval(&expr, args).unwrap();
        assert!(handle.is_empty());
        assert_eq!(handle.shape(), &[0]);
    }
}
