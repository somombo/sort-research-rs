use std::cmp;
use std::env;

use criterion::{criterion_group, criterion_main, Criterion};

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

#[allow(unused_imports)]
use sort_test_tools::ffi_types::{FFIOneKibiByte, FFIString, F128};

use sort_test_tools::patterns;
use sort_test_tools::{generate, DEFAULT_PATTERNS, PATTERNS};

#[allow(unused_imports)]
use sort_research_rs::{stable, unstable};

#[cfg(feature = "cold_benchmarks")]
mod trash_prediction;

mod modules;

use crate::modules::bench_len_type_pattern_combo;

fn pattern_input(pattern_name: &str, len: usize) -> Vec<i32> {
    // A fresh StdRng per input, seeded from the thread RNG. Drawing the values from ThreadRng
    // itself would have it reseed from the OS every 64 KiB, a syscall just before the timed sort.
    let mut rng = StdRng::seed_from_u64(rand::thread_rng().gen());

    generate(pattern_name, len, &mut rng).unwrap()
}

fn bench_patterns<T: Ord + std::fmt::Debug>(
    c: &mut Criterion,
    test_len: usize,
    transform_name: &str,
    transform: fn(Vec<i32>) -> Vec<T>,
) {
    if test_len > 100_000 && (transform_name == "string" || transform_name == "1k") {
        // These are just too expensive.
        return;
    }

    let pattern_names = if env::var("EXTRA_PATTERNS").is_ok() {
        PATTERNS
    } else {
        DEFAULT_PATTERNS
    };

    for &pattern_name in pattern_names {
        if test_len < 3 && pattern_name != "random" {
            continue;
        }

        bench_len_type_pattern_combo(
            c,
            test_len,
            transform_name,
            &transform,
            pattern_name,
            &|len| pattern_input(pattern_name, len),
        );
    }
}

fn shift_i32_to_u32(val: i32) -> u32 {
    (val as i64 + (i32::MAX as i64 + 1)) as u32
}

#[allow(unused)]
fn compress_i32(values: &[i32], compression_range: f64) -> impl Iterator<Item = u32> + '_ {
    let mut min_val = u32::MAX;
    let mut max_val = u32::MIN;

    for elem in values {
        let elem_as_u32 = shift_i32_to_u32(*elem);

        min_val = cmp::min(min_val, elem_as_u32);
        max_val = cmp::max(max_val, elem_as_u32);
    }

    let range = max_val - min_val;
    let mult = (compression_range - 1.0) / (range as f64);

    values
        .iter()
        .map(move |val| (((shift_i32_to_u32(*val) - min_val) as f64 * mult).round() as u32))
}

fn extend_i32_to_u64(val: i32) -> u64 {
    // Extends the value into the 64 bit range,
    // while preserving input order.
    (shift_i32_to_u32(val) as u64) * i32::MAX as u64
}

#[allow(unused)]
fn extend_i32_to_u128(val: i32) -> u128 {
    // Extends the value into the 64 bit range,
    // while preserving input order.
    (shift_i32_to_u32(val) as u128) * i64::MAX as u128
}

fn ensure_true_random() {
    // Ensure that the benchmark inputs are actually different from call to call.
    for (pattern_name, len) in [("random", 5), ("random_d1024", 5), ("random_z1", 50)] {
        assert_ne!(
            pattern_input(pattern_name, len),
            pattern_input(pattern_name, len),
            "{pattern_name}"
        );
    }

    // The same for the legacy patterns, which the benchmark no longer draws from.
    let random_vec_a = patterns::random(5);
    let random_vec_b = patterns::random(5);

    // I had a bug, where the test logic for fixed seeds, made the benchmarks always use the same
    // numbers, and random wasn't random at all anymore.
    assert_ne!(random_vec_a, random_vec_b);

    let random_uniform_vec_a = patterns::random_uniform(5, 0..1_000_000);
    let random_uniform_vec_b = patterns::random_uniform(5, 0..1_000_000);
    assert_ne!(random_uniform_vec_a, random_uniform_vec_b);

    let random_zipf_vec_a = patterns::random_zipf(50, 1.0);
    let random_zipf_vec_b = patterns::random_zipf(50, 1.0);
    assert_ne!(random_zipf_vec_a, random_zipf_vec_b);
}

