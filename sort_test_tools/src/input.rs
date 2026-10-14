// The benchmark pattern catalog from benches/bench.rs: one row per pattern, in the
// original order, followed by patterns added here. Recipes draw only from the caller's RNG.

use std::fmt;
use std::ops::{Range, RangeInclusive};

use rand::seq::SliceRandom;
use rand::{Rng, RngCore};

use crate::generators::{
    all_equal, ascending, descending, gen_mostly_ascending, gen_mostly_descending, pipe_organ,
    random, random_merge, random_sorted, random_uniform, random_zipf, saw_ascending,
    saw_descending, saw_mixed,
};
use Values::{Constant, Random, Uniform, UniformInclusive};

type Recipe = fn(usize, &mut dyn RngCore) -> Vec<i32>;

// Kept at one row per pattern, which rustfmt would otherwise wrap unevenly.
#[rustfmt::skip]
const CATALOG: &[(&str, Recipe)] = &[
    ("random", |len, rng| random(len, rng)),
    ("random_z1", |len, rng| random_zipf(len, 1.0, rng)),
    ("random_d20", |len, rng| random_uniform(len, 0..20, rng)),
    ("random_p5", |len, rng| random_x_percent(len, 5.0, rng)),
    ("random_s95", |len, rng| random_sorted(len, 95.0, rng)),
    ("ascending", |len, _| ascending(len)),
    ("descending", |len, _| descending(len)),
    // The seven rows above are DEFAULT_PATTERNS. The rest are custom patterns designed to find
    // worst case performance.
    ("random_d20_start_block", |len, rng| {
        let mut values = random_uniform(len, 0..20, rng);
        values[..len.min(100)].fill(0);
        values
    }),
    ("90_one_10_zero", |len, rng| mixture(len, 90.0, Constant(1), Constant(0), rng)),
    ("90_zero_10_one", |len, rng| mixture(len, 90.0, Constant(0), Constant(1), rng)),
    ("90_zero_10_random", |len, rng| mixture(len, 90.0, Constant(0), Random, rng)),
    ("90p_zero_10p_one", |len, rng| shuffled_mixture(len, 90.0, Constant(0), Constant(1), rng)),
    ("90p_zero_10p_random_dense_neg", |len, rng| shuffled_mixture(len, 90.0, Constant(0), UniformInclusive(-10..=10), rng)),
    ("90p_zero_10p_random_dense_pos", |len, rng| shuffled_mixture(len, 90.0, Constant(0), UniformInclusive(0..=10), rng)),
    ("90p_zero_10p_random", |len, rng| shuffled_mixture(len, 90.0, Constant(0), Random, rng)),
    ("95p_zero_5p_random", |len, rng| shuffled_mixture(len, 95.0, Constant(0), Random, rng)),
    ("95p_d4_5p_random", |len, rng| shuffled_mixture(len, 95.0, Uniform(0..4), Random, rng)),
    ("99p_zero_1p_random", |len, rng| shuffled_mixture(len, 99.0, Constant(0), Random, rng)),
    ("saw_ascending", |len, rng| saw_ascending(len, rounded_log2(len), rng)),
    ("saw_descending", |len, rng| saw_descending(len, rounded_log2(len), rng)),
    ("saws_long", |len, rng| saw_mixed(len, rounded_log2(len), rng)),
    ("pipe_organ", |len, rng| pipe_organ(len, rng)),
    ("random__div3", |len, rng| random_uniform(len, 0..=rounded_div(len, 3.0), rng)),
    ("random__div5", |len, rng| random_uniform(len, 0..=rounded_div(len, 5.0), rng)),
    ("random__div8", |len, rng| random_uniform(len, 0..=rounded_div(len, 8.0), rng)),
    ("random_d2", |len, rng| random_uniform(len, 0..2, rng)),
    ("random_d3", |len, rng| random_uniform(len, 0..3, rng)),
    ("random_d4", |len, rng| random_uniform(len, 0..4, rng)),
    ("random_d8", |len, rng| random_uniform(len, 0..8, rng)),
    ("random_d10", |len, rng| random_uniform(len, 0..10, rng)),
    ("random_d16", |len, rng| random_uniform(len, 0..16, rng)),
    ("random_d32", |len, rng| random_uniform(len, 0..32, rng)),
    ("random_d64", |len, rng| random_uniform(len, 0..64, rng)),
    ("random_d128", |len, rng| random_uniform(len, 0..128, rng)),
    ("random_d256", |len, rng| random_uniform(len, 0..256, rng)),
    ("random_d512", |len, rng| random_uniform(len, 0..512, rng)),
    ("random_d1024", |len, rng| random_uniform(len, 0..1024, rng)),
    ("random_p1", |len, rng| random_x_percent(len, 1.0, rng)),
    ("random_p2", |len, rng| random_x_percent(len, 2.0, rng)),
    ("random_p4", |len, rng| random_x_percent(len, 4.0, rng)),
    ("random_p6", |len, rng| random_x_percent(len, 6.0, rng)),
    ("random_p8", |len, rng| random_x_percent(len, 8.0, rng)),
    ("random_p10", |len, rng| random_x_percent(len, 10.0, rng)),
    ("random_p15", |len, rng| random_x_percent(len, 15.0, rng)),
    ("random_p20", |len, rng| random_x_percent(len, 20.0, rng)),
    ("random_p30", |len, rng| random_x_percent(len, 30.0, rng)),
    ("random_p40", |len, rng| random_x_percent(len, 40.0, rng)),
    ("random_p50", |len, rng| random_x_percent(len, 50.0, rng)),
    ("random_p60", |len, rng| random_x_percent(len, 60.0, rng)),
    ("random_p70", |len, rng| random_x_percent(len, 70.0, rng)),
    ("random_p80", |len, rng| random_x_percent(len, 80.0, rng)),
    ("random_p90", |len, rng| random_x_percent(len, 90.0, rng)),
    ("random_p95", |len, rng| random_x_percent(len, 95.0, rng)),
    ("random_p99", |len, rng| random_x_percent(len, 99.0, rng)),
    ("random_z1_05", |len, rng| random_zipf(len, 1.05, rng)),
    ("random_z1_1", |len, rng| random_zipf(len, 1.1, rng)),
    ("random_z1_2", |len, rng| random_zipf(len, 1.2, rng)),
    ("random_z1_3", |len, rng| random_zipf(len, 1.3, rng)),
    ("random_z1_4", |len, rng| random_zipf(len, 1.4, rng)),
    ("random_z1_6", |len, rng| random_zipf(len, 1.6, rng)),
    ("random_z2", |len, rng| random_zipf(len, 2.0, rng)),
    ("random_z3", |len, rng| random_zipf(len, 3.0, rng)),
    ("random_z4", |len, rng| random_zipf(len, 4.0, rng)),
    ("random_s5", |len, rng| random_sorted(len, 5.0, rng)),
    ("random_s10", |len, rng| random_sorted(len, 10.0, rng)),
    ("random_s30", |len, rng| random_sorted(len, 30.0, rng)),
    ("random_s50", |len, rng| random_sorted(len, 50.0, rng)),
    ("random_s70", |len, rng| random_sorted(len, 70.0, rng)),
    ("random_s90", |len, rng| random_sorted(len, 90.0, rng)),
    ("random_s99", |len, rng| random_sorted(len, 99.0, rng)),
    ("random_snl_1", |len, rng| random_sorted_not_last(len, 1, rng)),
    ("random_snl_2", |len, rng| random_sorted_not_last(len, 2, rng)),
    ("random_snl_5", |len, rng| random_sorted_not_last(len, 5, rng)),
    ("random_snl_10", |len, rng| random_sorted_not_last(len, 10, rng)),
    ("random_m5", |len, rng| random_merge(len, 5.0, rng)),
    ("random_m10", |len, rng| random_merge(len, 10.0, rng)),
    ("random_m30", |len, rng| random_merge(len, 30.0, rng)),
    ("random_m50", |len, rng| random_merge(len, 50.0, rng)),
    ("random_m70", |len, rng| random_merge(len, 70.0, rng)),
    ("random_m90", |len, rng| random_merge(len, 90.0, rng)),
    ("random_m95", |len, rng| random_merge(len, 99.0, rng)),
    ("random_m99", |len, rng| random_merge(len, 99.0, rng)),

    // Not part of the original benchmark selection.
    ("all_equal", |len, _| all_equal(len)),

    // From the rust-lang/rust slice sort benchmarks. Upstream's swap loop,
    // `(0usize..).take_while(|x| x * x <= len)`, starts at 0 rather than 1, so it makes
    // isqrt(len) + 1 swaps instead of isqrt(len). The + 1 reproduces that.
    ("mostly_ascending", |len, rng| gen_mostly_ascending(len, len.isqrt() + 1, rng)),
    ("mostly_descending", |len, rng| gen_mostly_descending(len, len.isqrt() + 1, rng)),
];

