use criterion::{criterion_group, criterion_main, Criterion};
use decimal64::{Decimal64, Scientific};
use std::hint::black_box;
use std::str::FromStr;

// Static string inputs — volatile-loaded so LLVM cannot constant-fold parse calls.
static INPUT_NOEXP: &str = "9999999999.9999";
static INPUT_POS_EXP: &str = "9.9999999999999e9";
static INPUT_NEG_EXP: &str = "1.0e-5";
static INPUT_ZERO: &str = "0e0";
static INPUT_BASE: &str = "9999999999.9999";

// Raw value for display benchmark: 123.4567 at scale 4.
static DISPLAY_RAW: i64 = 1_234_567;

fn bench_scientific_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("scientific_parse");

    group.bench_function("noexp", |b| {
        b.iter(|| {
            let s = unsafe { std::ptr::read_volatile(&INPUT_NOEXP) };
            black_box(Scientific::<Decimal64<4>>::from_str(s))
        })
    });

    group.bench_function("pos_exp", |b| {
        b.iter(|| {
            let s = unsafe { std::ptr::read_volatile(&INPUT_POS_EXP) };
            black_box(Scientific::<Decimal64<4>>::from_str(s))
        })
    });

    group.bench_function("neg_exp", |b| {
        b.iter(|| {
            let s = unsafe { std::ptr::read_volatile(&INPUT_NEG_EXP) };
            black_box(Scientific::<Decimal64<4>>::from_str(s))
        })
    });

    group.bench_function("zero", |b| {
        b.iter(|| {
            let s = unsafe { std::ptr::read_volatile(&INPUT_ZERO) };
            black_box(Scientific::<Decimal64<4>>::from_str(s))
        })
    });

    group.finish();
}

fn bench_scientific_display(c: &mut Criterion) {
    let mut group = c.benchmark_group("scientific_display");

    group.bench_function("decimal64_s4", |b| {
        b.iter(|| {
            let raw = unsafe { std::ptr::read_volatile(&DISPLAY_RAW) };
            let val = Decimal64::<4>::from_raw(raw);
            black_box(format!("{}", Scientific(val)))
        })
    });

    group.finish();
}

// Reference group: base Decimal64 parser on the same input as scientific_parse/noexp,
// so relative overhead can be measured directly.
fn bench_base_decimal64_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("decimal64_parse");

    group.bench_function("9999999999.9999", |b| {
        b.iter(|| {
            let s = unsafe { std::ptr::read_volatile(&INPUT_BASE) };
            black_box(Decimal64::<4>::parse(s))
        })
    });

    group.finish();
}

criterion_group!(
    scientific_benches,
    bench_scientific_parse,
    bench_scientific_display,
    bench_base_decimal64_parse
);
criterion_main!(scientific_benches);
