use crate::device::NetworkStats;
use std::collections::VecDeque;
use std::time::Duration;
#[cfg(test)]
use std::time::SystemTime;

pub struct StatsCalculator {
    // Data storage
    history: VecDeque<NetworkStats>,
    window_size: Duration,

    // Calculated values
    current_speed_in: u64,
    current_speed_out: u64,
    avg_speed_in: u64,
    avg_speed_out: u64,
    min_speed_in: u64,
    min_speed_out: u64,
    max_speed_in: u64,
    max_speed_out: u64,

    // Graph data for display
    graph_data_in: VecDeque<(f64, f64)>, // (time, value) pairs
    graph_data_out: VecDeque<(f64, f64)>,

    // Totals (from last sample)
    total_bytes_in: u64,
    total_bytes_out: u64,
    total_packets_in: u64,
    total_packets_out: u64,

    // First sample flag for initialization
    first_sample: bool,
}

impl StatsCalculator {
    pub fn new(window_size: Duration) -> Self {
        Self {
            history: VecDeque::new(),
            window_size,
            current_speed_in: 0,
            current_speed_out: 0,
            avg_speed_in: 0,
            avg_speed_out: 0,
            min_speed_in: 0,
            min_speed_out: 0,
            max_speed_in: 0,
            max_speed_out: 0,
            graph_data_in: VecDeque::new(),
            graph_data_out: VecDeque::new(),
            total_bytes_in: 0,
            total_bytes_out: 0,
            total_packets_in: 0,
            total_packets_out: 0,
            first_sample: true,
        }
    }

    pub fn add_sample(&mut self, stats: NetworkStats) {
        // Update totals
        self.total_bytes_in = stats.bytes_in;
        self.total_bytes_out = stats.bytes_out;
        self.total_packets_in = stats.packets_in;
        self.total_packets_out = stats.packets_out;

        // Calculate current speed if we have previous data
        if let Some(previous) = self.history.back() {
            let time_diff = stats
                .timestamp
                .duration_since(previous.timestamp)
                .unwrap_or_default()
                .as_secs_f64();

            if time_diff > 0.0 {
                // Handle counter overflow (32-bit counters can wrap)
                let bytes_in_diff = self.calculate_diff(stats.bytes_in, previous.bytes_in);
                let bytes_out_diff = self.calculate_diff(stats.bytes_out, previous.bytes_out);

                self.current_speed_in = (bytes_in_diff as f64 / time_diff) as u64;
                self.current_speed_out = (bytes_out_diff as f64 / time_diff) as u64;

                // Update min/max (skip first few samples for stability)
                if !self.first_sample {
                    self.update_min_max();
                }

                // Add to graph data
                self.add_graph_data(&stats);
            }
        }

        self.history.push_back(stats);
        self.trim_old_samples();
        self.calculate_averages();

        if self.first_sample {
            self.first_sample = false;
        }
    }

    fn calculate_diff(&self, current: u64, previous: u64) -> u64 {
        if current >= previous {
            current - previous
        } else {
            // Counter wrapped, assume 32-bit or 64-bit counter
            // Try 32-bit first, then 64-bit
            let diff_32 = (u32::MAX as u64) - previous + current + 1;
            let diff_64 = (u64::MAX) - previous + current + 1;

            // Choose the smaller, more reasonable difference
            if diff_32 < diff_64 / 1000 {
                diff_32
            } else {
                diff_64
            }
        }
    }

    fn update_min_max(&mut self) {
        if self.current_speed_in < self.min_speed_in || self.min_speed_in == 0 {
            self.min_speed_in = self.current_speed_in;
        }
        if self.current_speed_in > self.max_speed_in {
            self.max_speed_in = self.current_speed_in;
        }
        if self.current_speed_out < self.min_speed_out || self.min_speed_out == 0 {
            self.min_speed_out = self.current_speed_out;
        }
        if self.current_speed_out > self.max_speed_out {
            self.max_speed_out = self.current_speed_out;
        }
    }

    fn add_graph_data(&mut self, _stats: &NetworkStats) {
        // First, shift all existing points forward in time (age them)
        for (time, _) in self.graph_data_in.iter_mut() {
            *time += 0.5; // Assuming ~500ms refresh rate
        }
        for (time, _) in self.graph_data_out.iter_mut() {
            *time += 0.5; // Assuming ~500ms refresh rate
        }

        // Remove data older than 60 seconds
        self.graph_data_in.retain(|(time, _)| *time <= 60.0);
        self.graph_data_out.retain(|(time, _)| *time <= 60.0);

        // Now add new data point at time 0 (now)
        self.graph_data_in
            .push_back((0.0, self.current_speed_in as f64));
        self.graph_data_out
            .push_back((0.0, self.current_speed_out as f64));

        // Limit to reasonable number of points
        while self.graph_data_in.len() > 120 {
            self.graph_data_in.pop_front();
        }
        while self.graph_data_out.len() > 120 {
            self.graph_data_out.pop_front();
        }
    }

