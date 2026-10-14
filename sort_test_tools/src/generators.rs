// Sampling and structural helpers shared by the caller-owned RNG interface and
// the legacy seed/cache wrappers. RNG creation and cache policy stay with callers.

use rand::prelude::*;
use zipf::ZipfDistribution;

pub(crate) fn random(len: usize, rng: &mut (impl Rng + ?Sized)) -> Vec<i32> {
    (0..len).map(|_| rng.gen::<i32>()).collect()
}

pub(crate) fn random_uniform<R>(len: usize, range: R, rng: &mut (impl Rng + ?Sized)) -> Vec<i32>
where
    R: Into<rand::distributions::Uniform<i32>>,
{
    let dist: rand::distributions::Uniform<i32> = range.into();
    (0..len).map(|_| dist.sample(rng)).collect()
}

pub(crate) fn random_zipf(len: usize, exponent: f64, rng: &mut (impl Rng + ?Sized)) -> Vec<i32> {
    let dist = ZipfDistribution::new(len, exponent).unwrap();
    (0..len).map(|_| dist.sample(rng) as i32).collect()
}

pub(crate) fn all_equal(len: usize) -> Vec<i32> {
    (0..len).map(|_| 66).collect::<Vec<_>>()
}

pub(crate) fn ascending(len: usize) -> Vec<i32> {
    (0..len as i32).collect()
}

pub(crate) fn descending(len: usize) -> Vec<i32> {
    (0..len as i32).rev().collect()
}

pub(crate) fn sort_prefix(values: &mut [i32], sorted_percent: f64) {
    let sorted_len = ((values.len() as f64) * (sorted_percent / 100.0)).round() as usize;
    values[..sorted_len].sort_unstable();
}

pub(crate) fn sort_runs(values: &mut [i32], first_run_percent: f64) {
    let first_run_len = ((values.len() as f64) * (first_run_percent / 100.0)).round() as usize;
    values[..first_run_len].sort_unstable();
    values[first_run_len..].sort_unstable();
}

pub(crate) fn sort_chunk(chunk: &mut [i32], direction: i32) {
    match direction {
        0 => chunk.sort_unstable(),
        1 => chunk.sort_unstable_by_key(|&e| std::cmp::Reverse(e)),
        _ => unreachable!(),
    }
}

pub(crate) fn sort_saws(values: &mut [i32], saw_count: usize, direction: i32) {
    let chunk_size = values.len() / saw_count.max(1);
    for chunk in values.chunks_mut(chunk_size) {
        sort_chunk(chunk, direction);
    }
}

pub(crate) fn sort_mixed_saws(values: &mut [i32], chunk_size: usize, directions: &[i32]) {
    let chunks = directions.iter().map(|&direction| (chunk_size, direction));
    sort_mixed_range_saws(values, chunks);
}

pub(crate) fn sort_mixed_range_saws(
    values: &mut [i32],
    chunks: impl IntoIterator<Item = (usize, i32)>,
) {
    let mut chunks = chunks.into_iter();
    let len = values.len();
    let mut l = 0;
    while l < len {
        let (chunk_size, direction) = chunks.next().expect("not enough chunks");
        assert!(chunk_size != 0, "chunk size must be non-zero");

        let chunk_end = l.saturating_add(chunk_size).min(len);
        sort_chunk(&mut values[l..chunk_end], direction);

        l = chunk_end;
    }
}

pub(crate) fn sort_pipe_organ(values: &mut [i32]) {
    let half = values.len() / 2;
    values[..half].sort_unstable();
    values[half..].sort_unstable_by_key(|&e| std::cmp::Reverse(e));
}

pub(crate) fn random_sorted(
    len: usize,
    sorted_percent: f64,
    rng: &mut (impl Rng + ?Sized),
) -> Vec<i32> {
    let mut values = random(len, rng);
    sort_prefix(&mut values, sorted_percent);
    values
}

pub(crate) fn random_merge(
    len: usize,
    first_run_percent: f64,
    rng: &mut (impl Rng + ?Sized),
) -> Vec<i32> {
    let mut values = random(len, rng);
    sort_runs(&mut values, first_run_percent);
    values
}

pub(crate) fn saw_ascending(
    len: usize,
    saw_count: usize,
    rng: &mut (impl Rng + ?Sized),
) -> Vec<i32> {
    let mut values = random(len, rng);
    sort_saws(&mut values, saw_count, 0);
    values
}

pub(crate) fn saw_descending(
    len: usize,
    saw_count: usize,
    rng: &mut (impl Rng + ?Sized),
) -> Vec<i32> {
    let mut values = random(len, rng);
    sort_saws(&mut values, saw_count, 1);
    values
}

