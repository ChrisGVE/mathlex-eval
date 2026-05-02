use mathlex::{ExprKind, Expression};

use crate::error::CompileError;

/// Validate that an AST contains only numerically evaluable expressions.
///
/// Walks the AST recursively. Returns `Ok(())` if every node is a supported
/// variant, or `Err(CompileError::UnsupportedExpression)` on first unsupported node.
pub(crate) fn validate(ast: &Expression) -> Result<(), CompileError> {
    match &ast.kind {
        // Leaf nodes — always valid
        ExprKind::Integer(_)
        | ExprKind::Float(_)
        | ExprKind::Variable(_)
        | ExprKind::Constant(_) => Ok(()),

        // Compound nodes — validate children recursively
        ExprKind::Rational {
            numerator,
            denominator,
        } => validate(numerator).and(validate(denominator)),

        ExprKind::Complex { real, imaginary } => validate(real).and(validate(imaginary)),

        ExprKind::Binary { op, left, right } => {
            validate_binary_op(*op)?;
            validate(left).and(validate(right))
        }

        ExprKind::Unary { op, operand } => {
            validate_unary_op(*op)?;
            validate(operand)
        }

        ExprKind::Function { args, .. } => args.iter().try_for_each(validate),

        ExprKind::Sum {
            lower, upper, body, ..
        }
        | ExprKind::Product {
            lower, upper, body, ..
        } => {
            validate(lower)?;
            validate(upper).and(validate(body))
        }

        // All non-numerical variants
        other => Err(reject_kind(other)),
    }
}

