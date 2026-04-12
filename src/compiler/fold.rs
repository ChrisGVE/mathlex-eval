use std::collections::HashMap;

use mathlex::{Expression, MathConstant};

use crate::compiler::ir::{BinaryOp, BuiltinFn, CompiledExpr, CompiledNode, UnaryOp};
use crate::error::CompileError;
use crate::eval::functions;
use crate::eval::numeric::NumericResult;

/// Context for the folding pass — tracks variable bindings, argument ordering,
/// and sum/product index scoping.
struct FoldContext<'a> {
    /// User-provided constants to substitute.
    constants: &'a HashMap<&'a str, NumericResult>,
    /// Maps free variable names to argument indices (order of first appearance).
    arguments: Vec<String>,
    /// Stack of bound index variables from sum/product. Maps name → slot index.
    index_scopes: Vec<(String, usize)>,
    /// Next available index slot.
    next_index_slot: usize,
    /// Whether any complex literal or constant was seen.
    has_complex: bool,
}

impl<'a> FoldContext<'a> {
    fn new(constants: &'a HashMap<&'a str, NumericResult>) -> Self {
        Self {
            constants,
            arguments: Vec::new(),
            index_scopes: Vec::new(),
            next_index_slot: 0,
            has_complex: false,
        }
    }

    /// Look up a variable name: first in index scopes (innermost first),
    /// then in constants, then assign as argument.
    fn resolve_variable(&mut self, name: &str) -> CompiledNode {
        // Check index scopes (innermost = last in vec)
        for (idx_name, slot) in self.index_scopes.iter().rev() {
            if idx_name == name {
                return CompiledNode::Index(*slot);
            }
        }

        // Check user constants
        if let Some(val) = self.constants.get(name) {
            return match val {
                NumericResult::Real(r) => CompiledNode::Literal(*r),
                NumericResult::Complex(c) => {
                    self.has_complex = true;
                    CompiledNode::ComplexLiteral { re: c.re, im: c.im }
                }
            };
        }

        // Assign as argument (or reuse existing index)
        if let Some(pos) = self.arguments.iter().position(|a| a == name) {
            CompiledNode::Argument(pos)
        } else {
            let idx = self.arguments.len();
            self.arguments.push(name.to_string());
            CompiledNode::Argument(idx)
        }
    }

    fn push_index_scope(&mut self, name: &str) -> usize {
        let slot = self.next_index_slot;
        self.next_index_slot += 1;
        self.index_scopes.push((name.to_string(), slot));
        slot
    }

    fn pop_index_scope(&mut self) {
        self.index_scopes.pop();
    }
}

/// Fold an AST into a CompiledExpr. Assumes AST has already been validated.
pub(crate) fn fold(
    ast: &Expression,
    constants: &HashMap<&str, NumericResult>,
) -> Result<CompiledExpr, CompileError> {
    let mut ctx = FoldContext::new(constants);
    let root = fold_node(ast, &mut ctx)?;
    Ok(CompiledExpr {
        root,
        argument_names: ctx.arguments,
        is_complex: ctx.has_complex,
    })
}