/// Every benchmark pattern name, in original order, with later additions at the end.
/// The first seven are [`DEFAULT_PATTERNS`]; the benchmark enables the rest with EXTRA_PATTERNS.
pub const PATTERNS: &[&str] = &{
    let mut names = [""; CATALOG.len()];
    let mut i = 0;
    while i < names.len() {
        names[i] = CATALOG[i].0;
        i += 1;
    }
    names
};

/// The patterns the benchmark runs by default: the leading rows of [`PATTERNS`].
pub const DEFAULT_PATTERNS: &[&str] = PATTERNS.split_at(7).0;

/// Generate exactly `len` values with a benchmark pattern name.
///
/// Sampling and shuffling advance the supplied rand 0.8 RNG. Concrete RNGs and
/// borrowed `dyn rand::RngCore` are accepted; no `Clone` or `SeedableRng` bound is
/// required. Deterministic replay requires equivalent starting RNG state and
/// call sequence, with the same crate, dependencies, RNG implementation, and target.
/// Outputs and stochastic draw counts are not promised stable across upgrades.
///
/// All names in [`PATTERNS`] are accepted, including the original random_m95 alias.
/// Valid zero-length inputs return an empty vector without consuming randomness.
/// The fixed patterns `ascending`, `descending`, `90_one_10_zero`,
/// `90_zero_10_one`, and `all_equal` also leave the RNG untouched, as do argument
/// validation errors.
///
/// # Errors
/// Returns [`GenerateError::UnknownPattern`] for an unknown pattern and
/// [`GenerateError::LengthTooLarge`] for a length greater than i32::MAX.
pub fn generate(
    pattern: &str,
    len: usize,
    mut rng: &mut (impl Rng + ?Sized),
) -> Result<Vec<i32>, GenerateError> {
    if len > i32::MAX as usize {
        return Err(GenerateError::LengthTooLarge);
    }
    let Some(&(_, recipe)) = CATALOG.iter().find(|(name, _)| *name == pattern) else {
        return Err(GenerateError::UnknownPattern(pattern.to_owned()));
    };
    if len == 0 {
        return Ok(Vec::new());
    }
    // `&mut R` is a sized `RngCore` even when `R` is not, so it can become a trait object.
    Ok(recipe(len, &mut rng))
}

