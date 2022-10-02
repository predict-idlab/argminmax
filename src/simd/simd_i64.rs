use crate::generic::{max_index_value, min_index_value, simple_argminmax};
use crate::task::{find_final_index_minmax, split_array};
use ndarray::ArrayView1;
use std::arch::x86_64::*;

// ------------------------------------ ARGMINMAX --------------------------------------

pub fn argminmax_i64(arr: ArrayView1<i64>) -> Option<(usize, usize)> {
    match split_array(arr, 4) {
        (Some(rem), Some(sim)) => {
            let (rem_min_index, rem_max_index) = simple_argminmax(rem);
            let rem_result = (
                rem[rem_min_index],
                rem_min_index,
                rem[rem_max_index],
                rem_max_index,
            );
            let sim_result = unsafe { core_argminmax_256(sim, rem.len()) };
            find_final_index_minmax(rem_result, sim_result)
        }
        (Some(rem), None) => {
            let (rem_min_index, rem_max_index) = simple_argminmax(rem);
            Some((rem_min_index, rem_max_index))
        }
        (None, Some(sim)) => {
            let sim_result = unsafe { core_argminmax_256(sim, 0) };
            Some((sim_result.1, sim_result.3))
        }
        (None, None) => None,
    }
}

#[inline]
fn reg_to_i64_arr(reg: __m256i) -> [i64; 4] {
    unsafe { std::mem::transmute::<__m256i, [i64; 4]>(reg) }
}

#[target_feature(enable = "avx2")]
unsafe fn core_argminmax_256(sim_arr: ArrayView1<i64>, offset: usize) -> (i64, usize, i64, usize) {
    // Efficient calculation of argmin and argmax together
    let offset = _mm256_set1_epi64x(offset as i64);
    let mut new_index = _mm256_add_epi64(_mm256_set_epi64x(3, 2, 1, 0), offset);
    let mut index_low = new_index;
    let mut index_high = new_index;

    let increment = _mm256_set1_epi64x(4);

    let new_values = _mm256_loadu_si256(sim_arr.as_ptr() as *const __m256i);
    let mut values_low = new_values;
    let mut values_high = new_values;

    sim_arr
        .exact_chunks(4)
        .into_iter()
        .skip(1)
        .for_each(|step| {
            new_index = _mm256_add_epi64(new_index, increment);

            let new_values = _mm256_loadu_si256(step.as_ptr() as *const __m256i);
            let gt_mask = _mm256_cmpgt_epi64(new_values, values_high);
            // Below does not work (bc instruction is not available)
            //      let lt_mask = _mm256_cmplt_epi64(new_values, values_low);
            // Solution: swap parameters and use gt instead
            let lt_mask = _mm256_cmpgt_epi64(values_low, new_values);

            values_high = _mm256_blendv_epi8(values_high, new_values, gt_mask);
            values_low = _mm256_blendv_epi8(values_low, new_values, lt_mask);

            index_low = _mm256_blendv_epi8(index_low, new_index, lt_mask);
            index_high = _mm256_blendv_epi8(index_high, new_index, gt_mask);

            // Seems to be very marginally faster than blendv_epi8

            // values_high = _mm256_or_si256(
            //     _mm256_and_si256(new_values, gt_mask),
            //     _mm256_andnot_si256(gt_mask, values_high),
            // );
            // values_low = _mm256_or_si256(
            //     _mm256_and_si256(new_values, lt_mask),
            //     _mm256_andnot_si256(lt_mask, values_low),
            // );

            // index_high = _mm256_or_si256(
            //     _mm256_and_si256(new_index, gt_mask),
            //     _mm256_andnot_si256(gt_mask, index_high),
            // );
            // index_low = _mm256_or_si256(
            //     _mm256_and_si256(new_index, lt_mask),
            //     _mm256_andnot_si256(lt_mask, index_low),
            // );
        });

    // Select max_index and max_value
    let value_array = reg_to_i64_arr(values_high);
    let index_array = reg_to_i64_arr(index_high);
    let (index_max, value_max) = max_index_value(&index_array, &value_array);

    // Select min_index and min_value
    let value_array = reg_to_i64_arr(values_low);
    let index_array = reg_to_i64_arr(index_low);
    let (index_min, value_min) = min_index_value(&index_array, &value_array);

    (value_min, index_min as usize, value_max, index_max as usize)
}

//----- TESTS -----

#[cfg(test)]
mod tests {
    use super::{argminmax_i64, simple_argminmax};
    use ndarray::Array1;

    extern crate dev_utils;
    use dev_utils::utils;

    fn get_array_i64(n: usize) -> Array1<i64> {
        utils::get_random_array(n, i64::MIN, i64::MAX)
    }

    #[test]
    fn test_both_versions_return_the_same_results() {
        let data = get_array_i64(1025);
        assert_eq!(data.len() % 4, 1);

        let (argmin_index, argmax_index) = simple_argminmax(data.view());
        let (argmin_simd_index, argmax_simd_index) = argminmax_i64(data.view()).unwrap();
        assert_eq!(argmin_index, argmin_simd_index);
        assert_eq!(argmax_index, argmax_simd_index);
    }

    #[test]
    fn test_first_index_is_returned_when_identical_values_found() {
        let data = [
            std::i64::MIN,
            std::i64::MIN,
            4,
            6,
            9,
            std::i64::MAX,
            22,
            std::i64::MAX,
        ];
        let data: Vec<i64> = data.iter().map(|x| *x).collect();
        let data = Array1::from(data);

        let (argmin_index, argmax_index) = simple_argminmax(data.view());
        assert_eq!(argmin_index, 0);
        assert_eq!(argmax_index, 5);

        let (argmin_simd_index, argmax_simd_index) = argminmax_i64(data.view()).unwrap();
        assert_eq!(argmin_simd_index, 0);
        assert_eq!(argmax_simd_index, 5);
    }

    #[test]
    fn test_many_random_runs() {
        for _ in 0..10_000 {
            let data = get_array_i64(32 * 8 + 1);
            let (argmin_index, argmax_index) = simple_argminmax(data.view());
            let (argmin_simd_index, argmax_simd_index) = argminmax_i64(data.view()).unwrap();
            assert_eq!(argmin_index, argmin_simd_index);
            assert_eq!(argmax_index, argmax_simd_index);
        }
    }
}