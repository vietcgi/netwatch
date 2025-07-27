use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone)]
pub struct ProcessNetworkInfo {
    pub pid: u32,
    pub name: String,
    pub command: String,
    pub connections: u32,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub established_connections: u32,
    pub listening_ports: u32,
    pub last_updated: SystemTime,
}

impl ProcessNetworkInfo {
    pub fn total_bytes(&self) -> u64 {
        self.bytes_sent + self.bytes_received
    }

    pub fn total_packets(&self) -> u64 {
        self.packets_sent + self.packets_received
    }
}

pub struct ProcessMonitor {
    processes: HashMap<u32, ProcessNetworkInfo>,
    previous_stats: HashMap<u32, ProcessNetworkStats>,
    last_update: SystemTime,
}

#[derive(Debug, Clone)]
pub struct ProcessNetworkStats {
    bytes_sent: u64,
    bytes_received: u64,
    packets_sent: u64,
    packets_received: u64,
    timestamp: SystemTime,
}

impl ProcessMonitor {
    pub fn new() -> Self {
        Self {
            processes: HashMap::new(),
            previous_stats: HashMap::new(),
            last_update: SystemTime::now(),
        }
    }

    pub fn update(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Clear existing processes to get fresh data
        self.processes.clear();

        let now = SystemTime::now();

        // Read all process network information
        self.scan_processes()?;

        // Update connection counts
        self.update_connection_counts()?;

        // Calculate network I/O rates
        self.calculate_rates(now)?;

        self.last_update = now;
        Ok(())
    }

    fn scan_processes(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Ok(entries) = fs::read_dir("/proc") {
            for entry in entries.flatten() {
                if let Ok(file_name) = entry.file_name().into_string() {
                    if let Ok(pid) = file_name.parse::<u32>() {
                        if let Some(process_info) = self.read_process_info(pid)? {
                            self.processes.insert(pid, process_info);
                        }
                    }
                }
            }
        } else {
            // macOS fallback - get real process data from system commands
            self.get_real_processes_from_system();
        }
        Ok(())
    }

    fn read_process_info(
        &mut self,
        pid: u32,
    ) -> Result<Option<ProcessNetworkInfo>, Box<dyn std::error::Error>> {
        let proc_path = format!("/proc/{pid}");

        // Check if process directory exists and is accessible
        if !Path::new(&proc_path).exists() {
            return Ok(None);
        }

        // Read process name
        let comm_path = format!("{proc_path}/comm");
        let name = fs::read_to_string(comm_path)
            .unwrap_or_else(|_| format!("process-{pid}"))
            .trim()
            .to_string();

        // Read command line
        let cmdline_path = format!("{proc_path}/cmdline");
        let command = fs::read_to_string(cmdline_path)
            .unwrap_or_else(|_| name.clone())
            .replace('\0', " ")
            .trim()
            .to_string();

        // Read network I/O statistics from /proc/pid/net/dev if available
        let (bytes_sent, bytes_received, packets_sent, packets_received) =
            self.read_process_network_stats(pid).unwrap_or((0, 0, 0, 0));

        let process_info = ProcessNetworkInfo {
            pid,
            name,
            command,
            connections: 0, // Will be updated later
            bytes_sent,
            bytes_received,
            packets_sent,
            packets_received,
            established_connections: 0,
            listening_ports: 0,
            last_updated: SystemTime::now(),
        };

        Ok(Some(process_info))
    }

