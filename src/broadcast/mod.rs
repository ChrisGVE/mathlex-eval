use ndarray::ArrayD;

use crate::compiler::ir::CompiledExpr;
use crate::error::EvalError;
use crate::eval::input::EvalInput;
use crate::eval::numeric::NumericResult;
use crate::eval::scalar;

/// Resolved argument: scalars broadcast, arrays/iters materialized with known length.
pub(crate) enum ResolvedArg {
    Scalar(NumericResult),
    Array(Vec<NumericResult>),
}

impl ResolvedArg {
    /// Number of elements (1 for scalar).
    pub(crate) fn len(&self) -> usize {
        match self {
            ResolvedArg::Scalar(_) => 1,
            ResolvedArg::Array(v) => v.len(),
        }
    }

    pub(crate) fn is_scalar(&self) -> bool {
        matches!(self, ResolvedArg::Scalar(_))
    }

    /// Get value at index, broadcasting scalars.
    pub(crate) fn get(&self, idx: usize) -> NumericResult {
        match self {
            ResolvedArg::Scalar(v) => *v,
            ResolvedArg::Array(v) => v[idx],
        }
    }
}

/// Resolve an EvalInput into a ResolvedArg by materializing iterators.
pub(crate) fn resolve_input(input: EvalInput) -> ResolvedArg {
    match input {
        EvalInput::Scalar(v) => ResolvedArg::Scalar(NumericResult::Real(v)),
        EvalInput::Complex(c) => ResolvedArg::Scalar(NumericResult::Complex(c)),
        EvalInput::Array(arr) => {
            ResolvedArg::Array(arr.iter().map(|v| NumericResult::Real(*v)).collect())
        }
        EvalInput::ComplexArray(arr) => {
            ResolvedArg::Array(arr.iter().map(|v| NumericResult::Complex(*v)).collect())
        }
        EvalInput::Iter(iter) => ResolvedArg::Array(iter.map(NumericResult::Real).collect()),
        EvalInput::ComplexIter(iter) => {
            ResolvedArg::Array(iter.map(NumericResult::Complex).collect())
        }
    }
}

/// Compute output shape from resolved arguments.
/// Each non-scalar argument contributes one axis. Scalars contribute nothing.
/// Returns (shape, axis_arg_indices) where axis_arg_indices maps each output
/// axis to the argument index that provides it.
pub(crate) fn compute_shape(args: &[ResolvedArg]) -> (Vec<usize>, Vec<usize>) {
    let mut shape = Vec::new();
    let mut axis_args = Vec::new();
    for (i, arg) in args.iter().enumerate() {
        if !arg.is_scalar() {
            shape.push(arg.len());
            axis_args.push(i);
        }
    }
    (shape, axis_args)
}

/// Total number of output elements from shape.
pub(crate) fn total_elements(shape: &[usize]) -> usize {
    if shape.is_empty() {
        1 // scalar output
    } else {
        shape.iter().product()
    }
}

/// Convert a flat index to per-axis indices (row-major: last axis varies fastest).
pub(crate) fn flat_to_multi(mut flat: usize, shape: &[usize]) -> Vec<usize> {
    let mut indices = vec![0; shape.len()];
    for i in (0..shape.len()).rev() {
        indices[i] = flat % shape[i];
        flat /= shape[i];
    }
    indices
}

/// Build argument values for a given flat index in the output.
/// Scalar args get their single value; array args get the value at the
/// corresponding axis index.
pub(crate) fn build_args_for_index(
    resolved: &[ResolvedArg],
    axis_args: &[usize],
    multi_idx: &[usize],
) -> Vec<NumericResult> {
    let mut result = Vec::with_capacity(resolved.len());
    let mut axis_pos = 0;
    for (i, arg) in resolved.iter().enumerate() {
        if axis_pos < axis_args.len() && axis_args[axis_pos] == i {
            result.push(arg.get(multi_idx[axis_pos]));
            axis_pos += 1;
        } else {
            result.push(arg.get(0));
        }
    }
    result
}

/// Evaluate the compiled expression over all broadcast combinations.
/// Returns results in row-major order.
pub(crate) fn eval_broadcast(
    expr: &CompiledExpr,
    resolved: &[ResolvedArg],
) -> Result<(Vec<Result<NumericResult, EvalError>>, Vec<usize>), EvalError> {
    let (shape, axis_args) = compute_shape(resolved);
    let total = total_elements(&shape);
    let mut results = Vec::with_capacity(total);

    for flat in 0..total {
        let multi = flat_to_multi(flat, &shape);
        let args = build_args_for_index(resolved, &axis_args, &multi);
        let result = scalar::eval_node(&expr.root, &args, &mut vec![]);
        results.push(result);
    }

    Ok((results, shape))
}

