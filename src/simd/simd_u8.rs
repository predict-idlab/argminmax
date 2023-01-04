use super::config::SIMDInstructionSet;
use super::generic::SIMD;
use ndarray::ArrayView1;
#[cfg(target_arch = "aarch64")]
use std::arch::aarch64::*;
#[cfg(target_arch = "arm")]
use std::arch::arm::*;
#[cfg(target_arch = "x86")]
use std::arch::x86::*;
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
const XOR_VALUE: i8 = 0x7F;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[inline(always)]
fn _i8decrord_to_u8(decrord_i8: i8) -> u8 {
    // let v = ord_i8 ^ 0x7F;
    unsafe { std::mem::transmute::<i8, u8>(decrord_i8 ^ XOR_VALUE) }
}

// ------------------------------------------ AVX2 ------------------------------------------

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod avx2 {
    use super::super::config::AVX2;
    use super::*;

    const LANE_SIZE: usize = AVX2::LANE_SIZE_8;
    const XOR_MASK: __m256i = unsafe { std::mem::transmute([XOR_VALUE; LANE_SIZE]) };

    #[inline(always)]
    unsafe fn _u8_to_i8decrord(u8: __m256i) -> __m256i {
        // on a scalar: v^ 0x7F
        // transforms to monotonically **decreasing** order
        _mm256_xor_si256(u8, XOR_MASK) // Only 1 operation
    }

    #[inline(always)]
    unsafe fn _reg_to_i8_arr(reg: __m256i) -> [i8; LANE_SIZE] {
        std::mem::transmute::<__m256i, [i8; LANE_SIZE]>(reg)
    }

    impl SIMD<u8, __m256i, __m256i, LANE_SIZE> for AVX2 {
        const INITIAL_INDEX: __m256i = unsafe {
            std::mem::transmute([
                0i8, 1i8, 2i8, 3i8, 4i8, 5i8, 6i8, 7i8, 8i8, 9i8, 10i8, 11i8, 12i8, 13i8, 14i8,
                15i8, 16i8, 17i8, 18i8, 19i8, 20i8, 21i8, 22i8, 23i8, 24i8, 25i8, 26i8, 27i8, 28i8,
                29i8, 30i8, 31i8,
            ])
        };
        const MAX_INDEX: usize = i8::MAX as usize;

        #[inline(always)]
        unsafe fn _reg_to_arr(_: __m256i) -> [u8; LANE_SIZE] {
            // Not used because we work with i8ord and override _get_min_index_value and _get_max_index_value
            unimplemented!()
        }

        #[inline(always)]
        unsafe fn _mm_loadu(data: *const u8) -> __m256i {
            _u8_to_i8decrord(_mm256_loadu_si256(data as *const __m256i))
        }

        #[inline(always)]
        unsafe fn _mm_set1(a: usize) -> __m256i {
            _mm256_set1_epi8(a as i8)
        }

        #[inline(always)]
        unsafe fn _mm_add(a: __m256i, b: __m256i) -> __m256i {
            _mm256_add_epi8(a, b)
        }

        #[inline(always)]
        unsafe fn _mm_cmpgt(a: __m256i, b: __m256i) -> __m256i {
            _mm256_cmpgt_epi8(a, b)
        }

        #[inline(always)]
        unsafe fn _mm_cmplt(a: __m256i, b: __m256i) -> __m256i {
            _mm256_cmpgt_epi8(b, a)
        }

        #[inline(always)]
        unsafe fn _mm_blendv(a: __m256i, b: __m256i, mask: __m256i) -> __m256i {
            _mm256_blendv_epi8(a, b, mask)
        }

        // ------------------------------------ ARGMINMAX --------------------------------------

        #[target_feature(enable = "avx2")]
        unsafe fn argminmax(data: ArrayView1<u8>) -> (usize, usize) {
            Self::_argminmax(data)
        }

        #[inline(always)]
        unsafe fn _horiz_min(index: __m256i, value: __m256i) -> (usize, u8) {
            // 0. Find the minimum value
            let mut vmin: __m256i = value;
            vmin = _mm256_min_epi8(vmin, _mm256_permute2x128_si256(vmin, vmin, 1));
            vmin = _mm256_min_epi8(vmin, _mm256_alignr_epi8(vmin, vmin, 8));
            vmin = _mm256_min_epi8(vmin, _mm256_alignr_epi8(vmin, vmin, 4));
            vmin = _mm256_min_epi8(vmin, _mm256_alignr_epi8(vmin, vmin, 2));
            vmin = _mm256_min_epi8(vmin, _mm256_alignr_epi8(vmin, vmin, 1));
            let min_value: i8 = _mm256_extract_epi8(vmin, 0) as i8;

            // Extract the index of the minimum value
            // 1. Create a mask with the index of the minimum value
            let mask = _mm256_cmpeq_epi8(value, vmin);
            // 2. Blend the mask with the index
            let search_index = _mm256_blendv_epi8(
                _mm256_set1_epi8(i8::MAX), // if mask is 0, use i8::MAX
                index,                     // if mask is 1, use index
                mask,
            );
            // 3. Find the minimum index
            let mut imin: __m256i = search_index;
            imin = _mm256_min_epi8(imin, _mm256_permute2x128_si256(imin, imin, 1));
            imin = _mm256_min_epi8(imin, _mm256_alignr_epi8(imin, imin, 8));
            imin = _mm256_min_epi8(imin, _mm256_alignr_epi8(imin, imin, 4));
            imin = _mm256_min_epi8(imin, _mm256_alignr_epi8(imin, imin, 2));
            imin = _mm256_min_epi8(imin, _mm256_alignr_epi8(imin, imin, 1));
            let min_index: usize = _mm256_extract_epi8(imin, 0) as usize;

            (min_index, _i8decrord_to_u8(min_value))
        }

        #[inline(always)]
        unsafe fn _horiz_max(index: __m256i, value: __m256i) -> (usize, u8) {
            // 0. Find the maximum value
            let mut vmax: __m256i = value;
            vmax = _mm256_max_epi8(vmax, _mm256_permute2x128_si256(vmax, vmax, 1));
            vmax = _mm256_max_epi8(vmax, _mm256_alignr_epi8(vmax, vmax, 8));
            vmax = _mm256_max_epi8(vmax, _mm256_alignr_epi8(vmax, vmax, 4));
            vmax = _mm256_max_epi8(vmax, _mm256_alignr_epi8(vmax, vmax, 2));
            vmax = _mm256_max_epi8(vmax, _mm256_alignr_epi8(vmax, vmax, 1));
            let max_value: i8 = _mm256_extract_epi8(vmax, 0) as i8;

            // Extract the index of the maximum value
            // 1. Create a mask with the index of the maximum value
            let mask = _mm256_cmpeq_epi8(value, vmax);
            // 2. Blend the mask with the index
            let search_index = _mm256_blendv_epi8(
                _mm256_set1_epi8(i8::MAX), // if mask is 0, use i8::MAX
                index,                     // if mask is 1, use index
                mask,
            );
            // 3. Find the maximum index
            let mut imin: __m256i = search_index;
            imin = _mm256_min_epi8(imin, _mm256_permute2x128_si256(imin, imin, 1));
            imin = _mm256_min_epi8(imin, _mm256_alignr_epi8(imin, imin, 8));
            imin = _mm256_min_epi8(imin, _mm256_alignr_epi8(imin, imin, 4));
            imin = _mm256_min_epi8(imin, _mm256_alignr_epi8(imin, imin, 2));
            imin = _mm256_min_epi8(imin, _mm256_alignr_epi8(imin, imin, 1));
            let max_index: usize = _mm256_extract_epi8(imin, 0) as usize;

            (max_index, _i8decrord_to_u8(max_value))
        }

        #[inline(always)]
        unsafe fn _get_min_max_index_value(
            index_low: __m256i,
            values_low: __m256i,
            index_high: __m256i,
            values_high: __m256i,
        ) -> (usize, u8, usize, u8) {
            let (min_index, min_value) = Self::_horiz_min(index_low, values_low);
            let (max_index, max_value) = Self::_horiz_max(index_high, values_high);
            // Swap min and max here because we worked with i8ord in decreasing order (max => actual min, and vice versa)
            (max_index, max_value, min_index, min_value)
        }
    }

    // ------------------------------------ TESTS --------------------------------------

    #[cfg(test)]
    mod tests {
        use super::{AVX2, SIMD};
        use crate::scalar::generic::scalar_argminmax;

        use ndarray::Array1;

        extern crate dev_utils;
        use dev_utils::utils;

        fn get_array_u8(n: usize) -> Array1<u8> {
            utils::get_random_array(n, u8::MIN, u8::MAX)
        }

        #[test]
        fn test_both_versions_return_the_same_results() {
            if !is_x86_feature_detected!("avx2") {
                return;
            }

            let data = get_array_u8(65);
            assert_eq!(data.len() % 16, 1);

            let (argmin_index, argmax_index) = scalar_argminmax(data.view());
            let (simd_argmin_index, simd_argmax_index) = unsafe { AVX2::argminmax(data.view()) };
            assert_eq!(argmin_index, simd_argmin_index);
            assert_eq!(argmax_index, simd_argmax_index);
        }

        #[test]
        fn test_first_index_is_returned_when_identical_values_found() {
            if !is_x86_feature_detected!("avx2") {
                return;
            }

            let data = [10, std::u8::MIN, 6, 9, 9, 22, std::u8::MAX, 4, std::u8::MAX];
            let data: Vec<u8> = data.iter().map(|x| *x).collect();
            let data = Array1::from(data);

            let (argmin_index, argmax_index) = scalar_argminmax(data.view());
            assert_eq!(argmin_index, 1);
            assert_eq!(argmax_index, 6);

            let (argmin_simd_index, argmax_simd_index) = unsafe { AVX2::argminmax(data.view()) };
            assert_eq!(argmin_simd_index, 1);
            assert_eq!(argmax_simd_index, 6);
        }

        #[test]
        fn test_no_overflow() {
            if !is_x86_feature_detected!("avx2") {
                return;
            }

            let n: usize = 1 << 10;
            let data = get_array_u8(n);

            let (argmin_index, argmax_index) = scalar_argminmax(data.view());
            let (argmin_simd_index, argmax_simd_index) = unsafe { AVX2::argminmax(data.view()) };
            assert_eq!(argmin_index, argmin_simd_index);
            assert_eq!(argmax_index, argmax_simd_index);
        }

        #[test]
        fn test_many_random_runs() {
            if !is_x86_feature_detected!("avx2") {
                return;
            }

            for _ in 0..10_000 {
                let data = get_array_u8(32 * 2 + 1);
                let (argmin_index, argmax_index) = scalar_argminmax(data.view());
                let (argmin_simd_index, argmax_simd_index) =
                    unsafe { AVX2::argminmax(data.view()) };
                assert_eq!(argmin_index, argmin_simd_index);
                assert_eq!(argmax_index, argmax_simd_index);
            }
        }
    }
}

