#![cfg_attr(feature = "test-harness", feature(macro_metavar_expr_concat))]
//! Sorting benchmark input generators and an optional sort correctness harness.
//!
//! [`generate`] accepts the caller's rand 0.8 RNG and produces an owned `Vec<i32>`,
//! or a [`GenerateError`] for an unknown pattern or an oversized length.
//! [`PATTERNS`] lists all 85 benchmark pattern names and [`DEFAULT_PATTERNS`] the
//! seven the benchmark runs by default. Sampling and shuffling use
//! one advancing RNG, without consulting process seeds, caches, or environment
//! variables. The generator interface builds on stable Rust.
//!
//! ```
//! use rand::{rngs::StdRng, SeedableRng};
//! use sort_test_tools::{generate, GenerateError, PATTERNS};
//!
//! let mut rng = StdRng::seed_from_u64(42);
//! let values = generate("random_s95", 100_000, &mut rng)?;
//! assert_eq!(values.len(), 100_000);
//! let mut replay = StdRng::seed_from_u64(42);
//! assert_eq!(values, generate("random_s95", 100_000, &mut replay)?);
//! assert_eq!(
//!     generate("unknown", 100, &mut rng),
//!     Err(GenerateError::UnknownPattern("unknown".into()))
//! );
//! assert!(PATTERNS.contains(&"random_d20"));
//! # Ok::<(), GenerateError>(())
//! ```
//!
//! Each pattern name calls one of the pattern functions exported beside [`generate`],
//! such as [`random_x_percent`] or [`saw_mixed`], with fixed parameters. Call them
//! directly to choose other parameters. Like [`generate`], they draw only from the
//! given RNG and return an empty vector for length zero without drawing. Each one
//! documents the arguments that make it panic.
//!
//! ```
//! use rand::{rngs::StdRng, SeedableRng};
//! use sort_test_tools::{generate, random_x_percent, saw_mixed, GenerateError};
//!
//! let mut rng = StdRng::seed_from_u64(42);
//! let mut replay = rng.clone();
//! assert_eq!(
//!     generate("random_p5", 1_000, &mut rng)?,
//!     random_x_percent(1_000, 5.0, &mut replay)
//! );
//! let sparse = random_x_percent(1_000, 0.5, &mut rng);
//! let three_saws = saw_mixed(1_000, 3, &mut rng);
//! assert_eq!(sparse.len(), three_saws.len());
//! # Ok::<(), GenerateError>(())
//! ```
//!
//! The legacy [`patterns`] module retains its process-wide seeds and caches.
//! Enable `test-harness` on nightly Rust for the correctness tests, test macros,
//! and FFI types.

mod generators;
mod input;

pub use generators::{
    all_equal, ascending, descending, gen_mostly_ascending, gen_mostly_descending, pipe_organ,
    random, random_merge, random_sorted, random_uniform, random_zipf, saw_ascending,
    saw_descending, saw_mixed, saw_mixed_range,
};
pub use input::{
    generate, mixture, random_sorted_not_last, random_x_percent, shuffled_mixture, GenerateError,
    Values, DEFAULT_PATTERNS, PATTERNS,
};

pub trait Sort {
    fn name() -> String;

    fn sort<T>(v: &mut [T])
    where
        T: Ord;

    fn sort_by<T, F>(v: &mut [T], compare: F)
    where
        F: FnMut(&T, &T) -> std::cmp::Ordering;
}

#[cfg(feature = "test-harness")]
pub mod ffi_types;
pub mod patterns;
#[cfg(feature = "test-harness")]
pub mod tests;
