use criterion::{criterion_group, criterion_main, Criterion};
use decimal64::Decimal64;
use std::hint::black_box;

const LHS_RAW: i64 = 1_234_567; // 123.4567 at scale 4
const RHS_RAW: i64 = 9_876_543; // 987.6543 at scale 4

fn bench_arithmetic(c: &mut Criterion) {
    let mut group = c.benchmark_group("arithmetic");

    let lhs = Decimal64::<4>::from_raw(LHS_RAW);
    let rhs = Decimal64::<4>::from_raw(RHS_RAW);

    group.bench_function("decimal64_add", |b| {
        b.iter(|| black_box(lhs) + black_box(rhs))
    });

    group.bench_function("i64_add", |b| {
        b.iter(|| black_box(LHS_RAW) + black_box(RHS_RAW))
    });

    group.bench_function("decimal64_mul", |b| {
        b.iter(|| black_box(lhs) * black_box(rhs))
    });

    group.bench_function("decimal64_div", |b| {
        b.iter(|| black_box(lhs) / black_box(rhs))
    });

    group.finish();
}

criterion_group!(arith_benches, bench_arithmetic);
criterion_main!(arith_benches);