fn fold_node(ast: &Expression, ctx: &mut FoldContext) -> Result<CompiledNode, CompileError> {
    match ast {
        Expression::Integer(v) => Ok(CompiledNode::Literal(*v as f64)),

        Expression::Float(mf) => Ok(CompiledNode::Literal(f64::from(*mf))),

        Expression::Rational {
            numerator,
            denominator,
        } => {
            let num = fold_node(numerator, ctx)?;
            let den = fold_node(denominator, ctx)?;
            try_fold_binary(BinaryOp::Div, num, den)
        }

        Expression::Complex { real, imaginary } => {
            ctx.has_complex = true;
            let re = fold_node(real, ctx)?;
            let im = fold_node(imaginary, ctx)?;
            // a + b*i → build as Binary(Add, re, Binary(Mul, im, ComplexLiteral(0,1)))
            let i_unit = CompiledNode::ComplexLiteral { re: 0.0, im: 1.0 };
            let im_part = try_fold_binary(BinaryOp::Mul, im, i_unit)?;
            try_fold_binary(BinaryOp::Add, re, im_part)
        }

        Expression::Variable(name) => {
            let node = ctx.resolve_variable(name);
            Ok(node)
        }

        Expression::Constant(mc) => fold_math_constant(*mc, ctx),

        Expression::Binary { op, left, right } => {
            let bin_op = convert_binary_op(*op);
            let l = fold_node(left, ctx)?;
            let r = fold_node(right, ctx)?;
            try_fold_binary(bin_op, l, r)
        }

        Expression::Unary { op, operand } => {
            let node = fold_node(operand, ctx)?;
            match op {
                mathlex::UnaryOp::Neg => try_fold_unary(UnaryOp::Neg, node),
                mathlex::UnaryOp::Pos => Ok(node), // +x = x
                mathlex::UnaryOp::Factorial => try_fold_unary(UnaryOp::Factorial, node),
                mathlex::UnaryOp::Transpose => unreachable!("caught by validation"),
            }
        }

        Expression::Function { name, args } => {
            let kind = functions::resolve(name)
                .ok_or_else(|| CompileError::UnknownFunction { name: name.clone() })?;
            let expected = functions::arity(kind);
            if args.len() != expected {
                return Err(CompileError::ArityMismatch {
                    function: name.clone(),
                    expected,
                    got: args.len(),
                });
            }
            let compiled_args: Vec<CompiledNode> = args
                .iter()
                .map(|a| fold_node(a, ctx))
                .collect::<Result<_, _>>()?;
            try_fold_function(kind, compiled_args)
        }

        Expression::Sum {
            index,
            lower,
            upper,
            body,
        } => fold_sum_product(true, index, lower, upper, body, ctx),

        Expression::Product {
            index,
            lower,
            upper,
            body,
        } => fold_sum_product(false, index, lower, upper, body, ctx),

        // All other variants should have been rejected by validation
        _ => unreachable!("unvalidated expression variant reached fold"),
    }
}

fn fold_math_constant(
    mc: MathConstant,
    ctx: &mut FoldContext,
) -> Result<CompiledNode, CompileError> {
    match mc {
        MathConstant::Pi => Ok(CompiledNode::Literal(std::f64::consts::PI)),
        MathConstant::E => Ok(CompiledNode::Literal(std::f64::consts::E)),
        MathConstant::I => {
            ctx.has_complex = true;
            Ok(CompiledNode::ComplexLiteral { re: 0.0, im: 1.0 })
        }
        MathConstant::Infinity => Ok(CompiledNode::Literal(f64::INFINITY)),
        MathConstant::NegInfinity => Ok(CompiledNode::Literal(f64::NEG_INFINITY)),
        MathConstant::NaN => Ok(CompiledNode::Literal(f64::NAN)),
        // Quaternion basis vectors — not supported in v1
        MathConstant::J | MathConstant::K => Err(CompileError::UnsupportedExpression {
            variant: "MathConstant",
            context: format!("quaternion basis {:?} not supported in v1", mc),
        }),
    }
}

fn fold_sum_product(
    is_sum: bool,
    index_name: &str,
    lower: &Expression,
    upper: &Expression,
    body: &Expression,
    ctx: &mut FoldContext,
) -> Result<CompiledNode, CompileError> {
    let lower_node = fold_node(lower, ctx)?;
    let upper_node = fold_node(upper, ctx)?;

    let lower_val = extract_integer(&lower_node).ok_or_else(|| CompileError::NonIntegerBounds {
        construct: if is_sum { "sum" } else { "product" },
        bound: format!("{:?}", lower),
    })?;
    let upper_val = extract_integer(&upper_node).ok_or_else(|| CompileError::NonIntegerBounds {
        construct: if is_sum { "sum" } else { "product" },
        bound: format!("{:?}", upper),
    })?;

    let slot = ctx.push_index_scope(index_name);
    let body_node = fold_node(body, ctx)?;
    ctx.pop_index_scope();

    if is_sum {
        Ok(CompiledNode::Sum {
            index: slot,
            lower: lower_val,
            upper: upper_val,
            body: Box::new(body_node),
        })
    } else {
        Ok(CompiledNode::Product {
            index: slot,
            lower: lower_val,
            upper: upper_val,
            body: Box::new(body_node),
        })
    }
}

/// Try to fold a binary operation at compile time if both operands are literals.
fn try_fold_binary(
    op: BinaryOp,
    left: CompiledNode,
    right: CompiledNode,
) -> Result<CompiledNode, CompileError> {
    if let (Some(lv), Some(rv)) = (node_to_numeric(&left), node_to_numeric(&right)) {
        let result = eval_binary_op(op, lv, rv)?;
        return Ok(numeric_to_node(result));
    }
    Ok(CompiledNode::Binary {
        op,
        left: Box::new(left),
        right: Box::new(right),
    })
}

