use netwatch::{
    device::{NetworkReader, NetworkStats},
    stats::StatsCalculator,
};
use std::time::{Duration, SystemTime};

// Mock network reader for demonstration
struct MockReader;

impl NetworkReader for MockReader {
    fn list_devices(&self) -> netwatch::error::Result<Vec<String>> {
        Ok(vec!["eth0".to_string(), "wlan0".to_string()])
    }

    fn read_stats(&self, device: &str) -> netwatch::error::Result<NetworkStats> {
        // Generate some mock data
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        device.hash(&mut hasher);
        let seed = hasher.finish();

        Ok(NetworkStats {
            timestamp: SystemTime::now(),
            bytes_in: 1000000 + (seed % 100000),
            bytes_out: 500000 + (seed % 50000),
            packets_in: 1000 + (seed % 100),
            packets_out: 800 + (seed % 80),
            errors_in: 0,
            errors_out: 0,
            drops_in: 0,
            drops_out: 0,
        })
    }

    fn is_available(&self) -> bool {
        true
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("netwatch Demo");
    println!("=============");

    let reader = MockReader;
    let mut calculator = StatsCalculator::new(Duration::from_secs(60));

    // List available devices
    println!("Available devices:");
    for device in reader.list_devices()? {
        println!("  - {device}");
    }

    println!("\nSimulating traffic data collection...");

    // Simulate data collection for eth0
    let device = "eth0";
    for i in 0..5 {
        let stats = reader.read_stats(device)?;
        calculator.add_sample(stats);

        let (current_in, current_out) = calculator.current_speed();
        let (avg_in, avg_out) = calculator.average_speed();

        println!(
            "Sample {}: Current: {}/s in, {}/s out | Average: {}/s in, {}/s out",
            i + 1,
            format_bytes(current_in),
            format_bytes(current_out),
            format_bytes(avg_in),
            format_bytes(avg_out)
        );

        std::thread::sleep(Duration::from_millis(500));
    }

    Ok(())
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.1} {}", size, UNITS[unit_index])
}
