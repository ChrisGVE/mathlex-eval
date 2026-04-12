use criterion::{Criterion, criterion_group, criterion_main};

fn placeholder_bench(_c: &mut Criterion) {}

criterion_group!(benches, placeholder_bench);
criterion_main!(benches);