// ----------------------------------------- SSE -----------------------------------------

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod sse {
    use super::super::config::SSE;
    use super::*;

    const LANE_SIZE: usize = SSE::LANE_SIZE_8;
    const XOR_MASK: __m128i = unsafe { std::mem::transmute([XOR_VALUE; LANE_SIZE]) };

    #[inline(always)]
    unsafe fn _u8_to_i8decrord(u8: __m128i) -> __m128i {
        // on a scalar: v^ 0x7F
        // transforms to monotonically **decreasing** order
        _mm_xor_si128(u8, XOR_MASK)
    }

    #[inline(always)]
    unsafe fn _reg_to_i8_arr(reg: __m128i) -> [i8; LANE_SIZE] {
        std::mem::transmute::<__m128i, [i8; LANE_SIZE]>(reg)
    }

    impl SIMD<u8, __m128i, __m128i, LANE_SIZE> for SSE {
        const INITIAL_INDEX: __m128i = unsafe {
            std::mem::transmute([
                0i8, 1i8, 2i8, 3i8, 4i8, 5i8, 6i8, 7i8, 8i8, 9i8, 10i8, 11i8, 12i8, 13i8, 14i8,
                15i8,
            ])
        };
        const MAX_INDEX: usize = i8::MAX as usize;

        #[inline(always)]
        unsafe fn _reg_to_arr(_: __m128i) -> [u8; LANE_SIZE] {
            // Not used because we work with i8ord and override _get_min_index_value and _get_max_index_value
            unimplemented!()
        }

        #[inline(always)]
        unsafe fn _mm_loadu(data: *const u8) -> __m128i {
            _u8_to_i8decrord(_mm_loadu_si128(data as *const __m128i))
        }

        #[inline(always)]
        unsafe fn _mm_set1(a: usize) -> __m128i {
            _mm_set1_epi8(a as i8)
        }

        #[inline(always)]
        unsafe fn _mm_add(a: __m128i, b: __m128i) -> __m128i {
            _mm_add_epi8(a, b)
        }

        #[inline(always)]
        unsafe fn _mm_cmpgt(a: __m128i, b: __m128i) -> __m128i {
            _mm_cmpgt_epi8(a, b)
        }

        #[inline(always)]
        unsafe fn _mm_cmplt(a: __m128i, b: __m128i) -> __m128i {
            _mm_cmplt_epi8(a, b)
        }

        #[inline(always)]
        unsafe fn _mm_blendv(a: __m128i, b: __m128i, mask: __m128i) -> __m128i {
            _mm_blendv_epi8(a, b, mask)
        }

        // ------------------------------------ ARGMINMAX --------------------------------------

        #[target_feature(enable = "sse4.1")]
        unsafe fn argminmax(data: ArrayView1<u8>) -> (usize, usize) {
            Self::_argminmax(data)
        }

        #[inline(always)]
        unsafe fn _horiz_min(index: __m128i, value: __m128i) -> (usize, u8) {
            // 0. Find the minimum value
            let mut vmin: __m128i = value;
            vmin = _mm_min_epi8(vmin, _mm_alignr_epi8(vmin, vmin, 8));
            vmin = _mm_min_epi8(vmin, _mm_alignr_epi8(vmin, vmin, 4));
            vmin = _mm_min_epi8(vmin, _mm_alignr_epi8(vmin, vmin, 2));
            vmin = _mm_min_epi8(vmin, _mm_alignr_epi8(vmin, vmin, 1));
            let min_value: i8 = _mm_extract_epi8(vmin, 0) as i8;

            // Extract the index of the minimum value
            // 1. Create a mask with the index of the minimum value
            let mask = _mm_cmpeq_epi8(value, vmin);
            // 2. Blend the mask with the index
            let search_index = _mm_blendv_epi8(
                _mm_set1_epi8(i8::MAX), // if mask is 0, use i8::MAX
                index,                  // if mask is 1, use index
                mask,
            );
            // 3. Find the minimum index
            let mut imin: __m128i = search_index;
            imin = _mm_min_epi8(imin, _mm_alignr_epi8(imin, imin, 8));
            imin = _mm_min_epi8(imin, _mm_alignr_epi8(imin, imin, 4));
            imin = _mm_min_epi8(imin, _mm_alignr_epi8(imin, imin, 2));
            imin = _mm_min_epi8(imin, _mm_alignr_epi8(imin, imin, 1));
            let min_index: usize = _mm_extract_epi8(imin, 0) as usize;

            (min_index, _i8decrord_to_u8(min_value))
        }

        #[inline(always)]
        unsafe fn _horiz_max(index: __m128i, value: __m128i) -> (usize, u8) {
            // 0. Find the maximum value
            let mut vmax: __m128i = value;
            vmax = _mm_max_epi8(vmax, _mm_alignr_epi8(vmax, vmax, 8));
            vmax = _mm_max_epi8(vmax, _mm_alignr_epi8(vmax, vmax, 4));
            vmax = _mm_max_epi8(vmax, _mm_alignr_epi8(vmax, vmax, 2));
            vmax = _mm_max_epi8(vmax, _mm_alignr_epi8(vmax, vmax, 1));
            let max_value: i8 = _mm_extract_epi8(vmax, 0) as i8;

            // Extract the index of the maximum value
            // 1. Create a mask with the index of the maximum value
            let mask = _mm_cmpeq_epi8(value, vmax);
            // 2. Blend the mask with the index
            let search_index = _mm_blendv_epi8(
                _mm_set1_epi8(i8::MAX), // if mask is 0, use i8::MAX
                index,                  // if mask is 1, use index
                mask,
            );
            // 3. Find the minimum index
            let mut imin: __m128i = search_index;
            imin = _mm_min_epi8(imin, _mm_alignr_epi8(imin, imin, 8));
            imin = _mm_min_epi8(imin, _mm_alignr_epi8(imin, imin, 4));
            imin = _mm_min_epi8(imin, _mm_alignr_epi8(imin, imin, 2));
            imin = _mm_min_epi8(imin, _mm_alignr_epi8(imin, imin, 1));
            let max_index: usize = _mm_extract_epi8(imin, 0) as usize;

            (max_index, _i8decrord_to_u8(max_value))
        }

        #[inline(always)]
        unsafe fn _get_min_max_index_value(
            index_low: __m128i,
            values_low: __m128i,
            index_high: __m128i,
            values_high: __m128i,
        ) -> (usize, u8, usize, u8) {
            let (min_index, min_value) = Self::_horiz_min(index_low, values_low);
            let (max_index, max_value) = Self::_horiz_max(index_high, values_high);
            // Swap min and max here because we worked with i8ord in decreasing order (max => actual min, and vice versa)
            (max_index, max_value, min_index, min_value)
        }
    }

    // ----------------------------------------- TESTS -----------------------------------------

    #[cfg(test)]
    mod tests {
        use super::{SIMD, SSE};
        use crate::scalar::generic::scalar_argminmax;

        use ndarray::Array1;

        extern crate dev_utils;
        use dev_utils::utils;

        fn get_array_u8(n: usize) -> Array1<u8> {
            utils::get_random_array(n, u8::MIN, u8::MAX)
        }

        #[test]
        fn test_both_versions_return_the_same_results() {
            let data = get_array_u8(65);
            assert_eq!(data.len() % 16, 1);

            let (argmin_index, argmax_index) = scalar_argminmax(data.view());
            let (simd_argmin_index, simd_argmax_index) = unsafe { SSE::argminmax(data.view()) };
            assert_eq!(argmin_index, simd_argmin_index);
            assert_eq!(argmax_index, simd_argmax_index);
        }

        #[test]
        fn test_first_index_is_returned_when_identical_values_found() {
            let data = [10, std::u8::MIN, 6, 9, 9, 22, std::u8::MAX, 4, std::u8::MAX];
            let data: Vec<u8> = data.iter().map(|x| *x).collect();
            let data = Array1::from(data);

            let (argmin_index, argmax_index) = scalar_argminmax(data.view());
            assert_eq!(argmin_index, 1);
            assert_eq!(argmax_index, 6);

            let (argmin_simd_index, argmax_simd_index) = unsafe { SSE::argminmax(data.view()) };
            assert_eq!(argmin_simd_index, 1);
            assert_eq!(argmax_simd_index, 6);
        }

        #[test]
        fn test_no_overflow() {
            let n: usize = 1 << 10;
            let data = get_array_u8(n);

            let (argmin_index, argmax_index) = scalar_argminmax(data.view());
            let (argmin_simd_index, argmax_simd_index) = unsafe { SSE::argminmax(data.view()) };
            assert_eq!(argmin_index, argmin_simd_index);
            assert_eq!(argmax_index, argmax_simd_index);
        }

        #[test]
        fn test_many_random_runs() {
            for _ in 0..10_000 {
                let data = get_array_u8(32 * 2 + 1);
                let (argmin_index, argmax_index) = scalar_argminmax(data.view());
                let (argmin_simd_index, argmax_simd_index) = unsafe { SSE::argminmax(data.view()) };
                assert_eq!(argmin_index, argmin_simd_index);
                assert_eq!(argmax_index, argmax_simd_index);
            }
        }
    }
}

