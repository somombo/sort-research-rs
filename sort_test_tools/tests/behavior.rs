use rand::{rngs::StdRng, Rng, RngCore, SeedableRng};
use sort_test_tools::{generate, GenerateError, DEFAULT_PATTERNS, PATTERNS};

#[test]
fn catalog_length_and_replay_do_not_depend_on_call_history() {
    assert_eq!(PATTERNS.len(), 85);
    for (index, &pattern) in PATTERNS.iter().enumerate() {
        assert!(!PATTERNS[..index].contains(&pattern));
        for len in [0, 1, 2, 3, 17, 100, 101, 257, 2_048] {
            let first = seeded_generate(pattern, len, 42);
            assert_eq!(first.len(), len, "{pattern}");
            let _ = seeded_generate(pattern, len + 19, 7);
            assert_eq!(first, seeded_generate(pattern, len, 42), "{pattern}/{len}");
        }
    }
}

#[test]
fn order_and_duplicate_structure_matches_original_benchmark() {
    let len = 101;
    let ascending: Vec<i32> = (0..len as i32).collect();
    assert_eq!(seeded_generate("ascending", len, 0), ascending);
    let descending: Vec<_> = ascending.into_iter().rev().collect();
    assert_eq!(seeded_generate("descending", len, u64::MAX), descending);
    assert_eq!(seeded_generate("all_equal", len, 42), vec![66; len]);

    let raw = seeded_generate("random", len, 42);
    let sorted = seeded_generate("random_s95", len, 42);
    let prefix = (len as f64 * 0.95).round() as usize;
    assert!(sorted[..prefix].windows(2).all(|w| w[0] <= w[1]));
    assert_eq!(&sorted[prefix..], &raw[prefix..]);
    let mut expected_prefix = raw[..prefix].to_vec();
    expected_prefix.sort_unstable();
    assert_eq!(sorted[..prefix], expected_prefix);

    assert_eq!(
        seeded_generate("random_m95", len, 42),
        seeded_generate("random_m99", len, 42)
    );
    let binary = seeded_generate("random_d2", len, 42);
    assert!(binary.iter().all(|&v| v == 0 || v == 1));
    let blocked = seeded_generate("random_d20_start_block", len, 42);
    assert!(blocked[..100].iter().all(|&v| v == 0));
}

#[test]
fn seeded_shuffling_preserves_the_mixture() {
    let len = 1_000;
    let mut expected = vec![0; 950];
    expected.extend(seeded_generate("random", 50, 42));
    expected.sort_unstable();
    let mut actual = seeded_generate("random_p5", len, 42);
    actual.sort_unstable();
    assert_eq!(actual, expected);

    let mixture = seeded_generate("90p_zero_10p_one", len, 42);
    assert_eq!(mixture.iter().filter(|&&v| v == 0).count(), 900);
    assert_eq!(mixture.iter().filter(|&&v| v == 1).count(), 100);
    assert_ne!(mixture, seeded_generate("90p_zero_10p_one", len, 43));
}

#[test]
fn unknown_pattern_is_rejected_even_for_empty_input() {
    let error = generate("unknown", 0, &mut NoDrawRng).unwrap_err();
    assert_eq!(error, GenerateError::UnknownPattern("unknown".into()));
    assert_eq!(error.to_string(), "unknown benchmark pattern: unknown");
}

#[test]
fn overflowing_length_is_rejected_before_allocating() {
    assert_eq!(
        generate("random", i32::MAX as usize + 1, &mut NoDrawRng),
        Err(GenerateError::LengthTooLarge)
    );
}

fn seeded_generate(pattern: &str, len: usize, seed: u64) -> Vec<i32> {
    generate(pattern, len, &mut StdRng::seed_from_u64(seed)).unwrap()
}

struct NoDrawRng;

impl RngCore for NoDrawRng {
    fn next_u32(&mut self) -> u32 {
        panic!("unexpected RNG draw");
    }

    fn next_u64(&mut self) -> u64 {
        panic!("unexpected RNG draw");
    }

    fn fill_bytes(&mut self, _: &mut [u8]) {
        panic!("unexpected RNG draw");
    }

    fn try_fill_bytes(&mut self, _: &mut [u8]) -> Result<(), rand::Error> {
        panic!("unexpected RNG draw");
    }
}

