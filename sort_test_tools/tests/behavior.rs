use rand::{rngs::StdRng, Rng, RngCore, SeedableRng};
use sort_test_tools::{generate, GenerateError, DEFAULT_PATTERNS, PATTERNS};

#[test]
fn catalog_length_and_replay_do_not_depend_on_call_history() {
    assert_eq!(PATTERNS.len(), 83);
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