// --------------------------------------- AVX512 ----------------------------------------

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod avx512 {
    use super::super::config::AVX512;
    use super::*;

    const LANE_SIZE: usize = AVX512::LANE_SIZE_8;

    const XOR_MASK: __m512i = unsafe { std::mem::transmute([XOR_VALUE; LANE_SIZE]) };

    //  - comparison swappen => dan moeten we opt einde niet meer swappen?

    #[inline(always)]
    unsafe fn _u8_to_i8decrord(u8: __m512i) -> __m512i {
        // on a scalar: v^ 0x7F
        // transforms to monotonically **decreasing** order
        _mm512_xor_si512(u8, XOR_MASK)
    }

    #[inline(always)]
    unsafe fn _reg_to_i8_arr(reg: __m512i) -> [i8; LANE_SIZE] {
        std::mem::transmute::<__m512i, [i8; LANE_SIZE]>(reg)
    }

    impl SIMD<u8, __m512i, u64, LANE_SIZE> for AVX512 {
        const INITIAL_INDEX: __m512i = unsafe {
            std::mem::transmute([
                0i8, 1i8, 2i8, 3i8, 4i8, 5i8, 6i8, 7i8, 8i8, 9i8, 10i8, 11i8, 12i8, 13i8, 14i8,
                15i8, 16i8, 17i8, 18i8, 19i8, 20i8, 21i8, 22i8, 23i8, 24i8, 25i8, 26i8, 27i8, 28i8,
                29i8, 30i8, 31i8, 32i8, 33i8, 34i8, 35i8, 36i8, 37i8, 38i8, 39i8, 40i8, 41i8, 42i8,
                43i8, 44i8, 45i8, 46i8, 47i8, 48i8, 49i8, 50i8, 51i8, 52i8, 53i8, 54i8, 55i8, 56i8,
                57i8, 58i8, 59i8, 60i8, 61i8, 62i8, 63i8,
            ])
        };
        const MAX_INDEX: usize = i8::MAX as usize;

        #[inline(always)]
        unsafe fn _reg_to_arr(_: __m512i) -> [u8; LANE_SIZE] {
            unimplemented!(
                "We work with decrordi8 and override _get_min_index_value and _get_max_index_value"
            )
        }

        #[inline(always)]
        unsafe fn _mm_loadu(data: *const u8) -> __m512i {
            _u8_to_i8decrord(_mm512_loadu_epi8(data as *const i8))
        }

        #[inline(always)]
        unsafe fn _mm_set1(a: usize) -> __m512i {
            _mm512_set1_epi8(a as i8)
        }

        #[inline(always)]
        unsafe fn _mm_add(a: __m512i, b: __m512i) -> __m512i {
            _mm512_add_epi8(a, b)
        }

        #[inline(always)]
        unsafe fn _mm_cmpgt(a: __m512i, b: __m512i) -> u64 {
            _mm512_cmpgt_epi8_mask(a, b)
        }

        #[inline(always)]
        unsafe fn _mm_cmplt(a: __m512i, b: __m512i) -> u64 {
            _mm512_cmplt_epi8_mask(a, b)
        }

        #[inline(always)]
        unsafe fn _mm_blendv(a: __m512i, b: __m512i, mask: u64) -> __m512i {
            _mm512_mask_blend_epi8(mask, a, b)
        }

        // ------------------------------------ ARGMINMAX --------------------------------------

        #[target_feature(enable = "avx512bw")]
        unsafe fn argminmax(data: ArrayView1<u8>) -> (usize, usize) {
            Self::_argminmax(data)
        }

        #[inline(always)]
        unsafe fn _horiz_min(index: __m512i, value: __m512i) -> (usize, u8) {
            // 0. Find the minimum value
            let mut vmin: __m512i = value;
            vmin = _mm512_min_epi8(vmin, _mm512_alignr_epi32(vmin, vmin, 8));
            vmin = _mm512_min_epi8(vmin, _mm512_alignr_epi32(vmin, vmin, 4));
            vmin = _mm512_min_epi8(vmin, _mm512_alignr_epi8(vmin, vmin, 8));
            vmin = _mm512_min_epi8(vmin, _mm512_alignr_epi8(vmin, vmin, 4));
            vmin = _mm512_min_epi8(vmin, _mm512_alignr_epi8(vmin, vmin, 2));
            vmin = _mm512_min_epi8(vmin, _mm512_alignr_epi8(vmin, vmin, 1));
            let min_value: i8 = _mm_extract_epi8(_mm512_castsi512_si128(vmin), 0) as i8;

            // Extract the index of the minimum value
            // 1. Create a mask with the index of the minimum value
            let mask = _mm512_cmpeq_epi8_mask(value, vmin);
            // 2. Blend the mask with the index
            let search_index = _mm512_mask_blend_epi8(
                mask,
                _mm512_set1_epi8(i8::MAX), // if mask is 0, use i8::MAX
                index,                     // if mask is 1, use index
            );
            // 3. Find the minimum index
            let mut imin: __m512i = search_index;
            imin = _mm512_min_epi8(imin, _mm512_alignr_epi32(imin, imin, 8));
            imin = _mm512_min_epi8(imin, _mm512_alignr_epi32(imin, imin, 4));
            imin = _mm512_min_epi8(imin, _mm512_alignr_epi8(imin, imin, 8));
            imin = _mm512_min_epi8(imin, _mm512_alignr_epi8(imin, imin, 4));
            imin = _mm512_min_epi8(imin, _mm512_alignr_epi8(imin, imin, 2));
            imin = _mm512_min_epi8(imin, _mm512_alignr_epi8(imin, imin, 1));
            let min_index: usize = _mm_extract_epi8(_mm512_castsi512_si128(imin), 0) as usize;

            (min_index, _i8decrord_to_u8(min_value))
        }

        #[inline(always)]
        unsafe fn _horiz_max(index: __m512i, value: __m512i) -> (usize, u8) {
            // 0. Find the maximum value
            let mut vmax: __m512i = value;
            vmax = _mm512_max_epi8(vmax, _mm512_alignr_epi32(vmax, vmax, 8));
            vmax = _mm512_max_epi8(vmax, _mm512_alignr_epi32(vmax, vmax, 4));
            vmax = _mm512_max_epi8(vmax, _mm512_alignr_epi8(vmax, vmax, 8));
            vmax = _mm512_max_epi8(vmax, _mm512_alignr_epi8(vmax, vmax, 4));
            vmax = _mm512_max_epi8(vmax, _mm512_alignr_epi8(vmax, vmax, 2));
            vmax = _mm512_max_epi8(vmax, _mm512_alignr_epi8(vmax, vmax, 1));
            let max_value: i8 = _mm_extract_epi8(_mm512_castsi512_si128(vmax), 0) as i8;

            // Extract the index of the maximum value
            // 1. Create a mask with the index of the maximum value
            let mask = _mm512_cmpeq_epi8_mask(value, vmax);
            // 2. Blend the mask with the index
            let search_index = _mm512_mask_blend_epi8(
                mask,
                _mm512_set1_epi8(i8::MAX), // if mask is 0, use i8::MAX
                index,                     // if mask is 1, use index
            );
            // 3. Find the maximum index
            let mut imin: __m512i = search_index;
            imin = _mm512_min_epi8(imin, _mm512_alignr_epi32(imin, imin, 8));
            imin = _mm512_min_epi8(imin, _mm512_alignr_epi32(imin, imin, 4));
            imin = _mm512_min_epi8(imin, _mm512_alignr_epi8(imin, imin, 8));
            imin = _mm512_min_epi8(imin, _mm512_alignr_epi8(imin, imin, 4));
            imin = _mm512_min_epi8(imin, _mm512_alignr_epi8(imin, imin, 2));
            imin = _mm512_min_epi8(imin, _mm512_alignr_epi8(imin, imin, 1));
            let max_index: usize = _mm_extract_epi8(_mm512_castsi512_si128(imin), 0) as usize;

            (max_index, _i8decrord_to_u8(max_value))
        }

        #[inline(always)]
        unsafe fn _get_min_max_index_value(
            index_low: __m512i,
            values_low: __m512i,
            index_high: __m512i,
            values_high: __m512i,
        ) -> (usize, u8, usize, u8) {
            let (min_index, min_value) = Self::_horiz_min(index_low, values_low);
            let (max_index, max_value) = Self::_horiz_max(index_high, values_high);
            // Swap min and max here because we worked with i8ord in decreasing order (max => actual min, and vice versa)
            (max_index, max_value, min_index, min_value)
        }
    }

    // ----------------------------------------- TESTS -----------------------------------------

    #[cfg(test)]

    mod tests {
        use super::{AVX512, SIMD};
        use crate::scalar::generic::scalar_argminmax;

        use ndarray::Array1;

        extern crate dev_utils;
        use dev_utils::utils;

        fn get_array_u8(n: usize) -> Array1<u8> {
            utils::get_random_array(n, u8::MIN, u8::MAX)
        }

        #[test]
        fn test_both_versions_return_the_same_results() {
            if !is_x86_feature_detected!("avx512f") {
                return;
            }

            let data = get_array_u8(65);
            assert_eq!(data.len() % 16, 1);

            let (argmin_index, argmax_index) = scalar_argminmax(data.view());
            let (simd_argmin_index, simd_argmax_index) = unsafe { AVX512::argminmax(data.view()) };
            assert_eq!(argmin_index, simd_argmin_index);
            assert_eq!(argmax_index, simd_argmax_index);
        }

        #[test]
        fn test_first_index_is_returned_when_identical_values_found() {
            if !is_x86_feature_detected!("avx512f") {
                return;
            }

            let data = [10, std::u8::MIN, 6, 9, 9, 22, std::u8::MAX, 4, std::u8::MAX];
            let data: Vec<u8> = data.iter().map(|x| *x).collect();
            let data = Array1::from(data);

            let (argmin_index, argmax_index) = scalar_argminmax(data.view());
            assert_eq!(argmin_index, 1);
            assert_eq!(argmax_index, 6);

            let (argmin_simd_index, argmax_simd_index) = unsafe { AVX512::argminmax(data.view()) };
            assert_eq!(argmin_simd_index, 1);
            assert_eq!(argmax_simd_index, 6);
        }

        #[test]
        fn test_no_overflow() {
            if !is_x86_feature_detected!("avx512f") {
                return;
            }

            let n: usize = 1 << 10;
            let data = get_array_u8(n);

            let (argmin_index, argmax_index) = scalar_argminmax(data.view());
            let (argmin_simd_index, argmax_simd_index) = unsafe { AVX512::argminmax(data.view()) };
            assert_eq!(argmin_index, argmin_simd_index);
            assert_eq!(argmax_index, argmax_simd_index);
        }

        #[test]
        fn test_many_random_runs() {
            if !is_x86_feature_detected!("avx512f") {
                return;
            }

            for _ in 0..10_000 {
                let data = get_array_u8(32 * 4 + 1);
                let (argmin_index, argmax_index) = scalar_argminmax(data.view());
                let (argmin_simd_index, argmax_simd_index) =
                    unsafe { AVX512::argminmax(data.view()) };
                assert_eq!(argmin_index, argmin_simd_index);
                assert_eq!(argmax_index, argmax_simd_index);
            }
        }
    }
}