#[test]
fn empty_and_fixed_patterns_do_not_draw_randomness() {
    for &pattern in PATTERNS {
        assert!(
            generate(pattern, 0, &mut NoDrawRng).unwrap().is_empty(),
            "{pattern}"
        );
    }
    for pattern in [
        "ascending",
        "descending",
        "90_one_10_zero",
        "90_zero_10_one",
        "all_equal",
    ] {
        for len in [1, 2, 17, 257] {
            assert_eq!(generate(pattern, len, &mut NoDrawRng).unwrap().len(), len);
        }
    }
}

#[test]
fn consecutive_calls_continue_the_callers_stream() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut replay = rng.clone();
    let mut actual = generate("random", 17, &mut rng).unwrap();
    actual.extend(generate("random", 257, &mut rng).unwrap());
    assert_eq!(actual, generate("random", 274, &mut replay).unwrap());
    assert_eq!(rng.next_u64(), replay.next_u64());
}

#[test]
fn concrete_and_borrowed_trait_object_rngs_are_supported() {
    use rand::rngs::mock::StepRng;

    let mut concrete = StepRng::new(7, 11);
    let mut replay = StepRng::new(7, 11);
    let expected: Vec<i32> = (0..17).map(|_| replay.gen()).collect();
    assert_eq!(generate("random", 17, &mut concrete).unwrap(), expected);
    assert_eq!(concrete.next_u64(), replay.next_u64());

    let mut concrete = StdRng::seed_from_u64(42);
    let mut replay = concrete.clone();
    let borrowed: &mut dyn RngCore = &mut concrete;
    assert_eq!(
        generate("random_z1", 257, borrowed).unwrap(),
        generate("random_z1", 257, &mut replay).unwrap()
    );
    assert_eq!(concrete.next_u64(), replay.next_u64());
}

#[test]
fn shuffled_random_mixtures_share_the_sampling_stream() {
    use rand::{
        distributions::{Distribution, Uniform},
        seq::SliceRandom,
    };

    // Independent distribution calls also check the RNG state after shuffling.
    // Exercise each composition: one source, uniform source, and two sources.
    for seed in [0, 42, u64::MAX] {
        for &pattern in PATTERNS {
            let len = 257;
            let mut rng = StdRng::seed_from_u64(seed);
            let mut reference = rng.clone();
            let mut expected = if let Some(percent) = pattern
                .strip_prefix("random_p")
                .and_then(|percent| percent.parse::<f64>().ok())
            {
                let zero_len = ((len as f64 / 100.0) * (100.0 - percent)).round() as usize;
                let mut values = vec![0; zero_len];
                values.extend((zero_len..len).map(|_| reference.gen::<i32>()));
                values
            } else {
                let percent = match pattern {
                    "90p_zero_10p_random_dense_neg"
                    | "90p_zero_10p_random_dense_pos"
                    | "90p_zero_10p_random" => 90.0,
                    "95p_zero_5p_random" | "95p_d4_5p_random" => 95.0,
                    "99p_zero_1p_random" => 99.0,
                    _ => continue,
                };
                let first_len = ((len as f64 / 100.0) * percent).round() as usize;
                let mut values = if pattern == "95p_d4_5p_random" {
                    let dist = Uniform::from(0..4);
                    (0..first_len)
                        .map(|_| dist.sample(&mut reference))
                        .collect()
                } else {
                    vec![0; first_len]
                };
                match pattern {
                    "90p_zero_10p_random_dense_neg" => {
                        let dist = Uniform::from(-10..=10);
                        values.extend((first_len..len).map(|_| dist.sample(&mut reference)));
                    }
                    "90p_zero_10p_random_dense_pos" => {
                        let dist = Uniform::from(0..=10);
                        values.extend((first_len..len).map(|_| dist.sample(&mut reference)));
                    }
                    _ => values.extend((first_len..len).map(|_| reference.gen::<i32>())),
                }
                values
            };
            expected.shuffle(&mut reference);
            assert_eq!(
                generate(pattern, len, &mut rng).unwrap(),
                expected,
                "{pattern}/{seed}"
            );
            assert_eq!(
                rng.next_u64(),
                reference.next_u64(),
                "{pattern}/{seed} state"
            );
        }
    }
}

