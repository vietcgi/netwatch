use crate::{
    device::{NetworkReader, NetworkStats},
    error::{NetwatchError, Result},
};
use std::fs;
use std::time::SystemTime;

pub struct LinuxReader;

impl Default for LinuxReader {
    fn default() -> Self {
        Self::new()
    }
}

impl LinuxReader {
    pub fn new() -> Self {
        Self
    }

    fn parse_proc_net_dev(&self, content: &str, device: &str) -> Result<NetworkStats> {
        for line in content.lines().skip(2) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            let iface_name = parts[0].trim_end_matches(':');
            if iface_name == device {
                return Ok(NetworkStats {
                    timestamp: SystemTime::now(),
                    bytes_in: parts.get(1).unwrap_or(&"0").parse().unwrap_or(0),
                    packets_in: parts.get(2).unwrap_or(&"0").parse().unwrap_or(0),
                    errors_in: parts.get(3).unwrap_or(&"0").parse().unwrap_or(0),
                    drops_in: parts.get(4).unwrap_or(&"0").parse().unwrap_or(0),
                    bytes_out: parts.get(9).unwrap_or(&"0").parse().unwrap_or(0),
                    packets_out: parts.get(10).unwrap_or(&"0").parse().unwrap_or(0),
                    errors_out: parts.get(11).unwrap_or(&"0").parse().unwrap_or(0),
                    drops_out: parts.get(12).unwrap_or(&"0").parse().unwrap_or(0),
                });
            }
        }

        Err(NetwatchError::DeviceNotFound(device.to_string()))
    }
}

impl NetworkReader for LinuxReader {
    fn list_devices(&self) -> Result<Vec<String>> {
        let content = fs::read_to_string("/proc/net/dev")?;
        let mut devices = Vec::new();

        for line in content.lines().skip(2) {
            if let Some(device_part) = line.split(':').next() {
                let device_name = device_part.trim().to_string();
                if !device_name.is_empty() {
                    devices.push(device_name);
                }
            }
        }

        // Filter out loopback and virtual interfaces by default
        devices.retain(|name| {
            !name.starts_with("lo")
                && !name.starts_with("docker")
                && !name.starts_with("veth")
                && !name.starts_with("br-")
        });

        Ok(devices)
    }

    fn read_stats(&self, device: &str) -> Result<NetworkStats> {
        let content = fs::read_to_string("/proc/net/dev")?;
        self.parse_proc_net_dev(&content, device)
    }

    fn is_available(&self) -> bool {
        std::path::Path::new("/proc/net/dev").exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_proc_net_dev() {
        let reader = LinuxReader::new();
        let sample_data = r#"Inter-|   Receive                                                |  Transmit
 face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed
    lo: 1234567      100    0    0    0     0          0         0  1234567      100    0    0    0     0       0          0
  eth0: 9876543210   5000    0    0    0     0          0         0  1234567890   3000    0    0    0     0       0          0
"#;

        let stats = reader.parse_proc_net_dev(sample_data, "eth0").unwrap();
        assert_eq!(stats.bytes_in, 9876543210);
        assert_eq!(stats.bytes_out, 1234567890);
        assert_eq!(stats.packets_in, 5000);
        assert_eq!(stats.packets_out, 3000);
    }

    #[test]
    fn test_device_not_found() {
        let reader = LinuxReader::new();
        let sample_data = r#"Inter-|   Receive                                                |  Transmit
 face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed
    lo: 1234567      100    0    0    0     0          0         0  1234567      100    0    0    0     0       0          0
"#;

        let result = reader.parse_proc_net_dev(sample_data, "nonexistent");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            NetwatchError::DeviceNotFound(_)
        ));
    }
}
