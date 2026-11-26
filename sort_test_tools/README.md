# sort_test_tools

Sorting benchmark input generators and an optional correctness harness, from
sort-research-rs by Lukas Bergdoll and contributors. Licensed under Apache-2.0.

The generator interface builds on stable Rust:

```rust
use rand::{rngs::StdRng, SeedableRng};
use sort_test_tools::{generate, GenerateError, PATTERNS};

let mut rng = StdRng::seed_from_u64(42);
let values: Vec<i32> = generate("random_s95", 100_000, &mut rng)?;
assert_eq!(values.len(), 100_000);
let mut replay = StdRng::seed_from_u64(42);
assert_eq!(values, generate("random_s95", 100_000, &mut replay)?);
assert_eq!(
    generate("unknown", 100, &mut rng),
    Err(GenerateError::UnknownPattern("unknown".into()))
);
assert_eq!(PATTERNS.len(), 85);
```

Successful calls produce fresh owned vectors. Sampling and shuffling advance the
same caller-supplied rand 0.8 RNG; `generate` uses no internal seeds, caches, or
environment configuration. Concrete RNGs and borrowed `dyn rand::RngCore` are
accepted without `Clone` or `SeedableRng` bounds.

All accepted patterns return an empty vector for length zero without consuming
randomness. The five fixed patterns (`ascending`, `descending`, `90_one_10_zero`,
`90_zero_10_one`, and `all_equal`) also leave the RNG untouched. Unknown names and
lengths exceeding `i32::MAX` return a `GenerateError` before consuming randomness,
so the caller can keep using the RNG.

`PATTERNS` is the catalog `benches/bench.rs` iterates: its seven default
patterns, also exported as `DEFAULT_PATTERNS`, followed by 78 extra patterns.
Original signed distributions, range endpoints, rounding, and structural
formulas are preserved. The upstream `random_m95` name uses a 99% first run,
matching `random_m99`. The last three extra patterns are not part of the
original benchmark. `all_equal` repeats the value 66, like the legacy
`patterns::all_equal`. `mostly_ascending` and `mostly_descending` follow
`gen_mostly_ascending` and `gen_mostly_descending` from the rust-lang/rust slice
sort benchmarks (`library/alloctests/benches/slice.rs`): `0..len` in order, then
`isqrt(len) + 1` swaps of positions drawn as `u32 % len`, each swap taking two
`next_u32` draws.

Caller-owned RNG sequencing intentionally differs from the legacy harness for
mixed saws and shuffled random mixtures. Replay requires equivalent starting RNG
state and call sequence, with the same dependencies, RNG implementation, and
target. Outputs and draw counts are not promised stable across upgrades or
architectures. Direct dependencies are pinned to `rand = "=0.8.5"` and
`zipf = "=7.0.0"`; downstream projects resolve transitive versions in their own
`Cargo.lock`.

## Pattern functions

Each catalog name calls one of the pattern functions exported at the crate root,
with fixed parameters. Call them directly to choose other parameters:

```rust
use rand::{rngs::StdRng, SeedableRng};
use sort_test_tools::{generate, random_x_percent, saw_mixed, shuffled_mixture, Values};

let mut rng = StdRng::seed_from_u64(42);
let mut replay = rng.clone();
assert_eq!(
    generate("random_p5", 100_000, &mut rng)?,
    random_x_percent(100_000, 5.0, &mut replay)
);
let sparse = random_x_percent(100_000, 0.5, &mut rng);
let three_saws = saw_mixed(100_000, 3, &mut rng);
let low_cardinality = shuffled_mixture(
    100_000,
    80.0,
    Values::Constant(0),
    Values::UniformInclusive(-3..=3),
    &mut rng,
);
```

| Patterns | Function and parameters |
| --- | --- |
| `random` | `random` |
| `random_d<N>`, `random__div<N>` | `random_uniform` over `0..N`, or over `0..=round(len / N)` |
| `random_d20_start_block` | `random_uniform` over `0..20`, then the first 100 values set to 0 |
| `random_z<E>` | `random_zipf` with exponent `E`, reading `_` as a decimal point |
| `random_p<P>` | `random_x_percent` with `P` percent |
| `random_s<P>`, `random_m<P>` | `random_sorted`, `random_merge` with `P` percent (99 for `random_m95`) |
| `random_snl_<N>` | `random_sorted_not_last` with `N` unsorted |
| `90_one_10_zero`, `90_zero_10_one`, `90_zero_10_random` | `mixture` at 90 percent |
| `90p_*`, `95p_*`, `99p_*` | `shuffled_mixture` at the leading percentage |
| `saw_ascending`, `saw_descending`, `saws_long` | `saw_ascending`, `saw_descending`, `saw_mixed` with `round(log2(len))` saws |
| `mostly_ascending`, `mostly_descending` | `gen_mostly_ascending`, `gen_mostly_descending` with `isqrt(len) + 1` swaps |
| `ascending`, `descending`, `all_equal`, `pipe_organ` | the same name |

`src/input.rs` has the exact call for every name. Like `generate`, the functions
draw only from the caller's RNG and return an empty vector for length zero
without drawing. Each function's Rustdoc lists the arguments that make it panic,
such as a percentage outside `0..=100`, an empty range, or a Zipf exponent that
is not finite and positive.

## Using the library

For local development, depend on this directory:

```toml
[dependencies]
sort_test_tools = { path = "../sort-research-rs/sort_test_tools", default-features = false }
rand = "=0.8.5"
```

After the changes are committed and pushed, consumers can depend on the fork:

```toml
[dependencies]
sort_test_tools = { git = "https://github.com/somombo/sort-research-rs", default-features = false }
rand = "=0.8.5"
```

Cargo discovers the nested crate from the repository URL. Pin `rev` to a commit
containing this interface when fixing the generator version for a benchmark.
A workspace and crates.io publication are optional.

The generator build uses only `rand` and `zipf`; it does not build the research
crate, sort implementations, benchmark runner, or native build scripts. Both the
default features and `default-features = false` support stable Rust.

## Legacy harness

The existing `patterns` functions retain their process seeds, caches,
`OVERRIDE_SEED`, and `use_random_seed_each_time` behavior, and serve the
correctness tests. Use `generate` or the pattern functions, which share many
`patterns` names but take an RNG, when the caller should own the RNG. The
criterion benchmark is such a caller: it draws every input with `generate` and a
fresh `StdRng` seeded from `rand::thread_rng()`, so its inputs do not depend on
the process seed. At startup it checks that repeated inputs differ. It still
calls `use_random_seed_each_time` and runs the same check on `patterns`, so the
legacy module stays unseeded in that process.

Enable `test-harness` on nightly Rust to expose `tests`, `ffi_types`, and the
`instantiate_sort_tests!` macros. The research crate and ipnsort explicitly enable
this feature. `large_test_sizes` controls the largest correctness test inputs and
remains enabled by default.

```sh
# Run generator behavior tests, upstream fixtures, and the documentation example.
cargo +stable test --manifest-path sort_test_tools/Cargo.toml --locked
cargo +stable test --manifest-path sort_test_tools/Cargo.toml --locked --no-default-features

# Include the legacy sort correctness suite, with the largest sizes disabled.
cargo test --manifest-path sort_test_tools/Cargo.toml --locked --no-default-features --features test-harness
```

The fixtures retain 171 independently generated upstream hashes on Linux x86_64
GNU. Behavior tests cover all 85 names, boundary lengths, mixtures, aliases,
validation, replay, trait-object RNGs, and advancing RNG state, and check that
the pattern functions reproduce the catalog.
