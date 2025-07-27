use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafeSystemInfo {
    pub hostname: String,
    pub os_name: String,
    pub os_version: String,
    pub kernel_version: String,
    pub architecture: String,
    pub cpu_model: String,
    pub cpu_cores: u32,
    pub cpu_threads: u32,
    pub total_memory: u64,
    pub boot_time: SystemTime,
    pub uptime: Duration,
}

#[derive(Debug, Clone)]
pub struct SafeSystemStats {
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub memory_used: u64,
    pub memory_available: u64,
    pub load_average: (f64, f64, f64),
    pub disk_usage: HashMap<String, SafeDiskUsage>,
    pub top_processes: Vec<SafeProcessInfo>,
    pub timestamp: SystemTime,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SafeDiskUsage {
    pub total: u64,
    pub used: u64,
    pub available: u64,
    pub usage_percent: f64,
    pub filesystem: String,
}

#[derive(Debug, Clone)]
pub struct SafeProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f64,
    pub memory_percent: f64,
    pub memory_rss: u64,
    pub memory_vms: u64,
    pub command: String,
    pub user: String,
    pub state: String,
}

pub struct SafeSystemMonitor {
    last_cpu_stats: Option<SafeCpuStats>,
    last_update: SystemTime,
    system_info: Option<SafeSystemInfo>,
    errors: Vec<String>,
}

#[derive(Debug, Clone)]
struct SafeCpuStats {
    user: u64,
    nice: u64,
    system: u64,
    idle: u64,
    iowait: u64,
    irq: u64,
    softirq: u64,
    steal: u64,
}

impl SafeSystemMonitor {
    pub fn new() -> Self {
        let mut monitor = Self {
            last_cpu_stats: None,
            last_update: SystemTime::now(),
            system_info: None,
            errors: Vec::new(),
        };

        // Try to collect system info, but don't fail if it crashes
        match catch_unwind(AssertUnwindSafe(Self::collect_system_info_safe)) {
            Ok(Ok(info)) => monitor.system_info = Some(info),
            Ok(Err(e)) => monitor.errors.push(format!("System info error: {e}")),
            Err(_) => monitor
                .errors
                .push("System info collection panicked".to_string()),
        }

        monitor
    }

    pub fn get_system_info(&self) -> Option<&SafeSystemInfo> {
        self.system_info.as_ref()
    }

    pub fn get_current_stats(&mut self) -> SafeSystemStats {
        let now = SystemTime::now();
        let mut errors = Vec::new();

        // CPU usage with panic protection
        let cpu_usage = match catch_unwind(AssertUnwindSafe(|| self.calculate_cpu_usage_safe())) {
            Ok(Ok(usage)) => usage,
            Ok(Err(e)) => {
                errors.push(format!("CPU usage error: {e}"));
                0.0
            }
            Err(_) => {
                errors.push("CPU usage calculation panicked".to_string());
                0.0
            }
        };

        // Memory stats with panic protection
        let (memory_usage_percent, memory_used, memory_available) =
            match catch_unwind(AssertUnwindSafe(|| self.get_memory_stats_safe())) {
                Ok(Ok(stats)) => stats,
                Ok(Err(e)) => {
                    errors.push(format!("Memory stats error: {e}"));
                    (0.0, 0, 0)
                }
                Err(_) => {
                    errors.push("Memory stats calculation panicked".to_string());
                    (0.0, 0, 0)
                }
            };

        // Load average with panic protection
        let load_average = match catch_unwind(AssertUnwindSafe(|| self.get_load_average_safe())) {
            Ok(Ok(load)) => load,
            Ok(Err(e)) => {
                errors.push(format!("Load average error: {e}"));
                (0.0, 0.0, 0.0)
            }
            Err(_) => {
                errors.push("Load average calculation panicked".to_string());
                (0.0, 0.0, 0.0)
            }
        };

        // Disk usage with panic protection
        let disk_usage = match catch_unwind(AssertUnwindSafe(|| self.get_disk_usage_safe())) {
            Ok(Ok(usage)) => usage,
            Ok(Err(e)) => {
                errors.push(format!("Disk usage error: {e}"));
                HashMap::new()
            }
            Err(_) => {
                errors.push("Disk usage calculation panicked".to_string());
                HashMap::new()
            }
        };

        // Top processes with panic protection
        let top_processes = match catch_unwind(AssertUnwindSafe(|| self.get_top_processes_safe())) {
            Ok(Ok(processes)) => processes,
            Ok(Err(e)) => {
                errors.push(format!("Top processes error: {e}"));
                Vec::new()
            }
            Err(_) => {
                errors.push("Top processes calculation panicked".to_string());
                Vec::new()
            }
        };

        self.last_update = now;

        SafeSystemStats {
            cpu_usage_percent: cpu_usage,
            memory_usage_percent,
            memory_used,
            memory_available,
            load_average,
            disk_usage,
            top_processes,
            timestamp: now,
            errors,
        }
    }

