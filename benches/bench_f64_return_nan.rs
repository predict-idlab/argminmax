#![feature(stdsimd)]

use argminmax::ArgMinMax;
use codspeed_criterion_compat::*;
use dev_utils::{config, utils};

use argminmax::{FloatReturnNaN, ScalarArgMinMax, SCALAR};
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use argminmax::{SIMDArgMinMax, AVX2, AVX512, SSE};

// _rn stands for "return nan"

fn argminmax_rn_f64_random_array_long(c: &mut Criterion) {
    let n = config::ARRAY_LENGTH_LONG;
    let data: &[f64] = &utils::get_random_array::<f64>(n, f64::MIN, f64::MAX);
    c.bench_function("scalar_f64_argminmax_rn", |b| {
        b.iter(|| SCALAR::<FloatReturnNaN>::argminmax(black_box(data)))
    });
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    if is_x86_feature_detected!("sse4.2") {
        c.bench_function("sse_f64_argminmax_rn", |b| {
            b.iter(|| unsafe { SSE::<FloatReturnNaN>::argminmax(black_box(data)) })
        });
    }
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    if is_x86_feature_detected!("avx2") {
        c.bench_function("avx2_f64_argminmax_rn", |b| {
            b.iter(|| unsafe { AVX2::<FloatReturnNaN>::argminmax(black_box(data)) })
        });
    }
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    if is_x86_feature_detected!("avx512f") {
        c.bench_function("avx512_f64_argminmax_rn", |b| {
            b.iter(|| unsafe { AVX512::<FloatReturnNaN>::argminmax(black_box(data)) })
        });
    }
    c.bench_function("impl_f64_argminmax_rn", |b| {
        b.iter(|| black_box(data.nanargminmax()))
    });
}

criterion_group!(benches, argminmax_rn_f64_random_array_long,);
criterion_main!(benches);