fn criterion_benchmark(c: &mut Criterion) {
    // Distribute points somewhat evenly up to 1e7 in log10 space.
    let test_sizes = [
        0, 1, 2, 3, 4, 6, 8, 10, 12, 17, 24, 35, 49, 70, 100, 200, 400, 900, 2_048, 4_833, 10_000,
        22_367, 50_000, 100_000, 183_845, 400_000, 1_000_000, 2_000_000, 4_281_332, 10_000_000,
    ];

    // Benchmark inputs come from `pattern_input` and do not read the process seed. This keeps
    // the legacy `patterns` module unseeded too, should anything here use it.
    patterns::use_random_seed_each_time();
    ensure_true_random();

    for test_len in test_sizes {
        // Basic type often used to test sorting algorithms.
        bench_patterns(c, test_len, "i32", |values| -> Vec<i32> { values });

        // Common type for usize on 64-bit machines.
        // Sorting indices is very common.
        bench_patterns(c, test_len, "u64", |values| -> Vec<u64> {
            values.into_iter().map(extend_i32_to_u64).collect()
        });

        // Larger type that is not Copy and does heap access.
        // FFI String
        bench_patterns(c, test_len, "string", |values| {
            values
                .into_iter()
                .map(|val| FFIString::new(format!("{:010}", shift_i32_to_u32(val))))
                .collect()
        });

        // Very large stack value.
        bench_patterns(c, test_len, "1k", |values| {
            values.iter().map(|val| FFIOneKibiByte::new(*val)).collect()
        });

        // 16 byte stack value that is Copy but has a relatively expensive cmp implementation.
        bench_patterns(c, test_len, "f128", |values| {
            values.iter().map(|val| F128::new(*val)).collect()
        });

        #[cfg(feature = "bench_type_rust_string")]
        {
            bench_patterns(c, test_len, "rust_string", |values| {
                // Strings are compared lexicographically, so we zero extend them to maintain the input
                // order.
                // See: https://godbolt.org/z/M38zTK6nv and https://godbolt.org/z/G18Yb7zoE
                values
                    .iter()
                    .map(|val| format!("{:010}", val.saturating_abs()))
                    .collect()
            });
        }

        #[cfg(feature = "bench_type_u8")]
        {
            bench_patterns(c, test_len, "u8", |values| -> Vec<u8> {
                compress_i32(&values, 2u32.pow(u8::BITS) as f64)
                    .map(|val| val as u8)
                    .collect()
            });
        }

        #[cfg(feature = "bench_type_u16")]
        {
            bench_patterns(c, test_len, "u16", |values| -> Vec<u16> {
                compress_i32(&values, 2u32.pow(u16::BITS) as f64)
                    .map(|val| val as u16)
                    .collect()
            });
        }

        #[cfg(feature = "bench_type_u32")]
        {
            bench_patterns(c, test_len, "u32", |values| -> Vec<u32> {
                values.into_iter().map(shift_i32_to_u32).collect()
            });
        }

        #[cfg(feature = "bench_type_u128")]
        {
            bench_patterns(c, test_len, "u128", |values| -> Vec<u128> {
                values.into_iter().map(extend_i32_to_u128).collect()
            });
        }

        #[cfg(feature = "bench_type_f32")]
        {
            #[derive(Debug)]
            #[repr(transparent)]
            struct F32NonNanCmp(f32);

            impl F32NonNanCmp {
                fn new(val: i32) -> Self {
                    let val_f32 = f32::from_bits(shift_i32_to_u32(val));
                    let val_non_nan_f32 = if val_f32.is_nan() { 0.0 } else { val_f32 };
                    Self(val_non_nan_f32)
                }
            }

            impl PartialEq for F32NonNanCmp {
                fn eq(&self, other: &Self) -> bool {
                    self.cmp(other) == cmp::Ordering::Equal
                }
            }

            impl Eq for F32NonNanCmp {}

            impl PartialOrd for F32NonNanCmp {
                fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
                    Some(self.cmp(other))
                }
            }

            impl Ord for F32NonNanCmp {
                fn cmp(&self, other: &Self) -> cmp::Ordering {
                    self.0.partial_cmp(&other.0).unwrap()
                    // self.0.partial_cmp(&other.0).unwrap_or(cmp::Ordering::Equal)
                    // unsafe { self.0.partial_cmp(&other.0).unwrap_unchecked() }
                }
            }

            bench_patterns(c, test_len, "f32", |values| -> Vec<F32NonNanCmp> {
                values.into_iter().map(F32NonNanCmp::new).collect()
            });
        }

        #[cfg(feature = "bench_type_f64")]
        {
            #[derive(Debug)]
            #[repr(transparent)]
            struct F64NonNanCmp(f64);

            impl F64NonNanCmp {
                fn new(val: i32) -> Self {
                    let val_f64 = f64::from_bits(extend_i32_to_u64(val));
                    let val_non_nan_f32 = if val_f64.is_nan() { 0.0 } else { val_f64 };
                    Self(val_non_nan_f32)
                }
            }

            impl PartialEq for F64NonNanCmp {
                fn eq(&self, other: &Self) -> bool {
                    self.cmp(other) == cmp::Ordering::Equal
                }
            }

            impl Eq for F64NonNanCmp {}

            impl PartialOrd for F64NonNanCmp {
                fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
                    Some(self.cmp(other))
                }
            }

            impl Ord for F64NonNanCmp {
                fn cmp(&self, other: &Self) -> cmp::Ordering {
                    self.0.partial_cmp(&other.0).unwrap()
                    // self.0.partial_cmp(&other.0).unwrap_or(cmp::Ordering::Equal)
                    // unsafe { self.0.partial_cmp(&other.0).unwrap_unchecked() }
                }
            }

            bench_patterns(c, test_len, "f64", |values| -> Vec<F64NonNanCmp> {
                values.into_iter().map(F64NonNanCmp::new).collect()
            });
        }

        #[cfg(feature = "bench_type_f32_total")]
        {
            #[derive(Debug)]
            #[repr(transparent)]
            struct F32TotalCmp(f32);

            impl F32TotalCmp {
                fn new(val: i32) -> Self {
                    Self(f32::from_bits(shift_i32_to_u32(val)))
                }
            }

            impl PartialEq for F32TotalCmp {
                fn eq(&self, other: &Self) -> bool {
                    self.cmp(other) == cmp::Ordering::Equal
                }
            }

            impl Eq for F32TotalCmp {}

            impl PartialOrd for F32TotalCmp {
                fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
                    Some(self.cmp(other))
                }
            }

            impl Ord for F32TotalCmp {
                fn cmp(&self, other: &Self) -> cmp::Ordering {
                    self.0.total_cmp(&other.0)
                }
            }

            bench_patterns(c, test_len, "f32_total", |values| -> Vec<F32TotalCmp> {
                values.into_iter().map(F32TotalCmp::new).collect()
            });
        }

        #[cfg(feature = "bench_type_f64_total")]
        {
            #[derive(Debug)]
            #[repr(transparent)]
            struct F64TotalCmp(f64);

            impl F64TotalCmp {
                fn new(val: i32) -> Self {
                    Self(f64::from_bits(extend_i32_to_u64(val)))
                }
            }

            impl PartialEq for F64TotalCmp {
                fn eq(&self, other: &Self) -> bool {
                    self.cmp(other) == cmp::Ordering::Equal
                }
            }

            impl Eq for F64TotalCmp {}

            impl PartialOrd for F64TotalCmp {
                fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
                    Some(self.cmp(other))
                }
            }

            impl Ord for F64TotalCmp {
                fn cmp(&self, other: &Self) -> cmp::Ordering {
                    self.0.total_cmp(&other.0)
                }
            }

            bench_patterns(c, test_len, "f64_total", |values| -> Vec<F64TotalCmp> {
                values.into_iter().map(F64TotalCmp::new).collect()
            });
        }

        #[cfg(feature = "bench_type_val_with_mutex")]
        {
            use std::cmp::Ordering;
            use std::sync::Mutex;

            #[derive(Debug)]
            struct ValWithMutex {
                val: i32,
                mutex: Mutex<u64>,
            }

            impl PartialEq for ValWithMutex {
                fn eq(&self, other: &Self) -> bool {
                    self.val == other.val
                }
            }

            impl Eq for ValWithMutex {}

            impl PartialOrd for ValWithMutex {
                fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                    Some(self.cmp(other))
                }
            }

            impl Ord for ValWithMutex {
                fn cmp(&self, other: &Self) -> Ordering {
                    self.val.cmp(&other.val)
                }
            }

            bench_patterns(c, test_len, "val_with_mutex", |values| {
                values
                    .into_iter()
                    .map(|val| -> ValWithMutex {
                        let this = ValWithMutex {
                            val,
                            mutex: Mutex::new(val.wrapping_abs() as u64),
                        };

                        // To make sure the mutex is not optimized away.
                        criterion::black_box(this.val * *this.mutex.lock().unwrap() as i32);

                        this
                    })
                    .collect()
            });
        }

        #[allow(unused)]
        macro_rules! bench_type_u64_x_bytes {
            ($bytes:expr, $name:expr) => {
                #[derive(Debug)]
                #[repr(C)]
                struct U64XBytes {
                    val: u64,
                    _padding: [u8; ($bytes - 8)],
                }

                impl U64XBytes {
                    fn new(val: i32) -> Self {
                        Self {
                            val: extend_i32_to_u64(val),
                            _padding: [0; ($bytes - 8)],
                        }
                    }
                }

                impl PartialEq for U64XBytes {
                    fn eq(&self, other: &Self) -> bool {
                        self.cmp(other) == cmp::Ordering::Equal
                    }
                }

                impl Eq for U64XBytes {}

                impl PartialOrd for U64XBytes {
                    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
                        Some(self.cmp(other))
                    }
                }

                impl Ord for U64XBytes {
                    fn cmp(&self, other: &Self) -> cmp::Ordering {
                        self.val.cmp(&other.val)
                    }
                }

                assert_eq!(std::mem::size_of::<U64XBytes>(), $bytes);

                bench_patterns(c, test_len, $name, |values| -> Vec<U64XBytes> {
                    values.into_iter().map(U64XBytes::new).collect()
                });
            };
        }

        #[cfg(feature = "bench_type_u64_16b")]
        {
            bench_type_u64_x_bytes!(16, "u64_16b");
        }

        #[cfg(feature = "bench_type_u64_32b")]
        {
            bench_type_u64_x_bytes!(32, "u64_32b");
        }

        #[cfg(feature = "bench_type_u64_64b")]
        {
            bench_type_u64_x_bytes!(64, "u64_64b");
        }

        #[cfg(feature = "bench_type_u64_96b")]
        {
            bench_type_u64_x_bytes!(96, "u64_96b");
        }

        #[cfg(feature = "bench_type_u64_128b")]
        {
            bench_type_u64_x_bytes!(128, "u64_128b");
        }
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