// ---------------------------------------- NEON -----------------------------------------

#[cfg(any(target_arch = "arm", target_arch = "aarch64"))]
mod neon {
    use super::super::config::NEON;
    use super::*;

    const LANE_SIZE: usize = NEON::LANE_SIZE_8;

    impl SIMD<u8, uint8x16_t, uint8x16_t, LANE_SIZE> for NEON {
        const INITIAL_INDEX: uint8x16_t = unsafe {
            std::mem::transmute([
                0u8, 1u8, 2u8, 3u8, 4u8, 5u8, 6u8, 7u8, 8u8, 9u8, 10u8, 11u8, 12u8, 13u8, 14u8,
                15u8,
            ])
        };
        const MAX_INDEX: usize = u8::MAX as usize;

        #[inline(always)]
        unsafe fn _reg_to_arr(reg: uint8x16_t) -> [u8; LANE_SIZE] {
            std::mem::transmute::<uint8x16_t, [u8; LANE_SIZE]>(reg)
        }

        #[inline(always)]
        unsafe fn _mm_loadu(data: *const u8) -> uint8x16_t {
            vld1q_u8(data as *const u8)
        }

        #[inline(always)]
        unsafe fn _mm_set1(a: usize) -> uint8x16_t {
            vdupq_n_u8(a as u8)
        }

