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
//! The legacy [`patterns`] module retains its process-wide seeds and caches.
//! Enable `test-harness` on nightly Rust for the correctness tests, test macros,
//! and FFI types.

mod generators;
mod input;

pub use input::{generate, GenerateError, DEFAULT_PATTERNS, PATTERNS};

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