    fn collect_system_info_safe() -> Result<SafeSystemInfo> {
        let hostname = Self::safe_command("hostname", &[]).unwrap_or_else(|| "unknown".to_string());

        let (os_name, os_version) = Self::get_os_info_safe();

        let kernel_version =
            Self::safe_command("uname", &["-r"]).unwrap_or_else(|| "unknown".to_string());

        let architecture =
            Self::safe_command("uname", &["-m"]).unwrap_or_else(|| "unknown".to_string());

        let (cpu_model, cpu_cores, cpu_threads) = Self::get_cpu_info_safe();
        let total_memory = Self::get_total_memory_safe();
        let boot_time = Self::get_boot_time_safe();
        let uptime = SystemTime::now()
            .duration_since(boot_time)
            .unwrap_or_default();

        Ok(SafeSystemInfo {
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

    fn safe_command(cmd: &str, args: &[&str]) -> Option<String> {
        match Command::new(cmd).args(args).output() {
            Ok(output) => {
                let result = String::from_utf8_lossy(&output.stdout);
                Some(result.trim().to_string())
            }
            Err(_) => None,
        }
    }

    fn get_os_info_safe() -> (String, String) {
        #[cfg(target_os = "macos")]
        {
            let name = Self::safe_command("sw_vers", &["-productName"])
                .unwrap_or_else(|| "macOS".to_string());
            let version = Self::safe_command("sw_vers", &["-productVersion"])
                .unwrap_or_else(|| "Unknown".to_string());
            (name, version)
        }

        #[cfg(target_os = "linux")]
        {
            use std::fs;
            if let Ok(content) = fs::read_to_string("/etc/os-release") {
                let mut name = "Linux".to_string();
                let mut version = "Unknown".to_string();

                for line in content.lines() {
                    if let Some(value) = line.strip_prefix("PRETTY_NAME=") {
                        name = value.trim_matches('"').to_string();
                    } else if let Some(value) = line.strip_prefix("VERSION=") {
                        version = value.trim_matches('"').to_string();
                    }
                }
                (name, version)
            } else {
                ("Linux".to_string(), "Unknown".to_string())
            }
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            ("Unknown OS".to_string(), "Unknown".to_string())
        }
    }

    fn get_cpu_info_safe() -> (String, u32, u32) {
        #[cfg(target_os = "macos")]
        {
            let model = Self::safe_command("sysctl", &["-n", "machdep.cpu.brand_string"])
                .unwrap_or_else(|| "Unknown CPU".to_string());
            let cores = Self::safe_command("sysctl", &["-n", "hw.physicalcpu"])
                .and_then(|s| s.parse().ok())
                .unwrap_or(1);
            let threads = Self::safe_command("sysctl", &["-n", "hw.logicalcpu"])
                .and_then(|s| s.parse().ok())
                .unwrap_or(1);
            (model, cores, threads)
        }

        #[cfg(target_os = "linux")]
        {
            use std::fs;
            let mut model = "Unknown CPU".to_string();
            let mut cores = 1u32;
            let mut threads = 1u32;

            if let Ok(content) = fs::read_to_string("/proc/cpuinfo") {
                let mut cpu_count = 0;
                let mut core_ids = std::collections::HashSet::new();

                for line in content.lines() {
                    if let Some(name) = line
                        .strip_prefix("model name")
                        .and_then(|l| l.strip_prefix(":"))
                    {
                        model = name.trim().to_string();
                    } else if line.starts_with("processor") {
                        cpu_count += 1;
                    } else if let Some(id_str) = line
                        .strip_prefix("core id")
                        .and_then(|l| l.strip_prefix(":"))
                    {
                        if let Ok(core_id) = id_str.trim().parse::<u32>() {
                            core_ids.insert(core_id);
                        }
                    }
                }

                threads = cpu_count;
                cores = core_ids.len() as u32;
                if cores == 0 {
                    cores = threads;
                }
            }

            (model, cores, threads)
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            ("Unknown CPU".to_string(), 1, 1)
        }
    }

    fn get_total_memory_safe() -> u64 {
        #[cfg(target_os = "macos")]
        {
            Self::safe_command("sysctl", &["-n", "hw.memsize"])
                .and_then(|s| s.parse().ok())
                .unwrap_or(0)
        }

        #[cfg(target_os = "linux")]
        {
            use std::fs;
            if let Ok(content) = fs::read_to_string("/proc/meminfo") {
                for line in content.lines() {
                    if let Some(value_str) = line.strip_prefix("MemTotal:") {
                        let parts: Vec<&str> = value_str.split_whitespace().collect();
                        if let Some(kb_str) = parts.first() {
                            if let Ok(kb) = kb_str.parse::<u64>() {
                                return kb * 1024; // Convert KB to bytes
                            }
                        }
                    }
                }
            }
            0
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            0
        }
    }

    fn get_boot_time_safe() -> SystemTime {
        #[cfg(target_os = "macos")]
        {
            if let Some(boot_str) = Self::safe_command("sysctl", &["-n", "kern.boottime"]) {
                // Parse format like "{ sec = 1234567890, usec = 123456 }"
                if let Some(start) = boot_str.find("sec = ") {
                    let after_sec = &boot_str[start + 6..];
                    if let Some(end) = after_sec.find(',') {
                        if let Ok(secs) = after_sec[..end].parse::<u64>() {
                            return UNIX_EPOCH + Duration::from_secs(secs);
                        }
                    }
                }
            }
            SystemTime::now() - Duration::from_secs(3600) // 1 hour ago fallback
        }

        #[cfg(target_os = "linux")]
        {
            use std::fs;
            if let Ok(content) = fs::read_to_string("/proc/stat") {
                for line in content.lines() {
                    if let Some(time_str) = line.strip_prefix("btime ") {
                        if let Ok(secs) = time_str.trim().parse::<u64>() {
                            return UNIX_EPOCH + Duration::from_secs(secs);
                        }
                    }
                }
            }
            SystemTime::now() - Duration::from_secs(3600) // 1 hour ago fallback
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            SystemTime::now() - Duration::from_secs(3600) // 1 hour ago fallback
        }
    }

    fn calculate_cpu_usage_safe(&mut self) -> Result<f64> {
        let current_stats = self.read_cpu_stats_safe()?;

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

            let total_diff = total_current.saturating_sub(total_last);
            let idle_diff = current_stats.idle.saturating_sub(last_stats.idle);

            if total_diff > 0 {
                let usage =
                    ((total_diff.saturating_sub(idle_diff)) as f64 / total_diff as f64) * 100.0;
                self.last_cpu_stats = Some(current_stats);
                return Ok(usage.clamp(0.0, 100.0));
            }
        }

        self.last_cpu_stats = Some(current_stats);
        Ok(0.0)
    }

    fn read_cpu_stats_safe(&self) -> Result<SafeCpuStats> {
        #[cfg(target_os = "linux")]
        {
            use std::fs;
            let content = fs::read_to_string("/proc/stat")?;
            if let Some(line) = content.lines().next() {
                if line.starts_with("cpu ") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 8 {
                        return Ok(SafeCpuStats {
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

        // Fallback for macOS and other systems
        Ok(SafeCpuStats {
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

    fn get_memory_stats_safe(&self) -> Result<(f64, u64, u64)> {
        #[cfg(target_os = "linux")]
        {
            use std::fs;
            let content = fs::read_to_string("/proc/meminfo")?;
            let mut total = 0u64;
            let mut available = 0u64;

            for line in content.lines() {
                if let Some(value_str) = line.strip_prefix("MemTotal:") {
                    let parts: Vec<&str> = value_str.split_whitespace().collect();
                    if let Some(kb_str) = parts.first() {
                        total = kb_str.parse::<u64>().unwrap_or(0) * 1024;
                    }
                } else if let Some(value_str) = line.strip_prefix("MemAvailable:") {
                    let parts: Vec<&str> = value_str.split_whitespace().collect();
                    if let Some(kb_str) = parts.first() {
                        available = kb_str.parse::<u64>().unwrap_or(0) * 1024;
                    }
                }
            }

            let used = total.saturating_sub(available);
            let usage_percent = if total > 0 {
                (used as f64 / total as f64) * 100.0
            } else {
                0.0
            };

            Ok((usage_percent, used, available))
        }

        #[cfg(target_os = "macos")]
        {
            if let Ok(output) = Command::new("vm_stat").output() {
                let content = String::from_utf8_lossy(&output.stdout);

                let mut pages_free = 0u64;
                let mut pages_active = 0u64;
                let mut pages_inactive = 0u64;
                let mut pages_wired = 0u64;
                let mut pages_compressed = 0u64;

                for line in content.lines() {
                    if line.contains("Pages free:") {
                        pages_free = Self::extract_pages_safe(line);
                    } else if line.contains("Pages active:") {
                        pages_active = Self::extract_pages_safe(line);
                    } else if line.contains("Pages inactive:") {
                        pages_inactive = Self::extract_pages_safe(line);
                    } else if line.contains("Pages wired down:") {
                        pages_wired = Self::extract_pages_safe(line);
                    } else if line.contains("Pages stored in compressor:") {
                        pages_compressed = Self::extract_pages_safe(line);
                    }
                }

                let page_size = 4096u64;
                let total = if let Some(info) = &self.system_info {
                    info.total_memory
                } else {
                    0
                };
                let used =
                    (pages_active + pages_inactive + pages_wired + pages_compressed) * page_size;
                let available = pages_free * page_size;
                let usage_percent = if total > 0 {
                    (used as f64 / total as f64) * 100.0
                } else {
                    0.0
                };

                Ok((usage_percent, used, available))
            } else {
                Ok((0.0, 0, 0))
            }
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            Ok((0.0, 0, 0))
        }
    }

    #[cfg(target_os = "macos")]
    fn extract_pages_safe(line: &str) -> u64 {
        line.split_whitespace()
            .filter_map(|s| {
                // Only parse if all characters are digits
                if s.chars().all(|c| c.is_ascii_digit()) {
                    s.parse().ok()
                } else {
                    None
                }
            })
            .next()
            .unwrap_or(0)
    }

    fn get_load_average_safe(&self) -> Result<(f64, f64, f64)> {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            use std::fs;
            // Try /proc/loadavg first (Linux)
            if let Ok(content) = fs::read_to_string("/proc/loadavg") {
                let parts: Vec<&str> = content.split_whitespace().collect();
                if parts.len() >= 3 {
                    let load_1min = parts[0].parse::<f64>().unwrap_or(0.0);
                    let load_5min = parts[1].parse::<f64>().unwrap_or(0.0);
                    let load_quarter_hour = parts[2].parse::<f64>().unwrap_or(0.0);
                    return Ok((load_1min, load_5min, load_quarter_hour));
                }
            }

            // Fallback to uptime command (macOS)
            if let Some(uptime_output) = Self::safe_command("uptime", &[]) {
                if let Some(load_start) = uptime_output.find("load average") {
                    let load_section = &uptime_output[load_start..];
                    if let Some(colon_pos) = load_section.find(':') {
                        let numbers_section = &load_section[colon_pos + 1..];
                        let nums: Vec<&str> = numbers_section.split(',').collect();
                        if nums.len() >= 3 {
                            let load_1min = nums[0].trim().parse::<f64>().unwrap_or(0.0);
                            let load_5min = nums[1].trim().parse::<f64>().unwrap_or(0.0);
                            let load_quarter_hour = nums[2].trim().parse::<f64>().unwrap_or(0.0);
                            return Ok((load_1min, load_5min, load_quarter_hour));
                        }
                    }
                }
            }
        }

        Ok((0.0, 0.0, 0.0))
    }

    fn get_disk_usage_safe(&self) -> Result<HashMap<String, SafeDiskUsage>> {
        let mut disk_usage = HashMap::new();

        if let Ok(output) = Command::new("df").arg("-h").output() {
            let content = String::from_utf8_lossy(&output.stdout);
            for line in content.lines().skip(1) {
                // Skip header
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 6 {
                    let filesystem = parts[0].to_string();
                    let mount_point = parts[5].to_string();

                    let total = Self::parse_size_safe(parts[1]);
                    let used = Self::parse_size_safe(parts[2]);
                    let available = Self::parse_size_safe(parts[3]);
                    let usage_percent =
                        parts[4].trim_end_matches('%').parse::<f64>().unwrap_or(0.0);

                    disk_usage.insert(
                        mount_point,
                        SafeDiskUsage {
                            total,
                            used,
                            available,
                            usage_percent,
                            filesystem,
                        },
                    );
                }
            }
        }

        Ok(disk_usage)
    }

    fn parse_size_safe(size_str: &str) -> u64 {
        let size_str = size_str.trim();
        if size_str.is_empty() || size_str == "-" {
            return 0;
        }

        // Find where numbers end and suffix begins
        let mut number_end = size_str.len();
        for (i, c) in size_str.char_indices().rev() {
            if c.is_ascii_digit() || c == '.' {
                number_end = i + 1;
                break;
            }
        }

        let number_part = &size_str[..number_end];
        let suffix = if number_end < size_str.len() {
            &size_str[number_end..]
        } else {
            ""
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

    fn get_top_processes_safe(&self) -> Result<Vec<SafeProcessInfo>> {
        let output = Command::new("ps")
            .args(["aux", "--sort=-pcpu"])
            .output()
            .or_else(|_| Command::new("ps").args(["aux"]).output())?;

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
                let memory_vms = parts[4].parse::<u64>().unwrap_or(0) * 1024;
                let memory_rss = parts[5].parse::<u64>().unwrap_or(0) * 1024;
                let state = parts.get(7).unwrap_or(&"?").to_string();

                // Safely build command string
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

                processes.push(SafeProcessInfo {
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

        // Sort by CPU percentage and take top 5
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

impl Default for SafeSystemMonitor {
    fn default() -> Self {
        Self::new()
    }
}