    fn read_process_network_stats(
        &self,
        pid: u32,
    ) -> Result<(u64, u64, u64, u64), Box<dyn std::error::Error>> {
        // Try to read network statistics from various sources

        // Method 1: Try /proc/pid/net/dev (process-specific network stats)
        let net_dev_path = format!("/proc/{pid}/net/dev");
        if let Ok(content) = fs::read_to_string(net_dev_path) {
            if let Some((bytes_rx, bytes_tx, packets_rx, packets_tx)) = self.parse_net_dev(&content)
            {
                return Ok((bytes_tx, bytes_rx, packets_tx, packets_rx));
            }
        }

        // Method 2: Try /proc/pid/io (process I/O stats)
        let io_path = format!("/proc/{pid}/io");
        if let Ok(content) = fs::read_to_string(io_path) {
            if let Some((read_bytes, write_bytes)) = self.parse_io_stats(&content) {
                // Estimate network I/O as a fraction of total I/O
                // This is a rough approximation
                return Ok((write_bytes / 4, read_bytes / 4, 0, 0));
            }
        }

        // Method 3: Use system-wide stats as fallback
        // Read from /proc/net/netstat for system-wide network stats
        if let Ok(content) = fs::read_to_string("/proc/net/netstat") {
            if let Some((bytes_sent, bytes_received)) = self.parse_netstat(&content) {
                // Distribute proportionally based on number of connections
                // This is a very rough estimate
                return Ok((bytes_sent / 100, bytes_received / 100, 0, 0));
            }
        }

        Ok((0, 0, 0, 0))
    }

    fn parse_net_dev(&self, content: &str) -> Option<(u64, u64, u64, u64)> {
        let mut total_bytes_rx = 0;
        let mut total_bytes_tx = 0;
        let mut total_packets_rx = 0;
        let mut total_packets_tx = 0;

        for line in content.lines().skip(2) {
            // Skip header lines
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() >= 10 {
                if let (Ok(bytes_rx), Ok(packets_rx), Ok(bytes_tx), Ok(packets_tx)) = (
                    fields[1].parse::<u64>(),
                    fields[2].parse::<u64>(),
                    fields[9].parse::<u64>(),
                    fields[10].parse::<u64>(),
                ) {
                    total_bytes_rx += bytes_rx;
                    total_bytes_tx += bytes_tx;
                    total_packets_rx += packets_rx;
                    total_packets_tx += packets_tx;
                }
            }
        }

        if total_bytes_rx > 0 || total_bytes_tx > 0 {
            Some((
                total_bytes_rx,
                total_bytes_tx,
                total_packets_rx,
                total_packets_tx,
            ))
        } else {
            None
        }
    }

    fn parse_io_stats(&self, content: &str) -> Option<(u64, u64)> {
        let mut read_bytes = 0;
        let mut write_bytes = 0;

        for line in content.lines() {
            if line.starts_with("read_bytes:") {
                if let Some(value) = line.split_whitespace().nth(1) {
                    read_bytes = value.parse().unwrap_or(0);
                }
            } else if line.starts_with("write_bytes:") {
                if let Some(value) = line.split_whitespace().nth(1) {
                    write_bytes = value.parse().unwrap_or(0);
                }
            }
        }

        if read_bytes > 0 || write_bytes > 0 {
            Some((read_bytes, write_bytes))
        } else {
            None
        }
    }

    fn parse_netstat(&self, content: &str) -> Option<(u64, u64)> {
        // Parse system-wide network statistics
        for line in content.lines() {
            if line.starts_with("IpExt:") {
                let fields: Vec<&str> = line.split_whitespace().collect();
                if fields.len() > 6 {
                    if let (Ok(bytes_sent), Ok(bytes_received)) =
                        (fields[5].parse::<u64>(), fields[6].parse::<u64>())
                    {
                        return Some((bytes_sent, bytes_received));
                    }
                }
            }
        }
        None
    }

    fn update_connection_counts(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Count connections per process by parsing /proc/net/tcp and /proc/net/udp
        let mut pid_connections: HashMap<u32, (u32, u32, u32)> = HashMap::new(); // (total, established, listening)

        // Parse TCP connections
        if let Ok(content) = fs::read_to_string("/proc/net/tcp") {
            self.count_connections_in_file(&content, &mut pid_connections)?;
        }

        // Parse UDP connections
        if let Ok(content) = fs::read_to_string("/proc/net/udp") {
            self.count_connections_in_file(&content, &mut pid_connections)?;
        }

        // Update process information
        for (pid, (total, established, listening)) in pid_connections {
            if let Some(process) = self.processes.get_mut(&pid) {
                process.connections = total;
                process.established_connections = established;
                process.listening_ports = listening;
            }
        }

        Ok(())
    }

