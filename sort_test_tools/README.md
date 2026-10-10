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
assert_eq!(PATTERNS.len(), 83);
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
patterns, also exported as `DEFAULT_PATTERNS`, followed by 76 extra patterns.
Original signed distributions, range endpoints, rounding, and structural
formulas are preserved. The upstream `random_m95` name uses a 99% first run,
matching `random_m99`. The last extra pattern, `all_equal`, is not part of the
upstream benchmark: it repeats the value 66, like the legacy
`patterns::all_equal`.

Caller-owned RNG sequencing intentionally differs from the legacy harness for
mixed saws and shuffled random mixtures. Replay requires equivalent starting RNG
state and call sequence, with the same dependencies, RNG implementation, and
target. Outputs and draw counts are not promised stable across upgrades or
architectures. Direct dependencies are pinned to `rand = "=0.8.5"` and
`zipf = "=7.0.0"`; downstream projects resolve transitive versions in their own
`Cargo.lock`.

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
correctness tests. Use `generate` when the caller should own the RNG. The
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
GNU. Behavior tests cover all 83 names, boundary lengths, mixtures, aliases,
validation, replay, trait-object RNGs, and advancing RNG state.