#[test]
fn mixed_saws_sample_directions_after_values() {
    use rand::distributions::{Distribution, Uniform};

    for seed in [0, 42, u64::MAX] {
        for len in [1_usize, 2, 3, 17, 257, 2_048] {
            let mut rng = StdRng::seed_from_u64(seed);
            let mut reference = rng.clone();
            let mut expected: Vec<i32> = (0..len).map(|_| reference.gen()).collect();
            let saw_count = (len as f64).log2().round() as usize;
            let chunk_size = len / saw_count.max(1);
            let directions = Uniform::from(0..=1);
            let directions: Vec<i32> = (0..(len / chunk_size) + 1)
                .map(|_| directions.sample(&mut reference))
                .collect();
            for (chunk, direction) in expected.chunks_mut(chunk_size).zip(directions) {
                chunk.sort_unstable();
                if direction == 1 {
                    chunk.reverse();
                }
            }
            assert_eq!(
                generate("saws_long", len, &mut rng).unwrap(),
                expected,
                "{len}/{seed}"
            );
            assert_eq!(rng.next_u64(), reference.next_u64(), "{len}/{seed} state");
        }
    }
}

#[test]
fn default_patterns_are_the_original_benchmark_selection() {
    assert_eq!(
        DEFAULT_PATTERNS,
        [
            "random",
            "random_z1",
            "random_d20",
            "random_p5",
            "random_s95",
            "ascending",
            "descending",
        ]
    );
    assert_eq!(DEFAULT_PATTERNS, &PATTERNS[..DEFAULT_PATTERNS.len()]);
}

#[test]
fn mostly_sorted_patterns_match_upstream_swaps() {
    // gen_mostly_ascending and gen_mostly_descending in rust-lang/rust
    // library/alloctests/benches/slice.rs.
    for seed in [0, 42, u64::MAX] {
        for len in [1_usize, 2, 3, 4, 17, 100, 257, 2_048] {
            for pattern in ["mostly_ascending", "mostly_descending"] {
                let mut rng = StdRng::seed_from_u64(seed);
                let mut reference = rng.clone();
                let mut expected: Vec<i32> = (0..len as i32).collect();
                if pattern == "mostly_descending" {
                    expected.reverse();
                }
                let swaps = (0..).take_while(|x| x * x <= len).count();
                assert_eq!(swaps, (len as f64).sqrt().floor() as usize + 1);
                for _ in 0..swaps {
                    let x = reference.next_u32() as usize % len;
                    let y = reference.next_u32() as usize % len;
                    expected.swap(x, y);
                }

                let actual = generate(pattern, len, &mut rng).unwrap();
                assert_eq!(actual, expected, "{pattern}/{len}/{seed}");
                assert_eq!(
                    rng.next_u64(),
                    reference.next_u64(),
                    "{pattern}/{len}/{seed} state"
                );

                let mut sorted = actual;
                sorted.sort_unstable();
                assert!(sorted.iter().copied().eq(0..len as i32));
            }
        }
    }
}

// The pattern functions exported beside `generate`, called directly.
mod pattern_functions {
    use super::NoDrawRng;
    use rand::{rngs::StdRng, RngCore, SeedableRng};
    use sort_test_tools::{
        all_equal, ascending, descending, gen_mostly_ascending, gen_mostly_descending, generate,
        mixture, pipe_organ, random, random_merge, random_sorted, random_sorted_not_last,
        random_uniform, random_x_percent, random_zipf, saw_ascending, saw_descending, saw_mixed,
        saw_mixed_range, shuffled_mixture, Values,
    };

    type Call = fn(usize, &mut StdRng) -> Vec<i32>;

    // Catalog rows rewritten with the exported functions, covering each one the catalog uses.
    #[rustfmt::skip]
    const CATALOG_CALLS: &[(&str, Call)] = &[
        ("random", |len, rng| random(len, rng)),
        ("random_z1_05", |len, rng| random_zipf(len, 1.05, rng)),
        ("random_d20", |len, rng| random_uniform(len, 0..20, rng)),
        ("random__div5", |len, rng| random_uniform(len, 0..=(len as f64 / 5.0).round() as i32, rng)),
        ("random_p5", |len, rng| random_x_percent(len, 5.0, rng)),
        ("random_s95", |len, rng| random_sorted(len, 95.0, rng)),
        ("random_snl_2", |len, rng| random_sorted_not_last(len, 2, rng)),
        ("random_m30", |len, rng| random_merge(len, 30.0, rng)),
        ("ascending", |len, _| ascending(len)),
        ("descending", |len, _| descending(len)),
        ("all_equal", |len, _| all_equal(len)),
        ("90_zero_10_random", |len, rng| mixture(len, 90.0, Values::Constant(0), Values::Random, rng)),
        ("90p_zero_10p_random_dense_neg", |len, rng| {
            shuffled_mixture(len, 90.0, Values::Constant(0), Values::UniformInclusive(-10..=10), rng)
        }),
        ("95p_d4_5p_random", |len, rng| shuffled_mixture(len, 95.0, Values::Uniform(0..4), Values::Random, rng)),
        ("saw_ascending", |len, rng| saw_ascending(len, rounded_log2(len), rng)),
        ("saw_descending", |len, rng| saw_descending(len, rounded_log2(len), rng)),
        ("saws_long", |len, rng| saw_mixed(len, rounded_log2(len), rng)),
        ("pipe_organ", |len, rng| pipe_organ(len, rng)),
        ("mostly_ascending", |len, rng| gen_mostly_ascending(len, len.isqrt() + 1, rng)),
        ("mostly_descending", |len, rng| gen_mostly_descending(len, len.isqrt() + 1, rng)),
    ];