        #[inline(always)]
        unsafe fn _mm_add(a: uint8x16_t, b: uint8x16_t) -> uint8x16_t {
            vaddq_u8(a, b)
        }

        #[inline(always)]
        unsafe fn _mm_cmpgt(a: uint8x16_t, b: uint8x16_t) -> uint8x16_t {
            vcgtq_u8(a, b)
        }

        #[inline(always)]
        unsafe fn _mm_cmplt(a: uint8x16_t, b: uint8x16_t) -> uint8x16_t {
            vcltq_u8(a, b)
        }

        #[inline(always)]
        unsafe fn _mm_blendv(a: uint8x16_t, b: uint8x16_t, mask: uint8x16_t) -> uint8x16_t {
            vbslq_u8(mask, b, a)
        }

        // ------------------------------------ ARGMINMAX --------------------------------------

        #[target_feature(enable = "neon")]
        unsafe fn argminmax(data: ArrayView1<u8>) -> (usize, usize) {
            Self::_argminmax(data)
        }

        #[inline(always)]
        unsafe fn _horiz_min(index: uint8x16_t, value: uint8x16_t) -> (usize, u8) {
            // 0. Find the minimum value
            let mut vmin: uint8x16_t = value;
            vmin = vminq_u8(vmin, vextq_u8(vmin, vmin, 8));
            vmin = vminq_u8(vmin, vextq_u8(vmin, vmin, 4));
            vmin = vminq_u8(vmin, vextq_u8(vmin, vmin, 2));
            vmin = vminq_u8(vmin, vextq_u8(vmin, vmin, 1));
            let min_value: u8 = vgetq_lane_u8(vmin, 0);

            // Extract the index of the minimum value
            // 1. Create a mask with the index of the minimum value
            let mask = vceqq_u8(value, vmin);
            // 2. Blend the mask with the index
            let search_index = vbslq_u8(
                mask,
                index,               // if mask is 1, use index
                vdupq_n_u8(u8::MAX), // if mask is 0, use u8::MAX
            );
            // 3. Find the minimum index
            let mut imin: uint8x16_t = search_index;
            imin = vminq_u8(imin, vextq_u8(imin, imin, 8));
            imin = vminq_u8(imin, vextq_u8(imin, imin, 4));
            imin = vminq_u8(imin, vextq_u8(imin, imin, 2));
            imin = vminq_u8(imin, vextq_u8(imin, imin, 1));
            let min_index: usize = vgetq_lane_u8(imin, 0) as usize;

            (min_index, min_value)
        }

