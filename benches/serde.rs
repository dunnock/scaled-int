use criterion::{Criterion, criterion_group, criterion_main};
use scaledint::Decimal64;
use serde::{Deserialize, Serialize};
use std::hint::black_box;

// Decimal64<4> representing "123.4567" (raw = 1_234_567)
const D64_RAW: i64 = 1_234_567;

// JSON-encoded form: "\"123.4567\""
static S_D64_RAW: i64 = D64_RAW;
static S_JSON_STR: &str = r#""123.4567""#;

#[derive(Serialize, Deserialize)]
struct RawRow {
    #[serde(with = "scaledint::serde_as::raw_i64")]
    price: Decimal64<4>,
}

/// Load i64 via volatile read — prevents LLVM from constant-folding.
///
/// # Safety
/// Pointer is a reference to a static, so it is valid and aligned.
#[inline(always)]
unsafe fn vload_i64(p: &i64) -> i64 {
    unsafe { std::ptr::read_volatile(p) }
}

fn bench_serde_json_serialize_d64(c: &mut Criterion) {
    let mut group = c.benchmark_group("serde_json_serialize_d64");
    group.bench_function("123.4567", |b| {
        b.iter(|| {
            let raw = unsafe { vload_i64(&S_D64_RAW) };
            let d64 = Decimal64::<4>::from_raw(raw);
            black_box(serde_json::to_string(&d64).unwrap())
        })
    });
    group.finish();
}

fn bench_serde_json_deserialize_d64(c: &mut Criterion) {
    let mut group = c.benchmark_group("serde_json_deserialize_d64");
    group.bench_function("123.4567", |b| {
        b.iter(|| {
            // SAFETY: reading from an aligned static &str; volatile prevents constant-folding.
            let s: &str = unsafe { std::ptr::read_volatile(&S_JSON_STR) };
            black_box(serde_json::from_str::<Decimal64<4>>(s).unwrap())
        })
    });
    group.finish();
}

fn bench_postcard_serialize_string_d64(c: &mut Criterion) {
    let mut group = c.benchmark_group("postcard_serialize_string_d64");
    group.bench_function("123.4567", |b| {
        b.iter(|| {
            let raw = unsafe { vload_i64(&S_D64_RAW) };
            let d64 = Decimal64::<4>::from_raw(raw);
            black_box(postcard::to_allocvec(&d64).unwrap())
        })
    });
    group.finish();
}

fn bench_postcard_serialize_raw_d64(c: &mut Criterion) {
    let mut group = c.benchmark_group("postcard_serialize_raw_d64");
    group.bench_function("123.4567", |b| {
        b.iter(|| {
            let raw = unsafe { vload_i64(&S_D64_RAW) };
            let row = RawRow {
                price: Decimal64::<4>::from_raw(raw),
            };
            black_box(postcard::to_allocvec(&row).unwrap())
        })
    });
    group.finish();
}

fn bench_postcard_deserialize_raw_d64(c: &mut Criterion) {
    let setup_row = RawRow {
        price: Decimal64::<4>::from_raw(D64_RAW),
    };
    let bytes = postcard::to_allocvec(&setup_row).unwrap();

    let mut group = c.benchmark_group("postcard_deserialize_raw_d64");
    group.bench_function("123.4567", |b| {
        b.iter(|| black_box(postcard::from_bytes::<RawRow>(&bytes).unwrap()))
    });
    group.finish();
}

criterion_group!(
    serde_benches,
    bench_serde_json_serialize_d64,
    bench_serde_json_deserialize_d64,
    bench_postcard_serialize_string_d64,
    bench_postcard_serialize_raw_d64,
    bench_postcard_deserialize_raw_d64,
);
criterion_main!(serde_benches);