    fn rounded_log2(len: usize) -> usize {
        (len as f64).log2().round() as usize
    }

    // Length 0 included: `generate` returns early there, so the direct calls must not draw.
    #[test]
    fn catalog_patterns_are_direct_calls() {
        for &(pattern, call) in CATALOG_CALLS {
            for seed in [0, 42, u64::MAX] {
                for len in [0, 1, 2, 3, 17, 100, 101, 257, 2_048] {
                    let mut rng = StdRng::seed_from_u64(seed);
                    let mut replay = rng.clone();
                    let expected = generate(pattern, len, &mut rng).unwrap();
                    assert_eq!(call(len, &mut replay), expected, "{pattern}/{len}/{seed}");
                    assert_eq!(
                        replay.next_u64(),
                        rng.next_u64(),
                        "{pattern}/{len}/{seed} state"
                    );
                }
            }
        }
    }

    #[test]
    fn more_saws_than_values_leaves_the_values_unsorted() {
        for len in [1, 2, 17] {
            let raw = random(len, &mut StdRng::seed_from_u64(42));
            for saw_count in [len + 1, usize::MAX] {
                let seeded = || StdRng::seed_from_u64(42);
                assert_eq!(saw_ascending(len, saw_count, &mut seeded()), raw);
                assert_eq!(saw_descending(len, saw_count, &mut seeded()), raw);
                assert_eq!(saw_mixed(len, saw_count, &mut seeded()), raw);
            }
        }
    }

    // These use NoDrawRng, so a check that went missing fails on an unexpected draw instead of
    // in the ways each comment describes.

    // Without the check, ZipfDistribution would sample forever.
    #[test]
    #[should_panic(expected = "Zipf exponent NaN is not finite and positive")]
    fn zipf_rejects_a_nan_exponent() {
        random_zipf(100, f64::NAN, &mut NoDrawRng);
    }

    #[test]
    #[should_panic(expected = "Zipf exponent inf is not finite and positive")]
    fn zipf_rejects_an_infinite_exponent() {
        random_zipf(100, f64::INFINITY, &mut NoDrawRng);
    }

    // Without the check, a zero-length saw would panic only if drawn.
    #[test]
    #[should_panic(expected = "saw length range 0..5 must be nonempty, start above 0")]
    fn saw_mixed_range_rejects_zero_length_saws() {
        saw_mixed_range(100, 0..5, &mut NoDrawRng);
    }

    // Without the check, the length would wrap and give an empty vector.
    #[test]
    #[should_panic(expected = "length 2147483648 exceeds i32::MAX")]
    fn ascending_rejects_lengths_above_i32_max() {
        ascending(i32::MAX as usize + 1);
    }

    // Without the check, this would work below length 125 and panic from there on.
    #[test]
    #[should_panic(expected = "(0.0..=100.0).contains(&sorted_percent)")]
    fn random_sorted_rejects_a_percentage_above_100() {
        random_sorted(10, 100.4, &mut NoDrawRng);
    }

    // Without the check, NaN would count as 0% and give one fully sorted run.
    #[test]
    #[should_panic(expected = "(0.0..=100.0).contains(&first_run_percent)")]
    fn random_merge_rejects_a_nan_percentage() {
        random_merge(10, f64::NAN, &mut NoDrawRng);
    }

    // Without the check, a negative percentage would take every value from `second`.
    #[test]
    #[should_panic(expected = "(0.0..=100.0).contains(&first_percent)")]
    fn mixture_rejects_a_negative_percentage() {
        mixture(
            10,
            -5.0,
            Values::Constant(0),
            Values::Random,
            &mut NoDrawRng,
        );
    }
}
