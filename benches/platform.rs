use criterion::{criterion_group, criterion_main, Criterion};
use netwatch_rs::platform;
use std::hint::black_box;

fn benchmark_interface_listing(c: &mut Criterion) {
    c.bench_function("list_network_interfaces", |b| {
        let reader = platform::create_reader().expect("Failed to create platform reader");

        b.iter(|| {
            let interfaces = reader.list_devices().expect("Failed to list devices");
            black_box(interfaces);
        });
    });
}

fn benchmark_stats_reading(c: &mut Criterion) {
    let reader = platform::create_reader().expect("Failed to create platform reader");
    let interfaces = reader.list_devices().expect("Failed to list devices");

    if let Some(interface) = interfaces.first() {
        let interface_name = interface.clone();

        c.bench_function("read_interface_stats", |b| {
            b.iter(|| {
                match reader.read_stats(&interface_name) {
                    Ok(stats) => {
                        black_box(stats);
                    }
                    Err(_) => {
                        // Interface might not be available for reading, that's ok for benchmarking
                    }
                }
            });
        });
    }
}

fn benchmark_multiple_interface_reading(c: &mut Criterion) {
    let reader = platform::create_reader().expect("Failed to create platform reader");
    let interfaces = reader.list_devices().expect("Failed to list devices");

    if !interfaces.is_empty() {
        c.bench_function("read_multiple_interfaces", |b| {
            b.iter(|| {
                for interface in &interfaces {
                    match reader.read_stats(interface) {
                        Ok(stats) => {
                            black_box(stats);
                        }
                        Err(_) => {
                            // Some interfaces might not be readable
                        }
                    }
                }
            });
        });
    }
}

fn benchmark_platform_availability(c: &mut Criterion) {
    c.bench_function("platform_availability_check", |b| {
        let reader = platform::create_reader().expect("Failed to create platform reader");

        b.iter(|| {
            let available = reader.is_available();
            black_box(available);
        });
    });
}

criterion_group!(
    benches,
    benchmark_interface_listing,
    benchmark_stats_reading,
    benchmark_multiple_interface_reading,
    benchmark_platform_availability
);
criterion_main!(benches);