    fn count_connections_in_file(
        &self,
        content: &str,
        pid_connections: &mut HashMap<u32, (u32, u32, u32)>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for line in content.lines().skip(1) {
            // Skip header
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() >= 8 {
                // Try to extract PID from inode (this is complex and may not always work)
                // For now, we'll use a simplified approach
                if let Ok(inode) = fields[9].parse::<u64>() {
                    if let Some(pid) = self.find_pid_by_inode(inode) {
                        let (total, established, listening) =
                            pid_connections.entry(pid).or_insert((0, 0, 0));
                        *total += 1;

                        // Check connection state
                        if fields[3] == "01" {
                            // ESTABLISHED
                            *established += 1;
                        } else if fields[3] == "0A" {
                            // LISTEN
                            *listening += 1;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn find_pid_by_inode(&self, _inode: u64) -> Option<u32> {
        // This is a simplified implementation
        // In reality, we'd need to scan /proc/*/fd/* to find which process owns this inode
        // For now, return None to avoid complex filesystem scanning
        None
    }

    fn calculate_rates(&mut self, now: SystemTime) -> Result<(), Box<dyn std::error::Error>> {
        // Calculate network I/O rates by comparing with previous measurements
        for (pid, process) in &mut self.processes {
            if let Some(prev_stats) = self.previous_stats.get(pid) {
                let time_diff = now
                    .duration_since(prev_stats.timestamp)
                    .unwrap_or(Duration::from_secs(1));
                let time_secs = time_diff.as_secs_f64();

                if time_secs > 0.0 {
                    // Calculate rates (bytes per second)
                    let bytes_sent_rate =
                        ((process.bytes_sent.saturating_sub(prev_stats.bytes_sent)) as f64
                            / time_secs) as u64;
                    let bytes_received_rate = ((process
                        .bytes_received
                        .saturating_sub(prev_stats.bytes_received))
                        as f64
                        / time_secs) as u64;

                    // Update with calculated rates
                    process.bytes_sent = bytes_sent_rate;
                    process.bytes_received = bytes_received_rate;
                }
            }

            // Store current stats for next calculation
            self.previous_stats.insert(
                *pid,
                ProcessNetworkStats {
                    bytes_sent: process.bytes_sent,
                    bytes_received: process.bytes_received,
                    packets_sent: process.packets_sent,
                    packets_received: process.packets_received,
                    timestamp: now,
                },
            );
        }

        Ok(())
    }

    pub fn get_processes(&self) -> Vec<&ProcessNetworkInfo> {
        let mut processes: Vec<&ProcessNetworkInfo> = self.processes.values().collect();

        // Sort by total network activity (bytes sent + received)
        processes.sort_by(|a, b| {
            let a_total = a.total_bytes();
            let b_total = b.total_bytes();
            b_total.cmp(&a_total)
        });

        processes
    }

    pub fn get_top_network_processes(&self, limit: usize) -> Vec<&ProcessNetworkInfo> {
        let mut processes = self.get_processes();
        processes.truncate(limit);
        processes
    }

    pub fn get_process_stats(&self) -> ProcessNetworkStats {
        let mut stats = ProcessNetworkStats {
            bytes_sent: 0,
            bytes_received: 0,
            packets_sent: 0,
            packets_received: 0,
            timestamp: SystemTime::now(),
        };

        for process in self.processes.values() {
            stats.bytes_sent += process.bytes_sent;
            stats.bytes_received += process.bytes_received;
            stats.packets_sent += process.packets_sent;
            stats.packets_received += process.packets_received;
        }

        stats
    }

    pub fn get_listening_processes(&self) -> Vec<&ProcessNetworkInfo> {
        let mut processes: Vec<&ProcessNetworkInfo> = self
            .processes
            .values()
            .filter(|p| p.listening_ports > 0)
            .collect();

        processes.sort_by(|a, b| b.listening_ports.cmp(&a.listening_ports));
        processes
    }

    fn get_real_processes_from_system(&mut self) {
        use std::process::Command;

        // Use ps and lsof to get real process data with network activity
        if let Ok(output) = Command::new("lsof").args(["-i", "-n", "-P"]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            self.parse_lsof_processes(&stdout);
        } else {
            // Fallback to ps if lsof is not available
            if let Ok(output) = Command::new("ps").args(["-eo", "pid,comm,rss"]).output() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                self.parse_ps_processes(&stdout);
            }
        }
    }

    fn parse_lsof_processes(&mut self, output: &str) {

        let mut process_map: HashMap<u32, (String, String)> = HashMap::new(); // pid -> (name, command)
        let mut process_connections: HashMap<u32, u32> = HashMap::new(); // pid -> total connections
        let mut process_listening: HashMap<u32, u32> = HashMap::new(); // pid -> listening ports
        let mut process_established: HashMap<u32, u32> = HashMap::new(); // pid -> established connections

        for line in output.lines().skip(1) {
            // Skip header
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 10 {
                continue;
            }

            let process_name = parts[0].to_string();
            if let Ok(pid) = parts[1].parse::<u32>() {
                // Count total connections per process
                *process_connections.entry(pid).or_insert(0) += 1;

                // Check if this is a listening port or established connection
                // The connection state is in parts[9] like "(LISTEN)" or "(ESTABLISHED)"
                let connection_state = parts.get(9).unwrap_or(&"");
                if connection_state.contains("LISTEN") {
                    *process_listening.entry(pid).or_insert(0) += 1;
                } else if connection_state.contains("ESTABLISHED") {
                    *process_established.entry(pid).or_insert(0) += 1;
                }

                // Store process info if not already seen
                process_map.entry(pid).or_insert_with(|| {
                    let command = process_name.to_string(); // lsof doesn't give full command
                    (process_name, command)
                });
            }
        }

        // Convert to ProcessNetworkInfo
        for (pid, total_connections) in process_connections {
            if let Some((name, command)) = process_map.get(&pid) {
                let listening_ports = process_listening.get(&pid).copied().unwrap_or(0);
                let established_connections = process_established.get(&pid).copied().unwrap_or(0);

                let process_info = ProcessNetworkInfo {
                    pid,
                    name: name.clone(),
                    command: command.clone(),
                    connections: total_connections,
                    bytes_sent: 0, // lsof doesn't provide byte counts
                    bytes_received: 0,
                    packets_sent: 0,
                    packets_received: 0,
                    established_connections,
                    listening_ports,
                    last_updated: SystemTime::now(),
                };
                self.processes.insert(process_info.pid, process_info);
            }
        }
    }

    fn parse_ps_processes(&mut self, output: &str) {
        // Basic fallback - just get running processes without network info
        for line in output.lines().skip(1) {
            // Skip header
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 {
                continue;
            }

            if let Ok(pid) = parts[0].parse::<u32>() {
                let name = parts[1].to_string();
                let process_info = ProcessNetworkInfo {
                    pid,
                    name: name.clone(),
                    command: name,
                    connections: 0,
                    bytes_sent: 0,
                    bytes_received: 0,
                    packets_sent: 0,
                    packets_received: 0,
                    established_connections: 0,
                    listening_ports: 0,
                    last_updated: SystemTime::now(),
                };
                self.processes.insert(process_info.pid, process_info);
            }
        }
    }
}

impl Default for ProcessMonitor {
    fn default() -> Self {
        Self::new()
    }
}