pub(crate) fn saw_mixed(len: usize, saw_count: usize, rng: &mut (impl Rng + ?Sized)) -> Vec<i32> {
    let mut values = random(len, rng);
    let chunk_size = len / saw_count.max(1);
    let directions = random_uniform((len / chunk_size) + 1, 0..=1, rng);
    sort_mixed_saws(&mut values, chunk_size, &directions);
    values
}

pub(crate) fn saw_mixed_range(
    len: usize,
    range: std::ops::Range<usize>,
    rng: &mut (impl Rng + ?Sized),
) -> Vec<i32> {
    let mut values = random(len, rng);
    let directions = rand::distributions::Uniform::from(0..=1);
    let chunk_sizes = rand::distributions::Uniform::from((range.start as i32)..(range.end as i32));

    let chunks = std::iter::from_fn(|| {
        let direction = directions.sample(rng);
        let chunk_size = chunk_sizes.sample(rng) as usize;
        Some((chunk_size, direction))
    });
    sort_mixed_range_saws(&mut values, chunks);
    values
}

pub(crate) fn pipe_organ(len: usize, rng: &mut (impl Rng + ?Sized)) -> Vec<i32> {
    let mut values = random(len, rng);
    sort_pipe_organ(&mut values);
    values
}

// Swaps `swap_count` random pairs, drawing each position as `u32 % len` like upstream.
fn swap_random_pairs(values: &mut [i32], swap_count: usize, rng: &mut (impl Rng + ?Sized)) {
    let len = values.len();
    // An empty slice has no positions to draw, and `% len` would panic.
    if len == 0 {
        return;
    }
    for _ in 0..swap_count {
        let x = rng.gen::<u32>() as usize % len;
        let y = rng.gen::<u32>() as usize % len;
        values.swap(x, y);
    }
}

// Ports of gen_mostly_ascending and gen_mostly_descending from the rust-lang/rust slice sort
// benchmarks (library/alloctests/benches/slice.rs), with the swap count as a parameter.
pub(crate) fn gen_mostly_ascending(
    len: usize,
    swap_count: usize,
    rng: &mut (impl Rng + ?Sized),
) -> Vec<i32> {
    let mut values = ascending(len);
    swap_random_pairs(&mut values, swap_count, rng);
    values
}

pub(crate) fn gen_mostly_descending(
    len: usize,
    swap_count: usize,
    rng: &mut (impl Rng + ?Sized),
) -> Vec<i32> {
    let mut values = descending(len);
    swap_random_pairs(&mut values, swap_count, rng);
    values
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::distributions::Uniform;
    use rand::rngs::StdRng;

    #[test]
    fn mixed_range_saws_sort_each_chunk_in_its_direction() {
        let mut values = vec![3, 1, 2, 6, 4, 5, 9, 7, 8, 10];
        sort_mixed_range_saws(&mut values, [(3, 0), (3, 1), (10, 0)]);
        assert_eq!(values, [1, 2, 3, 6, 5, 4, 7, 8, 9, 10]);
    }

    #[test]
    fn mixed_range_saws_sample_each_chunk_after_values() {
        for seed in [0, 42, u64::MAX] {
            for range in [1..2, 3..7, 20..50, 1..100] {
                for len in [0_usize, 1, 2, 19, 20, 21, 257, 2_048] {
                    let mut rng = StdRng::seed_from_u64(seed);
                    let mut reference = rng.clone();
                    let mut expected: Vec<i32> = (0..len).map(|_| reference.gen()).collect();
                    let directions = Uniform::from(0..=1);
                    let chunk_sizes = Uniform::from((range.start as i32)..(range.end as i32));
                    let mut l = 0;
                    while l < len {
                        let direction = directions.sample(&mut reference);
                        let chunk_size = chunk_sizes.sample(&mut reference) as usize;
                        assert!(range.contains(&chunk_size));
                        let chunk_end = (l + chunk_size).min(len);
                        expected[l..chunk_end].sort_unstable();
                        if direction == 1 {
                            expected[l..chunk_end].reverse();
                        }
                        l = chunk_end;
                    }
                    assert_eq!(
                        saw_mixed_range(len, range.clone(), &mut rng),
                        expected,
                        "{len}/{range:?}/{seed}"
                    );
                    assert_eq!(
                        rng.next_u64(),
                        reference.next_u64(),
                        "{len}/{range:?}/{seed} state"
                    );
                }
            }
        }
    }
}