/// Why [`generate`] rejected its arguments, before drawing any randomness.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum GenerateError {
    /// The pattern name is not in [`PATTERNS`]. The variant holds the rejected name.
    UnknownPattern(String),
    /// The length is greater than `i32::MAX`.
    LengthTooLarge,
}

impl fmt::Display for GenerateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownPattern(name) => write!(f, "unknown benchmark pattern: {name}"),
            Self::LengthTooLarge => f.write_str("length exceeds i32::MAX"),
        }
    }
}

impl std::error::Error for GenerateError {}

enum Values {
    Constant(i32),
    Random,
    Uniform(Range<i32>),
    UniformInclusive(RangeInclusive<i32>),
}

impl Values {
    fn generate(self, len: usize, rng: &mut dyn RngCore) -> Vec<i32> {
        match self {
            Constant(value) => vec![value; len],
            Random => random(len, rng),
            Uniform(range) => random_uniform(len, range, rng),
            UniformInclusive(range) => random_uniform(len, range, rng),
        }
    }
}

// `first_percent` of `len` comes from `first`, the remainder from `second`.
fn mixture(
    len: usize,
    first_percent: f64,
    first: Values,
    second: Values,
    rng: &mut dyn RngCore,
) -> Vec<i32> {
    let first_len = ((len as f64 / 100.0) * first_percent).round() as usize;
    let mut values = first.generate(first_len, rng);
    values.extend(second.generate(len - first_len, rng));
    values
}

fn shuffled_mixture(
    len: usize,
    first_percent: f64,
    first: Values,
    second: Values,
    rng: &mut dyn RngCore,
) -> Vec<i32> {
    let mut values = mixture(len, first_percent, first, second, rng);
    values.shuffle(rng);
    values
}

fn random_x_percent(len: usize, percent: f64, rng: &mut dyn RngCore) -> Vec<i32> {
    assert!(percent > 0.0 && percent < 100.0);

    shuffled_mixture(len, 100.0 - percent, Constant(0), Random, rng)
}

fn random_sorted_not_last(len: usize, last_n_unsorted: usize, rng: &mut dyn RngCore) -> Vec<i32> {
    let sorted_percent = (1.0 - (last_n_unsorted as f64 / len as f64)) * 100.0;
    random_sorted(len, sorted_percent, rng)
}

fn rounded_log2(len: usize) -> usize {
    ((len as f64).log2().round()) as usize
}

fn rounded_div(len: usize, divisor: f64) -> i32 {
    ((len as f64 / divisor).round()) as i32
}
