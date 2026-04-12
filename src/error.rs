use thiserror::Error;

/// Errors that occur during AST compilation.
#[derive(Debug, Error)]
pub enum CompileError {
    #[error("unsupported expression variant '{variant}': {context}")]
    UnsupportedExpression {
        variant: &'static str,
        context: String,
    },

    #[error("unresolved variable '{name}'")]
    UnresolvedVariable { name: String },

    #[error("{construct} bounds must be integer: {bound}")]
    NonIntegerBounds {
        construct: &'static str,
        bound: String,
    },

    #[error("unknown function '{name}'")]
    UnknownFunction { name: String },

    #[error("function '{function}' expects {expected} args, got {got}")]
    ArityMismatch {
        function: String,
        expected: usize,
        got: usize,
    },

    #[error("division by zero during constant folding")]
    DivisionByZero,

    #[error("numeric overflow during constant folding: {context}")]
    NumericOverflow { context: String },
}

/// Errors that occur during expression evaluation.
#[derive(Debug, Error)]
pub enum EvalError {
    #[error("unknown argument '{name}'")]
    UnknownArgument { name: String },

    #[error("missing argument '{name}'")]
    MissingArgument { name: String },

    #[error("division by zero")]
    DivisionByZero,

    #[error("numeric overflow")]
    NumericOverflow,

    #[error("incompatible array shapes: {details}")]
    ShapeMismatch { details: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_error_unsupported_expression_display() {
        let err = CompileError::UnsupportedExpression {
            variant: "Vector",
            context: "vector evaluation deferred to v1.x".into(),
        };
        assert_eq!(
            err.to_string(),
            "unsupported expression variant 'Vector': vector evaluation deferred to v1.x"
        );
    }

    #[test]
    fn compile_error_unresolved_variable_display() {
        let err = CompileError::UnresolvedVariable { name: "x".into() };
        assert_eq!(err.to_string(), "unresolved variable 'x'");
    }

    #[test]
    fn compile_error_non_integer_bounds_display() {
        let err = CompileError::NonIntegerBounds {
            construct: "sum",
            bound: "x + 1".into(),
        };
        assert_eq!(err.to_string(), "sum bounds must be integer: x + 1");
    }

    #[test]
    fn compile_error_unknown_function_display() {
        let err = CompileError::UnknownFunction {
            name: "foobar".into(),
        };
        assert_eq!(err.to_string(), "unknown function 'foobar'");
    }

    #[test]
    fn compile_error_arity_mismatch_display() {
        let err = CompileError::ArityMismatch {
            function: "sin".into(),
            expected: 1,
            got: 2,
        };
        assert_eq!(err.to_string(), "function 'sin' expects 1 args, got 2");
    }

    #[test]
    fn compile_error_division_by_zero_display() {
        let err = CompileError::DivisionByZero;
        assert_eq!(err.to_string(), "division by zero during constant folding");
    }

    #[test]
    fn compile_error_numeric_overflow_display() {
        let err = CompileError::NumericOverflow {
            context: "2^1024".into(),
        };
        assert_eq!(
            err.to_string(),
            "numeric overflow during constant folding: 2^1024"
        );
    }

    #[test]
    fn eval_error_unknown_argument_display() {
        let err = EvalError::UnknownArgument { name: "z".into() };
        assert_eq!(err.to_string(), "unknown argument 'z'");
    }

    #[test]
    fn eval_error_missing_argument_display() {
        let err = EvalError::MissingArgument { name: "x".into() };
        assert_eq!(err.to_string(), "missing argument 'x'");
    }

    #[test]
    fn eval_error_division_by_zero_display() {
        let err = EvalError::DivisionByZero;
        assert_eq!(err.to_string(), "division by zero");
    }

    #[test]
    fn eval_error_numeric_overflow_display() {
        let err = EvalError::NumericOverflow;
        assert_eq!(err.to_string(), "numeric overflow");
    }

    #[test]
    fn eval_error_shape_mismatch_display() {
        let err = EvalError::ShapeMismatch {
            details: "[3] vs [4]".into(),
        };
        assert_eq!(err.to_string(), "incompatible array shapes: [3] vs [4]");
    }

    #[test]
    fn errors_implement_std_error() {
        fn assert_error<E: std::error::Error>() {}
        assert_error::<CompileError>();
        assert_error::<EvalError>();
    }
}
