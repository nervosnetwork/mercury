use xsql::{generate_id, SNOWFLAKE};

use arc_swap::ArcSwap;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dashmap::DashMap;

lazy_static::lazy_static! {
    static ref ARC_SNOWFLAKE: ArcSwap<AnotherSnowflake> = ArcSwap::from_pointee(AnotherSnowflake::default());
}

#[derive(Default)]
struct AnotherSnowflake {
    center_id: i64,
    machine_id: i64,
    sequence: DashMap<i64, i32>,
}

impl AnotherSnowflake {
    pub fn generate(&self, number: i64) -> i64 {
        let seq = if let Some(mut last) = self.sequence.get_mut(&number) {
            let now = (*last + 1) & (-1 ^ (-1 << 12));
            *last = now;
            now as i64
        } else {
            self.sequence.insert(number, 1);
            1i64
        };

        (number << 32) | (self.center_id << 26) | (self.machine_id << 16) | seq
    }
}

fn generate_id_another(number: u64) -> i64 {
    let number = number as i64;
    ARC_SNOWFLAKE.load().generate(number)
}

pub fn benchmark_with_atomic_i64(c: &mut Criterion) {
    let block_number = 0u64;
    c.bench_function("atomic_i64", |b| {
        b.iter(|| generate_id(black_box(block_number)))
    });
}

pub fn benchmark_with_arc_swap(c: &mut Criterion) {
    let block_number = 0u64;
    c.bench_function("arc_swap", |b| {
        b.iter(|| generate_id_another(black_box(block_number)))
    });
}

pub fn benchmark_exist_large_data(c: &mut Criterion) {
    let block_number = 100_001u64;
    (0..100_000).for_each(|i| {
        let _ = generate_id(i);
    });

    c.bench_function("large_data", |b| {
        b.iter(|| generate_id(black_box(block_number)))
    });
}

pub fn benchmark_without_large_data(c: &mut Criterion) {
    let block_number = 100_001u64;

    c.bench_function("empty_data", |b| {
        b.iter(|| generate_id(black_box(block_number)))
    });
}

pub fn bench_prepared(c: &mut Criterion) {
    let block_number = 5000u64;
    SNOWFLAKE.prepare(0, 10000);

    c.bench_function("prepared", |b| {
        b.iter(|| generate_id(black_box(block_number)))
    });
}

pub fn bench_unprepared(c: &mut Criterion) {
    let block_number = 5000u64;

    c.bench_function("unprepared", |b| {
        b.iter(|| generate_id(black_box(block_number)))
    });
}

criterion_group!(
    bench_atomic,
    benchmark_with_atomic_i64,
    benchmark_with_arc_swap
);

criterion_group!(
    bench_exist_data,
    benchmark_exist_large_data,
    benchmark_without_large_data
);

criterion_group!(bench_prepare_data, bench_prepared, bench_unprepared);

criterion_main!(bench_atomic, bench_exist_data, bench_prepare_data);
