use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub hostname: String,
    pub os_name: String,
    pub os_version: String,
    pub kernel_version: String,
    pub architecture: String,
    pub cpu_model: String,
    pub cpu_cores: u32,
    pub cpu_threads: u32,
    pub total_memory: u64, // bytes
    pub boot_time: SystemTime,
    pub uptime: Duration,
}

#[derive(Debug, Clone)]
pub struct SystemStats {
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub memory_used: u64,              // bytes
    pub memory_available: u64,         // bytes
    pub load_average: (f64, f64, f64), // 1min, 5min, 15min
    pub disk_usage: HashMap<String, DiskUsage>,
    pub top_processes: Vec<ProcessInfo>,
    pub timestamp: SystemTime,
}

#[derive(Debug, Clone)]
pub struct DiskUsage {
    pub total: u64,     // bytes
    pub used: u64,      // bytes
    pub available: u64, // bytes
    pub usage_percent: f64,
    pub filesystem: String,
}

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f64,
    pub memory_percent: f64,
    pub memory_rss: u64, // bytes
    pub memory_vms: u64, // bytes
    pub command: String,
    pub user: String,
    pub state: String,
}

pub struct SystemMonitor {
    system_info: SystemInfo,
    last_cpu_stats: Option<CpuStats>,
    last_update: SystemTime,
}

#[derive(Debug, Clone)]
struct CpuStats {
    user: u64,
    nice: u64,
    system: u64,
    idle: u64,
    iowait: u64,
    irq: u64,
    softirq: u64,
    steal: u64,
}

impl SystemMonitor {
    pub fn new() -> Result<Self> {
        let system_info = Self::collect_system_info()?;

        Ok(Self {
            system_info,
            last_cpu_stats: None,
            last_update: SystemTime::now(),
        })
    }

    pub fn get_system_info(&self) -> &SystemInfo {
        &self.system_info
    }

    pub fn get_current_stats(&mut self) -> Result<SystemStats> {
        let now = SystemTime::now();

        // Update CPU usage - with fallback
        let cpu_usage = self.calculate_cpu_usage().unwrap_or(0.0);

        // Get memory stats - with fallback
        let (memory_usage_percent, memory_used, memory_available) =
            self.get_memory_stats().unwrap_or((0.0, 0, 0));

        // Get load average - with fallback
        let load_average = self.get_load_average().unwrap_or((0.0, 0.0, 0.0));

        // Get disk usage - with fallback
        let disk_usage = self
            .get_disk_usage()
            .unwrap_or_else(|_| std::collections::HashMap::new());

        // Get top processes - with fallback
        let top_processes = self.get_top_processes().unwrap_or_else(|_| Vec::new());

        self.last_update = now;

        Ok(SystemStats {
            cpu_usage_percent: cpu_usage,
            memory_usage_percent,
            memory_used,
            memory_available,
            load_average,
            disk_usage,
            top_processes,
            timestamp: now,
        })
    }

    fn collect_system_info() -> Result<SystemInfo> {
        let hostname = Self::get_hostname()?;
        let (os_name, os_version) = Self::get_os_info()?;
        let kernel_version = Self::get_kernel_version()?;
        let architecture = Self::get_architecture()?;
        let (cpu_model, cpu_cores, cpu_threads) = Self::get_cpu_info()?;
        let total_memory = Self::get_total_memory()?;
        let boot_time = Self::get_boot_time()?;
        let uptime = SystemTime::now()
            .duration_since(boot_time)
            .unwrap_or_default();

        Ok(SystemInfo {
            hostname,
            os_name,
            os_version,
            kernel_version,
            architecture,
            cpu_model,
            cpu_cores,
            cpu_threads,
            total_memory,
            boot_time,
            uptime,
        })
    }

