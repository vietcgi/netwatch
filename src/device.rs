use crate::error::Result;
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct NetworkStats {
    pub timestamp: SystemTime,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub packets_in: u64,
    pub packets_out: u64,
    pub errors_in: u64,
    pub errors_out: u64,
    pub drops_in: u64,
    pub drops_out: u64,
}

impl Default for NetworkStats {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkStats {
    pub fn new() -> Self {
        Self {
            timestamp: SystemTime::now(),
            bytes_in: 0,
            bytes_out: 0,
            packets_in: 0,
            packets_out: 0,
            errors_in: 0,
            errors_out: 0,
            drops_in: 0,
            drops_out: 0,
        }
    }
}

pub trait NetworkReader: Send + Sync {
    fn list_devices(&self) -> Result<Vec<String>>;
    fn read_stats(&self, device: &str) -> Result<NetworkStats>;
    fn is_available(&self) -> bool;
}

#[derive(Debug, Clone)]
pub struct Device {
    pub name: String,
    pub stats: NetworkStats,
    pub is_active: bool,
}

impl Device {
    pub fn new(name: String) -> Self {
        Self {
            name,
            stats: NetworkStats::new(),
            is_active: false,
        }
    }

    pub fn update(&mut self, reader: &dyn NetworkReader) -> Result<()> {
        match reader.read_stats(&self.name) {
            Ok(stats) => {
                self.stats = stats;
                self.is_active = true;
                Ok(())
            }
            Err(e) => {
                self.is_active = false;
                Err(e)
            }
        }
    }
}
