use std::collections::HashMap;

use mathlex::Expression;

use crate::compiler::fold;
use crate::compiler::ir::CompiledExpr;
use crate::compiler::validate;
use crate::error::CompileError;
use crate::eval::numeric::NumericResult;

/// Compile a mathlex AST into a [`CompiledExpr`] ready for evaluation.
///
/// Takes a reference to the AST and a map of constant names to values.
/// Constants are substituted at compile time; remaining free variables
/// become arguments that must be provided at eval time.
///
/// # Errors
///
/// Returns [`CompileError`] if the AST contains unsupported expression
/// variants, unknown functions, arity mismatches, unresolvable bounds,
/// or division by zero during constant folding.
pub fn compile(
    ast: &Expression,
    constants: &HashMap<&str, NumericResult>,
) -> Result<CompiledExpr, CompileError> {
    validate::validate(ast)?;
    fold::fold(ast, constants)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;
    use mathlex::{BinaryOp, MathConstant, UnaryOp};

    use crate::compiler::ir::CompiledNode;

    fn int(v: i64) -> Expression {
        Expression::Integer(v)
    }

    fn var(name: &str) -> Expression {
        Expression::Variable(name.into())
    }

    #[test]
    fn compile_simple_expression() {
        // x + 1
        let ast = Expression::Binary {
            op: BinaryOp::Add,
            left: Box::new(var("x")),
            right: Box::new(int(1)),
        };
        let compiled = compile(&ast, &HashMap::new()).unwrap();
        assert_eq!(compiled.argument_names(), &["x"]);
        assert!(!compiled.is_complex());
    }

    #[test]
    fn compile_with_constants() {
        // a * x where a = 2.0
        let ast = Expression::Binary {
            op: BinaryOp::Mul,
            left: Box::new(var("a")),
            right: Box::new(var("x")),
        };
        let mut constants = HashMap::new();
        constants.insert("a", NumericResult::Real(2.0));
        let compiled = compile(&ast, &constants).unwrap();
        assert_eq!(compiled.argument_names(), &["x"]);
    }

    #[test]
    fn compile_pure_constant_folds() {
        // 2 * pi → single literal
        let ast = Expression::Binary {
            op: BinaryOp::Mul,
            left: Box::new(int(2)),
            right: Box::new(Expression::Constant(MathConstant::Pi)),
        };
        let compiled = compile(&ast, &HashMap::new()).unwrap();
        if let CompiledNode::Literal(v) = compiled.root {
            assert_abs_diff_eq!(v, 2.0 * std::f64::consts::PI, epsilon = 1e-15);
        } else {
            panic!("expected folded literal");
        }
    }

    #[test]
    fn compile_rejects_vector() {
        let ast = Expression::Vector(vec![int(1)]);
        let err = compile(&ast, &HashMap::new()).unwrap_err();
        assert!(matches!(err, CompileError::UnsupportedExpression { .. }));
    }

    #[test]
    fn compile_rejects_derivative() {
        let ast = Expression::Derivative {
            expr: Box::new(var("x")),
            var: "x".into(),
            order: 1,
        };
        let err = compile(&ast, &HashMap::new()).unwrap_err();
        assert!(matches!(err, CompileError::UnsupportedExpression { .. }));
    }

    #[test]
    fn compile_complex_constant_sets_flag() {
        let ast = Expression::Constant(MathConstant::I);
        let compiled = compile(&ast, &HashMap::new()).unwrap();
        assert!(compiled.is_complex());
    }

    #[test]
    fn compile_factorial() {
        let ast = Expression::Unary {
            op: UnaryOp::Factorial,
            operand: Box::new(int(5)),
        };
        let compiled = compile(&ast, &HashMap::new()).unwrap();
        if let CompiledNode::Literal(v) = compiled.root {
            assert_abs_diff_eq!(v, 120.0, epsilon = 1e-10);
        } else {
            panic!("expected folded literal");
        }
    }

    #[test]
    fn compile_sum() {
        let ast = Expression::Sum {
            index: "k".into(),
            lower: Box::new(int(1)),
            upper: Box::new(int(10)),
            body: Box::new(var("k")),
        };
        let compiled = compile(&ast, &HashMap::new()).unwrap();
        assert!(compiled.argument_names().is_empty());
    }
}
