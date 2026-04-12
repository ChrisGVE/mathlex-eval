use mathlex::Expression;

use crate::error::CompileError;

/// Validate that an AST contains only numerically evaluable expressions.
///
/// Walks the AST recursively. Returns `Ok(())` if every node is a supported
/// variant, or `Err(CompileError::UnsupportedExpression)` on first unsupported node.
pub(crate) fn validate(ast: &Expression) -> Result<(), CompileError> {
    match ast {
        // Leaf nodes — always valid
        Expression::Integer(_)
        | Expression::Float(_)
        | Expression::Variable(_)
        | Expression::Constant(_) => Ok(()),

        // Compound nodes — validate children recursively
        Expression::Rational {
            numerator,
            denominator,
        } => validate(numerator).and(validate(denominator)),

        Expression::Complex { real, imaginary } => validate(real).and(validate(imaginary)),

        Expression::Binary { op, left, right } => {
            validate_binary_op(*op)?;
            validate(left).and(validate(right))
        }

        Expression::Unary { op, operand } => {
            validate_unary_op(*op)?;
            validate(operand)
        }

        Expression::Function { args, .. } => args.iter().try_for_each(validate),

        Expression::Sum {
            lower, upper, body, ..
        }
        | Expression::Product {
            lower, upper, body, ..
        } => {
            validate(lower)?;
            validate(upper).and(validate(body))
        }

        // All non-numerical variants
        other => Err(reject(other)),
    }
}

/// Map unsupported Expression variants to descriptive errors.
fn reject(expr: &Expression) -> CompileError {
    match expr {
        Expression::Quaternion { .. } => unsupported("Quaternion", "not supported in v1"),
        Expression::Vector(_) => unsupported("Vector", "deferred to v1.x"),
        Expression::Matrix(_) => unsupported("Matrix", "deferred to v1.x"),
        Expression::Equation { .. } => unsupported("Equation", "relational, not numerical"),
        Expression::Inequality { .. } => unsupported("Inequality", "relational, not numerical"),
        Expression::Derivative { .. } => unsupported("Derivative", "requires symbolic engine"),
        Expression::PartialDerivative { .. } => {
            unsupported("PartialDerivative", "requires symbolic engine")
        }
        Expression::Integral { .. } => unsupported("Integral", "requires symbolic engine"),
        Expression::MultipleIntegral { .. } => {
            unsupported("MultipleIntegral", "requires symbolic engine")
        }
        Expression::ClosedIntegral { .. } => {
            unsupported("ClosedIntegral", "requires symbolic engine")
        }
        Expression::Limit { .. } => unsupported("Limit", "requires symbolic engine"),
        Expression::ForAll { .. } => unsupported("ForAll", "logical quantifier"),
        Expression::Exists { .. } => unsupported("Exists", "logical quantifier"),
        Expression::Logical { .. } => unsupported("Logical", "logical expression"),
        Expression::MarkedVector { .. } => unsupported("MarkedVector", "vector notation"),
        Expression::DotProduct { .. } => unsupported("DotProduct", "deferred to v1.x"),
        Expression::CrossProduct { .. } => unsupported("CrossProduct", "deferred to v1.x"),
        Expression::OuterProduct { .. } => unsupported("OuterProduct", "deferred to v1.x"),
        Expression::Gradient { .. } => unsupported("Gradient", "requires symbolic engine"),
        Expression::Divergence { .. } => unsupported("Divergence", "requires symbolic engine"),
        Expression::Curl { .. } => unsupported("Curl", "requires symbolic engine"),
        Expression::Laplacian { .. } => unsupported("Laplacian", "requires symbolic engine"),
        Expression::Nabla => unsupported("Nabla", "operator without operand"),
        Expression::Determinant { .. } => unsupported("Determinant", "deferred to v1.x"),
        Expression::Trace { .. } => unsupported("Trace", "deferred to v1.x"),
        Expression::Rank { .. } => unsupported("Rank", "deferred to v1.x"),
        Expression::ConjugateTranspose { .. } => {
            unsupported("ConjugateTranspose", "deferred to v1.x")
        }
        Expression::MatrixInverse { .. } => unsupported("MatrixInverse", "deferred to v1.x"),
        Expression::NumberSetExpr(_) => unsupported("NumberSetExpr", "set theory"),
        Expression::SetOperation { .. } => unsupported("SetOperation", "set theory"),
        Expression::SetRelationExpr { .. } => unsupported("SetRelationExpr", "set theory"),
        Expression::SetBuilder { .. } => unsupported("SetBuilder", "set theory"),
        Expression::EmptySet => unsupported("EmptySet", "set theory"),
        Expression::PowerSet { .. } => unsupported("PowerSet", "set theory"),
        Expression::Tensor { .. } => unsupported("Tensor", "tensor notation"),
        Expression::KroneckerDelta { .. } => unsupported("KroneckerDelta", "tensor notation"),
        Expression::LeviCivita { .. } => unsupported("LeviCivita", "tensor notation"),
        Expression::FunctionSignature { .. } => {
            unsupported("FunctionSignature", "type declaration")
        }
        Expression::Composition { .. } => unsupported("Composition", "function composition"),
        Expression::Differential { .. } => unsupported("Differential", "differential form"),
        Expression::WedgeProduct { .. } => unsupported("WedgeProduct", "differential form"),
        Expression::Relation { .. } => unsupported("Relation", "relational, not numerical"),
        // Supported variants already matched in validate() — unreachable here
        _ => unreachable!("supported variant reached reject()"),
    }
}