fn try_fold_unary(op: UnaryOp, operand: CompiledNode) -> Result<CompiledNode, CompileError> {
    if let Some(val) = node_to_numeric(&operand) {
        let result = eval_unary_op(op, val)?;
        return Ok(numeric_to_node(result));
    }
    Ok(CompiledNode::Unary {
        op,
        operand: Box::new(operand),
    })
}

fn try_fold_function(
    kind: BuiltinFn,
    args: Vec<CompiledNode>,
) -> Result<CompiledNode, CompileError> {
    let all_literal: Vec<NumericResult> = args.iter().filter_map(node_to_numeric).collect();
    if all_literal.len() == args.len() {
        let result = functions::dispatch(kind, &all_literal);
        return Ok(numeric_to_node(result));
    }
    Ok(CompiledNode::Function { kind, args })
}

fn node_to_numeric(node: &CompiledNode) -> Option<NumericResult> {
    match node {
        CompiledNode::Literal(v) => Some(NumericResult::Real(*v)),
        CompiledNode::ComplexLiteral { re, im } => {
            Some(NumericResult::Complex(num_complex::Complex::new(*re, *im)))
        }
        _ => None,
    }
}

fn numeric_to_node(val: NumericResult) -> CompiledNode {
    match val {
        NumericResult::Real(r) => CompiledNode::Literal(r),
        NumericResult::Complex(c) => CompiledNode::ComplexLiteral { re: c.re, im: c.im },
    }
}

fn eval_binary_op(
    op: BinaryOp,
    left: NumericResult,
    right: NumericResult,
) -> Result<NumericResult, CompileError> {
    match op {
        BinaryOp::Add => Ok(left + right),
        BinaryOp::Sub => Ok(left - right),
        BinaryOp::Mul => Ok(left * right),
        BinaryOp::Div => {
            if matches!(right, NumericResult::Real(r) if r == 0.0) {
                return Err(CompileError::DivisionByZero);
            }
            Ok(left / right)
        }
        BinaryOp::Pow => Ok(left.pow(right)),
        BinaryOp::Mod => Ok(left.modulo(right)),
    }
}

fn eval_unary_op(op: UnaryOp, val: NumericResult) -> Result<NumericResult, CompileError> {
    match op {
        UnaryOp::Neg => Ok(-val),
        UnaryOp::Factorial => match val {
            NumericResult::Real(r) => {
                let n = r as u64;
                if r < 0.0 || r != (n as f64) {
                    return Err(CompileError::NumericOverflow {
                        context: format!("factorial of non-integer {}", r),
                    });
                }
                Ok(NumericResult::Real(factorial(n)))
            }
            NumericResult::Complex(_) => Err(CompileError::NumericOverflow {
                context: "factorial of complex number".into(),
            }),
        },
    }
}

fn factorial(n: u64) -> f64 {
    (1..=n).fold(1.0, |acc, i| acc * i as f64)
}

