use criterion::{criterion_group, criterion_main, Criterion};
use decimal64::{Decimal64, UDecimal64};
use std::hint::black_box;

// Scale 4 — primary benchmark operands (financial prices)
const LHS_RAW: i64 = 1_234_567; // 123.4567 at scale 4
const RHS_RAW: i64 = 9_876_543; // 987.6543 at scale 4

// Scale 2 — common currency scale
const LHS_RAW_S2: i64 = 12_345; // 123.45 at scale 2
const RHS_RAW_S2: i64 = 98_765; // 987.65 at scale 2

// Scale 9 — high-precision (product of 1.234567e9 raw values overflows i64 at this scale)
const LHS_RAW_S9: i64 = 1_234_567_000; // 1.234567000 at scale 9
const RHS_RAW_S9: i64 = 9_876_543_000; // 9.876543000 at scale 9

// Large-magnitude mul (S=4): product overflows i64 but result fits (slow path triggered)
//   2e9 * 5e9 = 1e19 > i64::MAX; 1e19 / 10^4 = 1e15 < i64::MAX
const MUL_LARGE_LHS: i64 = 2_000_000_000;
const MUL_LARGE_RHS: i64 = 5_000_000_000;

// Large-magnitude div (S=4): scaled numerator overflows i64 but result fits (slow path triggered)
//   i64::MAX * 10^4 > i64::MAX; i64::MAX * 10^4 / 10^4 = i64::MAX
const DIV_LARGE_LHS: i64 = i64::MAX;
const DIV_LARGE_RHS: i64 = 10_000; // = 10^4 = const_pow10(4)

fn bench_arithmetic(c: &mut Criterion) {
    let mut group = c.benchmark_group("arithmetic");

    // ── Scale 4 (primary) ─────────────────────────────────────────────────────

    let lhs = Decimal64::<4>::from_raw(LHS_RAW);
    let rhs = Decimal64::<4>::from_raw(RHS_RAW);

    let ulhs = UDecimal64::<4>::from_raw(LHS_RAW as u64);
    let urhs = UDecimal64::<4>::from_raw(RHS_RAW as u64);

    group.bench_function("decimal64_add", |b| {
        b.iter(|| black_box(lhs) + black_box(rhs))
    });

    group.bench_function("udecimal64_add", |b| {
        b.iter(|| black_box(ulhs) + black_box(urhs))
    });

    group.bench_function("i64_add", |b| {
        b.iter(|| black_box(LHS_RAW) + black_box(RHS_RAW))
    });

    group.bench_function("decimal64_mul", |b| {
        b.iter(|| black_box(lhs) * black_box(rhs))
    });

    group.bench_function("udecimal64_mul", |b| {
        b.iter(|| black_box(ulhs) * black_box(urhs))
    });

    group.bench_function("i64_mul", |b| {
        b.iter(|| black_box(LHS_RAW) * black_box(RHS_RAW))
    });

    group.bench_function("decimal64_div", |b| {
        b.iter(|| black_box(lhs) / black_box(rhs))
    });

    group.bench_function("udecimal64_div", |b| {
        b.iter(|| black_box(ulhs) / black_box(urhs))
    });

    group.bench_function("i64_div", |b| {
        b.iter(|| black_box(LHS_RAW) / black_box(RHS_RAW))
    });

    // ── Scale 2 ───────────────────────────────────────────────────────────────

    let lhs_s2 = Decimal64::<2>::from_raw(LHS_RAW_S2);
    let rhs_s2 = Decimal64::<2>::from_raw(RHS_RAW_S2);

    let ulhs_s2 = UDecimal64::<2>::from_raw(LHS_RAW_S2 as u64);
    let urhs_s2 = UDecimal64::<2>::from_raw(RHS_RAW_S2 as u64);

    group.bench_function("decimal64_mul_s2", |b| {
        b.iter(|| black_box(lhs_s2) * black_box(rhs_s2))
    });

    group.bench_function("udecimal64_mul_s2", |b| {
        b.iter(|| black_box(ulhs_s2) * black_box(urhs_s2))
    });

    group.bench_function("decimal64_div_s2", |b| {
        b.iter(|| black_box(lhs_s2) / black_box(rhs_s2))
    });

    group.bench_function("udecimal64_div_s2", |b| {
        b.iter(|| black_box(ulhs_s2) / black_box(urhs_s2))
    });

    // ── Scale 9 ───────────────────────────────────────────────────────────────

    let lhs_s9 = Decimal64::<9>::from_raw(LHS_RAW_S9);
    let rhs_s9 = Decimal64::<9>::from_raw(RHS_RAW_S9);

    let ulhs_s9 = UDecimal64::<9>::from_raw(LHS_RAW_S9 as u64);
    let urhs_s9 = UDecimal64::<9>::from_raw(RHS_RAW_S9 as u64);

    group.bench_function("decimal64_mul_s9", |b| {
        b.iter(|| black_box(lhs_s9) * black_box(rhs_s9))
    });

    group.bench_function("udecimal64_mul_s9", |b| {
        b.iter(|| black_box(ulhs_s9) * black_box(urhs_s9))
    });

    group.bench_function("decimal64_div_s9", |b| {
        b.iter(|| black_box(lhs_s9) / black_box(rhs_s9))
    });

    group.bench_function("udecimal64_div_s9", |b| {
        b.iter(|| black_box(ulhs_s9) / black_box(urhs_s9))
    });

    // ── Large magnitude — i128 slow path ──────────────────────────────────────
    // mul: 2e9 * 5e9 = 1e19 > i64::MAX; result 1e15 fits. Fast path overflows → slow path.
    // div: i64::MAX * 10^4 > i64::MAX; result = i64::MAX fits. Fast path overflows → slow path.

    let mul_large_lhs = Decimal64::<4>::from_raw(MUL_LARGE_LHS);
    let mul_large_rhs = Decimal64::<4>::from_raw(MUL_LARGE_RHS);
    let div_large_lhs = Decimal64::<4>::from_raw(DIV_LARGE_LHS);
    let div_large_rhs = Decimal64::<4>::from_raw(DIV_LARGE_RHS);

    let umul_large_lhs = UDecimal64::<4>::from_raw(MUL_LARGE_LHS as u64);
    let umul_large_rhs = UDecimal64::<4>::from_raw(MUL_LARGE_RHS as u64);
    let udiv_large_lhs = UDecimal64::<4>::from_raw(DIV_LARGE_LHS as u64);
    let udiv_large_rhs = UDecimal64::<4>::from_raw(DIV_LARGE_RHS as u64);

    group.bench_function("decimal64_mul_large", |b| {
        b.iter(|| black_box(mul_large_lhs) * black_box(mul_large_rhs))
    });

    group.bench_function("udecimal64_mul_large", |b| {
        b.iter(|| black_box(umul_large_lhs) * black_box(umul_large_rhs))
    });

    group.bench_function("decimal64_div_large", |b| {
        b.iter(|| black_box(div_large_lhs) / black_box(div_large_rhs))
    });

    group.bench_function("udecimal64_div_large", |b| {
        b.iter(|| black_box(udiv_large_lhs) / black_box(udiv_large_rhs))
    });

    group.finish();
}

criterion_group!(arith_benches, bench_arithmetic);
criterion_main!(arith_benches);