    fn get_hostname() -> Result<String> {
        let output = Command::new("hostname").output()?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn get_os_info() -> Result<(String, String)> {
        #[cfg(target_os = "macos")]
        {
            let name_output = Command::new("sw_vers").arg("-productName").output()?;
            let version_output = Command::new("sw_vers").arg("-productVersion").output()?;

            let os_name = String::from_utf8_lossy(&name_output.stdout)
                .trim()
                .to_string();
            let os_version = String::from_utf8_lossy(&version_output.stdout)
                .trim()
                .to_string();

            Ok((os_name, os_version))
        }

        #[cfg(target_os = "linux")]
        {
            // Try to read from /etc/os-release
            if let Ok(content) = fs::read_to_string("/etc/os-release") {
                let mut name = "Linux".to_string();
                let mut version = "Unknown".to_string();

                for line in content.lines() {
                    if line.starts_with("PRETTY_NAME=") {
                        name = line
                            .trim_start_matches("PRETTY_NAME=")
                            .trim_matches('"')
                            .to_string();
                    } else if line.starts_with("VERSION=") {
                        version = line
                            .trim_start_matches("VERSION=")
                            .trim_matches('"')
                            .to_string();
                    }
                }

                Ok((name, version))
            } else {
                Ok(("Linux".to_string(), "Unknown".to_string()))
            }
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            Ok(("Unknown OS".to_string(), "Unknown".to_string()))
        }
    }

    fn get_kernel_version() -> Result<String> {
        let output = Command::new("uname").arg("-r").output()?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn get_architecture() -> Result<String> {
        let output = Command::new("uname").arg("-m").output()?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn get_cpu_info() -> Result<(String, u32, u32)> {
        #[cfg(target_os = "macos")]
        {
            let model_output = Command::new("sysctl")
                .arg("-n")
                .arg("machdep.cpu.brand_string")
                .output()?;
            let cores_output = Command::new("sysctl")
                .arg("-n")
                .arg("hw.physicalcpu")
                .output()?;
            let threads_output = Command::new("sysctl")
                .arg("-n")
                .arg("hw.logicalcpu")
                .output()?;

            let model = String::from_utf8_lossy(&model_output.stdout)
                .trim()
                .to_string();
            let cores = String::from_utf8_lossy(&cores_output.stdout)
                .trim()
                .parse::<u32>()
                .unwrap_or(1);
            let threads = String::from_utf8_lossy(&threads_output.stdout)
                .trim()
                .parse::<u32>()
                .unwrap_or(1);

            Ok((model, cores, threads))
        }

        #[cfg(target_os = "linux")]
        {
            let mut model = "Unknown CPU".to_string();
            let mut cores = 1u32;
            let mut threads = 1u32;

            if let Ok(content) = fs::read_to_string("/proc/cpuinfo") {
                let mut cpu_count = 0;
                let mut core_ids = std::collections::HashSet::new();

                for line in content.lines() {
                    if line.starts_with("model name") {
                        if let Some(name) = line.split(':').nth(1) {
                            model = name.trim().to_string();
                        }
                    } else if line.starts_with("processor") {
                        cpu_count += 1;
                    } else if line.starts_with("core id") {
                        if let Some(id) = line.split(':').nth(1) {
                            if let Ok(core_id) = id.trim().parse::<u32>() {
                                core_ids.insert(core_id);
                            }
                        }
                    }
                }

                threads = cpu_count;
                cores = core_ids.len() as u32;
                if cores == 0 {
                    cores = threads;
                } // Fallback
            }

            Ok((model, cores, threads))
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            Ok(("Unknown CPU".to_string(), 1, 1))
        }
    }

    fn get_total_memory() -> Result<u64> {
        #[cfg(target_os = "macos")]
        {
            let output = Command::new("sysctl")
                .arg("-n")
                .arg("hw.memsize")
                .output()?;
            let mem_string = String::from_utf8_lossy(&output.stdout);
            let mem_str = mem_string.trim();
            Ok(mem_str.parse::<u64>().unwrap_or(0))
        }

        #[cfg(target_os = "linux")]
        {
            if let Ok(content) = fs::read_to_string("/proc/meminfo") {
                for line in content.lines() {
                    if line.starts_with("MemTotal:") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 2 {
                            if let Ok(kb) = parts[1].parse::<u64>() {
                                return Ok(kb * 1024); // Convert KB to bytes
                            }
                        }
                    }
                }
            }
            Ok(0)
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            Ok(0)
        }
    }

    fn get_boot_time() -> Result<SystemTime> {
        #[cfg(target_os = "macos")]
        {
            let output = Command::new("sysctl")
                .arg("-n")
                .arg("kern.boottime")
                .output()?;
            let boot_str = String::from_utf8_lossy(&output.stdout);

            // Parse format like "{ sec = 1234567890, usec = 123456 }" - safely
            if let Some(sec_start) = boot_str.find("sec = ") {
                if sec_start + 6 < boot_str.len() {
                    let sec_str = &boot_str[sec_start + 6..];
                    if let Some(sec_end) = sec_str.find(',') {
                        if sec_end <= sec_str.len() {
                            if let Ok(secs) = sec_str[..sec_end].parse::<u64>() {
                                return Ok(UNIX_EPOCH + Duration::from_secs(secs));
                            }
                        }
                    }
                }
            }

            // Fallback
            Ok(SystemTime::now() - Duration::from_secs(3600))
        }

        #[cfg(target_os = "linux")]
        {
            if let Ok(content) = fs::read_to_string("/proc/stat") {
                for line in content.lines() {
                    if line.starts_with("btime ") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 2 {
                            if let Ok(secs) = parts[1].parse::<u64>() {
                                return Ok(UNIX_EPOCH + Duration::from_secs(secs));
                            }
                        }
                    }
                }
            }

            // Fallback
            Ok(SystemTime::now() - Duration::from_secs(3600))
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            Ok(SystemTime::now() - Duration::from_secs(3600))
        }
    }

    fn calculate_cpu_usage(&mut self) -> Result<f64> {
        let current_stats = self.read_cpu_stats()?;

        if let Some(ref last_stats) = self.last_cpu_stats {
            let total_last = last_stats.user
                + last_stats.nice
                + last_stats.system
                + last_stats.idle
                + last_stats.iowait
                + last_stats.irq
                + last_stats.softirq
                + last_stats.steal;

            let total_current = current_stats.user
                + current_stats.nice
                + current_stats.system
                + current_stats.idle
                + current_stats.iowait
                + current_stats.irq
                + current_stats.softirq
                + current_stats.steal;

            let total_diff = total_current - total_last;
            let idle_diff = current_stats.idle - last_stats.idle;

            if total_diff > 0 {
                let usage = ((total_diff - idle_diff) as f64 / total_diff as f64) * 100.0;
                self.last_cpu_stats = Some(current_stats);
                return Ok(usage.clamp(0.0, 100.0));
            }
        }

        self.last_cpu_stats = Some(current_stats);
        Ok(0.0)
    }

    fn read_cpu_stats(&self) -> Result<CpuStats> {
        #[cfg(target_os = "linux")]
        {
            let content = fs::read_to_string("/proc/stat")?;
            if let Some(line) = content.lines().next() {
                if line.starts_with("cpu ") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 8 {
                        return Ok(CpuStats {
                            user: parts[1].parse().unwrap_or(0),
                            nice: parts[2].parse().unwrap_or(0),
                            system: parts[3].parse().unwrap_or(0),
                            idle: parts[4].parse().unwrap_or(0),
                            iowait: parts[5].parse().unwrap_or(0),
                            irq: parts[6].parse().unwrap_or(0),
                            softirq: parts[7].parse().unwrap_or(0),
                            steal: parts.get(8).unwrap_or(&"0").parse().unwrap_or(0),
                        });
                    }
                }
            }
        }

        #[cfg(target_os = "macos")]
        {
            // macOS CPU usage via vm_stat and iostat
            let output = Command::new("iostat").arg("-c").arg("1").arg("1").output();

            if let Ok(output) = output {
                let _content = String::from_utf8_lossy(&output.stdout);
                // Parse iostat output for CPU percentages
                // This is a simplified version
                return Ok(CpuStats {
                    user: 0,
                    nice: 0,
                    system: 0,
                    idle: 1000, // Default to mostly idle
                    iowait: 0,
                    irq: 0,
                    softirq: 0,
                    steal: 0,
                });
            }
        }

        // Fallback
        Ok(CpuStats {
            user: 0,
            nice: 0,
            system: 0,
            idle: 1000,
            iowait: 0,
            irq: 0,
            softirq: 0,
            steal: 0,
        })
    }

