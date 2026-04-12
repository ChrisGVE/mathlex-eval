/// Compiled expression ready for repeated evaluation.
///
/// Opaque to callers — only inspectable via [`argument_names`](Self::argument_names)
/// and [`is_complex`](Self::is_complex).
#[derive(Debug, Clone)]
pub struct CompiledExpr {
    pub(crate) root: CompiledNode,
    pub(crate) argument_names: Vec<String>,
    pub(crate) is_complex: bool,
}

impl CompiledExpr {
    /// Names of free variables (arguments) in declaration order.
    pub fn argument_names(&self) -> &[String] {
        &self.argument_names
    }

    /// Whether the expression contains complex literals or imaginary unit.
    pub fn is_complex(&self) -> bool {
        self.is_complex
    }
}

/// Internal IR node — not exposed in public API.
#[derive(Debug, Clone)]
pub(crate) enum CompiledNode {
    /// Real literal (constant-folded)
    Literal(f64),
    /// Complex literal (constant-folded)
    ComplexLiteral { re: f64, im: f64 },
    /// Free variable, index into argument values by declaration order
    Argument(usize),
    /// Bound variable from sum/product loop, separate index space from arguments
    Index(usize),

    /// Binary arithmetic operation
    Binary {
        op: BinaryOp,
        left: Box<CompiledNode>,
        right: Box<CompiledNode>,
    },
    /// Unary operation
    Unary {
        op: UnaryOp,
        operand: Box<CompiledNode>,
    },

    /// Built-in math function call
    Function {
        kind: BuiltinFn,
        args: Vec<CompiledNode>,
    },

    /// Finite summation: Σ_{index=lower}^{upper} body
    Sum {
        index: usize,
        lower: i64,
        upper: i64,
        body: Box<CompiledNode>,
    },
    /// Finite product: Π_{index=lower}^{upper} body
    Product {
        index: usize,
        lower: i64,
        upper: i64,
        body: Box<CompiledNode>,
    },
}

/// Binary arithmetic operators supported by the evaluator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Mod,
}

/// Unary operators supported by the evaluator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnaryOp {
    Neg,
    Factorial,
}

/// Built-in math functions recognized by the compiler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BuiltinFn {
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Atan2,
    Sinh,
    Cosh,
    Tanh,
    Exp,
    Ln,
    Log2,
    Log10,
    /// log(base, value)
    Log,
    Sqrt,
    Cbrt,
    Abs,
    Floor,
    Ceil,
    Round,
    Min,
    Max,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiled_expr_argument_names() {
        let expr = CompiledExpr {
            root: CompiledNode::Literal(1.0),
            argument_names: vec!["x".into(), "y".into()],
            is_complex: false,
        };
        assert_eq!(expr.argument_names(), &["x", "y"]);
    }

    #[test]
    fn compiled_expr_is_complex() {
        let expr = CompiledExpr {
            root: CompiledNode::ComplexLiteral { re: 0.0, im: 1.0 },
            argument_names: vec![],
            is_complex: true,
        };
        assert!(expr.is_complex());
    }

    #[test]
    fn compiled_node_variants_constructible() {
        // Verify all variants can be constructed
        let _ = CompiledNode::Literal(1.0);
        let _ = CompiledNode::ComplexLiteral { re: 1.0, im: 2.0 };
        let _ = CompiledNode::Argument(0);
        let _ = CompiledNode::Index(0);
        let _ = CompiledNode::Binary {
            op: BinaryOp::Add,
            left: Box::new(CompiledNode::Literal(1.0)),
            right: Box::new(CompiledNode::Literal(2.0)),
        };
        let _ = CompiledNode::Unary {
            op: UnaryOp::Neg,
            operand: Box::new(CompiledNode::Literal(1.0)),
        };
        let _ = CompiledNode::Function {
            kind: BuiltinFn::Sin,
            args: vec![CompiledNode::Literal(0.0)],
        };
        let _ = CompiledNode::Sum {
            index: 0,
            lower: 1,
            upper: 10,
            body: Box::new(CompiledNode::Index(0)),
        };
        let _ = CompiledNode::Product {
            index: 0,
            lower: 1,
            upper: 5,
            body: Box::new(CompiledNode::Index(0)),
        };
    }

    #[test]
    fn binary_op_all_variants() {
        let ops = [
            BinaryOp::Add,
            BinaryOp::Sub,
            BinaryOp::Mul,
            BinaryOp::Div,
            BinaryOp::Pow,
            BinaryOp::Mod,
        ];
        assert_eq!(ops.len(), 6);
    }

    #[test]
    fn unary_op_all_variants() {
        let ops = [UnaryOp::Neg, UnaryOp::Factorial];
        assert_eq!(ops.len(), 2);
    }

    #[test]
    fn builtin_fn_all_variants() {
        let fns = [
            BuiltinFn::Sin,
            BuiltinFn::Cos,
            BuiltinFn::Tan,
            BuiltinFn::Asin,
            BuiltinFn::Acos,
            BuiltinFn::Atan,
            BuiltinFn::Atan2,
            BuiltinFn::Sinh,
            BuiltinFn::Cosh,
            BuiltinFn::Tanh,
            BuiltinFn::Exp,
            BuiltinFn::Ln,
            BuiltinFn::Log2,
            BuiltinFn::Log10,
            BuiltinFn::Log,
            BuiltinFn::Sqrt,
            BuiltinFn::Cbrt,
            BuiltinFn::Abs,
            BuiltinFn::Floor,
            BuiltinFn::Ceil,
            BuiltinFn::Round,
            BuiltinFn::Min,
            BuiltinFn::Max,
        ];
        assert_eq!(fns.len(), 23);
    }
}