fn unsupported(variant: &'static str, context: &str) -> CompileError {
    CompileError::UnsupportedExpression {
        variant,
        context: context.into(),
    }
}

fn validate_binary_op(op: mathlex::BinaryOp) -> Result<(), CompileError> {
    match op {
        mathlex::BinaryOp::Add
        | mathlex::BinaryOp::Sub
        | mathlex::BinaryOp::Mul
        | mathlex::BinaryOp::Div
        | mathlex::BinaryOp::Pow
        | mathlex::BinaryOp::Mod => Ok(()),
        mathlex::BinaryOp::PlusMinus => Err(unsupported(
            "BinaryOp::PlusMinus",
            "ambiguous ± not evaluable",
        )),
        mathlex::BinaryOp::MinusPlus => Err(unsupported(
            "BinaryOp::MinusPlus",
            "ambiguous ∓ not evaluable",
        )),
    }
}

fn validate_unary_op(op: mathlex::UnaryOp) -> Result<(), CompileError> {
    match op {
        mathlex::UnaryOp::Neg | mathlex::UnaryOp::Pos | mathlex::UnaryOp::Factorial => Ok(()),
        mathlex::UnaryOp::Transpose => Err(unsupported(
            "UnaryOp::Transpose",
            "matrix transpose deferred to v1.x",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mathlex::{BinaryOp, MathConstant, MathFloat, UnaryOp};

    fn int(v: i64) -> Expression {
        Expression::Integer(v)
    }

    fn var(name: &str) -> Expression {
        Expression::Variable(name.into())
    }

    // --- Accepted variants ---

    #[test]
    fn accept_integer() {
        assert!(validate(&Expression::Integer(42)).is_ok());
    }

    #[test]
    fn accept_float() {
        assert!(validate(&Expression::Float(MathFloat::from(2.75))).is_ok());
    }

    #[test]
    fn accept_variable() {
        assert!(validate(&var("x")).is_ok());
    }

    #[test]
    fn accept_constant() {
        assert!(validate(&Expression::Constant(MathConstant::Pi)).is_ok());
    }

    #[test]
    fn accept_rational() {
        let r = Expression::Rational {
            numerator: Box::new(int(3)),
            denominator: Box::new(int(4)),
        };
        assert!(validate(&r).is_ok());
    }

    #[test]
    fn accept_complex() {
        let c = Expression::Complex {
            real: Box::new(int(1)),
            imaginary: Box::new(int(2)),
        };
        assert!(validate(&c).is_ok());
    }

    #[test]
    fn accept_binary_add() {
        let b = Expression::Binary {
            op: BinaryOp::Add,
            left: Box::new(int(1)),
            right: Box::new(int(2)),
        };
        assert!(validate(&b).is_ok());
    }

    #[test]
    fn accept_unary_neg() {
        let u = Expression::Unary {
            op: UnaryOp::Neg,
            operand: Box::new(int(1)),
        };
        assert!(validate(&u).is_ok());
    }

    #[test]
    fn accept_unary_factorial() {
        let u = Expression::Unary {
            op: UnaryOp::Factorial,
            operand: Box::new(int(5)),
        };
        assert!(validate(&u).is_ok());
    }

    #[test]
    fn accept_unary_pos() {
        let u = Expression::Unary {
            op: UnaryOp::Pos,
            operand: Box::new(int(1)),
        };
        assert!(validate(&u).is_ok());
    }

    #[test]
    fn accept_function() {
        let f = Expression::Function {
            name: "sin".into(),
            args: vec![var("x")],
        };
        assert!(validate(&f).is_ok());
    }

    #[test]
    fn accept_sum() {
        let s = Expression::Sum {
            index: "k".into(),
            lower: Box::new(int(1)),
            upper: Box::new(int(10)),
            body: Box::new(var("k")),
        };
        assert!(validate(&s).is_ok());
    }

    #[test]
    fn accept_product() {
        let p = Expression::Product {
            index: "k".into(),
            lower: Box::new(int(1)),
            upper: Box::new(int(5)),
            body: Box::new(var("k")),
        };
        assert!(validate(&p).is_ok());
    }

    // --- Nested validation ---

    #[test]
    fn reject_vector_nested_in_binary() {
        let b = Expression::Binary {
            op: BinaryOp::Add,
            left: Box::new(int(1)),
            right: Box::new(Expression::Vector(vec![int(1), int(2)])),
        };
        let err = validate(&b).unwrap_err();
        assert!(err.to_string().contains("Vector"));
    }

    // --- Rejected variants ---

    #[test]
    fn reject_vector() {
        let v = Expression::Vector(vec![int(1)]);
        let err = validate(&v).unwrap_err();
        assert!(err.to_string().contains("Vector"));
    }

    #[test]
    fn reject_matrix() {
        let m = Expression::Matrix(vec![vec![int(1)]]);
        let err = validate(&m).unwrap_err();
        assert!(err.to_string().contains("Matrix"));
    }

    #[test]
    fn reject_derivative() {
        let d = Expression::Derivative {
            expr: Box::new(var("x")),
            var: "x".into(),
            order: 1,
        };
        let err = validate(&d).unwrap_err();
        assert!(err.to_string().contains("Derivative"));
    }

    #[test]
    fn reject_integral() {
        let i = Expression::Integral {
            integrand: Box::new(var("x")),
            var: "x".into(),
            bounds: None,
        };
        let err = validate(&i).unwrap_err();
        assert!(err.to_string().contains("Integral"));
    }

    #[test]
    fn reject_limit() {
        let l = Expression::Limit {
            expr: Box::new(var("x")),
            var: "x".into(),
            to: Box::new(int(0)),
            direction: mathlex::Direction::Both,
        };
        let err = validate(&l).unwrap_err();
        assert!(err.to_string().contains("Limit"));
    }

    #[test]
    fn reject_equation() {
        let e = Expression::Equation {
            left: Box::new(var("x")),
            right: Box::new(int(5)),
        };
        let err = validate(&e).unwrap_err();
        assert!(err.to_string().contains("Equation"));
    }

    #[test]
    fn reject_nabla() {
        let err = validate(&Expression::Nabla).unwrap_err();
        assert!(err.to_string().contains("Nabla"));
    }

    #[test]
    fn reject_empty_set() {
        let err = validate(&Expression::EmptySet).unwrap_err();
        assert!(err.to_string().contains("EmptySet"));
    }

    #[test]
    fn reject_plus_minus_op() {
        let b = Expression::Binary {
            op: BinaryOp::PlusMinus,
            left: Box::new(int(1)),
            right: Box::new(int(2)),
        };
        let err = validate(&b).unwrap_err();
        assert!(err.to_string().contains("PlusMinus"));
    }

    #[test]
    fn reject_transpose_op() {
        let u = Expression::Unary {
            op: UnaryOp::Transpose,
            operand: Box::new(int(1)),
        };
        let err = validate(&u).unwrap_err();
        assert!(err.to_string().contains("Transpose"));
    }

    #[test]
    fn reject_quaternion() {
        let q = Expression::Quaternion {
            real: Box::new(int(1)),
            i: Box::new(int(0)),
            j: Box::new(int(0)),
            k: Box::new(int(0)),
        };
        let err = validate(&q).unwrap_err();
        assert!(err.to_string().contains("Quaternion"));
    }
}