    fn trim_old_samples(&mut self) {
        if let Some(latest) = self.history.back() {
            let cutoff = latest.timestamp - self.window_size;
            while let Some(oldest) = self.history.front() {
                if oldest.timestamp < cutoff {
                    self.history.pop_front();
                } else {
                    break;
                }
            }
        }
    }

    fn calculate_averages(&mut self) {
        if self.history.len() < 2 {
            return;
        }

        let first = &self.history[0];
        let last = &self.history[self.history.len() - 1];

        let time_span = last
            .timestamp
            .duration_since(first.timestamp)
            .unwrap_or_default()
            .as_secs_f64();

        if time_span > 0.0 {
            let bytes_in_diff = self.calculate_diff(last.bytes_in, first.bytes_in);
            let bytes_out_diff = self.calculate_diff(last.bytes_out, first.bytes_out);

            self.avg_speed_in = (bytes_in_diff as f64 / time_span) as u64;
            self.avg_speed_out = (bytes_out_diff as f64 / time_span) as u64;
        }
    }

    // Public getters for UI
    pub fn current_speed(&self) -> (u64, u64) {
        (self.current_speed_in, self.current_speed_out)
    }

    pub fn average_speed(&self) -> (u64, u64) {
        (self.avg_speed_in, self.avg_speed_out)
    }

    pub fn min_speed(&self) -> (u64, u64) {
        (self.min_speed_in, self.min_speed_out)
    }

    pub fn max_speed(&self) -> (u64, u64) {
        (self.max_speed_in, self.max_speed_out)
    }

    pub fn total_bytes(&self) -> (u64, u64) {
        (self.total_bytes_in, self.total_bytes_out)
    }

    pub fn total_packets(&self) -> (u64, u64) {
        (self.total_packets_in, self.total_packets_out)
    }

    pub fn graph_data_in(&self) -> &VecDeque<(f64, f64)> {
        &self.graph_data_in
    }

    pub fn graph_data_out(&self) -> &VecDeque<(f64, f64)> {
        &self.graph_data_out
    }

    pub fn sample_count(&self) -> usize {
        self.history.len()
    }

    pub fn reset(&mut self) {
        self.history.clear();
        self.graph_data_in.clear();
        self.graph_data_out.clear();
        self.current_speed_in = 0;
        self.current_speed_out = 0;
        self.avg_speed_in = 0;
        self.avg_speed_out = 0;
        self.min_speed_in = 0;
        self.min_speed_out = 0;
        self.max_speed_in = 0;
        self.max_speed_out = 0;
        self.first_sample = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stats_calculation() {
        let mut calc = StatsCalculator::new(Duration::from_secs(60));

        let stats1 = NetworkStats {
            timestamp: SystemTime::now(),
            bytes_in: 1000,
            bytes_out: 500,
            packets_in: 10,
            packets_out: 5,
            errors_in: 0,
            errors_out: 0,
            drops_in: 0,
            drops_out: 0,
        };

        calc.add_sample(stats1);

        // First sample should not calculate speed
        assert_eq!(calc.current_speed(), (0, 0));

        // Add second sample after 1 second
        let stats2 = NetworkStats {
            timestamp: SystemTime::now() + Duration::from_secs(1),
            bytes_in: 2000,
            bytes_out: 1000,
            packets_in: 20,
            packets_out: 10,
            errors_in: 0,
            errors_out: 0,
            drops_in: 0,
            drops_out: 0,
        };

        calc.add_sample(stats2);

        // Should calculate 1000 bytes/sec for both directions
        let (in_speed, out_speed) = calc.current_speed();
        assert!(in_speed > 0);
        assert!(out_speed > 0);
    }

    #[test]
    fn test_counter_overflow() {
        let calc = StatsCalculator::new(Duration::from_secs(60));

        // Test 32-bit counter overflow
        let diff = calc.calculate_diff(100, u32::MAX as u64 - 50);
        assert_eq!(diff, 151); // (u32::MAX - (u32::MAX - 50)) + 100 + 1
    }
}
