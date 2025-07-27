use crate::cli::{Args, DataUnit, TrafficUnit};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

fn default_diagnostic_targets() -> Vec<String> {
    vec![
        "1.1.1.1".to_string(), // Cloudflare DNS (public, reliable)
        "8.8.8.8".to_string(), // Google DNS (public, reliable)
    ]
}

fn default_dns_domains() -> Vec<String> {
    vec![
        "cloudflare.com".to_string(), // Reliable test domain
        "google.com".to_string(),     // Reliable test domain
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(rename = "AverageWindow")]
    pub average_window: u32,

    #[serde(rename = "BarMaxIn")]
    pub max_incoming: u64,

    #[serde(rename = "BarMaxOut")]
    pub max_outgoing: u64,

    #[serde(rename = "DataFormat")]
    pub data_format: String,

    #[serde(rename = "Devices")]
    pub devices: String,

    #[serde(rename = "MultipleDevices")]
    pub multiple_devices: bool,

    #[serde(rename = "RefreshInterval")]
    pub refresh_interval: u64,

    #[serde(rename = "TrafficFormat")]
    pub traffic_format: String,

    #[serde(rename = "DiagnosticTargets", default = "default_diagnostic_targets")]
    pub diagnostic_targets: Vec<String>,

    #[serde(rename = "DNSDomains", default = "default_dns_domains")]
    pub dns_domains: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            average_window: 300,
            max_incoming: 0,
            max_outgoing: 0,
            data_format: "M".to_string(),
            devices: "all".to_string(),
            multiple_devices: false,
            refresh_interval: 500,
            traffic_format: "k".to_string(),
            diagnostic_targets: default_diagnostic_targets(),
            dns_domains: default_dns_domains(),
        }
    }
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        // Try to load from ~/.netwatch (modern) or ~/.nload (compatibility)
        if let Some(home) = dirs::home_dir() {
            let modern_config = home.join(".netwatch");
            let legacy_config = home.join(".nload");

            if modern_config.exists() {
                let content = std::fs::read_to_string(modern_config)?;
                return Ok(toml::from_str(&content)?);
            } else if legacy_config.exists() {
                // Parse nload format: Key="Value"
                return Self::parse_nload_format(&legacy_config);
            }
        }

        Ok(Self::default())
    }

    pub fn save(&self) -> anyhow::Result<()> {
        if let Some(home) = dirs::home_dir() {
            let config_path = home.join(".netwatch");
            let content = toml::to_string_pretty(self)?;
            std::fs::write(config_path, content)?;
        }
        Ok(())
    }

    pub fn apply_args(&mut self, args: &Args) {
        self.average_window = args.average_window;
        self.max_incoming = args.max_incoming;
        self.max_outgoing = args.max_outgoing;
        self.refresh_interval = args.refresh_interval;
        self.traffic_format = args.traffic_unit.to_string().to_string();
        self.data_format = args.data_unit.to_string().to_string();
        self.multiple_devices = args.multiple_devices;
    }

    #[must_use]
    pub fn get_traffic_unit(&self) -> TrafficUnit {
        TrafficUnit::from_string(&self.traffic_format).unwrap_or(TrafficUnit::KiloBit)
    }

    #[must_use]
    pub fn get_data_unit(&self) -> DataUnit {
        DataUnit::from_string(&self.data_format).unwrap_or(DataUnit::MegaByte)
    }

    fn parse_nload_format(path: &PathBuf) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let mut config = Self::default();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches('"');

                match key {
                    "AverageWindow" => config.average_window = value.parse().unwrap_or(300),
                    "BarMaxIn" => config.max_incoming = value.parse().unwrap_or(0),
                    "BarMaxOut" => config.max_outgoing = value.parse().unwrap_or(0),
                    "DataFormat" => config.data_format = value.to_string(),
                    "Devices" => config.devices = value.to_string(),
                    "MultipleDevices" => config.multiple_devices = value.parse().unwrap_or(false),
                    "RefreshInterval" => config.refresh_interval = value.parse().unwrap_or(500),
                    "TrafficFormat" => config.traffic_format = value.to_string(),
                    _ => {} // Ignore unknown keys
                }
            }
        }

        Ok(config)
    }
}
