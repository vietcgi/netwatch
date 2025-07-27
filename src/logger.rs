use crate::stats::StatsCalculator;
use crate::validation;
use chrono::Local;
use std::fs::OpenOptions;
use std::io::Write;

pub struct TrafficLogger {
    file: Option<std::fs::File>,
    use_stdout: bool,
}

impl TrafficLogger {
    pub fn new(path: Option<String>) -> anyhow::Result<Self> {
        let (file, use_stdout) = if let Some(path) = path {
            if path == "-" {
                (None, true) // stdout logging
            } else {
                // Validate log file path for security
                validation::validate_file_path(&path, Some("log"))?;
                let f = OpenOptions::new().create(true).append(true).open(path)?;
                (Some(f), false)
            }
        } else {
            (None, false)
        };

        let mut logger = Self { file, use_stdout };

        // Write header if file is new or empty
        if let Some(ref mut f) = logger.file {
            // Check if file is empty (new)
            let metadata = f.metadata()?;
            if metadata.len() == 0 {
                logger.write_header()?;
            }
        } else if logger.use_stdout {
            logger.write_header()?;
        }

        Ok(logger)
    }

    fn write_header(&mut self) -> anyhow::Result<()> {
        let header = "Date Time DeviceName DataInTotal DataOutTotal DataInPerSecond DataOutPerSecond DataInAverage DataOutAverage DataInMin DataOutMin DataInMax DataOutMax TimeSeconds TimeMicroSeconds\n";

        match (&mut self.file, self.use_stdout) {
            (Some(f), _) => f.write_all(header.as_bytes())?,
            (None, true) => print!("{header}"),
            _ => {} // No output
        }

        Ok(())
    }

    pub fn log_traffic(&mut self, device: &str, stats: &StatsCalculator) -> anyhow::Result<()> {
        // Validate device name for security
        validation::validate_interface_name(device)?;

        let now = Local::now();
        let timestamp = now.timestamp();
        let microseconds = now.timestamp_subsec_micros();

        let (current_in, current_out) = stats.current_speed();
        let (avg_in, avg_out) = stats.average_speed();
        let (min_in, min_out) = stats.min_speed();
        let (max_in, max_out) = stats.max_speed();
        let (total_in, total_out) = stats.total_bytes();

        let log_line = format!(
            "{} {} {} {} {} {} {} {} {} {} {} {} {} {} {}\n",
            now.format("%Y-%m-%d"),
            now.format("%H:%M:%S"),
            device,
            total_in,
            total_out,
            current_in,
            current_out,
            avg_in,
            avg_out,
            min_in,
            min_out,
            max_in,
            max_out,
            timestamp,
            microseconds
        );

        match (&mut self.file, self.use_stdout) {
            (Some(f), _) => {
                f.write_all(log_line.as_bytes())?;
                f.flush()?;
            }
            (None, true) => print!("{log_line}"),
            _ => {} // No output
        }

        Ok(())
    }
}
