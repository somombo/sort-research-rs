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
    for (i, chunk) in values.chunks_mut(chunk_size).enumerate() {
        sort_chunk(chunk, directions[i]);
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

pub(crate) fn pipe_organ(len: usize, rng: &mut (impl Rng + ?Sized)) -> Vec<i32> {
    let mut values = random(len, rng);
    sort_pipe_organ(&mut values);
    values
}