fn extract_integer(node: &CompiledNode) -> Option<i64> {
    match node {
        CompiledNode::Literal(v) => {
            let rounded = v.round();
            if (*v - rounded).abs() < 1e-10 {
                Some(rounded as i64)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn convert_binary_op(op: mathlex::BinaryOp) -> BinaryOp {
    match op {
        mathlex::BinaryOp::Add => BinaryOp::Add,
        mathlex::BinaryOp::Sub => BinaryOp::Sub,
        mathlex::BinaryOp::Mul => BinaryOp::Mul,
        mathlex::BinaryOp::Div => BinaryOp::Div,
        mathlex::BinaryOp::Pow => BinaryOp::Pow,
        mathlex::BinaryOp::Mod => BinaryOp::Mod,
        // PlusMinus/MinusPlus caught by validation
        _ => unreachable!("unsupported binary op reached fold"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;
    use mathlex::MathFloat;

    fn int(v: i64) -> Expression {
        Expression::Integer(v)
    }

    fn var(name: &str) -> Expression {
        Expression::Variable(name.into())
    }

    fn float(v: f64) -> Expression {
        Expression::Float(MathFloat::from(v))
    }

    fn empty_constants() -> HashMap<&'static str, NumericResult> {
        HashMap::new()
    }

    #[test]
    fn fold_integer_literal() {
        let expr = fold(&int(42), &empty_constants()).unwrap();
        assert!(matches!(expr.root, CompiledNode::Literal(v) if v == 42.0));
        assert!(expr.argument_names.is_empty());
    }

    #[test]
    fn fold_float_literal() {
        let expr = fold(&float(3.14), &empty_constants()).unwrap();
        assert!(matches!(expr.root, CompiledNode::Literal(v) if (v - 3.14).abs() < 1e-10));
    }

    #[test]
    fn fold_variable_becomes_argument() {
        let expr = fold(&var("x"), &empty_constants()).unwrap();
        assert!(matches!(expr.root, CompiledNode::Argument(0)));
        assert_eq!(expr.argument_names(), &["x"]);
    }

    #[test]
    fn fold_two_variables_get_distinct_indices() {
        let ast = Expression::Binary {
            op: mathlex::BinaryOp::Add,
            left: Box::new(var("x")),
            right: Box::new(var("y")),
        };
        let expr = fold(&ast, &empty_constants()).unwrap();
        assert_eq!(expr.argument_names(), &["x", "y"]);
    }

    #[test]
    fn fold_same_variable_reuses_index() {
        let ast = Expression::Binary {
            op: mathlex::BinaryOp::Add,
            left: Box::new(var("x")),
            right: Box::new(var("x")),
        };
        let expr = fold(&ast, &empty_constants()).unwrap();
        assert_eq!(expr.argument_names(), &["x"]);
    }

    #[test]
    fn fold_constant_substitution() {
        let mut constants = HashMap::new();
        constants.insert("a", NumericResult::Real(5.0));
        let expr = fold(&var("a"), &constants).unwrap();
        assert!(matches!(expr.root, CompiledNode::Literal(v) if v == 5.0));
        assert!(expr.argument_names.is_empty());
    }

    #[test]
    fn fold_pi_constant() {
        let ast = Expression::Constant(MathConstant::Pi);
        let expr = fold(&ast, &empty_constants()).unwrap();
        if let CompiledNode::Literal(v) = expr.root {
            assert_abs_diff_eq!(v, std::f64::consts::PI, epsilon = 1e-15);
        } else {
            panic!("expected literal");
        }
    }

    #[test]
    fn fold_e_constant() {
        let ast = Expression::Constant(MathConstant::E);
        let expr = fold(&ast, &empty_constants()).unwrap();
        if let CompiledNode::Literal(v) = expr.root {
            assert_abs_diff_eq!(v, std::f64::consts::E, epsilon = 1e-15);
        } else {
            panic!("expected literal");
        }
    }

    #[test]
    fn fold_imaginary_unit() {
        let ast = Expression::Constant(MathConstant::I);
        let expr = fold(&ast, &empty_constants()).unwrap();
        assert!(matches!(
            expr.root,
            CompiledNode::ComplexLiteral { re, im } if re == 0.0 && im == 1.0
        ));
        assert!(expr.is_complex());
    }

    #[test]
    fn fold_constant_expression_folded() {
        // 2 * pi should fold to a single literal
        let ast = Expression::Binary {
            op: mathlex::BinaryOp::Mul,
            left: Box::new(int(2)),
            right: Box::new(Expression::Constant(MathConstant::Pi)),
        };
        let expr = fold(&ast, &empty_constants()).unwrap();
        if let CompiledNode::Literal(v) = expr.root {
            assert_abs_diff_eq!(v, 2.0 * std::f64::consts::PI, epsilon = 1e-15);
        } else {
            panic!("expected folded literal, got {:?}", expr.root);
        }
    }

    #[test]
    fn fold_mixed_constant_variable_not_folded() {
        // x + 1 should not fold
        let ast = Expression::Binary {
            op: mathlex::BinaryOp::Add,
            left: Box::new(var("x")),
            right: Box::new(int(1)),
        };
        let expr = fold(&ast, &empty_constants()).unwrap();
        assert!(matches!(expr.root, CompiledNode::Binary { .. }));
    }

    #[test]
    fn fold_division_by_zero_error() {
        let ast = Expression::Binary {
            op: mathlex::BinaryOp::Div,
            left: Box::new(int(1)),
            right: Box::new(int(0)),
        };
        let err = fold(&ast, &empty_constants()).unwrap_err();
        assert!(matches!(err, CompileError::DivisionByZero));
    }

    #[test]
    fn fold_unknown_function_error() {
        let ast = Expression::Function {
            name: "foobar".into(),
            args: vec![int(1)],
        };
        let err = fold(&ast, &empty_constants()).unwrap_err();
        assert!(matches!(err, CompileError::UnknownFunction { .. }));
    }

    #[test]
    fn fold_arity_mismatch_error() {
        let ast = Expression::Function {
            name: "sin".into(),
            args: vec![int(1), int(2)],
        };
        let err = fold(&ast, &empty_constants()).unwrap_err();
        assert!(matches!(err, CompileError::ArityMismatch { .. }));
    }

    #[test]
    fn fold_sum_basic() {
        // Σ_{k=1}^{5} k
        let ast = Expression::Sum {
            index: "k".into(),
            lower: Box::new(int(1)),
            upper: Box::new(int(5)),
            body: Box::new(var("k")),
        };
        let expr = fold(&ast, &empty_constants()).unwrap();
        assert!(matches!(
            expr.root,
            CompiledNode::Sum {
                lower: 1,
                upper: 5,
                ..
            }
        ));
        // k should be an index, not an argument
        assert!(expr.argument_names.is_empty());
    }

    #[test]
    fn fold_sum_index_shadows_variable() {
        // If x is both a free var and a sum index, the sum body uses the index
        let ast = Expression::Binary {
            op: mathlex::BinaryOp::Add,
            left: Box::new(var("x")), // free variable
            right: Box::new(Expression::Sum {
                index: "x".into(), // shadows the free x
                lower: Box::new(int(1)),
                upper: Box::new(int(3)),
                body: Box::new(var("x")), // refers to index, not argument
            }),
        };
        let expr = fold(&ast, &empty_constants()).unwrap();
        assert_eq!(expr.argument_names(), &["x"]);
        // The sum body should use Index, not Argument
        if let CompiledNode::Binary { right, .. } = &expr.root {
            if let CompiledNode::Sum { body, .. } = right.as_ref() {
                assert!(matches!(body.as_ref(), CompiledNode::Index(_)));
            } else {
                panic!("expected Sum");
            }
        } else {
            panic!("expected Binary");
        }
    }

    #[test]
    fn fold_sum_non_integer_bounds_error() {
        let ast = Expression::Sum {
            index: "k".into(),
            lower: Box::new(float(1.5)),
            upper: Box::new(int(5)),
            body: Box::new(var("k")),
        };
        let err = fold(&ast, &empty_constants()).unwrap_err();
        assert!(matches!(err, CompileError::NonIntegerBounds { .. }));
    }

    #[test]
    fn fold_rational() {
        let ast = Expression::Rational {
            numerator: Box::new(int(3)),
            denominator: Box::new(int(4)),
        };
        let expr = fold(&ast, &empty_constants()).unwrap();
        if let CompiledNode::Literal(v) = expr.root {
            assert_abs_diff_eq!(v, 0.75, epsilon = 1e-15);
        } else {
            panic!("expected folded literal");
        }
    }

    #[test]
    fn fold_function_with_literal_args_folded() {
        // sin(0) should fold to 0
        let ast = Expression::Function {
            name: "sin".into(),
            args: vec![int(0)],
        };
        let expr = fold(&ast, &empty_constants()).unwrap();
        if let CompiledNode::Literal(v) = expr.root {
            assert_abs_diff_eq!(v, 0.0, epsilon = 1e-15);
        } else {
            panic!("expected folded literal");
        }
    }

    #[test]
    fn fold_function_with_variable_args_not_folded() {
        let ast = Expression::Function {
            name: "sin".into(),
            args: vec![var("x")],
        };
        let expr = fold(&ast, &empty_constants()).unwrap();
        assert!(matches!(expr.root, CompiledNode::Function { .. }));
    }

    #[test]
    fn fold_factorial() {
        let ast = Expression::Unary {
            op: mathlex::UnaryOp::Factorial,
            operand: Box::new(int(5)),
        };
        let expr = fold(&ast, &empty_constants()).unwrap();
        if let CompiledNode::Literal(v) = expr.root {
            assert_abs_diff_eq!(v, 120.0, epsilon = 1e-10);
        } else {
            panic!("expected folded literal");
        }
    }

    #[test]
    fn fold_negation() {
        let ast = Expression::Unary {
            op: mathlex::UnaryOp::Neg,
            operand: Box::new(int(5)),
        };
        let expr = fold(&ast, &empty_constants()).unwrap();
        assert!(matches!(expr.root, CompiledNode::Literal(v) if v == -5.0));
    }

    #[test]
    fn fold_pos_is_identity() {
        let ast = Expression::Unary {
            op: mathlex::UnaryOp::Pos,
            operand: Box::new(int(5)),
        };
        let expr = fold(&ast, &empty_constants()).unwrap();
        assert!(matches!(expr.root, CompiledNode::Literal(v) if v == 5.0));
    }

    #[test]
    fn fold_complex_constant_sets_flag() {
        let mut constants = HashMap::new();
        constants.insert(
            "z",
            NumericResult::Complex(num_complex::Complex::new(1.0, 2.0)),
        );
        let expr = fold(&var("z"), &constants).unwrap();
        assert!(expr.is_complex());
    }
}