        #[inline(always)]
        unsafe fn _horiz_max(index: uint8x16_t, value: uint8x16_t) -> (usize, u8) {
            // 0. Find the maximum value
            let mut vmax: uint8x16_t = value;
            vmax = vmaxq_u8(vmax, vextq_u8(vmax, vmax, 8));
            vmax = vmaxq_u8(vmax, vextq_u8(vmax, vmax, 4));
            vmax = vmaxq_u8(vmax, vextq_u8(vmax, vmax, 2));
            vmax = vmaxq_u8(vmax, vextq_u8(vmax, vmax, 1));
            let max_value: u8 = vgetq_lane_u8(vmax, 0);

            // Extract the index of the maximum value
            // 1. Create a mask with the index of the maximum value
            let mask = vceqq_u8(value, vmax);
            // 2. Blend the mask with the index
            let search_index = vbslq_u8(
                mask,
                index,               // if mask is 1, use index
                vdupq_n_u8(u8::MAX), // if mask is 0, use u8::MAX
            );
            // 3. Find the maximum index
            let mut imin: uint8x16_t = search_index;
            imin = vminq_u8(imin, vextq_u8(imin, imin, 8));
            imin = vminq_u8(imin, vextq_u8(imin, imin, 4));
            imin = vminq_u8(imin, vextq_u8(imin, imin, 2));
            imin = vminq_u8(imin, vextq_u8(imin, imin, 1));
            let max_index: usize = vgetq_lane_u8(imin, 0) as usize;

            (max_index, max_value)
        }
    }

