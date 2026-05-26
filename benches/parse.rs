use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use std::str::FromStr;

const CORPUS: &[&str] = &[
    "0",
    "1.23",
    "123.4567",
    "9999999999.9999",
    "-0.000001",
];

fn bench_parse_decimal64(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_decimal64");
    for s in CORPUS.iter().copied() {
        group.bench_function(s, |b| {
            b.iter(|| black_box(decimal64::Decimal64::<4>::parse(black_box(s))))
        });
    }
    group.finish();
}

fn bench_parse_f64(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_f64");
    for s in CORPUS.iter().copied() {
        group.bench_function(s, |b| {
            b.iter(|| black_box(f64::from_str(black_box(s))))
        });
    }
    group.finish();
}

fn bench_parse_rust_decimal(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_rust_decimal");
    for s in CORPUS.iter().copied() {
        group.bench_function(s, |b| {
            b.iter(|| black_box(rust_decimal::Decimal::from_str(black_box(s))))
        });
    }
    group.finish();
}

fn bench_parse_bigdecimal(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_bigdecimal");
    for s in CORPUS.iter().copied() {
        group.bench_function(s, |b| {
            b.iter(|| black_box(bigdecimal::BigDecimal::from_str(black_box(s))))
        });
    }
    group.finish();
}

criterion_group!(
    parse_benches,
    bench_parse_decimal64,
    bench_parse_f64,
    bench_parse_rust_decimal,
    bench_parse_bigdecimal
);
criterion_main!(parse_benches);