    fn get_memory_stats(&self) -> Result<(f64, u64, u64)> {
        #[cfg(target_os = "linux")]
        {
            let content = fs::read_to_string("/proc/meminfo")?;
            let mut total = 0u64;
            let mut available = 0u64;

            for line in content.lines() {
                if line.starts_with("MemTotal:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        total = parts[1].parse::<u64>().unwrap_or(0) * 1024; // KB to bytes
                    }
                } else if line.starts_with("MemAvailable:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        available = parts[1].parse::<u64>().unwrap_or(0) * 1024;
                        // KB to bytes
                    }
                }
            }

            let used = total - available;
            let usage_percent = if total > 0 {
                (used as f64 / total as f64) * 100.0
            } else {
                0.0
            };

            Ok((usage_percent, used, available))
        }

        #[cfg(target_os = "macos")]
        {
            let output = Command::new("vm_stat").output()?;
            let content = String::from_utf8_lossy(&output.stdout);

            let mut pages_free = 0u64;
            let mut pages_active = 0u64;
            let mut pages_inactive = 0u64;
            let mut pages_wired = 0u64;
            let mut pages_compressed = 0u64;

            for line in content.lines() {
                if line.contains("Pages free:") {
                    pages_free = Self::extract_pages(line);
                } else if line.contains("Pages active:") {
                    pages_active = Self::extract_pages(line);
                } else if line.contains("Pages inactive:") {
                    pages_inactive = Self::extract_pages(line);
                } else if line.contains("Pages wired down:") {
                    pages_wired = Self::extract_pages(line);
                } else if line.contains("Pages stored in compressor:") {
                    pages_compressed = Self::extract_pages(line);
                }
            }

            let page_size = 4096u64; // 4KB pages on macOS
            let total = self.system_info.total_memory;
            let used = (pages_active + pages_inactive + pages_wired + pages_compressed) * page_size;
            let available = pages_free * page_size;
            let usage_percent = if total > 0 {
                (used as f64 / total as f64) * 100.0
            } else {
                0.0
            };

            Ok((usage_percent, used, available))
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            Ok((0.0, 0, 0))
        }
    }

    #[cfg(target_os = "macos")]
    fn extract_pages(line: &str) -> u64 {
        line.split_whitespace()
            .find(|s| s.chars().all(|c| c.is_ascii_digit()))
            .and_then(|s| s.parse().ok())
            .unwrap_or(0)
    }

    fn get_load_average(&self) -> Result<(f64, f64, f64)> {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            let content = fs::read_to_string("/proc/loadavg").or_else(|_| -> Result<String> {
                // macOS fallback using uptime command
                let output = Command::new("uptime").output()?;
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            })?;

            let parts: Vec<&str> = content.split_whitespace().collect();
            if parts.len() >= 3 {
                let load_1min = parts[0].parse::<f64>().unwrap_or(0.0);
                let load_5min = parts[1].parse::<f64>().unwrap_or(0.0);
                let load_quarter_hour = parts[2].parse::<f64>().unwrap_or(0.0);
                return Ok((load_1min, load_5min, load_quarter_hour));
            }

            // Alternative parsing for uptime output - safely
            if let Some(load_start) = content.find("load average") {
                if load_start < content.len() {
                    let load_str = &content[load_start..];
                    if let Some(colon_pos) = load_str.find(':') {
                        if colon_pos + 1 < load_str.len() {
                            let numbers = &load_str[colon_pos + 1..];
                            let nums: Vec<&str> = numbers.split(',').collect();
                            if nums.len() >= 3 {
                                let load_1min = nums[0].trim().parse::<f64>().unwrap_or(0.0);
                                let load_5min = nums[1].trim().parse::<f64>().unwrap_or(0.0);
                                let load_quarter_hour =
                                    nums[2].trim().parse::<f64>().unwrap_or(0.0);
                                return Ok((load_1min, load_5min, load_quarter_hour));
                            }
                        }
                    }
                }
            }
        }

        Ok((0.0, 0.0, 0.0))
    }

    fn get_disk_usage(&self) -> Result<HashMap<String, DiskUsage>> {
        let mut disk_usage = HashMap::new();

        let output = Command::new("df").arg("-h").output()?;

        let content = String::from_utf8_lossy(&output.stdout);
        for line in content.lines().skip(1) {
            // Skip header
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 6 {
                let filesystem = parts[0].to_string();
                let size_str = parts[1];
                let used_str = parts[2];
                let avail_str = parts[3];
                let use_percent_str = parts[4];
                let mount_point = parts[5].to_string();

                // Parse sizes (handle K, M, G, T suffixes)
                let total = Self::parse_size(size_str);
                let used = Self::parse_size(used_str);
                let available = Self::parse_size(avail_str);
                let usage_percent = use_percent_str
                    .trim_end_matches('%')
                    .parse::<f64>()
                    .unwrap_or(0.0);

                disk_usage.insert(
                    mount_point,
                    DiskUsage {
                        total,
                        used,
                        available,
                        usage_percent,
                        filesystem,
                    },
                );
            }
        }

        Ok(disk_usage)
    }

    fn parse_size(size_str: &str) -> u64 {
        let size_str = size_str.trim();
        if size_str.is_empty() || size_str == "-" {
            return 0;
        }

        let (number_part, suffix) = if size_str.ends_with(char::is_alphabetic) && size_str.len() > 1
        {
            let len = size_str.len();
            (&size_str[..len - 1], &size_str[len - 1..])
        } else {
            (size_str, "")
        };

        let number: f64 = number_part.parse().unwrap_or(0.0);

        match suffix.to_uppercase().as_str() {
            "K" => (number * 1024.0) as u64,
            "M" => (number * 1024.0 * 1024.0) as u64,
            "G" => (number * 1024.0 * 1024.0 * 1024.0) as u64,
            "T" => (number * 1024.0 * 1024.0 * 1024.0 * 1024.0) as u64,
            _ => number as u64,
        }
    }

    fn get_top_processes(&self) -> Result<Vec<ProcessInfo>> {
        // Try Linux-style sorting first, fallback to basic ps on macOS
        let output = Command::new("ps")
            .args(["aux", "--sort=-pcpu"])
            .output()
            .or_else(|_| {
                // Fallback for macOS - use basic ps and sort in code
                Command::new("ps").args(["aux"]).output()
            })?;

        let content = String::from_utf8_lossy(&output.stdout);
        let mut processes = Vec::new();

        for line in content.lines().skip(1) {
            // Skip header
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 11 {
                let user = parts[0].to_string();
                let pid = parts[1].parse::<u32>().unwrap_or(0);
                let cpu_percent = parts[2].parse::<f64>().unwrap_or(0.0);
                let memory_percent = parts[3].parse::<f64>().unwrap_or(0.0);
                let memory_vms = parts[4].parse::<u64>().unwrap_or(0) * 1024; // KB to bytes
                let memory_rss = parts[5].parse::<u64>().unwrap_or(0) * 1024; // KB to bytes
                let state = parts.get(7).unwrap_or(&"?").to_string();

                // Command is everything from column 10 onwards (safer indexing)
                let command = if parts.len() > 10 {
                    parts[10..].join(" ")
                } else {
                    "unknown".to_string()
                };
                let name = if parts.len() > 10 {
                    parts[10]
                        .split('/')
                        .next_back()
                        .unwrap_or("unknown")
                        .to_string()
                } else {
                    "unknown".to_string()
                };

                processes.push(ProcessInfo {
                    pid,
                    name,
                    cpu_percent,
                    memory_percent,
                    memory_rss,
                    memory_vms,
                    command,
                    user,
                    state,
                });
            }
        }

        // Sort by CPU percentage (descending) and take top 5
        processes.sort_by(|a, b| {
            b.cpu_percent
                .partial_cmp(&a.cpu_percent)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        processes.truncate(5);

        Ok(processes)
    }

    pub fn format_bytes(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        // Safe array access with bounds checking
        let unit = UNITS.get(unit_index).unwrap_or(&"B");

        if size >= 100.0 {
            format!("{size:.0} {unit}")
        } else if size >= 10.0 {
            format!("{size:.1} {unit}")
        } else {
            format!("{size:.2} {unit}")
        }
    }

    pub fn format_uptime(duration: Duration) -> String {
        let total_secs = duration.as_secs();
        let days = total_secs / 86400;
        let hours = (total_secs % 86400) / 3600;
        let minutes = (total_secs % 3600) / 60;

        if days > 0 {
            format!("{days}d {hours}h {minutes}m")
        } else if hours > 0 {
            format!("{hours}h {minutes}m")
        } else {
            format!("{minutes}m")
        }
    }
}