/// Map unsupported ExprKind variants to descriptive errors.
fn reject_kind(kind: &ExprKind) -> CompileError {
    match kind {
        ExprKind::Quaternion { .. } => unsupported("Quaternion", "not supported in v1"),
        ExprKind::Vector(_) => unsupported("Vector", "deferred to v1.x"),
        ExprKind::Matrix(_) => unsupported("Matrix", "deferred to v1.x"),
        ExprKind::Equation { .. } => unsupported("Equation", "relational, not numerical"),
        ExprKind::Inequality { .. } => unsupported("Inequality", "relational, not numerical"),
        ExprKind::Derivative { .. } => unsupported("Derivative", "requires symbolic engine"),
        ExprKind::PartialDerivative { .. } => {
            unsupported("PartialDerivative", "requires symbolic engine")
        }
        ExprKind::Integral { .. } => unsupported("Integral", "requires symbolic engine"),
        ExprKind::MultipleIntegral { .. } => {
            unsupported("MultipleIntegral", "requires symbolic engine")
        }
        ExprKind::ClosedIntegral { .. } => {
            unsupported("ClosedIntegral", "requires symbolic engine")
        }
        ExprKind::Limit { .. } => unsupported("Limit", "requires symbolic engine"),
        ExprKind::ForAll { .. } => unsupported("ForAll", "logical quantifier"),
        ExprKind::Exists { .. } => unsupported("Exists", "logical quantifier"),
        ExprKind::Logical { .. } => unsupported("Logical", "logical expression"),
        ExprKind::MarkedVector { .. } => unsupported("MarkedVector", "vector notation"),
        ExprKind::DotProduct { .. } => unsupported("DotProduct", "deferred to v1.x"),
        ExprKind::CrossProduct { .. } => unsupported("CrossProduct", "deferred to v1.x"),
        ExprKind::OuterProduct { .. } => unsupported("OuterProduct", "deferred to v1.x"),
        ExprKind::Gradient { .. } => unsupported("Gradient", "requires symbolic engine"),
        ExprKind::Divergence { .. } => unsupported("Divergence", "requires symbolic engine"),
        ExprKind::Curl { .. } => unsupported("Curl", "requires symbolic engine"),
        ExprKind::Laplacian { .. } => unsupported("Laplacian", "requires symbolic engine"),
        ExprKind::Nabla => unsupported("Nabla", "operator without operand"),
        ExprKind::Determinant { .. } => unsupported("Determinant", "deferred to v1.x"),
        ExprKind::Trace { .. } => unsupported("Trace", "deferred to v1.x"),
        ExprKind::Rank { .. } => unsupported("Rank", "deferred to v1.x"),
        ExprKind::ConjugateTranspose { .. } => {
            unsupported("ConjugateTranspose", "deferred to v1.x")
        }
        ExprKind::MatrixInverse { .. } => unsupported("MatrixInverse", "deferred to v1.x"),
        ExprKind::NumberSetExpr(_) => unsupported("NumberSetExpr", "set theory"),
        ExprKind::SetOperation { .. } => unsupported("SetOperation", "set theory"),
        ExprKind::SetRelationExpr { .. } => unsupported("SetRelationExpr", "set theory"),
        ExprKind::SetBuilder { .. } => unsupported("SetBuilder", "set theory"),
        ExprKind::EmptySet => unsupported("EmptySet", "set theory"),
        ExprKind::PowerSet { .. } => unsupported("PowerSet", "set theory"),
        ExprKind::Tensor { .. } => unsupported("Tensor", "tensor notation"),
        ExprKind::KroneckerDelta { .. } => unsupported("KroneckerDelta", "tensor notation"),
        ExprKind::LeviCivita { .. } => unsupported("LeviCivita", "tensor notation"),
        ExprKind::FunctionSignature { .. } => unsupported("FunctionSignature", "type declaration"),
        ExprKind::Composition { .. } => unsupported("Composition", "function composition"),
        ExprKind::Differential { .. } => unsupported("Differential", "differential form"),
        ExprKind::WedgeProduct { .. } => unsupported("WedgeProduct", "differential form"),
        ExprKind::Relation { .. } => unsupported("Relation", "relational, not numerical"),
        // Supported variants already matched in validate() — unreachable here
        _ => unreachable!("supported variant reached reject_kind()"),
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
    use mathlex::{BinaryOp, ExprKind, MathConstant, MathFloat, UnaryOp};

    fn int(v: i64) -> Expression {
        Expression::integer(v)
    }

    fn var(name: &str) -> Expression {
        Expression::variable(name)
    }

    // --- Accepted variants ---

    #[test]
    fn accept_integer() {
        assert!(validate(&Expression::integer(42)).is_ok());
    }

    #[test]
    fn accept_float() {
        assert!(validate(&Expression::float(MathFloat::from(2.75))).is_ok());
    }

    #[test]
    fn accept_variable() {
        assert!(validate(&var("x")).is_ok());
    }

    #[test]
    fn accept_constant() {
        assert!(validate(&Expression::constant(MathConstant::Pi)).is_ok());
    }

    #[test]
    fn accept_rational() {
        let r = ExprKind::Rational {
            numerator: Box::new(int(3)),
            denominator: Box::new(int(4)),
        }
        .into();
        assert!(validate(&r).is_ok());
    }

    #[test]
    fn accept_complex() {
        let c = ExprKind::Complex {
            real: Box::new(int(1)),
            imaginary: Box::new(int(2)),
        }
        .into();
        assert!(validate(&c).is_ok());
    }

    #[test]
    fn accept_binary_add() {
        let b = ExprKind::Binary {
            op: BinaryOp::Add,
            left: Box::new(int(1)),
            right: Box::new(int(2)),
        }
        .into();
        assert!(validate(&b).is_ok());
    }

    #[test]
    fn accept_unary_neg() {
        let u = ExprKind::Unary {
            op: UnaryOp::Neg,
            operand: Box::new(int(1)),
        }
        .into();
        assert!(validate(&u).is_ok());
    }

    #[test]
    fn accept_unary_factorial() {
        let u = ExprKind::Unary {
            op: UnaryOp::Factorial,
            operand: Box::new(int(5)),
        }
        .into();
        assert!(validate(&u).is_ok());
    }

    #[test]
    fn accept_unary_pos() {
        let u = ExprKind::Unary {
            op: UnaryOp::Pos,
            operand: Box::new(int(1)),
        }
        .into();
        assert!(validate(&u).is_ok());
    }

    #[test]
    fn accept_function() {
        let f = ExprKind::Function {
            name: "sin".into(),
            args: vec![var("x")],
        }
        .into();
        assert!(validate(&f).is_ok());
    }

    #[test]
    fn accept_sum() {
        let s = ExprKind::Sum {
            index: "k".into(),
            lower: Box::new(int(1)),
            upper: Box::new(int(10)),
            body: Box::new(var("k")),
        }
        .into();
        assert!(validate(&s).is_ok());
    }

    #[test]
    fn accept_product() {
        let p = ExprKind::Product {
            index: "k".into(),
            lower: Box::new(int(1)),
            upper: Box::new(int(5)),
            body: Box::new(var("k")),
        }
        .into();
        assert!(validate(&p).is_ok());
    }

    // --- Nested validation ---

    #[test]
    fn reject_vector_nested_in_binary() {
        let b = ExprKind::Binary {
            op: BinaryOp::Add,
            left: Box::new(int(1)),
            right: Box::new(Expression::vector(vec![int(1), int(2)])),
        }
        .into();
        let err = validate(&b).unwrap_err();
        assert!(err.to_string().contains("Vector"));
    }

    // --- Rejected variants ---

    #[test]
    fn reject_vector() {
        let v = Expression::vector(vec![int(1)]);
        let err = validate(&v).unwrap_err();
        assert!(err.to_string().contains("Vector"));
    }

    #[test]
    fn reject_matrix() {
        let m = Expression::matrix(vec![vec![int(1)]]);
        let err = validate(&m).unwrap_err();
        assert!(err.to_string().contains("Matrix"));
    }

    #[test]
    fn reject_derivative() {
        let d = ExprKind::Derivative {
            expr: Box::new(var("x")),
            var: "x".into(),
            order: 1,
        }
        .into();
        let err = validate(&d).unwrap_err();
        assert!(err.to_string().contains("Derivative"));
    }

    #[test]
    fn reject_integral() {
        let i = ExprKind::Integral {
            integrand: Box::new(var("x")),
            var: "x".into(),
            bounds: None,
        }
        .into();
        let err = validate(&i).unwrap_err();
        assert!(err.to_string().contains("Integral"));
    }

    #[test]
    fn reject_limit() {
        let l = ExprKind::Limit {
            expr: Box::new(var("x")),
            var: "x".into(),
            to: Box::new(int(0)),
            direction: mathlex::Direction::Both,
        }
        .into();
        let err = validate(&l).unwrap_err();
        assert!(err.to_string().contains("Limit"));
    }

    #[test]
    fn reject_equation() {
        let e = ExprKind::Equation {
            left: Box::new(var("x")),
            right: Box::new(int(5)),
        }
        .into();
        let err = validate(&e).unwrap_err();
        assert!(err.to_string().contains("Equation"));
    }

    #[test]
    fn reject_nabla() {
        let err = validate(&Expression::nabla()).unwrap_err();
        assert!(err.to_string().contains("Nabla"));
    }

    #[test]
    fn reject_empty_set() {
        let err = validate(&Expression::empty_set()).unwrap_err();
        assert!(err.to_string().contains("EmptySet"));
    }

    #[test]
    fn reject_plus_minus_op() {
        let b = ExprKind::Binary {
            op: BinaryOp::PlusMinus,
            left: Box::new(int(1)),
            right: Box::new(int(2)),
        }
        .into();
        let err = validate(&b).unwrap_err();
        assert!(err.to_string().contains("PlusMinus"));
    }

    #[test]
    fn reject_transpose_op() {
        let u = ExprKind::Unary {
            op: UnaryOp::Transpose,
            operand: Box::new(int(1)),
        }
        .into();
        let err = validate(&u).unwrap_err();
        assert!(err.to_string().contains("Transpose"));
    }

    #[test]
    fn reject_quaternion() {
        let q = ExprKind::Quaternion {
            real: Box::new(int(1)),
            i: Box::new(int(0)),
            j: Box::new(int(0)),
            k: Box::new(int(0)),
        }
        .into();
        let err = validate(&q).unwrap_err();
        assert!(err.to_string().contains("Quaternion"));
    }
}
