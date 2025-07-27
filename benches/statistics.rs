use criterion::{criterion_group, criterion_main, Criterion};
use netwatch::{device::NetworkStats, stats::StatsCalculator};
use std::hint::black_box;
use std::time::{Duration, SystemTime};

fn create_sample_stats(bytes_in: u64, bytes_out: u64) -> NetworkStats {
    NetworkStats {
        timestamp: SystemTime::now(),
        bytes_in,
        bytes_out,
        packets_in: bytes_in / 1000,
        packets_out: bytes_out / 1000,
        errors_in: 0,
        errors_out: 0,
        drops_in: 0,
        drops_out: 0,
    }
}

fn benchmark_stats_calculation(c: &mut Criterion) {
    c.bench_function("stats_single_sample", |b| {
        let mut calculator = StatsCalculator::new(Duration::from_secs(300));
        b.iter(|| {
            let stats = create_sample_stats(black_box(1000000), black_box(500000));
            calculator.add_sample(black_box(stats));
        });
    });
}

fn benchmark_stats_batch(c: &mut Criterion) {
    c.bench_function("stats_batch_100_samples", |b| {
        b.iter(|| {
            let mut calculator = StatsCalculator::new(Duration::from_secs(300));
            for i in 0..100 {
                let stats =
                    create_sample_stats(black_box(1000000 + i * 1000), black_box(500000 + i * 500));
                calculator.add_sample(stats);
            }
        });
    });
}

fn benchmark_stats_window_trimming(c: &mut Criterion) {
    c.bench_function("stats_window_trimming", |b| {
        let mut calculator = StatsCalculator::new(Duration::from_secs(60));

        // Pre-populate with old data
        for i in 0..1000 {
            let stats = NetworkStats {
                timestamp: SystemTime::now() - Duration::from_secs(3600 + i), // Old data
                bytes_in: 1000000 + i,
                bytes_out: 500000 + i,
                packets_in: 1000 + i,
                packets_out: 500 + i,
                errors_in: 0,
                errors_out: 0,
                drops_in: 0,
                drops_out: 0,
            };
            calculator.add_sample(stats);
        }

        b.iter(|| {
            let stats = create_sample_stats(black_box(2000000), black_box(1000000));
            calculator.add_sample(black_box(stats));
        });
    });
}

fn benchmark_counter_overflow_handling(c: &mut Criterion) {
    c.bench_function("counter_overflow_calculation", |b| {
        let mut calculator = StatsCalculator::new(Duration::from_secs(300));

        // Add initial sample
        let initial_stats = create_sample_stats(u32::MAX as u64 - 1000, u32::MAX as u64 - 500);
        calculator.add_sample(initial_stats);

        b.iter(|| {
            // Simulate counter overflow
            let overflow_stats = create_sample_stats(
                black_box(1000), // Wrapped around
                black_box(500),  // Wrapped around
            );
            calculator.add_sample(black_box(overflow_stats));
        });
    });
}

fn benchmark_graph_data_management(c: &mut Criterion) {
    c.bench_function("graph_data_updates", |b| {
        let mut calculator = StatsCalculator::new(Duration::from_secs(300));

        b.iter(|| {
            for i in 0..50 {
                let stats =
                    create_sample_stats(black_box(1000000 + i * 1024), black_box(500000 + i * 512));
                calculator.add_sample(stats);

                // Access graph data (simulating UI updates)
                let _graph_in = calculator.graph_data_in();
                let _graph_out = calculator.graph_data_out();
            }
        });
    });
}

criterion_group!(
    benches,
    benchmark_stats_calculation,
    benchmark_stats_batch,
    benchmark_stats_window_trimming,
    benchmark_counter_overflow_handling,
    benchmark_graph_data_management
);
criterion_main!(benches);