    // ----------------------------------------- TESTS -----------------------------------------

    #[cfg(test)]
    mod tests {
        use super::{NEON, SIMD};
        use crate::scalar::generic::scalar_argminmax;

        use ndarray::Array1;

        extern crate dev_utils;
        use dev_utils::utils;

        fn get_array_u8(n: usize) -> Array1<u8> {
            utils::get_random_array(n, u8::MIN, u8::MAX)
        }

        #[test]
        fn test_both_versions_return_the_same_results() {
            let data = get_array_u8(513);
            assert_eq!(data.len() % 16, 1);

            let (argmin_index, argmax_index) = scalar_argminmax(data.view());
            let (simd_argmin_index, simd_argmax_index) = unsafe { NEON::argminmax(data.view()) };
            assert_eq!(argmin_index, simd_argmin_index);
            assert_eq!(argmax_index, simd_argmax_index);
        }

        #[test]
        fn test_first_index_is_returned_when_identical_values_found() {
            let data = [10, std::u8::MIN, 6, 9, 9, 22, std::u8::MAX, 4, std::u8::MAX];
            let data: Vec<u8> = data.iter().map(|x| *x).collect();
            let data = Array1::from(data);

            let (argmin_index, argmax_index) = scalar_argminmax(data.view());
            assert_eq!(argmin_index, 1);
            assert_eq!(argmax_index, 6);

            let (argmin_simd_index, argmax_simd_index) = unsafe { NEON::argminmax(data.view()) };
            assert_eq!(argmin_simd_index, 1);
            assert_eq!(argmax_simd_index, 6);
        }

        #[test]
        fn test_no_overflow() {
            let n: usize = 1 << 10;
            let data = get_array_u8(n);

            let (argmin_index, argmax_index) = scalar_argminmax(data.view());
            let (argmin_simd_index, argmax_simd_index) = unsafe { NEON::argminmax(data.view()) };
            assert_eq!(argmin_index, argmin_simd_index);
            assert_eq!(argmax_index, argmax_simd_index);
        }

        #[test]
        fn test_many_random_runs() {
            for _ in 0..10_000 {
                let data = get_array_u8(32 * 2 + 1);
                let (argmin_index, argmax_index) = scalar_argminmax(data.view());
                let (argmin_simd_index, argmax_simd_index) =
                    unsafe { NEON::argminmax(data.view()) };
                assert_eq!(argmin_index, argmin_simd_index);
                assert_eq!(argmax_index, argmax_simd_index);
            }
        }
    }
}
