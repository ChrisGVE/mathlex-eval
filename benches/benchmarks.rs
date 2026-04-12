use std::collections::HashMap;
use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use mathlex::{BinaryOp, Expression, UnaryOp};

use mathlex_eval::{EvalInput, NumericResult, compile, eval};

// ---------------------------------------------------------------------------
// AST construction helpers
// ---------------------------------------------------------------------------

/// `2*x + 3`
fn ast_linear() -> Expression {
    Expression::Binary {
        op: BinaryOp::Add,
        left: Box::new(Expression::Binary {
            op: BinaryOp::Mul,
            left: Box::new(Expression::Integer(2)),
            right: Box::new(Expression::Variable("x".into())),
        }),
        right: Box::new(Expression::Integer(3)),
    }
}

/// `sin(x^2 + cos(x))`
fn ast_trig_complex() -> Expression {
    let x_sq = Expression::Binary {
        op: BinaryOp::Pow,
        left: Box::new(Expression::Variable("x".into())),
        right: Box::new(Expression::Integer(2)),
    };
    let cos_x = Expression::Function {
        name: "cos".into(),
        args: vec![Expression::Variable("x".into())],
    };
    let inner = Expression::Binary {
        op: BinaryOp::Add,
        left: Box::new(x_sq),
        right: Box::new(cos_x),
    };
    Expression::Function {
        name: "sin".into(),
        args: vec![inner],
    }
}

/// `x^2 + 1`
fn ast_quadratic() -> Expression {
    Expression::Binary {
        op: BinaryOp::Add,
        left: Box::new(Expression::Binary {
            op: BinaryOp::Pow,
            left: Box::new(Expression::Variable("x".into())),
            right: Box::new(Expression::Integer(2)),
        }),
        right: Box::new(Expression::Integer(1)),
    }
}

/// `x^2 + y`
fn ast_grid() -> Expression {
    Expression::Binary {
        op: BinaryOp::Add,
        left: Box::new(Expression::Binary {
            op: BinaryOp::Pow,
            left: Box::new(Expression::Variable("x".into())),
            right: Box::new(Expression::Integer(2)),
        }),
        right: Box::new(Expression::Variable("y".into())),
    }
}

/// `-(x)` — unary negation, used to verify compile path for unary ops
fn ast_negation() -> Expression {
    Expression::Unary {
        op: UnaryOp::Neg,
        operand: Box::new(Expression::Variable("x".into())),
    }
}

// ---------------------------------------------------------------------------
// Helper: build a Vec<f64> of `n` linearly spaced points in [0.0, 1.0)
// ---------------------------------------------------------------------------

fn linspace(n: usize) -> Vec<f64> {
    (0..n).map(|i| i as f64 / n as f64).collect()
}

// ---------------------------------------------------------------------------
// Benchmark 1: single-point scalar evaluation
// ---------------------------------------------------------------------------

fn bench_scalar_eval(c: &mut Criterion) {
    let consts: HashMap<&str, NumericResult> = HashMap::new();

    let compiled_linear = compile(&ast_linear(), &consts).expect("compile linear");
    let compiled_trig = compile(&ast_trig_complex(), &consts).expect("compile trig");

    let mut group = c.benchmark_group("scalar_eval");

    group.bench_function("linear_2x_plus_3", |b| {
        b.iter(|| {
            let mut args = HashMap::new();
            args.insert("x", EvalInput::Scalar(black_box(1.5)));
            let handle = eval(black_box(&compiled_linear), args).expect("eval");
            black_box(handle.scalar().expect("scalar"))
        });
    });

    group.bench_function("complex_sin_x2_plus_cos_x", |b| {
        b.iter(|| {
            let mut args = HashMap::new();
            args.insert("x", EvalInput::Scalar(black_box(0.7)));
            let handle = eval(black_box(&compiled_trig), args).expect("eval");
            black_box(handle.scalar().expect("scalar"))
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Benchmark 2: compile cost vs eval cost (separated phases)
// ---------------------------------------------------------------------------

fn bench_compile_vs_eval(c: &mut Criterion) {
    let consts: HashMap<&str, NumericResult> = HashMap::new();
    let compiled_trig = compile(&ast_trig_complex(), &consts).expect("compile");

    let mut group = c.benchmark_group("compile_vs_eval");

    // Measure compile alone
    group.bench_function("compile_sin_x2_plus_cos_x", |b| {
        b.iter(|| {
            let ast = black_box(ast_trig_complex());
            black_box(compile(&ast, &consts).expect("compile"))
        });
    });

    // Measure compile of a simpler negation expression
    group.bench_function("compile_negation", |b| {
        b.iter(|| {
            let ast = black_box(ast_negation());
            black_box(compile(&ast, &consts).expect("compile"))
        });
    });

    // Measure eval alone (compile amortised outside loop)
    group.bench_function("eval_only_sin_x2_plus_cos_x", |b| {
        b.iter(|| {
            let mut args = HashMap::new();
            args.insert("x", EvalInput::Scalar(black_box(0.7)));
            let handle = eval(black_box(&compiled_trig), args).expect("eval");
            black_box(handle.scalar().expect("scalar"))
        });
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Benchmark 3: broadcasting — x^2+1 over 1-D arrays of increasing length
// ---------------------------------------------------------------------------

fn bench_broadcast_1d(c: &mut Criterion) {
    let consts: HashMap<&str, NumericResult> = HashMap::new();
    let compiled = compile(&ast_quadratic(), &consts).expect("compile quadratic");

    let sizes: &[usize] = &[10, 100, 1_000, 10_000, 100_000];

    let mut group = c.benchmark_group("broadcast_1d");

    for &n in sizes {
        let xs = linspace(n);

        group.bench_with_input(BenchmarkId::from_parameter(n), &xs, |b, xs| {
            b.iter(|| {
                let mut args = HashMap::new();
                args.insert("x", EvalInput::from(black_box(xs.clone())));
                let handle = eval(black_box(&compiled), args).expect("eval");
                black_box(handle.to_array().expect("to_array"))
            });
        });
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// Benchmark 4: grid broadcasting — x^2+y over N×M Cartesian grids
// ---------------------------------------------------------------------------

fn bench_broadcast_grid(c: &mut Criterion) {
    let consts: HashMap<&str, NumericResult> = HashMap::new();
    let compiled = compile(&ast_grid(), &consts).expect("compile grid");

    // (N, M) pairs — total elements: 100, 10_000
    let grids: &[(usize, usize)] = &[(10, 10), (100, 100)];

    let mut group = c.benchmark_group("broadcast_grid");

    for &(nx, ny) in grids {
        let xs = linspace(nx);
        let ys = linspace(ny);
        let label = format!("{nx}x{ny}");

        group.bench_with_input(
            BenchmarkId::new("grid", &label),
            &(xs, ys),
            |b, (xs, ys)| {
                b.iter(|| {
                    let mut args = HashMap::new();
                    args.insert("x", EvalInput::from(black_box(xs.clone())));
                    args.insert("y", EvalInput::from(black_box(ys.clone())));
                    let handle = eval(black_box(&compiled), args).expect("eval");
                    black_box(handle.to_array().expect("to_array"))
                });
            },
        );
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// Criterion wiring
// ---------------------------------------------------------------------------

criterion_group!(
    benches,
    bench_scalar_eval,
    bench_compile_vs_eval,
    bench_broadcast_1d,
    bench_broadcast_grid,
);
criterion_main!(benches);