/// Convert broadcast results into an ndarray.
pub(crate) fn results_to_array(
    results: Vec<Result<NumericResult, EvalError>>,
    shape: &[usize],
) -> Result<ArrayD<NumericResult>, EvalError> {
    let flat: Vec<NumericResult> = results.into_iter().collect::<Result<_, _>>()?;
    let nd_shape: Vec<usize> = if shape.is_empty() {
        vec![] // 0-d array
    } else {
        shape.to_vec()
    };
    Ok(ArrayD::from_shape_vec(nd_shape, flat).expect("shape mismatch in results_to_array"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::ir::{BinaryOp, CompiledNode};

    fn make_expr(root: CompiledNode, arg_names: Vec<&str>) -> CompiledExpr {
        CompiledExpr {
            root,
            argument_names: arg_names.into_iter().map(String::from).collect(),
            is_complex: false,
        }
    }

    #[test]
    fn flat_to_multi_2d() {
        let shape = vec![3, 2];
        // Row-major: last axis varies fastest
        assert_eq!(flat_to_multi(0, &shape), vec![0, 0]);
        assert_eq!(flat_to_multi(1, &shape), vec![0, 1]);
        assert_eq!(flat_to_multi(2, &shape), vec![1, 0]);
        assert_eq!(flat_to_multi(5, &shape), vec![2, 1]);
    }

    #[test]
    fn flat_to_multi_3d() {
        let shape = vec![2, 3, 4];
        assert_eq!(flat_to_multi(0, &shape), vec![0, 0, 0]);
        assert_eq!(flat_to_multi(23, &shape), vec![1, 2, 3]);
    }

    #[test]
    fn compute_shape_all_scalars() {
        let args = vec![
            ResolvedArg::Scalar(NumericResult::Real(1.0)),
            ResolvedArg::Scalar(NumericResult::Real(2.0)),
        ];
        let (shape, axis_args) = compute_shape(&args);
        assert!(shape.is_empty());
        assert!(axis_args.is_empty());
    }

    #[test]
    fn compute_shape_one_array() {
        let args = vec![ResolvedArg::Array(vec![
            NumericResult::Real(1.0),
            NumericResult::Real(2.0),
            NumericResult::Real(3.0),
        ])];
        let (shape, axis_args) = compute_shape(&args);
        assert_eq!(shape, vec![3]);
        assert_eq!(axis_args, vec![0]);
    }

    #[test]
    fn compute_shape_mixed() {
        let args = vec![
            ResolvedArg::Array(vec![NumericResult::Real(1.0), NumericResult::Real(2.0)]),
            ResolvedArg::Scalar(NumericResult::Real(5.0)),
            ResolvedArg::Array(vec![
                NumericResult::Real(10.0),
                NumericResult::Real(20.0),
                NumericResult::Real(30.0),
            ]),
        ];
        let (shape, axis_args) = compute_shape(&args);
        assert_eq!(shape, vec![2, 3]);
        assert_eq!(axis_args, vec![0, 2]);
    }

    #[test]
    fn broadcast_all_scalars() {
        // x + y with x=2, y=3 → 5
        let expr = make_expr(
            CompiledNode::Binary {
                op: BinaryOp::Add,
                left: Box::new(CompiledNode::Argument(0)),
                right: Box::new(CompiledNode::Argument(1)),
            },
            vec!["x", "y"],
        );
        let resolved = vec![
            ResolvedArg::Scalar(NumericResult::Real(2.0)),
            ResolvedArg::Scalar(NumericResult::Real(3.0)),
        ];
        let (results, shape) = eval_broadcast(&expr, &resolved).unwrap();
        assert!(shape.is_empty());
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].as_ref().unwrap().to_f64().unwrap(), 5.0);
    }

    #[test]
    fn broadcast_one_array() {
        // x^2 with x=[1,2,3] → [1,4,9]
        let expr = make_expr(
            CompiledNode::Binary {
                op: BinaryOp::Pow,
                left: Box::new(CompiledNode::Argument(0)),
                right: Box::new(CompiledNode::Literal(2.0)),
            },
            vec!["x"],
        );
        let resolved = vec![ResolvedArg::Array(vec![
            NumericResult::Real(1.0),
            NumericResult::Real(2.0),
            NumericResult::Real(3.0),
        ])];
        let (results, shape) = eval_broadcast(&expr, &resolved).unwrap();
        assert_eq!(shape, vec![3]);
        let vals: Vec<f64> = results
            .into_iter()
            .map(|r| r.unwrap().to_f64().unwrap())
            .collect();
        assert_eq!(vals, vec![1.0, 4.0, 9.0]);
    }

    #[test]
    fn broadcast_two_arrays_cartesian() {
        // x^2 + y with x=[1,2,3], y=[10,20]
        // Output shape: [3, 2]
        //    y=10 y=20
        // x=1: 11   21
        // x=2: 14   24
        // x=3: 19   29
        let x_sq = CompiledNode::Binary {
            op: BinaryOp::Pow,
            left: Box::new(CompiledNode::Argument(0)),
            right: Box::new(CompiledNode::Literal(2.0)),
        };
        let expr = make_expr(
            CompiledNode::Binary {
                op: BinaryOp::Add,
                left: Box::new(x_sq),
                right: Box::new(CompiledNode::Argument(1)),
            },
            vec!["x", "y"],
        );
        let resolved = vec![
            ResolvedArg::Array(vec![
                NumericResult::Real(1.0),
                NumericResult::Real(2.0),
                NumericResult::Real(3.0),
            ]),
            ResolvedArg::Array(vec![NumericResult::Real(10.0), NumericResult::Real(20.0)]),
        ];
        let (results, shape) = eval_broadcast(&expr, &resolved).unwrap();
        assert_eq!(shape, vec![3, 2]);
        let vals: Vec<f64> = results
            .into_iter()
            .map(|r| r.unwrap().to_f64().unwrap())
            .collect();
        // Row-major: [f(1,10), f(1,20), f(2,10), f(2,20), f(3,10), f(3,20)]
        assert_eq!(vals, vec![11.0, 21.0, 14.0, 24.0, 19.0, 29.0]);
    }

    #[test]
    fn broadcast_mixed_scalar_array() {
        // x^2 + y with x=2(scalar), y=[10,20,30]
        // Output shape: [3]
        let x_sq = CompiledNode::Binary {
            op: BinaryOp::Pow,
            left: Box::new(CompiledNode::Argument(0)),
            right: Box::new(CompiledNode::Literal(2.0)),
        };
        let expr = make_expr(
            CompiledNode::Binary {
                op: BinaryOp::Add,
                left: Box::new(x_sq),
                right: Box::new(CompiledNode::Argument(1)),
            },
            vec!["x", "y"],
        );
        let resolved = vec![
            ResolvedArg::Scalar(NumericResult::Real(2.0)),
            ResolvedArg::Array(vec![
                NumericResult::Real(10.0),
                NumericResult::Real(20.0),
                NumericResult::Real(30.0),
            ]),
        ];
        let (results, shape) = eval_broadcast(&expr, &resolved).unwrap();
        assert_eq!(shape, vec![3]);
        let vals: Vec<f64> = results
            .into_iter()
            .map(|r| r.unwrap().to_f64().unwrap())
            .collect();
        assert_eq!(vals, vec![14.0, 24.0, 34.0]);
    }

    #[test]
    fn broadcast_empty_array() {
        let expr = make_expr(CompiledNode::Argument(0), vec!["x"]);
        let resolved = vec![ResolvedArg::Array(vec![])];
        let (results, shape) = eval_broadcast(&expr, &resolved).unwrap();
        assert_eq!(shape, vec![0]);
        assert!(results.is_empty());
    }

    #[test]
    fn broadcast_per_element_error() {
        // 1/x with x=[1, 0, 2] → [Ok(1), Err(DivByZero), Ok(0.5)]
        let expr = make_expr(
            CompiledNode::Binary {
                op: BinaryOp::Div,
                left: Box::new(CompiledNode::Literal(1.0)),
                right: Box::new(CompiledNode::Argument(0)),
            },
            vec!["x"],
        );
        let resolved = vec![ResolvedArg::Array(vec![
            NumericResult::Real(1.0),
            NumericResult::Real(0.0),
            NumericResult::Real(2.0),
        ])];
        let (results, _shape) = eval_broadcast(&expr, &resolved).unwrap();
        assert!(results[0].is_ok());
        assert!(results[1].is_err());
        assert!(results[2].is_ok());
    }

    #[test]
    fn resolve_input_scalar() {
        let r = resolve_input(EvalInput::Scalar(5.0));
        assert!(r.is_scalar());
    }

    #[test]
    fn resolve_input_array() {
        let r = resolve_input(EvalInput::from(vec![1.0, 2.0, 3.0]));
        assert_eq!(r.len(), 3);
    }

    #[test]
    fn resolve_input_iter() {
        let r = resolve_input(EvalInput::Iter(Box::new(vec![1.0, 2.0].into_iter())));
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn results_to_array_0d() {
        let results = vec![Ok(NumericResult::Real(42.0))];
        let arr = results_to_array(results, &[]).unwrap();
        assert_eq!(arr.ndim(), 0);
        assert_eq!(*arr.first().unwrap(), NumericResult::Real(42.0));
    }

    #[test]
    fn results_to_array_1d() {
        let results = vec![Ok(NumericResult::Real(1.0)), Ok(NumericResult::Real(2.0))];
        let arr = results_to_array(results, &[2]).unwrap();
        assert_eq!(arr.shape(), &[2]);
    }

    #[test]
    fn results_to_array_with_error() {
        let results = vec![Ok(NumericResult::Real(1.0)), Err(EvalError::DivisionByZero)];
        let err = results_to_array(results, &[2]).unwrap_err();
        assert!(matches!(err, EvalError::DivisionByZero));
    }
}
